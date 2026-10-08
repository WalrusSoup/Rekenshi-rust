//! Skip file reads for DDS levels discarded by the game's texture-quality setting.
#![forbid(unsafe_code)]
use crate::native;
pub use native::skipped_bytes;
use rekenshi_plugin_runtime::{Context, Control, Plugin, Setting, Settings, Value};
#[derive(Debug, Default)]
pub struct DdsMipSkip {
    policy: rekenshi_plugin_sdk::policies::DesiredPolicy,
}
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
    fn register(
        &mut self,
        context: &Context,
        settings: &Settings,
    ) -> Result<(), rekenshi_plugin_runtime::Error> {
        self.policy.set(
            context.host.native(),
            rekenshi_plugin_sdk::policies::DDS,
            "rekenshi.builtin.dds-mip-skip@1",
            (settings.enabled(SETTINGS[0].key)).then_some([1., -1., 0., 0.]),
        )
    }
    fn install(
        &mut self,
        context: &Context,
        settings: &Settings,
    ) -> Result<(), rekenshi_plugin_runtime::Error> {
        native::start(
            context.host.native(),
            context.root.parent().ok_or("game folder")?,
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
