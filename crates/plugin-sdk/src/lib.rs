//! Safe plugin lifecycle, callbacks, gameplay observations, and host-owned rendering effects.
//!
//! Implement [`Plugin`] and use [`export_plugin!`] in a Windows x64 `cdylib`.
//! The loader creates a [`Context`] at startup; use its host for managed callbacks
//! and optional services. Native tables and allocations remain in their owning DLL.
//!
//! Register callbacks once during startup. They may execute concurrently and must
//! own their captures. A panic quarantines every callback belonging to that plugin
//! and requests a Kenshi restart; a quarantined plugin cannot register new callbacks.
//! Shutdown waits for admitted callbacks before invoking the plugin's cleanup.
//!
//! [`PostProcess`] and [`RainRendering`] release host registrations on drop.
//! [`gameplay`] copies snapshots and queues commands for the game update thread.
//! Services can be absent on an unsupported host or game build. Inspect [`Error`]
//! for a host status or original cause instead of parsing its display text.
//!
//! ```
//! use rekenshi_plugin_sdk::{Context, Error, Plugin, Settings};
//! use std::sync::{Arc, atomic::{AtomicU64, Ordering}};
//!
//! #[derive(Default)]
//! struct Observer { frames: Arc<AtomicU64> }
//! impl Plugin for Observer {
//!     fn id(&self) -> &'static str { "observer" }
//!     fn install(&mut self, context: &Context, _: &Settings) -> Result<(), Error> {
//!         let frames = self.frames.clone();
//!         context.host.hook_ui_frame(move |_| { frames.fetch_add(1, Ordering::Relaxed); })?;
//!         Ok(())
//!     }
//! }
//! ```
//!
//! Return [`Error`] from `register`, `install`, `settings_changed`, and `stop`.
//! Use [`gameplay::Gameplay`] as its canonical path.
#![deny(missing_docs)]
pub mod adapters;
pub mod codecs;
mod dll;
pub mod events;
pub mod gameplay;
mod graphics;
pub mod operations;
pub mod policies;
pub mod render;
mod transport;
mod weather;

pub(crate) use _private::abi;
#[doc(inline)]
pub use graphics::PostProcess;
/// Implementation paths used by the exported DLL macro.
#[doc(hidden)]
pub mod _private {
    pub use super::dll::Dll;
    pub use rekenshi_plugin_api as abi;
}
pub use rekenshi_plugin_runtime::{
    Context, Control, Error, Hook, Plugin, Setting, Settings, Value,
};
#[doc(inline)]
pub use weather::RainRendering;

/// Export an independently loadable plugin. Implement Default + Plugin; build
/// this crate as cdylib for Windows x64. No Rust trait objects cross DLLs.
#[macro_export]
macro_rules! export_plugin {
    ($plugin:ty) => {
        mod __rekenshi_export {
            use super::*;
            static REKENSHI_PLUGIN: $crate::_private::Dll<$plugin> = $crate::_private::Dll::new();
            /// Negotiate the plugin descriptor and its lifecycle entry points.
            ///
            /// # Safety
            /// Caller supplies negotiated writable PluginV1 output.
            // SAFETY: Unique plugin DLL entry, generated from the SDK contract.
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn rekenshi_plugin_entry(
                abi: u32,
                output: *mut $crate::_private::abi::PluginV1,
                size: usize,
            ) -> i32 {
                // SAFETY: Forward the loader's negotiated output contract.
                unsafe {
                    REKENSHI_PLUGIN.entry(abi, output, size, start, stop, env!("CARGO_PKG_VERSION"))
                }
            }
            unsafe extern "C" fn start(host: *const $crate::_private::abi::HostV1) -> i32 {
                // SAFETY: Loader supplies a readable negotiated HostV1 table.
                unsafe { REKENSHI_PLUGIN.start(host) }
            }
            /// Commit desired registrations before the host activates native providers.
            ///
            /// # Safety
            /// Host supplies a stable negotiated HostV1 table.
            // SAFETY: Unique mandatory SDK preparation export; frozen PluginV1 unchanged.
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn rekenshi_plugin_prepare(
                version: u32,
                host: *const $crate::_private::abi::HostV1,
            ) -> i32 {
                if version != 1 {
                    return $crate::_private::abi::INCOMPATIBLE;
                }
                // SAFETY: Forward the loader's stable host table contract.
                unsafe { REKENSHI_PLUGIN.prepare(host) }
            }
            unsafe extern "C" fn stop() -> i32 {
                REKENSHI_PLUGIN.stop()
            }
            /// Deliver copied lifecycle transitions at the verified game dispatch point.
            // SAFETY: Unique optional scalar-only lifecycle delivery export.
            #[unsafe(no_mangle)]
            pub extern "C" fn rekenshi_plugin_dispatch(version: u32) -> i32 {
                REKENSHI_PLUGIN.dispatch(version)
            }
            /// Negotiate the plugin's declarative settings interface.
            ///
            /// # Safety
            /// Caller supplies negotiated writable PluginUiV1 output.
            // SAFETY: Optional independent settings table, generated by the SDK.
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn rekenshi_plugin_ui(
                version: u32,
                output: *mut $crate::_private::abi::services::PluginUiV1,
                size: usize,
            ) -> i32 {
                // SAFETY: Forward the caller's writable output contract.
                unsafe {
                    $crate::_private::Dll::<$plugin>::ui(version, output, size, count, row, set)
                }
            }
            extern "C" fn count() -> u32 {
                REKENSHI_PLUGIN.count()
            }
            unsafe extern "C" fn row(
                index: u32,
                output: *mut $crate::_private::abi::services::SettingV1,
                size: usize,
            ) -> i32 {
                // SAFETY: Host supplies exact-sized writable row output.
                unsafe { REKENSHI_PLUGIN.row(index, output, size) }
            }
            unsafe extern "C" fn set(key: *const u8, len: usize, kind: u32, value: f64) -> i32 {
                // SAFETY: Host supplies bounded readable UTF-8 key input.
                unsafe { REKENSHI_PLUGIN.set(key, len, kind, value) }
            }
        }
    };
}
