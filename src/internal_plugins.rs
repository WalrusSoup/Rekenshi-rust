//! Internal plugins are registered here and linked into the single host DLL.
#![forbid(unsafe_code)]
use crate::{extensions, logging};
use rekenshi_collider_cache::ColliderCache;
use rekenshi_compressed_textures::CompressedTextures;
use rekenshi_dds_mip_skip::DdsMipSkip;
use rekenshi_graphics::GraphicsBackend;
use rekenshi_heightmap_cache::HeightmapCache;
use rekenshi_plugin_runtime::{Plugins, Row, Runtime, Value};
use rekenshi_shader_cache::ShaderCache;
use std::sync::Mutex;
static PLUGINS: Mutex<Option<Runtime>> = Mutex::new(None);
pub(crate) fn ids() -> Vec<&'static str> {
    PLUGINS
        .lock()
        .ok()
        .and_then(|p| p.as_ref().map(Runtime::ids))
        .unwrap_or_default()
}
pub(crate) fn available(id: &str) -> bool {
    PLUGINS
        .lock()
        .is_ok_and(|p| p.as_ref().is_some_and(|r| r.available(id)))
}

pub(crate) fn prepare() -> Result<(), String> {
    let root = logging::directory()
        .ok_or("host folder")?
        .join("RE_Kenshi_Rust");
    // Existing DLL-only installations remain usable; the installer opts in to
    // the integrated plugins and migrates the three built-in DLL entries.
    if !root.join("modpack.toml").is_file() {
        return Ok(());
    }
    let mut plugins = PLUGINS
        .lock()
        .map_err(|_| "internal plugin registry lock poisoned")?;
    if plugins.is_some() {
        return Ok(());
    }
    crate::sample_config::migrate_settings(&root)?;
    let registry = Plugins::new()
        .register(ShaderCache::default())
        .register(HeightmapCache::default())
        .register(ColliderCache::default())
        .register(DdsMipSkip::default())
        .register(CompressedTextures::default())
        .register(GraphicsBackend::default())
        .register(rekenshi_dark_ui::DarkUi::default())
        .register(rekenshi_weather::Weather::default())
        .register(crate::diagnostics::Diagnostics);
    *plugins = Some(registry.prepare(root, |id, token| {
        extensions::internal_host(id, token).map_err(Into::into)
    })?);
    logging::write("internal plugins mounted; settings apply without restart");
    Ok(())
}
pub(crate) fn start() -> Result<(), String> {
    let mut plugins = PLUGINS
        .lock()
        .map_err(|_| "internal plugin registry lock poisoned")?;
    if let Some(runtime) = plugins.as_mut() {
        runtime.install().map_err(|e| e.to_string())?;
    }
    Ok(())
}
pub(crate) fn active_count() -> usize {
    PLUGINS
        .lock()
        .ok()
        .and_then(|p| p.as_ref().map(Runtime::active_count))
        .unwrap_or(0)
}
pub(crate) fn rows() -> Vec<Row> {
    PLUGINS
        .lock()
        .ok()
        .and_then(|p| p.as_ref().map(Runtime::rows))
        .unwrap_or_default()
}
pub(crate) fn set(key: &str, value: Value) -> Result<(), String> {
    PLUGINS
        .lock()
        .map_err(|_| "internal plugin registry lock poisoned")?
        .as_mut()
        .ok_or("internal plugin registry unavailable")?
        .set(key, value)
        .map_err(Into::into)
}
pub(crate) fn stop() -> Result<(), String> {
    let mut plugins = PLUGINS
        .lock()
        .map_err(|_| "internal plugin registry lock poisoned")?;
    if let Some(runtime) = plugins.as_mut() {
        runtime.stop()?;
        extensions::release_internal()?;
    }
    *plugins = None;
    Ok(())
}
pub(crate) fn dispatch_lifecycle() {
    if let Ok(mut plugins) = PLUGINS.try_lock() {
        if let Some(runtime) = plugins.as_mut() {
            let _ = runtime.dispatch_lifecycle();
        }
    }
}
