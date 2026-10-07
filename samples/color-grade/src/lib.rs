//! Independent color-grade sample DLL.
mod native;
mod plugin;
#[doc(inline)]
pub use plugin::{ColorGrade, start_legacy, stop};
