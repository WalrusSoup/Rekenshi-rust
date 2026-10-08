//! Shared weather renderer with an independently switchable built-in policy.
#![forbid(unsafe_code)]
use crate::native;
use rekenshi_plugin_runtime::{Context, Control, Plugin, Setting, Settings, Value};
/// Built-in weather policy over the public particle appearance contract.
#[derive(Debug, Default)]
pub struct Weather {
    policy: rekenshi_plugin_sdk::policies::DesiredPolicy,
}
const SETTING: Setting = Setting {
    key: "weather.tone_down",
    label: "Tone down weather effects",
    description: "Soften rain, dust storms, ash and particle fog to 35% visibility. Applies immediately; weather gameplay stays the same.",
    default: Value::Toggle(true),
    control: Control::Toggle,
};
impl Plugin for Weather {
    fn id(&self) -> &'static str {
        "weather"
    }
    fn title(&self) -> &'static str {
        "Weather effects"
    }
    fn settings(&self) -> &'static [Setting] {
        std::slice::from_ref(&SETTING)
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
    fn register(
        &mut self,
        context: &Context,
        settings: &Settings,
    ) -> Result<(), rekenshi_plugin_runtime::Error> {
        self.settings_changed(context, settings)
    }
    fn settings_changed(
        &mut self,
        context: &Context,
        settings: &Settings,
    ) -> Result<(), rekenshi_plugin_runtime::Error> {
        self.policy.set(
            context.host.native(),
            rekenshi_plugin_sdk::policies::WEATHER,
            "rekenshi.builtin.weather@1",
            settings
                .enabled(SETTING.key)
                .then_some([0.35, 0.35, 0., 0.]),
        )?;
        Ok(())
    }
    fn stop(&mut self) -> Result<(), rekenshi_plugin_runtime::Error> {
        self.policy.remove()?;
        native::stop().map_err(rekenshi_plugin_runtime::Error::from)
    }
}
