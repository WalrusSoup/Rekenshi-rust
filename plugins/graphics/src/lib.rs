//! Internal graphics plugin, linked into RE_Kenshi_Rust.dll.
mod native;
mod plugin;
mod ssao;
#[doc(inline)]
pub use plugin::{GraphicsBackend, export_api, export_api_v2, passes, stop};
#[doc(inline)]
pub use ssao::lighting_shader;
