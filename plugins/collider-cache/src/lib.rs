//! Internal collider-cache plugin, linked into RE_Kenshi_Rust.dll.
mod native;
mod plugin;
#[doc(inline)]
pub use plugin::{ColliderCache, hits};
use rekenshi_native::ogre;
