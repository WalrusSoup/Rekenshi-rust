//! Host transport for independently compiled SDK plugins.
use rekenshi_plugin_api::{ERROR, INCOMPATIBLE, OK, services::*};
use rekenshi_plugin_runtime::{HookEvents, Host};
use std::{
    collections::HashMap,
    ffi::c_void,
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicU64, Ordering},
    },
};
static SUBS: OnceLock<Mutex<HashMap<u64, Subscription>>> = OnceLock::new();
static SERIAL: AtomicU64 = AtomicU64::new(1);
struct Subscription {
    token: u64,
    host: Arc<Host>,
    closed: bool,
}
fn subs() -> &'static Mutex<HashMap<u64, Subscription>> {
    SUBS.get_or_init(Mutex::default)
}
pub(crate) fn release(token: u64) -> Result<(), String> {
    let handles: Vec<_> = subs()
        .lock()
        .map_err(|_| "subscriptions poisoned")?
        .iter()
        .filter(|(_, s)| s.token == token && !s.closed)
        .map(|(id, _)| *id)
        .collect();
    for id in handles {
        if unsubscribe(token, id) != OK {
            return Err("callback cleanup failed".into());
        }
    }
    Ok(())
}
/// # Safety
/// Accepted requests supply aligned writable ServicesV1 storage.
// SAFETY: Unique negotiated SDK service export.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rekenshi_services_api(
    version: u32,
    output: *mut ServicesV1,
    size: usize,
) -> i32 {
    if version != VERSION || size != size_of::<ServicesV1>() {
        return INCOMPATIBLE;
    }
    if output.is_null() {
        return ERROR;
    }
    // SAFETY: Caller owns the negotiated output buffer.
    unsafe {
        output.write(ServicesV1 {
            struct_size: size as u32,
            abi_version: VERSION,
            root,
            dependency,
            supports,
            subscribe,
            unsubscribe,
        });
    }
    OK
}
unsafe extern "C" fn root(token: u64, output: *mut u8, capacity: usize, length: *mut usize) -> i32 {
    std::panic::catch_unwind(|| {
        if !crate::extensions::allowed(token) || length.is_null() {
            return ERROR;
        }
        let Some(path) = crate::logging::directory().map(|d| d.join("RE_Kenshi_Rust")) else {
            return ERROR;
        };
        let Some(path) = path.to_str() else {
            return ERROR;
        };
        // SAFETY: Caller provides writable length, and output when capacity suffices.
        unsafe {
            length.write(path.len());
            if output.is_null() || capacity < path.len() {
                return ERROR;
            }
            std::ptr::copy_nonoverlapping(path.as_ptr(), output, path.len());
        }
        OK
    })
    .unwrap_or(ERROR)
}
unsafe extern "C" fn dependency(token: u64, name: *const u8, len: usize) -> i32 {
    std::panic::catch_unwind(|| {
        if !crate::extensions::allowed(token) || name.is_null() || len == 0 || len > 128 {
            return ERROR;
        }
        // SAFETY: Caller supplies this bounded readable UTF-8 input.
        let Ok(name) = std::str::from_utf8(unsafe { std::slice::from_raw_parts(name, len) }) else {
            return ERROR;
        };
        if crate::internal_plugins::available(name) || crate::extensions::available(name) {
            OK
        } else {
            ERROR
        }
    })
    .unwrap_or(ERROR)
}
extern "C" fn supports(kind: u32) -> bool {
    match kind {
        OGRE_CALLS => crate::rekenshi_status() == 1,
        UI_FRAME => crate::rekenshi_status() == 1 && crate::ui::menu::prepared(),
        _ => false,
    }
}
unsafe extern "C" fn subscribe(
    token: u64,
    kind: u32,
    callback: Event,
    userdata: *mut c_void,
    output: *mut u64,
) -> i32 {
    std::panic::catch_unwind(|| {
        if !crate::extensions::allowed(token)
            || !supports(kind)
            || output.is_null()
            || userdata.is_null()
        {
            return ERROR;
        }
        if crate::plugin::pin(callback as *const ()).is_err() {
            return ERROR;
        }
        let mut subscriptions = match subs().lock() {
            Ok(s) => s,
            Err(_) => return ERROR,
        };
        if subscriptions.len() >= 8192 {
            return ERROR;
        }
        let host = Arc::new(Host::new(
            crate::extensions::host_table(token),
            crate::callbacks::events(),
        ));
        let address = userdata as usize;
        let call = move |delta: f32| {
            // SAFETY: SDK owns userdata until unsubscribe quiesces this group;
            // pinned callback implements Event and contains its own panics.
            unsafe {
                callback(address as *mut c_void, delta);
            }
        };
        let result = match kind {
            OGRE_CALLS => host.hook_ogre_calls(move || call(0.)),
            UI_FRAME => host.hook_ui_frame(call),
            _ => return ERROR,
        };
        if result.is_err() {
            return ERROR;
        }
        let handle = SERIAL.fetch_add(1, Ordering::Relaxed);
        subscriptions.insert(
            handle,
            Subscription {
                token,
                host,
                closed: false,
            },
        );
        // SAFETY: Caller provided writable handle output.
        unsafe {
            output.write(handle);
        }
        OK
    })
    .unwrap_or(ERROR)
}
extern "C" fn unsubscribe(token: u64, handle: u64) -> i32 {
    std::panic::catch_unwind(|| {
        if HookEvents::check_shutdown().is_err() {
            return ERROR;
        }
        let host = {
            let Ok(s) = subs().lock() else { return ERROR };
            let Some(s) = s.get(&handle) else {
                return ERROR;
            };
            if s.token != token {
                return ERROR;
            }
            if s.closed {
                return OK;
            }
            s.host.clone()
        };
        if host.close().is_err() {
            return ERROR;
        }
        if let Ok(mut s) = subs().lock() {
            if let Some(s) = s.get_mut(&handle) {
                s.closed = true;
            }
        }
        OK
    })
    .unwrap_or(ERROR)
}
