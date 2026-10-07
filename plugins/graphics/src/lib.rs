//! Internal graphics plugin, linked into RE_Kenshi_Rust.dll.
mod native;
mod plugin;
#[doc(inline)]
pub use plugin::{
    GraphicsBackend, PostProcess, export_api, export_api_v2, passes, start_legacy, stop,
};
