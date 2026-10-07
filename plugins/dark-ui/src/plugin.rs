//! Safe built-in setting policy for the shared native UI adapter.
#![forbid(unsafe_code)]
use super::native;
use rekenshi_plugin_runtime::{Context, Control, Plugin, Setting, Settings, Value};

/// Built-in dark UI settings using the host-prepared native adapter.
#[derive(Debug)]
pub struct DarkUi;
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
    fn start(
        &mut self,
        context: &Context,
        settings: &Settings,
    ) -> Result<(), rekenshi_plugin_runtime::Error> {
        if !native::prepared() {
            return Err("supported MyGUI adapter is unavailable".into());
        }
        self.update(context, settings)
    }
    fn update(
        &mut self,
        _: &Context,
        settings: &Settings,
    ) -> Result<(), rekenshi_plugin_runtime::Error> {
        native::configure(
            settings.enabled("ui.dark_mode"),
            settings.number("ui.panel_brightness"),
        );
        Ok(())
    }
    fn stop(&mut self) -> Result<(), rekenshi_plugin_runtime::Error> {
        native::configure(false, 0.6);
        Ok(())
    }
}
