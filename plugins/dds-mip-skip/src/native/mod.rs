mod plan;
use crate::ogre::{self, Shared};
use detour::{RawDetour, Threads, Transaction};
use rekenshi_plugin_api::HostV1;
use std::ffi::c_void;
use std::path::Path;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
type Opened = unsafe extern "C-unwind" fn(
    *mut c_void,
    *const c_void,
    *const c_void,
    *mut c_void,
    *mut Shared,
);
struct State {
    hook: RawDetour,
    target: usize,
    quality: usize,
}
static STATE: OnceLock<State> = OnceLock::new();
static ACTIVE: AtomicBool = AtomicBool::new(false);
static ENABLED: AtomicBool = AtomicBool::new(false);
static SKIPPED: AtomicU64 = AtomicU64::new(0);
pub(super) fn start(host: HostV1, game: &Path) -> Result<(), String> {
    ogre::initialize(game)?;
    let base = ogre::game_module(game)?;
    if STATE.get().is_none() {
        let target = base + 0x82ddc0;
        // SAFETY: Full EXE fingerprint pins DDSListener::resourceStreamOpened,
        // its five-argument member ABI and register-save prologue. C-unwind
        // preserves native Ogre exceptions through forwarding calls.
        let hook = unsafe { RawDetour::new(target as _, opened as *const ()) }
            .map_err(|e| e.to_string())?;
        STATE
            .set(State {
                hook,
                target,
                quality: base + 0x213352c,
            })
            .map_err(|_| "DDS adapter initialized twice")?;
    }
    let state = STATE.get().ok_or("DDS adapter")?;
    host.reserve_hook(std::ptr::NonNull::new(state.target as *mut c_void).ok_or("DDS target")?)
        .map_err(|e| format!("DDS hook conflict: {e}"))?;
    let mut tx = Transaction::new();
    tx.enable(&state.hook);
    // SAFETY: Outside DllMain, suspend threads during the patch. State is pinned.
    unsafe { tx.commit(Threads::All) }.map_err(|e| e.to_string())?;
    ACTIVE.store(true, Ordering::Release);
    host.write_log("dds-mip-skip: native 1.0.68 loading listener verified");
    Ok(())
}
pub(super) fn set_enabled(value: bool) {
    ENABLED.store(value, Ordering::Release);
}
pub(super) fn stop() -> Result<(), String> {
    ACTIVE.store(false, Ordering::Release);
    ENABLED.store(false, Ordering::Release);
    if let Some(state) = STATE.get() {
        let mut tx = Transaction::new();
        tx.disable(&state.hook);
        // SAFETY: Retain trampoline for in-flight native calls; suspend while unpatching.
        unsafe { tx.commit(Threads::All) }.map_err(|e| e.to_string())?;
    }
    Ok(())
}
pub fn skipped_bytes() -> u64 {
    SKIPPED.load(Ordering::Relaxed)
}
unsafe extern "C-unwind" fn opened(
    this: *mut c_void,
    name: *const c_void,
    group: *const c_void,
    resource: *mut c_void,
    stream: *mut Shared,
) {
    let Some(state) = STATE.get() else { return };
    // SAFETY: Arguments are live native objects for this callback; the retained
    // trampoline has the identical member ABI and allows native exception unwind.
    unsafe {
        let original: Opened = std::mem::transmute(state.hook.trampoline());
        if ACTIVE.load(Ordering::Acquire) && ENABLED.load(Ordering::Acquire) && !stream.is_null() {
            if let (Some(name), Some(group)) = (ogre::string(name), ogre::string(group)) {
                let dropped = plan::levels(&name, &group, *(state.quality as *const i32));
                if dropped > 0 && optimize(&mut *stream, dropped).is_ok() {
                    return;
                }
            }
        }
        original(this, name, group, resource, stream);
    }
}
unsafe fn optimize(stream: &mut Shared, dropped: u32) -> Result<(), String> {
    // SAFETY: Live listener stream. The read adapter restricts native vtables,
    // and every read is bounded by the DDS planner and native file size.
    unsafe {
        let result = (|| {
            let size = ogre::stream_size(stream).ok_or("empty DDS stream")?;
            let mut header = [0; 128];
            ogre::read(stream, 0, &mut header)?;
            let plan =
                plan::make(&header, size, dropped).ok_or("DDS format left with native loader")?;
            let mut bytes = plan.header;
            bytes.resize(128 + plan.pixels.len(), 0);
            ogre::read(stream, plan.pixels.start, &mut bytes[128..])?;
            let api = ogre::initialize(Path::new("."))?;
            let replacement = api.stream(bytes)?;
            api.replace(stream, replacement);
            SKIPPED.fetch_add((plan.pixels.start - 128) as u64, Ordering::Relaxed);
            Ok(())
        })();
        if result.is_err() {
            let mut empty = [];
            let _ = ogre::read(stream, 0, &mut empty);
        }
        result
    }
}
