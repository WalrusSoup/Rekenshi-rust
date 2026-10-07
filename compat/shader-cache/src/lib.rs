//! Legacy DLL adapter; internal plugin implementation lives in plugins/shader-cache.
use rekenshi_plugin_api::{ERROR, HostV1, OK, PluginV1};
/// # Safety
/// Caller supplies the SDK's negotiated writable descriptor buffer.
// SAFETY: Unique V1 export with the SDK's declared signature.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rekenshi_plugin_entry(
    abi: u32,
    output: *mut PluginV1,
    size: usize,
) -> i32 {
    // SAFETY: Forward the caller's output contract unchanged.
    unsafe {
        rekenshi_plugin_api::export_descriptor(
            abi,
            output,
            size,
            PluginV1::new(
                "shader-cache",
                "Persistent shader cache",
                env!("CARGO_PKG_VERSION"),
                start,
                stop,
            ),
        )
    }
}
unsafe extern "C" fn start(pointer: *const HostV1) -> i32 {
    std::panic::catch_unwind(|| {
        // SAFETY: Loader supplies the negotiated readable host table.
        let host = match unsafe { HostV1::read(pointer) } {
            Ok(host) => host,
            Err(e) => return e,
        };
        let result = (|| {
            let plugins = rekenshi_native::directory(start as *const ())?;
            let root = plugins.parent().ok_or("plugin folder")?;
            rekenshi_shader_cache::start_legacy(host, root)
        })();
        match result {
            Ok(()) => OK,
            Err(e) => {
                host.write_log(&e.to_string());
                ERROR
            }
        }
    })
    .unwrap_or(ERROR)
}
unsafe extern "C" fn stop() -> i32 {
    std::panic::catch_unwind(|| match rekenshi_shader_cache::stop() {
        Ok(()) => OK,
        Err(_) => ERROR,
    })
    .unwrap_or(ERROR)
}
// SAFETY: Unique scalar diagnostic export.
#[unsafe(no_mangle)]
pub extern "C" fn rekenshi_shader_cache_hits() -> u64 {
    rekenshi_shader_cache::hits()
}
