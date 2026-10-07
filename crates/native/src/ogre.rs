//! Private ABI adapter for the fingerprinted VC100 Ogre binary. All native
//! allocations and final releases use Ogre's allocator; pixel buffers stay owned.
use crate::symbols::ogre::{dds_codec, memory_data_stream, ned_pooling_impl, shared_ptr};
use sha2::{Digest, Sha256};
use std::ffi::{c_char, c_void};
use std::path::Path;
use std::sync::OnceLock;
use std::sync::atomic::AtomicU32;

const HASH: &str = "7D3888D21398647519F10A89B1F300D8606F761AD159E6021A797B1B9B3DF5F6";
pub const GAME_HASH: &str = "A596AB4E407C67B58599C54FFB32DC1BF2B64510CDEBD3FA9359EF05A576AEB1";
type Allocate = unsafe extern "C" fn(usize, *const c_char, i32, *const c_char) -> *mut c_void;
type Free = unsafe extern "C" fn(*mut c_void);
type Construct = unsafe extern "C" fn(*mut c_void, *mut c_void, usize, bool, bool) -> *mut c_void;
type Release = unsafe extern "C" fn(*mut Shared);
pub type Decode =
    unsafe extern "C-unwind" fn(*const c_void, *mut DecodeResult, *mut Shared) -> *mut DecodeResult;
#[derive(Debug)]
pub struct Api {
    pub base: usize,
    allocate: Allocate,
    free: Free,
    construct: Construct,
    destroy_stream: Free,
    release: Release,
    pub dds_decode: Decode,
}
static API: OnceLock<Api> = OnceLock::new();

pub fn initialize(game: &Path) -> Result<&'static Api, String> {
    if let Some(api) = API.get() {
        return Ok(api);
    }
    let path = game.join("OgreMain_x64.dll");
    verify(&path, HASH)?;
    let base = crate::load_pinned(&path)?;
    // SAFETY: Exact file fingerprint and independently inspected exported ABIs.
    let api = unsafe {
        Api {
            base,
            allocate: std::mem::transmute::<*const c_void, Allocate>(crate::export(
                base,
                ned_pooling_impl::ALLOC_BYTES,
            )?),
            free: std::mem::transmute::<*const c_void, Free>(crate::export(
                base,
                ned_pooling_impl::DEALLOC_BYTES,
            )?),
            construct: std::mem::transmute::<*const c_void, Construct>(crate::export(
                base,
                memory_data_stream::CONSTRUCT,
            )?),
            destroy_stream: std::mem::transmute::<*const c_void, Free>(crate::export(
                base,
                memory_data_stream::DESTROY,
            )?),
            release: std::mem::transmute::<*const c_void, Release>(crate::export(
                base,
                shared_ptr::DESTROY_HARDWARE_COUNTER_BUFFER,
            )?),
            dds_decode: std::mem::transmute::<*const c_void, Decode>(crate::export(
                base,
                dds_codec::DECODE,
            )?),
        }
    };
    let _ = API.set(api);
    API.get().ok_or("Ogre adapter initialization".into())
}
pub fn verify(path: &Path, hash: &str) -> Result<(), String> {
    if format!(
        "{:X}",
        Sha256::digest(std::fs::read(path).map_err(|e| e.to_string())?)
    ) != hash
    {
        return Err(format!(
            "unsupported native fingerprint: {}",
            path.display()
        ));
    }
    Ok(())
}
pub fn game_module(game: &Path) -> Result<usize, String> {
    use windows::Win32::System::LibraryLoader::{GetModuleFileNameW, GetModuleHandleW};
    use windows::core::w;
    // SAFETY: Queries only a mapped executable; never loads or initializes it.
    let module = unsafe { GetModuleHandleW(w!("kenshi_x64.exe")) }
        .map_err(|_| "Kenshi executable is not mapped")?;
    let mut path = [0u16; 32768];
    // SAFETY: Live module handle and writable path buffer.
    let len = unsafe { GetModuleFileNameW(Some(module), &mut path) } as usize;
    if len == 0 || len >= path.len() {
        return Err("Kenshi module path".into());
    }
    let loaded =
        std::path::PathBuf::from(String::from_utf16(&path[..len]).map_err(|e| e.to_string())?);
    if loaded.canonicalize().map_err(|e| e.to_string())?
        != game
            .join("kenshi_x64.exe")
            .canonicalize()
            .map_err(|e| e.to_string())?
    {
        return Err("mapped Kenshi executable is from a different installation".into());
    }
    verify(&loaded, GAME_HASH)?;
    Ok(module.0 as usize)
}

