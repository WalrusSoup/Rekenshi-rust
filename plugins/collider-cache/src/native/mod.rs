//! Native NXU collections remain independently owned by each game load.
use crate::ogre;
use detour::{RawDetour, Threads, Transaction};
use rekenshi_plugin_api::HostV1;
use rekenshi_plugin_sdk::adapters::{self, AdapterOperations};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::{
    collections::VecDeque,
    ffi::{CString, c_char, c_void},
    path::Path,
    sync::{Mutex, OnceLock},
};
type Load = unsafe extern "C-unwind" fn(*const c_void, i32) -> usize;
type CloneCollection =
    unsafe extern "C-unwind" fn(usize, *const c_char, *const [f32; 3], usize) -> usize;
type Release = unsafe extern "C" fn(usize);
type Sdk = unsafe extern "C" fn() -> usize;
struct State {
    hook: RawDetour,
    target: usize,
    clone: CloneCollection,
    release: Release,
    sdk: Sdk,
}
struct Collection {
    pointer: usize,
    release: Release,
}
impl Drop for Collection {
    fn drop(&mut self) {
        // SAFETY: Template has sole ownership; native NXU release destroys its
        // descriptors and uses the executable's matching allocator.
        unsafe {
            (self.release)(self.pointer);
        }
    }
}
struct Entry {
    name: Vec<u8>,
    kind: i32,
    template: Collection,
}
#[derive(Default)]
struct Cache {
    sdk: usize,
    entries: VecDeque<Entry>,
}
static STATE: OnceLock<State> = OnceLock::new();
static CACHE: Mutex<Cache> = Mutex::new(Cache {
    sdk: 0,
    entries: VecDeque::new(),
});
static ACTIVE: AtomicBool = AtomicBool::new(false);
static OPERATIONS: std::sync::RwLock<Option<AdapterOperations>> = std::sync::RwLock::new(None);
static ENABLED: AtomicBool = AtomicBool::new(false);
static HITS: AtomicU64 = AtomicU64::new(0);
const CAPACITY: usize = 128;
static POLICY: std::sync::RwLock<Option<rekenshi_plugin_sdk::policies::PolicyProvider>> =
    std::sync::RwLock::new(None);
