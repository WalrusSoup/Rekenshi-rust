use rekenshi_plugin_api::{ERROR, HostV1, OK, PluginV1};
use std::sync::OnceLock;
static HOST: OnceLock<HostV1> = OnceLock::new();
/// # Safety
/// Caller must supply the SDK's writable-output contract.
// SAFETY: Unique export with V1's exact C signature.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rekenshi_plugin_entry(
    abi: u32,
    output: *mut PluginV1,
    size: usize,
) -> i32 {
    // SAFETY: Host guarantees output storage when ABI/size are accepted.
    unsafe {
        rekenshi_plugin_api::export_descriptor(
            abi,
            output,
            size,
            PluginV1::new(
                "fixture-reject",
                "Reject initialization",
                "0.1.0",
                start,
                stop,
            ),
        )
    }
}
unsafe extern "C" fn start(pointer: *const HostV1) -> i32 {
    // SAFETY: Host supplies live V1 storage.
    if let Ok(host) = unsafe { HostV1::read(pointer) } {
        let _ = HOST.set(host);
    }
    ERROR
}
unsafe extern "C" fn stop() -> i32 {
    if let Some(host) = HOST.get() {
        host.write_log("failed start cleaned up");
    }
    OK
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
