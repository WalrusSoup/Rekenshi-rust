//! Adapter for the fingerprinted terrain DLL's TIFF reader. Hooking its reader
//! preserves native coordinate conversion, height scaling and terrain caches.
mod mapped;
#[cfg(test)]
mod tests;
mod tiff;
use detour::{Threads, Transaction, TypedDetour};
use rekenshi_plugin_api::HostV1;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::ffi::{CStr, c_char, c_void};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

const TERRAIN_HASH: &str = "53AFD1E4FAF35020556D33F57007D51C2A9D2330B630E39A67161EE850B7A239";
type Open = unsafe extern "C" fn(*const c_char, i32) -> *mut c_void;
type Close = unsafe extern "C" fn(*mut c_void);
type Pixel = unsafe extern "C" fn(*mut c_void, i32, i32, *mut c_void) -> usize;
type Rectangle = unsafe extern "C" fn(*mut c_void, i32, i32, i32, i32, *mut c_void) -> usize;
struct Hooks {
    open: TypedDetour<Open>,
    close: TypedDetour<Close>,
    pixel: TypedDetour<Pixel>,
    rectangle: TypedDetour<Rectangle>,
    targets: [usize; 4],
}
struct Session {
    host: HostV1,
    path: PathBuf,
    heightmap: Arc<mapped::Heightmap>,
    streams: HashMap<usize, Arc<mapped::Heightmap>>,
}
static HOOKS: OnceLock<Hooks> = OnceLock::new();
static SESSION: Mutex<Option<Session>> = Mutex::new(None);
static ENABLED: AtomicBool = AtomicBool::new(false);
static ACTIVE: AtomicBool = AtomicBool::new(false);
static READS: AtomicU64 = AtomicU64::new(0);

