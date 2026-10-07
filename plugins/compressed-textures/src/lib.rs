//! Internal compressed-textures plugin, linked into RE_Kenshi_Rust.dll.
mod native;
mod plugin;
#[doc(inline)]
pub use plugin::{CompressedTextures, decoded, sidecar_hits};
use rekenshi_native::ogre;
use rekenshi_sic as sic;
