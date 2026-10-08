//! Ogre-loadable host for independent DLL mods, with a visible loaded indicator.
#![cfg(all(windows, target_arch = "x86_64"))]
#![allow(non_snake_case)] // The DLL filename follows Kenshi's plugin naming.

mod callbacks;
mod catalog;
mod diagnostics;
mod extensions;
mod gameplay;
mod internal_plugins;
mod logging;
mod platform;
mod plugin;
mod sample_config;
mod services;
mod settings;
mod ui;

use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

static STATUS: AtomicU32 = AtomicU32::new(0);
static CALLS: AtomicU64 = AtomicU64::new(0);

/// # Safety
/// Accepted requests provide aligned writable GraphicsV1 output storage.
// SAFETY: Unique versioned graphics service export for built-in and DLL effects.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rekenshi_graphics_api(
    version: u32,
    output: *mut rekenshi_plugin_api::graphics::GraphicsV1,
    size: usize,
) -> i32 {
    // SAFETY: Forward the negotiated caller contract unchanged.
    unsafe { rekenshi_graphics::export_api(version, output, size) }
}

/// # Safety
/// Accepted requests supply aligned writable GraphicsV2 storage.
// SAFETY: Unique versioned graphics update service.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rekenshi_graphics_api_v2(
    version: u32,
    output: *mut rekenshi_plugin_api::graphics::GraphicsV2,
    size: usize,
) -> i32 {
    // SAFETY: Forward the negotiated output contract unchanged.
    unsafe { rekenshi_graphics::export_api_v2(version, output, size) }
}
/// Negotiate the host-owned weather renderer service.
/// # Safety
/// On an accepted version, output must be aligned writable WeatherV1 storage.
// SAFETY: Unique versioned export with a negotiated C ABI table.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rekenshi_weather_api(
    version: u32,
    output: *mut rekenshi_plugin_api::weather::WeatherV1,
    size: usize,
) -> i32 {
    // SAFETY: Forward the caller's negotiated output contract.
    unsafe { rekenshi_weather::export_api(version, output, size) }
}

/// Ogre plugin startup. Runs outside the Windows loader's DllMain callback.
// SAFETY: This unique C export matches Ogre's void(void) plugin interface.
#[unsafe(no_mangle)]
pub extern "C" fn dllStartPlugin() {
    boundary("startup", plugin::start);
}

/// Ogre plugin shutdown. Retains trampoline memory and pins the DLL for safety.
// SAFETY: This unique C export matches Ogre's void(void) plugin interface.
#[unsafe(no_mangle)]
pub extern "C" fn dllStopPlugin() {
    boundary("shutdown", plugin::stop);
}

/// 0: not started; 1: hooked; 2: failed; 3: stopped; 4: Ogre unavailable.
// SAFETY: This unique export has no pointer arguments or caller preconditions.
#[unsafe(no_mangle)]
pub extern "C" fn rekenshi_status() -> u32 {
    STATUS.load(Ordering::Acquire)
}

/// Number of calls forwarded through the Ogre hook, including startup's probe.
// SAFETY: This unique export has no pointer arguments or caller preconditions.
#[unsafe(no_mangle)]
pub extern "C" fn rekenshi_hook_calls() -> u64 {
    CALLS.load(Ordering::Relaxed)
}

/// 0: inactive; 1: UI hook armed; 2: label applied; 3: UI unavailable/unsupported.
// SAFETY: This unique export has no arguments or caller preconditions.
#[unsafe(no_mangle)]
pub extern "C" fn rekenshi_ui_status() -> u32 {
    ui::status()
}

/// Number of external mods that completed startup successfully.
// SAFETY: Unique C export without pointer arguments or caller preconditions.
#[unsafe(no_mangle)]
pub extern "C" fn rekenshi_plugins_loaded() -> u32 {
    extensions::active_count()
}

/// Number of successfully started internal plugins in this DLL.
// SAFETY: Unique scalar diagnostic export with no caller preconditions.
#[unsafe(no_mangle)]
pub extern "C" fn rekenshi_internal_plugins() -> u32 {
    internal_plugins::active_count() as u32
}

/// Compatibility name for rekenshi_internal_plugins.
// SAFETY: Unique scalar diagnostic export with no caller preconditions.
#[unsafe(no_mangle)]
pub extern "C" fn rekenshi_pack_layers() -> u32 {
    rekenshi_internal_plugins()
}

/// Shader-cache hits (compatibility export).
// SAFETY: Unique scalar diagnostic export with no caller preconditions.
#[unsafe(no_mangle)]
pub extern "C" fn rekenshi_performance_hits() -> u64 {
    rekenshi_shader_cache::hits()
}

