use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};
use windows_sys::Win32::System::Diagnostics::Debug::OutputDebugStringW;
use windows_sys::Win32::System::LibraryLoader::{
    GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS, GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
    GetModuleFileNameW, GetModuleHandleExW,
};

static LOG_PATH: OnceLock<Option<PathBuf>> = OnceLock::new();

pub(crate) fn write(message: &str) {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs());
    let line = format!("[{seconds}] RE_Kenshi_Rust: {message}\n");
    let debug: Vec<u16> = line.encode_utf16().chain(Some(0)).collect();
    // SAFETY: debug is a live, null-terminated UTF-16 buffer for this call.
    unsafe { OutputDebugStringW(debug.as_ptr()) };
    if let Some(path) = LOG_PATH.get_or_init(path) {
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
            let _ = file.write_all(line.as_bytes());
        }
    }
}

fn path() -> Option<PathBuf> {
    Some(directory()?.join("RE_Kenshi_Rust.log"))
}

pub(crate) fn directory() -> Option<PathBuf> {
    let mut module = std::ptr::null_mut();
    // SAFETY: FROM_ADDRESS interprets the second argument as a mapped address,
    // not a string. This function is in our DLL; the output pointer is valid.
    let found = unsafe {
        GetModuleHandleExW(
            GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
            directory as *const () as *const u16,
            &mut module,
        )
    };
    if found == 0 {
        return None;
    }
    let mut buffer = vec![0u16; 32768];
    // SAFETY: module is mapped and buffer has the declared writable capacity.
    let length = unsafe { GetModuleFileNameW(module, buffer.as_mut_ptr(), 32768) } as usize;
    if length == 0 || length >= buffer.len() {
        return None;
    }
    use std::os::windows::ffi::OsStringExt;
    let dll_path = PathBuf::from(std::ffi::OsString::from_wide(&buffer[..length]));
    Some(dll_path.parent()?.to_path_buf())
}