#[repr(C)]
#[derive(Default, Debug)]
pub struct Shared {
    pub object: usize,
    pub info: usize,
}
#[repr(C)]
#[derive(Default, Debug)]
pub struct DecodeResult {
    pub pixels: Shared,
    pub metadata: Shared,
}
#[repr(C)]
struct Info {
    table: &'static InfoTable,
    mutex_storage: [u8; 24],
    count: AtomicU32,
    padding: u32,
    owner: *mut Owner,
}
#[repr(C)]
struct InfoTable {
    destroy: unsafe extern "C" fn(*mut Info, u32) -> *mut Info,
}
static INFO_TABLE: InfoTable = InfoTable {
    destroy: destroy_info,
};
const _: () = {
    assert!(std::mem::size_of::<Shared>() == 16);
    assert!(std::mem::size_of::<DecodeResult>() == 32);
    assert!(std::mem::size_of::<Info>() == 48);
    assert!(std::mem::offset_of!(Info, count) == 0x20);
    assert!(std::mem::offset_of!(Info, owner) == 0x28);
    assert!(std::mem::size_of::<NativeString>() == 32);
};
enum Owner {
    Stream { object: usize, _bytes: Vec<u8> },
    Metadata { object: usize },
    NativeStream { object: usize },
}
impl Drop for Owner {
    fn drop(&mut self) {
        if let Some(api) = API.get() {
            // SAFETY: Each owner uniquely owns its native allocation; stream
            // freeOnClose is false, and its Rust pixel vector is dropped afterward.
            unsafe {
                match self {
                    Self::Stream { object, .. } | Self::NativeStream { object } => {
                        (api.destroy_stream)(*object as _);
                        (api.free)(*object as _);
                    }
                    Self::Metadata { object } => (api.free)(*object as _),
                }
            }
        }
    }
}
unsafe extern "C" fn destroy_info(info: *mut Info, flags: u32) -> *mut Info {
    // SAFETY: Native SharedPtr final release calls this sole virtual destructor
    // with flags=0, then frees the info with Ogre. Deleting calls use flag 1.
    unsafe {
        let owner = (*info).owner;
        (*info).owner = std::ptr::null_mut();
        if !owner.is_null() {
            drop(Box::from_raw(owner));
        }
        if flags & 1 != 0 {
            if let Some(api) = API.get() {
                (api.free)(info.cast());
            }
        }
        info
    }
}
impl Api {
    fn allocate(&self, size: usize) -> Result<*mut c_void, String> {
        // SAFETY: Native exported allocator; no diagnostic strings retained.
        let memory = unsafe { (self.allocate)(size, std::ptr::null(), 0, std::ptr::null()) };
        if memory.is_null() {
            Err("Ogre allocation failed".into())
        } else {
            Ok(memory)
        }
    }
    fn shared(&self, object: usize, owner: Owner) -> Result<Shared, String> {
        let info = self.allocate(std::mem::size_of::<Info>())?.cast::<Info>();
        // SAFETY: Native allocation has sufficient alignment and size. Inspected
        // SharedPtr code only increments/decrements its counter at offset 0x20.
        unsafe {
            info.write(Info {
                table: &INFO_TABLE,
                mutex_storage: [0; 24],
                count: AtomicU32::new(1),
                padding: 0,
                owner: Box::into_raw(Box::new(owner)),
            });
        }
        Ok(Shared {
            object,
            info: info as usize,
        })
    }
    pub fn stream(&self, mut bytes: Vec<u8>) -> Result<OwnedShared, String> {
        let object = self.allocate(0x60)?;
        // SAFETY: Verified constructor uses 0x60 bytes. Buffer remains stable in
        // its owner; freeOnClose=false keeps the native CRT from freeing Rust data.
        unsafe {
            (self.construct)(object, bytes.as_mut_ptr().cast(), bytes.len(), false, true);
        }
        self.shared(
            object as usize,
            Owner::Stream {
                object: object as usize,
                _bytes: bytes,
            },
        )
        .map(OwnedShared)
    }
    /// Image::load adopts this native buffer and turns stream freeOnClose off.
    pub fn pixel_stream(&self, bytes: &[u8]) -> Result<OwnedShared, String> {
        let object = self.allocate(0x60)?;
        let buffer = match self.allocate(bytes.len()) {
            Ok(buffer) => buffer,
            Err(error) => {
                // SAFETY: Unconstructed allocation belongs solely to this call.
                unsafe {
                    (self.free)(object);
                }
                return Err(error);
            }
        };
        // SAFETY: Matching native allocations. The stream initially owns its
        // buffer, and native Image::load can transfer it to the image safely.
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), buffer.cast(), bytes.len());
            (self.construct)(object, buffer, bytes.len(), true, true);
        }
        self.shared(
            object as usize,
            Owner::NativeStream {
                object: object as usize,
            },
        )
        .map(OwnedShared)
    }
    pub fn metadata(
        &self,
        width: u32,
        height: u32,
        format: u32,
        size: usize,
    ) -> Result<OwnedShared, String> {
        let object = self.allocate(48)?;
        // SAFETY: Inspected ImageData ABI: vptr, h,w,d,pad,size,mips,pad,flags,
        // format,pad. Its dataType method shares ImageCodec's native implementation.
        unsafe {
            object.cast::<u8>().write_bytes(0, 48);
            object.cast::<*const MetadataTable>().write(&METADATA_TABLE);
            object.cast::<u8>().add(8).cast::<u32>().write(height);
            object.cast::<u8>().add(12).cast::<u32>().write(width);
            object.cast::<u8>().add(16).cast::<u32>().write(1);
            object.cast::<u8>().add(24).cast::<usize>().write(size);
            object.cast::<u8>().add(40).cast::<u32>().write(format);
        }
        self.shared(
            object as usize,
            Owner::Metadata {
                object: object as usize,
            },
        )
        .map(OwnedShared)
    }
    /// Replace a native shared pointer, transferring and releasing its references.
    /// # Safety
    /// destination must be aligned, writable storage for a live native SharedPtr
    /// from this engine, owning its previous reference. It must not be concurrently used.
    pub unsafe fn replace(&self, destination: *mut Shared, replacement: OwnedShared) {
        // SAFETY: Caller supplies a live writable native SharedPtr. Replacing it
        // transfers our reference and releases precisely the previous reference.
        unsafe {
            let mut old = destination.read();
            destination.write(replacement.into_raw());
            (self.release)(&mut old);
        }
    }
}
#[repr(C)]
struct MetadataTable {
    destroy: unsafe extern "C" fn(*mut c_void, u32) -> *mut c_void,
    data_type: unsafe extern "C" fn(*const c_void, *mut NativeString) -> *mut NativeString,
}
static METADATA_TABLE: MetadataTable = MetadataTable {
    destroy: destroy_metadata,
    data_type: metadata_type,
};
unsafe extern "C" fn destroy_metadata(object: *mut c_void, flags: u32) -> *mut c_void {
    if flags & 1 != 0 {
        if let Some(api) = API.get() {
            // SAFETY: Owned metadata was allocated by this native allocator.
            unsafe {
                (api.free)(object);
            }
        }
    }
    object
}
unsafe extern "C" fn metadata_type(
    _: *const c_void,
    output: *mut NativeString,
) -> *mut NativeString {
    // SAFETY: Native caller supplies writable sret storage for a 32-byte string.
    unsafe {
        output.write(NativeString::short(b"ImageData"));
    }
    output
}
#[derive(Debug)]
pub struct OwnedShared(Shared);
impl OwnedShared {
    /// Pointer for synchronous native calls. The caller must preserve this owned
    /// reference and use the pointer only while this value remains live.
    pub fn as_mut_ptr(&mut self) -> *mut Shared {
        &mut self.0
    }
    pub fn into_raw(mut self) -> Shared {
        std::mem::take(&mut self.0)
    }
}
impl Drop for OwnedShared {
    fn drop(&mut self) {
        if let Some(api) = API.get() {
            // SAFETY: Exactly one owned native SharedPtr reference, possibly null.
            unsafe {
                (api.release)(&mut self.0);
            }
        }
    }
}
#[repr(C)]
#[derive(Debug)]
pub struct NativeString {
    storage: [u8; 16],
    length: usize,
    capacity: usize,
}
impl NativeString {
    pub fn short(text: &[u8]) -> Self {
        assert!(
            text.len() < 16,
            "native inline string needs fewer than 16 bytes, got {}",
            text.len()
        );
        let mut storage = [0; 16];
        storage[..text.len()].copy_from_slice(text);
        Self {
            storage,
            length: text.len(),
            capacity: 15,
        }
    }
}
/// Copy a string from the verified engine's VC100 representation.
/// # Safety
/// input must be null or a live, aligned native std::string whose storage remains
/// readable and unmodified throughout this synchronous copy.
pub unsafe fn string(input: *const c_void) -> Option<Vec<u8>> {
    if input.is_null() {
        return None;
    }
    // SAFETY: Layout corroborated by DDSListener and native string construction.
    unsafe {
        let value = &*input.cast::<NativeString>();
        if value.length > value.capacity || value.length > 8192 {
            return None;
        }
        let pointer = if value.capacity < 16 {
            value.storage.as_ptr()
        } else {
            input.cast::<*const u8>().read()
        };
        if pointer.is_null() {
            return None;
        }
        Some(std::slice::from_raw_parts(pointer, value.length).to_vec())
    }
}
/// Read a range from a verified native DataStream, changing its seek position.
/// # Safety
/// stream must borrow a live native DataStream from this engine for this call.
/// Its object must not be destroyed or concurrently read/seeked during the call.
pub unsafe fn read(stream: &Shared, offset: usize, buffer: &mut [u8]) -> Result<(), String> {
    let api = API.get().ok_or("Ogre not initialized")?;
    if stream.object == 0 {
        return Err("null native stream".into());
    }
    // SAFETY: Native stream and its verified virtual table live for this call.
    unsafe {
        let table = *(stream.object as *const usize);
        if ![0x6810d8, 0x681158, 0x6811d8]
            .iter()
            .any(|rva| table == api.base + rva)
        {
            return Err("unrecognized native stream type".into());
        }
        let seek: unsafe extern "C-unwind" fn(usize, usize) =
            std::mem::transmute(*((table + 10 * 8) as *const usize));
        let read: unsafe extern "C-unwind" fn(usize, *mut c_void, usize) -> usize =
            std::mem::transmute(*((table + 3 * 8) as *const usize));
        seek(stream.object, offset);
        if read(stream.object, buffer.as_mut_ptr().cast(), buffer.len()) != buffer.len() {
            return Err("short native stream read".into());
        }
    }
    Ok(())
}
/// Query a native DataStream's size.
/// # Safety
/// stream must be null or borrow a live, aligned native DataStream from the
/// verified engine, whose object remains readable throughout this call.
pub unsafe fn stream_size(stream: &Shared) -> Option<usize> {
    if stream.object == 0 {
        None
    } else {
        // SAFETY: Native DataStream base stores mSize at verified offset 0x30.
        Some(unsafe { *((stream.object + 0x30) as *const usize) })
    }
}
