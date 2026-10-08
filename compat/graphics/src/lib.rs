//! Independently compiled SDK plugin using the same public registrations as bundled policy.
rekenshi_plugin_sdk::export_plugin!(rekenshi_graphics::GraphicsBackend);
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
