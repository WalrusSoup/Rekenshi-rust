//! Resolve known names once, cache pinned process-local addresses, and expose
//! ownership snapshots. Verified UI/vtable providers publish their real bodies.
use crate::{extensions, logging, plugin};
use rekenshi_native::symbols::ogre::root;
use rekenshi_plugin_api::{ERROR, HOOK_BUSY, INCOMPATIBLE, OK, hooks::*};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::ffi::{CStr, c_void};
use std::sync::{Mutex, OnceLock};
use windows_sys::Win32::Foundation::{FreeLibrary, HMODULE};
use windows_sys::Win32::System::LibraryLoader::*;

const COMPILER_HASH: &str = "44C3A7E330B54A35A9EFA015831392593AA02E7DA1460BE429D17C3644850E8A";
#[derive(Default)]
struct Catalog {
    addresses: HashMap<HookId, usize>,
    compilers: HashMap<usize, bool>,
}
static CATALOG: OnceLock<Mutex<Catalog>> = OnceLock::new();
fn catalog() -> &'static Mutex<Catalog> {
    CATALOG.get_or_init(|| Mutex::new(Catalog::default()))
}

/// Central startup prefill. Optional modules do not become bootstrap failures.
pub(crate) fn warm_up() {
    // Shader compiler is a game-bundled dependency, loaded only after verifying
    // its identity. Other targets are already published during core preparation.
    if let Some(path) = logging::directory().map(|d| d.join("D3DCompiler_43.dll")) {
        if std::fs::read(&path)
            .is_ok_and(|bytes| format!("{:X}", Sha256::digest(bytes)) == COMPILER_HASH)
        {
            use std::os::windows::ffi::OsStrExt;
            let wide: Vec<_> = path.as_os_str().encode_wide().chain(Some(0)).collect();
            // SAFETY: Absolute terminated verified path, called outside DllMain.
            let module = unsafe {
                LoadLibraryExW(
                    wide.as_ptr(),
                    std::ptr::null_mut(),
                    LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_DEFAULT_DIRS,
                )
            };
            if !module.is_null() {
                let _ = plugin::pin(module.cast());
                // SAFETY: Release exactly our loader reference after pinning.
                unsafe { FreeLibrary(module) };
            }
        }
    }
    for id in HookId::ALL {
        if let Ok(address) = target(id) {
            logging::write(&format!("hook catalog: {} = {address:p}", id.name()));
        }
    }
}
pub(crate) fn publish_core(id: HookId, address: *const c_void) -> Result<(), String> {
    plugin::pin(address.cast())?;
    let mut c = catalog().lock().map_err(|_| "hook catalog poisoned")?;
    store(&mut c, id, address as usize).map_err(|e| format!("catalog publication conflict: {e}"))
}
fn store(c: &mut Catalog, id: HookId, address: usize) -> Result<(), i32> {
    if address == 0 {
        return Err(ERROR);
    }
    match c.addresses.get(&id) {
        Some(old) if *old != address => Err(HOOK_BUSY),
        _ => {
            c.addresses.insert(id, address);
            Ok(())
        }
    }
}
pub(crate) fn target(id: HookId) -> Result<*const c_void, i32> {
    let mut c = catalog().lock().map_err(|_| ERROR)?;
    if let Some(address) = c.addresses.get(&id) {
        return Ok(*address as *const c_void);
    }
    let (module, export) = match id {
        HookId::OgreRootSingleton => (c"OgreMain_x64.dll", root::GET_SINGLETON_PTR),
        HookId::D3DCompile => (c"D3DCompiler_43.dll", c"D3DCompile"),
        HookId::D3DCreateBlob => (c"D3DCompiler_43.dll", c"D3DCreateBlob"),
        // An export thunk is not the UI implementation; never guess its body.
        HookId::MyGuiSetCaption | HookId::D3D11Draw => return Err(UNAVAILABLE),
    };
    let handle = loaded(module).ok_or(UNAVAILABLE)?;
    if matches!(id, HookId::D3DCompile | HookId::D3DCreateBlob) {
        let verified = *c
            .compilers
            .entry(handle as usize)
            .or_insert_with(|| verify_compiler(handle));
        if !verified {
            return Err(UNAVAILABLE);
        }
    }
    // SAFETY: Module is pinned; export C string remains valid throughout lookup.
    let symbol = unsafe { GetProcAddress(handle, export.as_ptr().cast()) }.ok_or(UNAVAILABLE)?;
    plugin::pin(symbol as *const ()).map_err(|_| ERROR)?;
    let address = symbol as usize;
    store(&mut c, id, address)?;
    Ok(address as *const c_void)
}
fn loaded(name: &CStr) -> Option<HMODULE> {
    let wide: Vec<_> = name.to_str().ok()?.encode_utf16().chain(Some(0)).collect();
    let mut handle = std::ptr::null_mut();
    // SAFETY: Terminated name and writable output; pin before looking up code.
    (unsafe { GetModuleHandleExW(GET_MODULE_HANDLE_EX_FLAG_PIN, wide.as_ptr(), &mut handle) } != 0)
        .then_some(handle)
}
fn verify_compiler(handle: HMODULE) -> bool {
    let mut path = vec![0u16; 32768];
    // SAFETY: Loaded module and writable output buffer.
    let n = unsafe { GetModuleFileNameW(handle, path.as_mut_ptr(), path.len() as u32) } as usize;
    if n == 0 || n >= path.len() {
        return false;
    }
    let Ok(path) = String::from_utf16(&path[..n]) else {
        return false;
    };
    std::fs::read(path).is_ok_and(|bytes| format!("{:X}", Sha256::digest(bytes)) == COMPILER_HASH)
}

