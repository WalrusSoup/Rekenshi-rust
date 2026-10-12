//! Bounded creation-stack resolution; no resource pointers or COM references.
use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::Mutex;
use windows::Win32::Foundation::HMODULE;
use windows::Win32::System::{Diagnostics::Debug::RtlCaptureStackBackTrace, LibraryLoader::*};
use windows::core::PCWSTR;

// Host-local cache for native callbacks (M-AVOID-STATICS exception).
static FRAMES: Mutex<Option<HashMap<usize, String>>> = Mutex::new(None);

pub(super) fn capture() -> Vec<String> {
    let mut frames = [std::ptr::null_mut::<c_void>(); 32];
    // SAFETY: Windows writes at most the supplied slice length; frame addresses
    // are used only with the documented module lookup API, never dereferenced.
    let count = unsafe { RtlCaptureStackBackTrace(2, &mut frames, None) } as usize;
    let Ok(mut cache) = FRAMES.lock() else {
        return Vec::new();
    };
    let cache = cache.get_or_insert_with(HashMap::new);
    frames[..count]
        .iter()
        .map(|&frame| {
            let key = frame as usize;
            if let Some(name) = cache.get(&key) {
                return name.clone();
            }
            let mut module = HMODULE::default();
            let mut path = [0u16; 1024];
            // SAFETY: FROM_ADDRESS interprets the argument as a code address rather
            // than a string. UNCHANGED_REFCOUNT borrows the already-loaded module.
            let resolved = unsafe {
                GetModuleHandleExW(
                    GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS
                        | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
                    PCWSTR(frame.cast()),
                    &mut module,
                )
            }
            .is_ok();
            let name = if resolved {
                // SAFETY: Valid borrowed module and writable bounded UTF-16 buffer.
                let length = unsafe { GetModuleFileNameW(Some(module), &mut path) } as usize;
                let path = String::from_utf16_lossy(&path[..length.min(path.len())]);
                let file = std::path::Path::new(&path)
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy();
                format!("{file}+0x{:x}", key.saturating_sub(module.0 as usize))
            } else {
                "<unresolved frame>".to_string()
            };
            if cache.len() < 8192 {
                cache.insert(key, name.clone());
            }
            name
        })
        .collect()
}
