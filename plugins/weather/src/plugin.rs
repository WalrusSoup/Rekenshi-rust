//! Shared weather renderer with an independently switchable built-in policy.
#![forbid(unsafe_code)]
use crate::native;
use rekenshi_plugin_runtime::{Context, Control, Plugin, Setting, Settings, Value};
/// Built-in weather policy and shared renderer with host-owned token authorization.
#[derive(Debug)]
pub struct Weather {
    allowed: fn(u64) -> bool,
}
impl Weather {
    /// Construct the weather plugin with the host's live token authorization check.
    ///
    /// The check must return false once an independent plugin's token is revoked.
    /// The host calls release after revocation to remove that plugin's rain style.
    pub const fn new(allowed: fn(u64) -> bool) -> Self {
        Self { allowed }
    }
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
    fn start(
        &mut self,
        context: &Context,
        settings: &Settings,
    ) -> Result<(), rekenshi_plugin_runtime::Error> {
        native::start(
            context.host.native(),
            context.root.parent().ok_or("game directory")?,
            self.allowed,
        )?;
        self.update(context, settings)
    }
    fn update(
        &mut self,
        _: &Context,
        settings: &Settings,
    ) -> Result<(), rekenshi_plugin_runtime::Error> {
        native::configure(settings.enabled(SETTING.key));
        Ok(())
    }
    fn stop(&mut self) -> Result<(), rekenshi_plugin_runtime::Error> {
        native::stop().map_err(rekenshi_plugin_runtime::Error::from)
    }
}
