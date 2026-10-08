//! SIC codec plus optional TGA sidecars through the native archive API.
use crate::{
    ogre::{self, DecodeResult, NativeString, OwnedShared, Shared},
    sic,
};
use detour::{RawDetour, Threads, Transaction};
use rekenshi_native::symbols::ogre::{archive, codec, file_system_archive};
use rekenshi_plugin_api::HostV1;
use rekenshi_plugin_sdk::adapters::{self, AdapterOperations};
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
static POLICY: std::sync::RwLock<Option<rekenshi_plugin_sdk::policies::PolicyProvider>> =
    std::sync::RwLock::new(None);
static ACTIVE: AtomicBool = AtomicBool::new(false);
static OPERATIONS: std::sync::RwLock<Option<AdapterOperations>> = std::sync::RwLock::new(None);
static DECODED: AtomicU64 = AtomicU64::new(0);
static SIDECARS: AtomicU64 = AtomicU64::new(0);
#[repr(C)]
struct Codec {
    table: &'static CodecTable,
    extension: [u8; 16],
    magic: [u8; 16],
    magic_len: usize,
}
impl Codec {
    fn new(extension: &[u8], magic: &[u8]) -> Self {
        let mut codec = Self {
            table: &TABLE,
            extension: [0; 16],
            magic: [0; 16],
            magic_len: magic.len(),
        };
        codec.extension[..extension.len()].copy_from_slice(extension);
        codec.magic[..magic.len()].copy_from_slice(magic);
        codec
    }
    fn extension(&self) -> &[u8] {
        &self.extension[..self.extension.iter().position(|b| *b == 0).unwrap_or(16)]
    }
}
struct CodecProviders {
    claims: rekenshi_plugin_sdk::operations::Provider,
    decode: rekenshi_plugin_sdk::operations::Provider,
    objects: Vec<usize>,
}
static CODECS: std::sync::RwLock<Option<CodecProviders>> = std::sync::RwLock::new(None);
fn publish_codecs(host: HostV1, state: &State) -> Result<(), String> {
    use rekenshi_plugin_sdk::{
        codecs,
        operations::{Contract, Operations},
    };
    let operations = Operations::from_host(host).map_err(|e| e.to_string())?;
    let claims=operations.publish(Contract::new(codecs::CLAIMS,"rekenshi.codec-declarations@1",
        "Preparation only; declarations consumed before native loads; late declarations require restart",
        "Fingerprint-gated Ogre Codec registry and inspected virtual table; native thunks pinned until provider shutdown",4096).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    let decode=operations.publish(Contract::new(codecs::DECODE,"rekenshi.codec-image-le@1",
        "Synchronous resource loader; copied encoded input and validated image result; no native pointers",
        "Ogre native MemoryDataStream/ImageData allocations and shared ownership; dimensions and pixel length checked before native commit",16*1024*1024).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    let declarations = claims
        .dispatch(&[], &[])
        .and_then(|bytes| codecs::declarations(&bytes))
        .map_err(|e| e.to_string())?;
    // Check every conflict before changing Ogre's registry.
    for (extension, _) in &declarations {
        // SAFETY: Native API borrows this bounded short string synchronously.
        if unsafe { (state.registered)(&NativeString::short(extension.as_bytes())) } {
            return Err(format!("codec extension already registered: {extension}"));
        }
    }
    let mut objects = Vec::new();
    for (extension, magic) in declarations {
        let object = Box::into_raw(Box::new(Codec::new(extension.as_bytes(), &magic))) as usize;
        // SAFETY: Registry borrows the pinned native vtable object; lifetime retained beyond shutdown.
        unsafe { (state.register)(object) };
        objects.push(object);
    }
    *CODECS.write().map_err(|_| "codec provider lock")? = Some(CodecProviders {
        claims,
        decode,
        objects,
    });
    Ok(())
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
                codec: Box::into_raw(Box::new(Codec::new(b"sic", b"ShIt"))) as usize,
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
    *OPERATIONS.write().map_err(|_| "operation registry lock")? = Some(AdapterOperations::publish(host, "Existing adapter fingerprint and native signatures validated before hook enable; see implementation inventory and adapter source for evidence limits", &[(adapters::TEXTURE_SELECT,"rekenshi.utf8-path@1",8192),(adapters::TEXTURE_DECODE,"rekenshi.image-pixels@1",16777216)]).map_err(|e| e.to_string())?);
    *POLICY.write().map_err(|_| "texture policy lock")? = rekenshi_plugin_sdk::policies::PolicyProvider::optional(host, rekenshi_plugin_sdk::policies::TEXTURES,
        "Fingerprint-gated Ogre FileSystemArchive::open and SIC codec registry; sidecars validate source stamp and decoded size; native-owned replacement streams").map_err(|e| e.to_string())?;
    publish_codecs(host, state)?;
    host.write_log("compressed-textures: SIC codec mounted; public codec declarations activated");
    Ok(())
}

pub(super) fn stop() -> Result<(), String> {
    let mut codecs = CODECS.write().map_err(|_| "codec provider lock")?;
    if let Some(providers) = codecs.as_mut() {
        providers.decode.revoke().map_err(|e| e.to_string())?;
        providers.claims.revoke().map_err(|e| e.to_string())?;
        if let Some(state) = STATE.get() {
            for object in &providers.objects {
                // SAFETY: Native loading is quiescent at provider shutdown; retain object allocation.
                unsafe { (state.unregister)(*object) };
            }
        }
    }
    *codecs = None;
    drop(codecs);
    if let Some(mut operations) = OPERATIONS
        .write()
        .map_err(|_| "operation registry lock")?
        .take()
    {
        operations.revoke().map_err(|e| e.to_string())?;
    }
    if let Some(mut policy) = POLICY.write().map_err(|_| "texture policy lock")?.take() {
        policy.revoke().map_err(|e| e.to_string())?;
    }
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
        let selected = if readonly && ACTIVE.load(Ordering::Acquire) {
            ogre::string(name)
                .and_then(|filename| {
                    let folder = ogre::string((state.archive_name)(this))?;
                    OPERATIONS.read().ok()?.as_ref()?.transform(
                        adapters::TEXTURE_SELECT,
                        &folder,
                        &filename,
                    )
                })
                .filter(|v| {
                    !v.is_empty()
                        && !v.contains(&0)
                        && std::str::from_utf8(v).ok().is_some_and(|s| {
                            Path::new(s)
                                .components()
                                .all(|c| matches!(c, std::path::Component::Normal(_)))
                        })
                })
        } else {
            None
        };
        let selected_native = selected.as_deref().map(NativeString::borrowed);
        let name = selected_native.as_ref().map_or(name, |s| s.as_ptr());
        let enabled = POLICY
            .read()
            .ok()
            .and_then(|p| p.as_ref().map(|p| p.values([0.; 4])[0] == 1.))
            .unwrap_or_else(|| ENABLED.load(Ordering::Acquire));
        if readonly && ACTIVE.load(Ordering::Acquire) && enabled {
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
unsafe extern "C" fn get_type(this: *const c_void, output: *mut NativeString) -> *mut NativeString {
    // SAFETY: Native caller supplies writable std::string sret storage.
    unsafe {
        output.write(NativeString::short((&*this.cast::<Codec>()).extension()));
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
unsafe extern "C" fn magic_match(this: *const c_void, bytes: *const u8, len: usize) -> bool {
    if this.is_null() || bytes.is_null() {
        return false;
    }
    // SAFETY: Ogre borrows our pinned Codec and supplies len readable magic bytes.
    unsafe {
        let codec = &*this.cast::<Codec>();
        len >= codec.magic_len
            && std::slice::from_raw_parts(bytes, codec.magic_len) == &codec.magic[..codec.magic_len]
    }
}
unsafe extern "C" fn magic_ext(
    this: *const c_void,
    output: *mut NativeString,
    bytes: *const u8,
    len: usize,
) -> *mut NativeString {
    // SAFETY: Caller supplies writable native string return storage and our pinned codec.
    unsafe {
        output.write(NativeString::short(if magic_match(this, bytes, len) {
            (&*this.cast::<Codec>()).extension()
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
    this: *const c_void,
    output: *mut DecodeResult,
    input: *mut Shared,
) -> *mut DecodeResult {
    // SAFETY: Native codec caller supplies valid stream reference and sret storage.
    let result = unsafe {
        if (&*this.cast::<Codec>()).extension() == b"sic" {
            decode_image(&*input)
        } else {
            decode_custom((&*this.cast::<Codec>()).extension(), &*input)
        }
    };
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
unsafe fn decode_custom(extension: &[u8], input: &Shared) -> Result<DecodeResult, String> {
    // SAFETY: Native codec caller owns a live stream; checked byte bound precedes reads.
    unsafe {
        let size = ogre::stream_size(input)
            .filter(|n| *n <= 16 * 1024 * 1024)
            .ok_or("custom codec input too large")?;
        let mut bytes = vec![0; size];
        ogre::read(input, 0, &mut bytes)?;
        let providers = CODECS.read().map_err(|_| "codec provider lock")?;
        let providers = providers.as_ref().ok_or("codec provider revoked")?;
        let result = providers
            .decode
            .dispatch(extension, &bytes)
            .map_err(|e| e.to_string())?;
        if result == bytes {
            return Err("codec decoder revoked or absent".into());
        }
        let image =
            rekenshi_plugin_sdk::codecs::Image::decode(&result).map_err(|e| e.to_string())?;
        let api = ogre::initialize(Path::new("."))?;
        let metadata = api.metadata(image.width, image.height, image.format, image.pixels.len())?;
        let pixels = api.pixel_stream(&image.pixels)?;
        Ok(DecodeResult {
            pixels: pixels.into_raw(),
            metadata: metadata.into_raw(),
        })
    }
}
unsafe fn decode_image(input: &Shared) -> Result<DecodeResult, String> {
    // SAFETY: Live codec stream, bounded size and hash-verified stream adapter.
    unsafe {
        let size = ogre::stream_size(input)
            .filter(|n| *n <= sic::LIMIT)
            .ok_or("SIC stream too large")?;
        let mut bytes = vec![0; size];
        ogre::read(input, 0, &mut bytes)?;
        let mut image = sic::decode(&bytes)?;
        let metadata: Vec<_> = [image.width, image.height, image.format.ogre()]
            .into_iter()
            .flat_map(u32::to_le_bytes)
            .collect();
        if let Some(pixels) = OPERATIONS
            .read()
            .ok()
            .and_then(|o| {
                o.as_ref()
                    .and_then(|o| o.transform(adapters::TEXTURE_DECODE, &metadata, &image.pixels))
            })
            .filter(|p| p.len() == image.pixels.len())
        {
            image.pixels = pixels;
        }
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
