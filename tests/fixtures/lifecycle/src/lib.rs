use rekenshi_plugin_api::{ERROR, HostV1, OK, PluginV1};
use std::sync::OnceLock;
static HOST: OnceLock<HostV1> = OnceLock::new();
/// # Safety
/// Caller provides valid writable V1 output for an accepted request.
// SAFETY: Unique export matching the SDK entry signature.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rekenshi_plugin_entry(
    abi: u32,
    output: *mut PluginV1,
    size: usize,
) -> i32 {
    // SAFETY: Host output contract is forwarded to the SDK helper.
    unsafe {
        rekenshi_plugin_api::export_descriptor(
            abi,
            output,
            size,
            PluginV1::new("fixture-lifecycle", "Lifecycle trace", "0.1.0", start, stop),
        )
    }
}
unsafe extern "C" fn start(pointer: *const HostV1) -> i32 {
    // SAFETY: Host supplies a valid table for this synchronous call.
    let host = match unsafe { HostV1::read(pointer) } {
        Ok(host) => host,
        Err(error) => return error,
    };
    let _ = HOST.set(host);
    host.write_log("start");
    // A mod can reserve/release its own mapped code. Another owner cannot release
    // it; calls after stop are denied by the host's token lifecycle.
    let Some(target) = std::ptr::NonNull::new(probe as *const () as *mut std::ffi::c_void) else {
        return ERROR;
    };
    if host.reserve_hook(target).is_err() {
        return ERROR;
    }
    if host.relinquish_hook(target).is_err() {
        return ERROR;
    }
    OK
}
unsafe extern "C" fn stop() -> i32 {
    if let Some(host) = HOST.get() {
        host.write_log("stop");
        if std::ptr::NonNull::new(probe as *const () as *mut std::ffi::c_void)
            .is_some_and(|target| host.reserve_hook(target) != Err(ERROR))
        {
            return ERROR;
        }
    }
    OK
}
extern "C" fn probe() {}