pub(super) fn start(host: HostV1, game: &Path) -> Result<(), String> {
    let mut session = SESSION.lock().map_err(|_| "heightmap lock poisoned")?;
    let path = game
        .join("data/newland/land/fullmap.tif")
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let heightmap = Arc::new(mapped::Heightmap::open(&path)?);
    let dll = game.join("Plugin_Terrain_x64.dll");
    if format!(
        "{:X}",
        Sha256::digest(std::fs::read(&dll).map_err(|e| e.to_string())?)
    ) != TERRAIN_HASH
    {
        return Err("heightmap-cache: unsupported terrain DLL fingerprint".into());
    }
    let base = rekenshi_native::load_pinned(&dll)?;
    if HOOKS.get().is_none() {
        let targets = [0x20880, 0x21c40, 0x21640, 0x21800].map(|rva| base + rva);
        // SAFETY: Full hash pins the inspected implementation and each x64 ABI.
        // These prologues save argument registers, with no initial calls.
        let (open, close, pixel, rectangle) = unsafe {
            (
                TypedDetour::new(
                    std::mem::transmute::<usize, Open>(targets[0]),
                    open_hook as Open,
                ),
                TypedDetour::new(
                    std::mem::transmute::<usize, Close>(targets[1]),
                    close_hook as Close,
                ),
                TypedDetour::new(
                    std::mem::transmute::<usize, Pixel>(targets[2]),
                    pixel_hook as Pixel,
                ),
                TypedDetour::new(
                    std::mem::transmute::<usize, Rectangle>(targets[3]),
                    rectangle_hook as Rectangle,
                ),
            )
        };
        let hooks = Hooks {
            open: open.map_err(|e| e.to_string())?,
            close: close.map_err(|e| e.to_string())?,
            pixel: pixel.map_err(|e| e.to_string())?,
            rectangle: rectangle.map_err(|e| e.to_string())?,
            targets,
        };
        HOOKS
            .set(hooks)
            .map_err(|_| "heightmap hooks already initialized")?;
    }
    let hooks = HOOKS.get().ok_or("heightmap hooks missing")?;
    for target in hooks.targets {
        host.reserve_hook(std::ptr::NonNull::new(target as *mut c_void).ok_or("heightmap target")?)
            .map_err(|e| format!("heightmap hook conflict: {e}"))?;
    }
    host.write_log(&format!(
        "heightmap-cache mapped {}x{}; native terrain behavior retained",
        heightmap.layout.width, heightmap.layout.height
    ));
    *session = Some(Session {
        host,
        path,
        heightmap,
        streams: HashMap::new(),
    });
    let mut tx = Transaction::new();
    tx.enable(&hooks.open);
    tx.enable(&hooks.close);
    tx.enable(&hooks.pixel);
    tx.enable(&hooks.rectangle);
    // SAFETY: Outside DllMain, suspend threads while patching; code is process-pinned.
    unsafe { tx.commit(Threads::All) }.map_err(|e| e.to_string())?;
    ACTIVE.store(true, Ordering::Release);
    Ok(())
}
pub(super) fn set_enabled(value: bool) {
    ENABLED.store(value, Ordering::Release);
}
pub(super) fn stop() -> Result<(), String> {
    ACTIVE.store(false, Ordering::Release);
    ENABLED.store(false, Ordering::Release);
    if let Some(hooks) = HOOKS.get() {
        let mut tx = Transaction::new();
        tx.disable(&hooks.rectangle);
        tx.disable(&hooks.pixel);
        tx.disable(&hooks.close);
        tx.disable(&hooks.open);
        // SAFETY: Same patching contract as start. Trampolines remain alive.
        unsafe { tx.commit(Threads::All) }.map_err(|e| e.to_string())?;
    }
    if let Some(session) = SESSION
        .lock()
        .map_err(|_| "heightmap lock poisoned")?
        .take()
    {
        session.host.write_log(&format!(
            "heightmap-cache stopped; mapped reads={}",
            reads()
        ));
    }
    Ok(())
}
pub fn reads() -> u64 {
    READS.load(Ordering::Relaxed)
}
extern "C" fn open_hook(path: *const c_char, mode: i32) -> *mut c_void {
    let Some(hooks) = HOOKS.get() else {
        return std::ptr::null_mut();
    };
    // SAFETY: Native caller supplies its original filename and open mode.
    let stream = unsafe { hooks.open.call(path, mode) };
    let _ = std::panic::catch_unwind(|| {
        if stream.is_null() || path.is_null() || mode != 1 || !ACTIVE.load(Ordering::Acquire) {
            return;
        }
        // SAFETY: Native open just successfully consumed this terminated path.
        let Ok(path) = (unsafe { CStr::from_ptr(path) }).to_str() else {
            return;
        };
        let Ok(path) = Path::new(path).canonicalize() else {
            return;
        };
        let Ok(mut session) = SESSION.lock() else {
            return;
        };
        let Some(session) = session.as_mut() else {
            return;
        };
        if path != session.path {
            return;
        }
        // SAFETY: Successful fingerprinted open allocates a 40-byte TIFF reader;
        // fields at 8/12/16/20 are width/height/bits/samples, verified in disassembly.
        let dimensions = unsafe {
            let p = stream.cast::<u8>();
            [8, 12, 16, 20].map(|offset| p.add(offset).cast::<u32>().read_unaligned())
        };
        if dimensions
            != [
                session.heightmap.layout.width as u32,
                session.heightmap.layout.height as u32,
                16,
                1,
            ]
        {
            return;
        }
        session
            .streams
            .insert(stream as usize, Arc::clone(&session.heightmap));
        session
            .host
            .write_log("heightmap-cache attached to native fullmap reader");
    });
    stream
}
extern "C" fn close_hook(stream: *mut c_void) {
    let Some(hooks) = HOOKS.get() else { return };
    let _ = std::panic::catch_unwind(|| {
        if let Ok(mut session) = SESSION.lock() {
            if let Some(session) = session.as_mut() {
                session.streams.remove(&(stream as usize));
            }
        }
    });
    // SAFETY: Native destructor receives its unchanged live reader pointer.
    unsafe { hooks.close.call(stream) };
}
fn heightmap(stream: *mut c_void) -> Option<Arc<mapped::Heightmap>> {
    if !ACTIVE.load(Ordering::Acquire) || !ENABLED.load(Ordering::Acquire) {
        return None;
    }
    SESSION
        .lock()
        .ok()?
        .as_ref()?
        .streams
        .get(&(stream as usize))
        .cloned()
}
extern "C" fn pixel_hook(stream: *mut c_void, x: i32, y: i32, out: *mut c_void) -> usize {
    let result = std::panic::catch_unwind(|| {
        let map = heightmap(stream)?;
        let row = map.row(usize::try_from(x).ok()?, usize::try_from(y).ok()?, 1)?;
        if out.is_null() {
            return None;
        }
        // SAFETY: Native 16-bit grayscale read contract supplies two writable
        // output bytes. Mapped source is read-only and lives through this Arc.
        unsafe { std::ptr::copy_nonoverlapping(row.as_ptr(), out.cast(), 2) };
        READS.fetch_add(1, Ordering::Relaxed);
        Some(1)
    })
    .ok()
    .flatten();
    if let Some(result) = result {
        return result;
    }
    // SAFETY: Forward original read arguments through retained native trampoline.
    HOOKS
        .get()
        .map_or(0, |h| unsafe { h.pixel.call(stream, x, y, out) })
}
extern "C" fn rectangle_hook(
    stream: *mut c_void,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    out: *mut c_void,
) -> usize {
    let result = std::panic::catch_unwind(|| {
        let map = heightmap(stream)?;
        let (x, y, w, h) = (
            usize::try_from(x).ok()?,
            usize::try_from(y).ok()?,
            usize::try_from(width).ok()?,
            usize::try_from(height).ok()?,
        );
        let count = w.checked_mul(h)?;
        if out.is_null()
            || count == 0
            || count > 32 * 1024 * 1024
            || y.checked_add(h)? > map.layout.height
        {
            return None;
        }
        map.row(x, y, w)?;
        for row in 0..h {
            let source = map.row(x, y + row, w)?;
            // SAFETY: Native rectangle contract supplies w*h*2 writable bytes.
            // Prevalidated rectangle is wholly inside the immutable map. Edge
            // requests take the original clipping path, including its return count.
            unsafe {
                std::ptr::copy_nonoverlapping(
                    source.as_ptr(),
                    out.cast::<u8>().add(row * w * 2),
                    w * 2,
                )
            };
        }
        READS.fetch_add(1, Ordering::Relaxed);
        Some(count)
    })
    .ok()
    .flatten();
    if let Some(result) = result {
        return result;
    }
    // SAFETY: Preserve original clipping/output behavior for all bypassed reads.
    HOOKS.get().map_or(0, |h| unsafe {
        h.rectangle.call(stream, x, y, width, height, out)
    })
}