fn enabled() -> bool {
    POLICY
        .read()
        .ok()
        .and_then(|p| p.as_ref().map(|p| p.values([0.; 4])[0] == 1.))
        .unwrap_or_else(|| ENABLED.load(Ordering::Acquire))
}
pub fn hits() -> u64 {
    HITS.load(Ordering::Relaxed)
}
pub(super) fn start(host: HostV1, game: &Path) -> Result<(), String> {
    let base = ogre::game_module(game)?;
    let loader = rekenshi_native::load_pinned(&game.join("PhysXLoader64.dll"))?;
    if STATE.get().is_none() {
        let target = base + 0x7e53f0;
        // SAFETY: Exact root EXE hash; RVAs matched from the installed KenshiLib
        // table and corroborated against load, scale-copy and release call sites.
        let state = unsafe {
            State {
                hook: RawDetour::new(target as _, load as *const ()).map_err(|e| e.to_string())?,
                target,
                clone: std::mem::transmute::<usize, CloneCollection>(base + 0x20b860),
                release: std::mem::transmute::<usize, Release>(base + 0x1d99f0),
                sdk: std::mem::transmute::<*const c_void, Sdk>(rekenshi_native::export(
                    loader,
                    c"NxGetPhysicsSDK",
                )?),
            }
        };
        STATE
            .set(state)
            .map_err(|_| "collider adapter initialized twice")?;
    }
    let state = STATE.get().ok_or("collider adapter")?;
    host.reserve_hook(
        std::ptr::NonNull::new(state.target as *mut c_void).ok_or("collider target")?,
    )
    .map_err(|e| format!("collider hook conflict: {e}"))?;
    let mut tx = Transaction::new();
    tx.enable(&state.hook);
    // SAFETY: Outside DllMain; suspend other threads during instruction patching.
    unsafe { tx.commit(Threads::All) }.map_err(|e| e.to_string())?;
    ACTIVE.store(true, Ordering::Release);
    *OPERATIONS.write().map_err(|_| "operation registry lock")? = Some(AdapterOperations::publish(host, "Existing adapter fingerprint and native signatures validated before hook enable; see implementation inventory and adapter source for evidence limits", &[(adapters::COLLIDER_LOAD,"rekenshi.collider-request-f32x4@1",16)]).map_err(|e| e.to_string())?);
    *POLICY.write().map_err(|_| "collider policy lock")? = rekenshi_plugin_sdk::policies::PolicyProvider::optional(host, rekenshi_plugin_sdk::policies::COLLIDER,
        "Kenshi 1.0.68 executable hash-gated by game_module; load 7e53f0 clone 20b860 release 1d99f0; independent NXU clones; SDK pre-destruction remains unverified").map_err(|e| e.to_string())?;
    host.write_log("collider-cache: native load/clone/release verified; 128 independent templates");
    Ok(())
}
pub(super) fn set_enabled(enabled: bool) {
    ENABLED.store(enabled, Ordering::Release);
    if !enabled {
        if let Ok(mut cache) = CACHE.lock() {
            cache.entries.clear();
            cache.sdk = 0;
        }
    }
}
pub(super) fn stop() -> Result<(), String> {
    if let Some(mut operations) = OPERATIONS
        .write()
        .map_err(|_| "operation registry lock")?
        .take()
    {
        operations.revoke().map_err(|e| e.to_string())?;
    }
    if let Some(mut policy) = POLICY.write().map_err(|_| "collider policy lock")?.take() {
        policy.revoke().map_err(|e| e.to_string())?;
    }
    ACTIVE.store(false, Ordering::Release);
    set_enabled(false);
    if let Some(state) = STATE.get() {
        let mut tx = Transaction::new();
        tx.disable(&state.hook);
        // SAFETY: Retain the trampoline for in-flight calls; suspend for unpatch.
        unsafe { tx.commit(Threads::All) }.map_err(|e| e.to_string())?;
    }
    Ok(())
}
unsafe extern "C-unwind" fn load(name: *const c_void, kind: i32) -> usize {
    let Some(state) = STATE.get() else {
        return 0;
    };
    // SAFETY: Native loader's live std::string and identical retained trampoline.
    unsafe {
        let original: Load = std::mem::transmute(state.hook.trampoline());
        let mut cache_enabled = enabled();
        let mut scale = [1.; 3];
        if let Some(filename) = ogre::string(name) {
            let metadata = [kind.to_le_bytes().as_slice(), &filename].concat();
            if let Some(values) = OPERATIONS
                .read()
                .ok()
                .and_then(|o| {
                    o.as_ref().and_then(|o| {
                        o.transform(
                            adapters::COLLIDER_LOAD,
                            &metadata,
                            &rekenshi_plugin_sdk::policies::encode([
                                cache_enabled as u8 as f32,
                                1.,
                                1.,
                                1.,
                            ])
                            .ok()?,
                        )
                    })
                })
                .and_then(|b| rekenshi_plugin_sdk::policies::decode(&b).ok())
                .filter(|v| v[1..].iter().all(|s| (0.001..=1000.).contains(s)))
            {
                cache_enabled = values[0] == 1.;
                scale.copy_from_slice(&values[1..]);
            }
        }
        if !ACTIVE.load(Ordering::Acquire) {
            return original(name, kind);
        }
        if !cache_enabled {
            if let Ok(mut cache) = CACHE.lock() {
                cache.entries.clear();
                cache.sdk = 0;
            }
        }
        let sdk = (state.sdk)();
        let Some(bytes) = ogre::string(name).filter(|name| eligible(name, kind, sdk)) else {
            return original(name, kind);
        };
        let Ok(id) = CString::new(bytes.clone()) else {
            return original(name, kind);
        };
        if let Ok(mut cache) = CACHE.lock() {
            if cache.sdk != sdk {
                cache.entries.clear();
                cache.sdk = sdk;
            }
            if cache_enabled {
                if let Some(index) = cache
                    .entries
                    .iter()
                    .position(|entry| entry.kind == kind && entry.name == bytes)
                {
                    let entry = cache
                        .entries
                        .remove(index)
                        .expect("position is within locked cache");
                    let copy = (state.clone)(entry.template.pointer, id.as_ptr(), &scale, sdk);
                    cache.entries.push_back(entry);
                    if copy != 0 {
                        HITS.fetch_add(1, Ordering::Relaxed);
                        return copy;
                    }
                }
            }
        }
        // Never hold the cache lock across resource loading (Ogre can re-enter).
        let loaded = original(name, kind);
        if loaded == 0 {
            return loaded;
        }
        // If cloning throws, the caller has not yet received this collection.
        // Release it as the native exception unwinds through C-unwind.
        let loaded_owner = Collection {
            pointer: loaded,
            release: state.release,
        };
        if let Ok(mut cache) = CACHE.lock() {
            if ACTIVE.load(Ordering::Acquire)
                && cache_enabled
                && cache.sdk == sdk
                && !cache
                    .entries
                    .iter()
                    .any(|entry| entry.name == bytes && entry.kind == kind)
            {
                let template_id = CString::new([bytes.as_slice(), b"_RUST_TEMPLATE"].concat())
                    .expect("validated name has no zero");
                let template = (state.clone)(loaded, template_id.as_ptr(), &[1.; 3], sdk);
                if template != 0 {
                    if cache.entries.len() == CAPACITY {
                        cache.entries.pop_front();
                    }
                    cache.entries.push_back(Entry {
                        name: bytes,
                        kind,
                        template: Collection {
                            pointer: template,
                            release: state.release,
                        },
                    });
                }
            }
        }
        if scale != [1.; 3] {
            let scaled = (state.clone)(loaded, id.as_ptr(), &scale, sdk);
            if scaled != 0 {
                return scaled;
            }
        }
        std::mem::forget(loaded_owner);
        loaded
    }
}
fn eligible(name: &[u8], kind: i32, sdk: usize) -> bool {
    kind == 0 && sdk != 0 && name.len() > 4 && !name[name.len() - 4..].eq_ignore_ascii_case(b".xml")
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn xml_and_sdkless_loads_bypass() {
        assert!(!eligible(b"collision.XML", 0, 1));
        assert!(!eligible(b"collision.bin", 0, 0));
        assert!(!eligible(b"collision.bin", 1, 1));
        assert!(eligible(b"collision.bin", 0, 1));
    }
}
