//! Internal dds-mip-skip plugin, linked into RE_Kenshi_Rust.dll.
mod native;
mod plugin;
#[doc(inline)]
pub use plugin::{DdsMipSkip, skipped_bytes};
use rekenshi_native::ogre;
