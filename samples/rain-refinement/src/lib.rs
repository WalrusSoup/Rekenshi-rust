//! Independent rain refinement plugin using the safe host-owned weather service.
#![forbid(unsafe_code)]
mod plugin;
#[doc(inline)]
pub use plugin::RainRefinement;
rekenshi_plugin_sdk::export_plugin!(RainRefinement);
