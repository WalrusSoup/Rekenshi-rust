//! Original Kenshi frame artwork with dark surfaces and readable text.
//!
//! The host prepares the verified native adapter before mounting [`DarkUi`].
//! It includes these hooks in its shared startup/shutdown transaction, and calls
//! [`frame`] on MyGUI's thread. Settings and reversible colour policy live here.
#![deny(missing_docs)]
mod native;
mod palette;
mod plugin;
#[doc(inline)]
pub use native::{Adapter, disable, enable, frame, prepare};
#[doc(inline)]
pub use plugin::DarkUi;
