//! Native SIC support and optional converted TGA sidecars.
#![forbid(unsafe_code)]
use crate::native;
pub use native::{decoded, sidecar_hits};
use rekenshi_plugin_runtime::{Context, Control, Plugin, Setting, Settings, Value};
#[derive(Debug)]
pub struct CompressedTextures;
const SETTINGS: &[Setting] = &[Setting {
    key: "performance.compressed_textures",
    label: "Enabled",
    description: "Load smaller converted TGA sidecars. SIC assets are supported while the pack is loaded.",
    default: Value::Toggle(false),
    control: Control::Toggle,
}];
impl Plugin for CompressedTextures {
    fn id(&self) -> &'static str {
        "compressed-textures"
    }
    fn title(&self) -> &'static str {
        "Compressed textures"
    }
    fn settings(&self) -> &'static [Setting] {
        SETTINGS
    }
    fn start(
        &mut self,
        context: &Context,
        settings: &Settings,
    ) -> Result<(), rekenshi_plugin_runtime::Error> {
        native::start(
            context.host.native(),
            context.root.parent().ok_or("game directory")?,
        )?;
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
        native::stop().map_err(rekenshi_plugin_runtime::Error::from)
    }
}
