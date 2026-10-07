//! Shader caching policy; all native compiler operations live in the adapter.
#![forbid(unsafe_code)]
use crate::native;
use rekenshi_plugin_api::{HostV1, OK};
use rekenshi_plugin_runtime::{Context, Control, Plugin, Setting, Settings, Value};
use std::path::Path;

#[derive(Debug)]
pub struct ShaderCache;
const SETTINGS: &[Setting] = &[Setting {
    key: "performance.shader_cache",
    label: "Enabled",
    description: "Reuse compiled shaders to reduce loading work. Applies immediately.",
    default: Value::Toggle(true),
    control: Control::Toggle,
}];
impl Plugin for ShaderCache {
    fn id(&self) -> &'static str {
        "shader-cache"
    }
    fn title(&self) -> &'static str {
        "Shader cache"
    }
    fn settings(&self) -> &'static [Setting] {
        SETTINGS
    }
    fn start(
        &mut self,
        context: &Context,
        settings: &Settings,
    ) -> Result<(), rekenshi_plugin_runtime::Error> {
        let mut config: native::Settings = rekenshi_native::settings(&config_path(&context.root))?;
        config.shader_cache = true;
        native::initialize(context.host.native(), &context.root, config)?;
        self.update(context, settings)
    }
    fn update(
        &mut self,
        _: &Context,
        settings: &Settings,
    ) -> Result<(), rekenshi_plugin_runtime::Error> {
        native::set_enabled(settings.enabled(SETTINGS[0].key));
        Ok(())
    }
    fn stop(&mut self) -> Result<(), rekenshi_plugin_runtime::Error> {
        stop()
    }
}
pub fn start_legacy(host: HostV1, root: &Path) -> Result<(), rekenshi_plugin_runtime::Error> {
    native::initialize(host, root, rekenshi_native::settings(&config_path(root))?)
        .map_err(rekenshi_plugin_runtime::Error::from)
}
pub fn stop() -> Result<(), rekenshi_plugin_runtime::Error> {
    if native::stop() == OK {
        Ok(())
    } else {
        Err("shader cache shutdown failed".into())
    }
}
pub use native::hits;

fn config_path(root: &Path) -> std::path::PathBuf {
    let path = root.join("shader-cache.toml");
    if path.is_file() {
        path
    } else {
        root.join("performance.toml")
    }
}
