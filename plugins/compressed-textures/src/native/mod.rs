//! SIC codec plus optional TGA sidecars through the native archive API.
use crate::{
    ogre::{self, DecodeResult, NativeString, OwnedShared, Shared},
    sic,
};
use detour::{RawDetour, Threads, Transaction};
use rekenshi_native::symbols::ogre::{archive, codec, file_system_archive};
use rekenshi_plugin_api::HostV1;
use std::ffi::c_void;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
type ArchiveOpen =
    unsafe extern "C-unwind" fn(*mut c_void, *mut Shared, *const c_void, bool) -> *mut Shared;
type Register = unsafe extern "C" fn(usize);
type Registered = unsafe extern "C" fn(*const NativeString) -> bool;
type ArchiveName = unsafe extern "C" fn(*const c_void) -> *const c_void;
struct State {
    hook: RawDetour,
    target: usize,
    codec: usize,
    register: Register,
    unregister: Register,
    registered: Registered,
    archive_name: ArchiveName,
}
static STATE: OnceLock<State> = OnceLock::new();
static ROOT: Mutex<Option<PathBuf>> = Mutex::new(None);
static ENABLED: AtomicBool = AtomicBool::new(false);
static ACTIVE: AtomicBool = AtomicBool::new(false);
static DECODED: AtomicU64 = AtomicU64::new(0);
static SIDECARS: AtomicU64 = AtomicU64::new(0);
#[repr(C)]
struct Codec {
    table: &'static CodecTable,
}
#[repr(C)]
struct CodecTable {
    destroy: unsafe extern "C" fn(*mut c_void, u32) -> *mut c_void,
    encode: unsafe extern "C-unwind" fn(
        *const c_void,
        *mut Shared,
        *mut Shared,
        *mut Shared,
    ) -> *mut Shared,
    encode_file:
        unsafe extern "C-unwind" fn(*const c_void, *mut Shared, *const c_void, *mut Shared),
    decode: ogre::Decode,
    get_type: unsafe extern "C" fn(*const c_void, *mut NativeString) -> *mut NativeString,
    data_type: unsafe extern "C" fn(*const c_void, *mut NativeString) -> *mut NativeString,
    magic_match: unsafe extern "C" fn(*const c_void, *const u8, usize) -> bool,
    magic_ext: unsafe extern "C" fn(
        *const c_void,
        *mut NativeString,
        *const u8,
        usize,
    ) -> *mut NativeString,
}
static TABLE: CodecTable = CodecTable {
    destroy,
    encode,
    encode_file,
    decode,
    get_type,
    data_type,
    magic_match,
    magic_ext,
};
pub(super) fn start(host: HostV1, game: &Path) -> Result<(), String> {
    let api = ogre::initialize(game)?;
    if STATE.get().is_none() {
        let base = api.base;
        let target = rekenshi_native::export(base, file_system_archive::OPEN)? as usize;
        // SAFETY: Hash-gated export/member signatures, inspected archive sret ABI.
        let state = unsafe {
            State {
                hook: RawDetour::new(target as _, archive_open as *const ())
                    .map_err(|e| e.to_string())?,
                target,
                codec: Box::into_raw(Box::new(Codec { table: &TABLE })) as usize,
                register: std::mem::transmute::<*const c_void, Register>(rekenshi_native::export(
                    base,
                    codec::REGISTER_CODEC,
                )?),
                unregister: std::mem::transmute::<*const c_void, Register>(
                    rekenshi_native::export(base, codec::UNREGISTER_CODEC)?,
                ),
                registered: std::mem::transmute::<*const c_void, Registered>(
                    rekenshi_native::export(base, codec::IS_CODEC_REGISTERED)?,
                ),
                archive_name: std::mem::transmute::<*const c_void, ArchiveName>(
                    rekenshi_native::export(base, archive::GET_NAME)?,
                ),
            }
        };
        STATE
            .set(state)
            .map_err(|_| "SIC adapter initialized twice")?;
    }
    let state = STATE.get().ok_or("SIC adapter")?;
    // SAFETY: Native short string is 32 bytes and owns no heap allocation.
    if unsafe { (state.registered)(&NativeString::short(b"sic")) } {
        return Err("another SIC codec is already registered".into());
    }
    host.reserve_hook(std::ptr::NonNull::new(state.target as *mut c_void).ok_or("archive target")?)
        .map_err(|e| format!("texture hook conflict: {e}"))?;
    *ROOT.lock().map_err(|_| "texture root lock poisoned")? = Some(game.to_path_buf());
    let mut tx = Transaction::new();
    tx.enable(&state.hook);
    // SAFETY: Startup outside DllMain; suspend other threads while patching.
    unsafe { tx.commit(Threads::All) }.map_err(|e| e.to_string())?;
    // SAFETY: Codec registry is mounted during plugin startup before loads. It
    // borrows this pinned object and never owns its Rust allocation.
    unsafe { (state.register)(state.codec) };
    ACTIVE.store(true, Ordering::Release);
    host.write_log("compressed-textures: SIC codec mounted; converted TGA sidecars supported");
    Ok(())
}
pub(super) fn set_enabled(value: bool) {
    ENABLED.store(value, Ordering::Release);
}
pub(super) fn stop() -> Result<(), String> {
    ENABLED.store(false, Ordering::Release);
    if ACTIVE.swap(false, Ordering::AcqRel) {
        if let Some(state) = STATE.get() {
            // SAFETY: Plugin shutdown after resource loading ceases. Retain code/object.
            unsafe { (state.unregister)(state.codec) };
            let mut tx = Transaction::new();
            tx.disable(&state.hook);
            // SAFETY: Suspend native callers while restoring the archive entrance.
            unsafe { tx.commit(Threads::All) }.map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
pub fn decoded() -> u64 {
    DECODED.load(Ordering::Relaxed)
}
pub fn sidecar_hits() -> u64 {
    SIDECARS.load(Ordering::Relaxed)
}
unsafe extern "C-unwind" fn archive_open(
    this: *mut c_void,
    output: *mut Shared,
    name: *const c_void,
    readonly: bool,
) -> *mut Shared {
    let Some(state) = STATE.get() else {
        return output;
    };
    // SAFETY: Live archive and string arguments; native exceptions propagate
    // through C-unwind to the engine's own handler.
    unsafe {
        if readonly && ACTIVE.load(Ordering::Acquire) && ENABLED.load(Ordering::Acquire) {
            if let (Some(name), Some(folder)) =
                (ogre::string(name), ogre::string((state.archive_name)(this)))
            {
                if let (Ok(name), Ok(folder)) = (String::from_utf8(name), String::from_utf8(folder))
                {
                    if let Some(replacement) = sidecar(&folder, &name) {
                        output.write(replacement.into_raw());
                        SIDECARS.fetch_add(1, Ordering::Relaxed);
                        return output;
                    }
                }
            }
        }
        let original: ArchiveOpen = std::mem::transmute(state.hook.trampoline());
        original(this, output, name, readonly)
    }
}
fn sidecar(folder: &str, name: &str) -> Option<OwnedShared> {
    use std::io::Read;
    if !name.to_ascii_lowercase().ends_with(".tga") {
        return None;
    }
    let root = ROOT.lock().ok()?.clone()?;
    let file = root.join(folder).join(name);
    let packed = PathBuf::from(format!("{}.sic", file.display()));
    let mut manifest = Vec::new();
    std::fs::File::open(format!("{}.toml", packed.display()))
        .ok()?
        .take(4097)
        .read_to_end(&mut manifest)
        .ok()?;
    if manifest.len() > 4096 {
        return None;
    }
    let stamp: sic::SourceStamp = toml::from_str(std::str::from_utf8(&manifest).ok()?).ok()?;
    if stamp != sic::SourceStamp::capture(&file).ok()? {
        return None;
    }
    let mut bytes = Vec::new();
    std::fs::File::open(packed)
        .ok()?
        .take((sic::LIMIT + 1) as u64)
        .read_to_end(&mut bytes)
        .ok()?;
    let tga = sic::tga(&sic::decode(&bytes).ok()?).ok()?;
    ogre::initialize(Path::new(".")).ok()?.stream(tga).ok()
}
unsafe extern "C" fn destroy(this: *mut c_void, _: u32) -> *mut c_void {
    this
}
unsafe extern "C" fn get_type(_: *const c_void, output: *mut NativeString) -> *mut NativeString {
    // SAFETY: Native caller supplies writable std::string sret storage.
    unsafe {
        output.write(NativeString::short(b"sic"));
    }
    output
}
unsafe extern "C" fn data_type(_: *const c_void, output: *mut NativeString) -> *mut NativeString {
    // SAFETY: Native caller supplies writable std::string sret storage.
    unsafe {
        output.write(NativeString::short(b"ImageData"));
    }
    output
}
unsafe extern "C" fn magic_match(_: *const c_void, bytes: *const u8, len: usize) -> bool {
    // SAFETY: Codec contract supplies len readable magic bytes.
    !bytes.is_null() && len >= 4 && unsafe { std::slice::from_raw_parts(bytes, 4) == b"ShIt" }
}
unsafe extern "C" fn magic_ext(
    this: *const c_void,
    output: *mut NativeString,
    bytes: *const u8,
    len: usize,
) -> *mut NativeString {
    // SAFETY: Same magic contract; caller supplies native string return storage.
    unsafe {
        output.write(NativeString::short(if magic_match(this, bytes, len) {
            b"sic"
        } else {
            b""
        }));
    }
    output
}
unsafe extern "C-unwind" fn encode(
    _: *const c_void,
    _output: *mut Shared,
    _: *mut Shared,
    _: *mut Shared,
) -> *mut Shared {
    // SAFETY: Produce an Ogre exception rather than return invalid encoded data.
    unsafe { native_error() }
}
unsafe extern "C-unwind" fn encode_file(
    _: *const c_void,
    _: *mut Shared,
    _: *const c_void,
    _: *mut Shared,
) {
    // SAFETY: Runtime encoding is unsupported; the standalone converter owns it.
    unsafe {
        native_error();
    }
}
unsafe extern "C-unwind" fn native_error() -> ! {
    if let Ok(api) = ogre::initialize(Path::new(".")) {
        if let Ok(mut input) = api.stream(b"bad!".to_vec()) {
            let mut output = DecodeResult::default();
            // SAFETY: Fingerprinted DDS decoder checks magic before using this. Its
            // ordinary Ogre exception propagates through C-unwind, cleaning input.
            unsafe {
                (api.dds_decode)(std::ptr::null(), &mut output, input.as_mut_ptr());
            }
        }
    }
    // Allocation failure or a changed decoder contract cannot produce a valid
    // native return object. Never return uninitialized sret storage.
    std::process::abort()
}
unsafe extern "C-unwind" fn decode(
    _: *const c_void,
    output: *mut DecodeResult,
    input: *mut Shared,
) -> *mut DecodeResult {
    // SAFETY: Native codec caller supplies valid stream reference and sret storage.
    let result = unsafe { decode_image(&*input) };
    match result {
        Ok(result) => {
            // SAFETY: Caller owns writable sret storage; references transfer to it.
            unsafe {
                output.write(result);
            }
            DECODED.fetch_add(1, Ordering::Relaxed);
        }
        Err(_) => {
            // SAFETY: Report failure through Ogre's native exception mechanism.
            unsafe {
                native_error();
            }
        }
    }
    output
}
unsafe fn decode_image(input: &Shared) -> Result<DecodeResult, String> {
    // SAFETY: Live codec stream, bounded size and hash-verified stream adapter.
    unsafe {
        let size = ogre::stream_size(input)
            .filter(|n| *n <= sic::LIMIT)
            .ok_or("SIC stream too large")?;
        let mut bytes = vec![0; size];
        ogre::read(input, 0, &mut bytes)?;
        let image = sic::decode(&bytes)?;
        let api = ogre::initialize(Path::new("."))?;
        let metadata = api.metadata(
            image.width,
            image.height,
            image.format.ogre(),
            image.pixels.len(),
        )?;
        let pixels = api.pixel_stream(&image.pixels)?;
        Ok(DecodeResult {
            pixels: pixels.into_raw(),
            metadata: metadata.into_raw(),
        })
    }
}
