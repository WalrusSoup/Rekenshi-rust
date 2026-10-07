//! Shared native weather rendering and its configurable built-in plugin.
//!
//! The host supplies token authorization when constructing [`Weather`], forwards
//! [`export_api`] through its frozen weather export, and calls [`release`] after
//! revoking an independent plugin. All native rendering and policy live here.
#![deny(missing_docs)]
mod native;
mod plugin;
mod policy;
mod service;

#[doc(inline)]
pub use plugin::Weather;
#[doc(inline)]
pub use service::{export_api, release};
