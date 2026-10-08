//! Adapter for the fingerprinted terrain DLL's TIFF reader. Hooking its reader
//! preserves native coordinate conversion, height scaling and terrain caches.
use rekenshi_plugin_sdk::adapters::{self, AdapterOperations};
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
    gigacache: Option<Arc<mapped::Heightmap>>,
    streams: HashMap<usize, Arc<mapped::Heightmap>>,
}
static HOOKS: OnceLock<Hooks> = OnceLock::new();
static SESSION: Mutex<Option<Session>> = Mutex::new(None);
// Serialize lifecycle/configuration without holding SESSION during the eager copy.
static CONFIGURATION: Mutex<()> = Mutex::new(());
static ENABLED: AtomicBool = AtomicBool::new(false);
static ACTIVE: AtomicBool = AtomicBool::new(false);
static OPERATIONS: std::sync::RwLock<Option<AdapterOperations>> = std::sync::RwLock::new(None);
static READS: AtomicU64 = AtomicU64::new(0);
static POLICY: std::sync::RwLock<Option<rekenshi_plugin_sdk::policies::PolicyProvider>> =
    std::sync::RwLock::new(None);
static MODE: AtomicU64 = AtomicU64::new(0);

fn start_bindings(host: HostV1, game: &Path) -> Result<(), String> {
    let _configuration = CONFIGURATION
        .lock()
        .map_err(|_| "heightmap configuration poisoned")?;
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
        gigacache: None,
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
pub(super) fn start(host: HostV1, game: &Path) -> Result<(), String> {
    start_bindings(host, game)?;
    *OPERATIONS.write().map_err(|_| "operation registry lock")? = Some(AdapterOperations::publish(host, "Existing adapter fingerprint and native signatures validated before hook enable; see implementation inventory and adapter source for evidence limits", &[(adapters::HEIGHTMAP_READ,"rekenshi.u16-height-row-le@1",16777216),(adapters::HEIGHTMAP_LOAD,"rekenshi.utf8-path@1",32768)]).map_err(|e| e.to_string())?);
    *POLICY.write().map_err(|_| "heightmap policy lock")? = rekenshi_plugin_sdk::policies::PolicyProvider::optional(host, rekenshi_plugin_sdk::policies::HEIGHTMAP,
        "Plugin_Terrain_x64 SHA256 53AFD1E4FAF35020556D33F57007D51C2A9D2330B630E39A67161EE850B7A239; TIFF open/close/pixel/rectangle; Arc-owned maps survive readers; not world-loaded evidence").map_err(|e| e.to_string())?;
    Ok(())
}
pub(super) fn configure(mapped: bool, eager: bool) -> Result<(), String> {
    let _configuration = CONFIGURATION
        .lock()
        .map_err(|_| "heightmap configuration poisoned")?;
    let source = {
        let guard = SESSION.lock().map_err(|_| "heightmap lock poisoned")?;
        let session = guard.as_ref().ok_or("heightmap session unavailable")?;
        (eager && session.gigacache.is_none()).then(|| Arc::clone(&session.heightmap))
    };
    // Native reads continue using the previous mode while the copy is prepared.
    // On allocation/validation failure both the previous source and toggle survive.
    let prepared = source
        .map(|map| map.eager_copy().map(Arc::new))
        .transpose()?;
    let retired = {
        let mut guard = SESSION.lock().map_err(|_| "heightmap lock poisoned")?;
        let session = guard.as_mut().ok_or("heightmap session unavailable")?;
        if let Some(prepared) = prepared {
            session.host.write_log(&format!(
                "Gigacache ready: {} bytes eagerly copied; OS paging remains possible",
                prepared.eager_bytes()
            ));
            session.gigacache = Some(prepared);
        }
        let retired = if eager {
            None
        } else {
            session.gigacache.take()
        };
        ENABLED.store(mapped || eager, Ordering::Release);
        MODE.store(mapped as u64 | ((eager as u64) << 1), Ordering::Release);
        retired
    };
    // Existing readers own an Arc; release the large allocation outside SESSION.
    drop(retired);
    Ok(())
}
pub(super) fn stop() -> Result<(), String> {
    if let Some(mut operations) = OPERATIONS
        .write()
        .map_err(|_| "operation registry lock")?
        .take()
    {
        operations.revoke().map_err(|e| e.to_string())?;
    }
    if let Some(mut policy) = POLICY.write().map_err(|_| "heightmap policy lock")?.take() {
        policy.revoke().map_err(|e| e.to_string())?;
    }
    let _configuration = CONFIGURATION
        .lock()
        .map_err(|_| "heightmap configuration poisoned")?;
    ACTIVE.store(false, Ordering::Release);
    ENABLED.store(false, Ordering::Release);
    MODE.store(0, Ordering::Release);
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
    let selected = if !path.is_null() && ACTIVE.load(Ordering::Acquire) {
        // SAFETY: Native open supplies a terminated filename valid through this call.
        let input = unsafe { CStr::from_ptr(path) }.to_bytes();
        OPERATIONS
            .read()
            .ok()
            .and_then(|operations| {
                operations.as_ref().and_then(|operations| {
                    operations.transform(adapters::HEIGHTMAP_LOAD, &mode.to_le_bytes(), input)
                })
            })
            .filter(|bytes| !bytes.is_empty() && std::str::from_utf8(bytes).is_ok())
            .and_then(|bytes| std::ffi::CString::new(bytes).ok())
    } else {
        None
    };
    let path = selected.as_ref().map_or(path, |selected| selected.as_ptr());
    // SAFETY: Original or owned replacement filename and open mode remain live.
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
    let desired = POLICY
        .read()
        .ok()
        .and_then(|p| p.as_ref().map(|p| p.values([0.; 4])));
    if let Some(policy) = desired {
        let mapped = policy[0] == 1.;
        let eager = policy[1] == 1.;
        let mode = mapped as u64 | ((eager as u64) << 1);
        if MODE.load(Ordering::Acquire) != mode && configure(mapped, eager).is_err() {
            return None;
        }
    }
    if !ACTIVE.load(Ordering::Acquire) || !ENABLED.load(Ordering::Acquire) {
        return None;
    }
    let guard = SESSION.lock().ok()?;
    let session = guard.as_ref()?;
    let mapped = session.streams.get(&(stream as usize))?;
    Some(Arc::clone(session.gigacache.as_ref().unwrap_or(mapped)))
}
fn transformed_row(x: i32, y: i32, width: i32, bytes: &[u8]) -> Option<Vec<u8>> {
    let metadata: Vec<_> = [x, y, width, 1]
        .into_iter()
        .flat_map(i32::to_le_bytes)
        .collect();
    OPERATIONS
        .read()
        .ok()?
        .as_ref()?
        .transform(adapters::HEIGHTMAP_READ, &metadata, bytes)
        .filter(|b| b.len() == bytes.len())
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
        let transformed = transformed_row(x, y, 1, row);
        // SAFETY: Native caller supplies two writable bytes; source lives through the copy.
        unsafe {
            std::ptr::copy_nonoverlapping(
                transformed.as_deref().unwrap_or(row).as_ptr(),
                out.cast(),
                2,
            )
        };
        READS.fetch_add(1, Ordering::Relaxed);
        Some(1)
    })
    .ok()
    .flatten();
    if let Some(result) = result {
        return result;
    }
    // SAFETY: Forward original read arguments through retained native trampoline.
    let result = HOOKS
        .get()
        .map_or(0, |h| unsafe { h.pixel.call(stream, x, y, out) });
    if result == 1 && !out.is_null() {
        // SAFETY: Successful native pixel read initialized the caller's two bytes.
        unsafe {
            if let Some(bytes) = transformed_row(x, y, 1, std::slice::from_raw_parts(out.cast(), 2))
            {
                std::ptr::copy_nonoverlapping(bytes.as_ptr(), out.cast(), 2);
            }
        }
    }
    result
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
            let transformed = transformed_row(x as i32, (y + row) as i32, width, source);
            // SAFETY: Native rectangle contract supplies w*h*2 writable bytes.
            // Prevalidated rectangle is wholly inside the immutable map. Edge
            // requests take the original clipping path, including its return count.
            unsafe {
                std::ptr::copy_nonoverlapping(
                    transformed.as_deref().unwrap_or(source).as_ptr(),
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
    let result = HOOKS.get().map_or(0, |h| unsafe {
        h.rectangle.call(stream, x, y, width, height, out)
    });
    if width > 0 && height > 0 && !out.is_null() {
        let count = (width as usize).checked_mul(height as usize);
        if count == Some(result) && result <= 32 * 1024 * 1024 {
            for row in 0..height {
                let Some(row_y) = y.checked_add(row) else {
                    break;
                };
                // SAFETY: Full successful native rectangle initialized width*height u16 pixels.
                unsafe {
                    let pointer = out.cast::<u8>().add(row as usize * width as usize * 2);
                    let bytes = std::slice::from_raw_parts(pointer, width as usize * 2);
                    if let Some(transformed) = transformed_row(x, row_y, width, bytes) {
                        std::ptr::copy_nonoverlapping(transformed.as_ptr(), pointer, bytes.len());
                    }
                }
            }
        }
    }
    result
}
