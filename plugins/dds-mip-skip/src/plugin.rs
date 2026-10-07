//! Skip file reads for DDS levels discarded by the game's texture-quality setting.
#![forbid(unsafe_code)]
use crate::native;
pub use native::skipped_bytes;
use rekenshi_plugin_runtime::{Context, Control, Plugin, Setting, Settings, Value};
#[derive(Debug)]
pub struct DdsMipSkip;
const SETTINGS: &[Setting] = &[Setting {
    key: "performance.dds_mip_skip",
    label: "Enabled",
    description: "Avoid discarded DDS mip reads at reduced texture quality. Applies to new loads.",
    default: Value::Toggle(false),
    control: Control::Toggle,
}];
impl Plugin for DdsMipSkip {
    fn id(&self) -> &'static str {
        "dds-mip-skip"
    }
    fn title(&self) -> &'static str {
        "DDS mip skipping"
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
            context.root.parent().ok_or("game folder")?,
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
