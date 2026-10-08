//! Example independent mod: records session duration and Ogre forwarding calls.
//! It validates host services; it does not claim any FPS improvement.
use rekenshi_plugin_api::{ERROR, HOOK_BUSY, HostV1, OK, PluginV1, hooks::HookId};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

struct Session {
    host: HostV1,
    began: Instant,
    calls: u64,
}
static SESSION: Mutex<Option<Session>> = Mutex::new(None);
static STARTS: OnceLock<Mutex<u32>> = OnceLock::new();

/// # Safety
/// The caller must meet export_descriptor's writable-output contract.
// SAFETY: Unique export with the SDK's V1 entry signature.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rekenshi_plugin_entry(
    abi: u32,
    output: *mut PluginV1,
    size: usize,
) -> i32 {
    // SAFETY: Forward the host's output contract to the SDK helper.
    unsafe {
        rekenshi_plugin_api::export_descriptor(
            abi,
            output,
            size,
            PluginV1::new(
                "diagnostics",
                "Session diagnostics",
                env!("CARGO_PKG_VERSION"),
                start,
                stop,
            ),
        )
    }
}

unsafe extern "C" fn start(pointer: *const HostV1) -> i32 {
    std::panic::catch_unwind(|| {
        // SAFETY: V1 host provides a valid stable table for this call.
        let host = match unsafe { HostV1::read(pointer) } {
            Ok(host) => host,
            Err(error) => return error,
        };
        let Ok(catalog) = host.hooks() else {
            return ERROR;
        };
        let Ok(getter) = catalog.target(HookId::OgreRootSingleton) else {
            return ERROR;
        };
        // The bootstrap owns this target. Verify cooperative conflict rejection.
        if host.reserve_hook(getter) != Err(HOOK_BUSY) {
            return ERROR;
        }
        let Ok(mut session) = SESSION.lock() else {
            return ERROR;
        };
        if session.is_some() {
            return ERROR;
        }
        let Ok(mut starts) = STARTS.get_or_init(|| Mutex::new(0)).lock() else {
            return ERROR;
        };
        *starts += 1;
        let calls = host.ogre_hook_calls();
        host.write_log(&format!(
            "started #{starts}; host interface and hook ownership checks passed; Ogre calls={calls}"
        ));
        *session = Some(Session {
            host,
            began: Instant::now(),
            calls,
        });
        OK
    })
    .unwrap_or(ERROR)
}

unsafe extern "C" fn stop() -> i32 {
    std::panic::catch_unwind(|| {
        let Ok(mut session) = SESSION.lock() else {
            return ERROR;
        };
        if let Some(session) = session.take() {
            let calls = session.host.ogre_hook_calls().saturating_sub(session.calls);
            session.host.write_log(&format!(
                "stopped; session={:.2}s; forwarded Ogre calls={calls}",
                session.began.elapsed().as_secs_f64()
            ));
        }
        OK
    })
    .unwrap_or(ERROR)
}

/// Prepare this scalar-only ABI fixture; it owns no desired operation registrations.
/// # Safety
/// Host provides a stable negotiated table.
// SAFETY: Unique preparation export implementing the host's current C contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rekenshi_plugin_prepare(
    version: u32,
    _: *const rekenshi_plugin_api::HostV1,
) -> i32 {
    if version == 1 {
        rekenshi_plugin_api::OK
    } else {
        rekenshi_plugin_api::INCOMPATIBLE
    }
}