/// # Safety
/// Accepted requests provide aligned writable HooksV1 storage.
// SAFETY: Unique versioned C export; HostV1's existing layout stays frozen.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rekenshi_hooks_api(
    version: u32,
    output: *mut HooksV1,
    size: usize,
) -> i32 {
    if version != VERSION || size != size_of::<HooksV1>() {
        return INCOMPATIBLE;
    }
    if output.is_null() {
        return ERROR;
    }
    // SAFETY: Accepted caller guarantees complete writable output storage.
    unsafe {
        output.write(HooksV1 {
            struct_size: size_of::<HooksV1>() as u32,
            abi_version: VERSION,
            resolve,
            publish,
        })
    };
    OK
}
unsafe extern "C" fn resolve(id: u32, output: *mut HookInfoV1, size: usize) -> i32 {
    std::panic::catch_unwind(|| {
        if size != size_of::<HookInfoV1>() {
            return INCOMPATIBLE;
        }
        if output.is_null() {
            return ERROR;
        }
        let Some(id) = HookId::from_raw(id) else {
            return INCOMPATIBLE;
        };
        let address = match target(id) {
            Ok(p) => p,
            Err(e) => return e,
        };
        let owner = match extensions::hook_owner(address) {
            Ok(o) => o,
            Err(e) => return e,
        };
        // SAFETY: V1 callback caller guarantees writable aligned output storage.
        unsafe {
            output.write(HookInfoV1 {
                struct_size: size_of::<HookInfoV1>() as u32,
                id: id as u32,
                address,
                owner_token: owner.unwrap_or(0),
                status: if owner.is_some() { RESERVED } else { AVAILABLE },
                reserved: 0,
            })
        };
        OK
    })
    .unwrap_or(ERROR)
}
unsafe extern "C" fn publish(token: u64, id: u32, address: *const c_void) -> i32 {
    std::panic::catch_unwind(|| {
        if HookId::from_raw(id) != Some(HookId::D3D11Draw) {
            return INCOMPATIBLE;
        }
        if !extensions::owns_active_hook(token, address) {
            return ERROR;
        }
        match publish_core(HookId::D3D11Draw, address) {
            Ok(()) => OK,
            Err(_) => HOOK_BUSY,
        }
    })
    .unwrap_or(ERROR)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn published_addresses_cannot_be_replaced_or_null() {
        let mut c = Catalog::default();
        assert_eq!(store(&mut c, HookId::D3D11Draw, 0), Err(ERROR));
        assert!(store(&mut c, HookId::D3D11Draw, 123).is_ok());
        assert!(store(&mut c, HookId::D3D11Draw, 123).is_ok());
        assert_eq!(store(&mut c, HookId::D3D11Draw, 456), Err(HOOK_BUSY));
        assert_eq!(c.addresses[&HookId::D3D11Draw], 123);
    }
}
