//! Native SIC support and optional converted TGA sidecars.
#![forbid(unsafe_code)]
use crate::native;
pub use native::{decoded, sidecar_hits};
use rekenshi_plugin_runtime::{Context, Control, Plugin, Setting, Settings, Value};
#[derive(Debug, Default)]
pub struct CompressedTextures {
    policy: rekenshi_plugin_sdk::policies::DesiredPolicy,
}
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
    fn register(
        &mut self,
        context: &Context,
        settings: &Settings,
    ) -> Result<(), rekenshi_plugin_runtime::Error> {
        self.policy.set(
            context.host.native(),
            rekenshi_plugin_sdk::policies::TEXTURES,
            "rekenshi.builtin.compressed-textures@1",
            (settings.enabled(SETTINGS[0].key)).then_some([1., 0., 0., 0.]),
        )
    }
    fn install(
        &mut self,
        context: &Context,
        settings: &Settings,
    ) -> Result<(), rekenshi_plugin_runtime::Error> {
        native::start(
            context.host.native(),
            context.root.parent().ok_or("game directory")?,
        )?;
        self.settings_changed(context, settings)
    }
    fn settings_changed(
        &mut self,
        context: &Context,
        settings: &Settings,
    ) -> Result<(), rekenshi_plugin_runtime::Error> {
        self.register(context, settings)
    }

    fn stop(&mut self) -> Result<(), rekenshi_plugin_runtime::Error> {
        self.policy.remove()?;
        native::stop().map_err(rekenshi_plugin_runtime::Error::from)
    }
}
