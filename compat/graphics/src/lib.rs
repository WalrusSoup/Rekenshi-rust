//! Legacy DLL adapter; internal plugin implementation lives in plugins/graphics.
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
    // SAFETY: Forward the host's output contract unchanged.
    unsafe {
        rekenshi_plugin_api::export_descriptor(
            abi,
            output,
            size,
            PluginV1::new(
                "graphics",
                "D3D11 graphics service",
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
            rekenshi_graphics::start_legacy(host, root)
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
    std::panic::catch_unwind(|| match rekenshi_graphics::stop() {
        Ok(()) => OK,
        Err(_) => ERROR,
    })
    .unwrap_or(ERROR)
}
/// # Safety
/// Negotiated calls supply aligned writable GraphicsV1 output storage.
// SAFETY: Unique versioned graphics service export.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rekenshi_graphics_api(
    version: u32,
    output: *mut rekenshi_plugin_api::graphics::GraphicsV1,
    size: usize,
) -> i32 {
    // SAFETY: Forward the service's caller contract unchanged.
    unsafe { rekenshi_graphics::export_api(version, output, size) }
}
// SAFETY: Unique scalar diagnostic export.
#[unsafe(no_mangle)]
pub extern "C" fn rekenshi_graphics_passes() -> u64 {
    rekenshi_graphics::passes()
}