/// Cache hit count for the shader-cache plugin.
// SAFETY: Unique scalar diagnostic export with no caller preconditions.
#[unsafe(no_mangle)]
pub extern "C" fn rekenshi_shader_cache_hits() -> u64 {
    rekenshi_shader_cache::hits()
}

/// Number of native TIFF reads served by the heightmap-cache plugin.
// SAFETY: Unique scalar diagnostic export with no caller preconditions.
#[unsafe(no_mangle)]
pub extern "C" fn rekenshi_heightmap_cache_reads() -> u64 {
    rekenshi_heightmap_cache::reads()
}

/// Native collider cache copies returned to the game.
// SAFETY: Unique diagnostic export with no caller preconditions.
#[unsafe(no_mangle)]
pub extern "C" fn rekenshi_collider_cache_hits() -> u64 {
    rekenshi_collider_cache::hits()
}

/// DDS payload bytes excluded from native file reads.
// SAFETY: Unique diagnostic export with no caller preconditions.
#[unsafe(no_mangle)]
pub extern "C" fn rekenshi_dds_skipped_bytes() -> u64 {
    rekenshi_dds_mip_skip::skipped_bytes()
}

/// Successful native SIC codec decodes.
// SAFETY: Unique diagnostic export with no caller preconditions.
#[unsafe(no_mangle)]
pub extern "C" fn rekenshi_sic_decoded() -> u64 {
    rekenshi_compressed_textures::decoded()
}

/// Converted TGA sidecars substituted by the native archive adapter.
// SAFETY: Unique diagnostic export with no caller preconditions.
#[unsafe(no_mangle)]
pub extern "C" fn rekenshi_texture_sidecar_hits() -> u64 {
    rekenshi_compressed_textures::sidecar_hits()
}

/// Apply and persist a declared internal plugin setting. kind=1 accepts toggle 0/1;
/// kind=2 accepts a finite number validated against that setting's range.
/// # Safety
/// key points to key_len readable UTF-8 bytes for this synchronous call.
// SAFETY: Unique C export using only scalars and caller-owned input bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rekenshi_plugin_setting_set(
    key: *const u8,
    key_len: usize,
    kind: u32,
    value: f64,
) -> i32 {
    std::panic::catch_unwind(|| {
        if key.is_null() || key_len == 0 || key_len > 128 {
            return rekenshi_plugin_api::ERROR;
        }
        // SAFETY: Caller guarantees the bounded key slice is readable for the call.
        let key = match std::str::from_utf8(unsafe { std::slice::from_raw_parts(key, key_len) }) {
            Ok(key) => key,
            Err(_) => return rekenshi_plugin_api::ERROR,
        };
        let value = match (kind, value) {
            (1, 0.) => rekenshi_plugin_runtime::Value::Toggle(false),
            (1, 1.) => rekenshi_plugin_runtime::Value::Toggle(true),
            (2, x) if x.is_finite() => rekenshi_plugin_runtime::Value::Number(x),
            _ => return rekenshi_plugin_api::ERROR,
        };
        match settings::set(key, value) {
            Ok(()) => rekenshi_plugin_api::OK,
            Err(_) => rekenshi_plugin_api::ERROR,
        }
    })
    .unwrap_or(rekenshi_plugin_api::ERROR)
}

/// Compatibility name for rekenshi_plugin_setting_set.
/// # Safety
/// key points to key_len readable UTF-8 bytes for this synchronous call.
// SAFETY: Existing export retains the original caller contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rekenshi_pack_setting_set(
    key: *const u8,
    key_len: usize,
    kind: u32,
    value: f64,
) -> i32 {
    // SAFETY: Forward the caller's bounded readable key contract unchanged.
    unsafe { rekenshi_plugin_setting_set(key, key_len, kind, value) }
}

/// Number of config, discovery, or lifecycle failures this process encountered.
// SAFETY: Unique C export without pointer arguments or caller preconditions.
#[unsafe(no_mangle)]
pub extern "C" fn rekenshi_plugins_failed() -> u32 {
    extensions::failed_count()
}

fn boundary(name: &str, action: fn() -> Result<(), String>) {
    let outcome = std::panic::catch_unwind(action);
    match outcome {
        Ok(Ok(())) => {}
        Ok(Err(error)) => {
            STATUS.store(2, Ordering::Release);
            logging::write(&format!("{name} failed: {error}"));
        }
        Err(_) => {
            STATUS.store(2, Ordering::Release);
            logging::write(&format!("{name} caught a Rust panic"));
        }
    }
}
