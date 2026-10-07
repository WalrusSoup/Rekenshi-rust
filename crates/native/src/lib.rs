//! Windows module ownership helpers shared by native mods.
pub mod ogre;
pub mod symbols;
use std::ffi::{CStr, c_void};
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use windows_sys::Win32::System::LibraryLoader::*;

/// Find the directory containing a function in this DLL.
pub fn directory(address: *const ()) -> Result<PathBuf, String> {
    let mut module = std::ptr::null_mut();
    // SAFETY: FROM_ADDRESS treats the pointer as an address, not a string.
    if unsafe {
        GetModuleHandleExW(
            GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
            address.cast(),
            &mut module,
        )
    } == 0
    {
        return Err("locate module".into());
    }
    let mut path = vec![0u16; 32768];
    // SAFETY: Live module and writable UTF-16 output buffer.
    let n = unsafe { GetModuleFileNameW(module, path.as_mut_ptr(), path.len() as u32) } as usize;
    if n == 0 || n >= path.len() {
        return Err("module path".into());
    }
    let path = PathBuf::from(String::from_utf16(&path[..n]).map_err(|_| "module path encoding")?);
    path.parent()
        .map(Path::to_path_buf)
        .ok_or("module parent".into())
}

/// Load an absolute DLL path and pin it for process life.
pub fn load_pinned(path: &Path) -> Result<usize, String> {
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    // SAFETY: Absolute terminated path and restricted dependency search.
    let module = unsafe {
        LoadLibraryExW(
            wide.as_ptr(),
            std::ptr::null_mut(),
            LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_DEFAULT_DIRS,
        )
    };
    if module.is_null() {
        return Err(format!(
            "load {}: {}",
            path.display(),
            std::io::Error::last_os_error()
        ));
    }
    let mut pinned = std::ptr::null_mut();
    // SAFETY: Pin the loaded module by its mapped address before releasing our reference.
    let success = unsafe {
        GetModuleHandleExW(
            GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_PIN,
            module.cast(),
            &mut pinned,
        )
    };
    // SAFETY: Balance exactly the reference returned by LoadLibraryExW.
    unsafe { windows_sys::Win32::Foundation::FreeLibrary(module) };
    if success == 0 {
        return Err("pin module".into());
    }
    Ok(pinned as usize)
}

pub fn export(module: usize, name: &CStr) -> Result<*const c_void, String> {
    // SAFETY: Callers use handles returned by load_pinned; name is terminated.
    unsafe { GetProcAddress(module as _, name.as_ptr().cast()) }
        .map(|symbol| symbol as *const c_void)
        .ok_or_else(|| format!("missing export {name:?}"))
}

/// Read a small optional TOML file; malformed settings fail initialization.
pub fn settings<T: serde::de::DeserializeOwned + Default>(path: &Path) -> Result<T, String> {
    match std::fs::read(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(T::default()),
        Err(error) => Err(error.to_string()),
        Ok(bytes) if bytes.len() > 65536 => Err("settings exceed 64 KiB".into()),
        Ok(bytes) => toml::from_str(std::str::from_utf8(&bytes).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string()),
    }
}
