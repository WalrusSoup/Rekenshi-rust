//! Internal graphics plugin, linked into RE_Kenshi_Rust.dll.
mod landscape;
mod native;
mod plugin;
mod resources;
mod ssao;
/// Refresh the opt-in graphics resource inventory on the existing UI lane.
pub fn resource_capture_tick() -> Result<(), String> {
    native::capture::tick()
}
/// Copied counters for the optional runtime scenery optimizer.
pub fn landscape_diagnostics() -> String {
    native::instancing::diagnostics()
}
#[doc(inline)]
pub use plugin::{GraphicsBackend, export_api, export_api_v2, passes, stop};
#[doc(inline)]
pub use ssao::lighting_shader;
