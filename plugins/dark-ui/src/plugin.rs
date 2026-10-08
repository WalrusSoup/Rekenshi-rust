//! Safe built-in setting policy for the shared native UI adapter.
#![forbid(unsafe_code)]
use super::native;
use rekenshi_plugin_runtime::{Context, Control, Plugin, Setting, Settings, Value};

/// Built-in dark UI settings using the host-prepared native adapter.
#[derive(Debug, Default)]
pub struct DarkUi {
    policy: rekenshi_plugin_sdk::policies::DesiredPolicy,
}
const SETTINGS: &[Setting] = &[
    Setting {
        key: "ui.dark_mode",
        label: "Enabled",
        description: "Darken Kenshi's original panels and brighten text, including the bottom HUD. Keeps frame artwork. Applies immediately.",
        default: Value::Toggle(true),
        control: Control::Toggle,
    },
    Setting {
        key: "ui.panel_brightness",
        label: "Frame brightness",
        description: "Brightness of the original metal and parchment frames. Higher values retain more texture detail.",
        default: Value::Number(0.6),
        control: Control::Number {
            min: 0.16,
            max: 0.60,
            step: 0.04,
        },
    },
];
impl Plugin for DarkUi {
    fn id(&self) -> &'static str {
        "dark-ui"
    }
    fn title(&self) -> &'static str {
        "Dark game UI"
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
            rekenshi_plugin_sdk::policies::UI,
            "rekenshi.builtin.dark-ui@1",
            (settings.enabled("ui.dark_mode")).then_some([
                1.,
                settings.number("ui.panel_brightness") as f32,
                0.,
                0.,
            ]),
        )
    }
    fn install(
        &mut self,
        context: &Context,
        settings: &Settings,
    ) -> Result<(), rekenshi_plugin_runtime::Error> {
        if !native::prepared() {
            return Err("supported MyGUI adapter is unavailable".into());
        }
        native::publish_policy(context.host.native())?;
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
        native::revoke_policy()?;
        native::configure(false, 0.6);
        Ok(())
    }
}
