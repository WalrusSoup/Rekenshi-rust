//! Refine native rain geometry without changing weather simulation or assets.
#![forbid(unsafe_code)]
use rekenshi_plugin_sdk::{Context, Control, Plugin, RainRendering, Setting, Settings, Value};

#[derive(Default, Debug)]
pub struct RainRefinement {
    rendering: Option<RainRendering>,
}
const SETTINGS: &[Setting] = &[
    Setting {
        key: "weather.rain_refinement",
        label: "Rain refinement",
        description: "Thinner, shorter rain with a camera proximity fade. Applies immediately.",
        default: Value::Toggle(true),
        control: Control::Toggle,
    },
    Setting {
        key: "weather.rain_visibility",
        label: "Rain visibility",
        description: "Rain brightness; zero hides the streaks. Regional colors are preserved.",
        default: Value::Number(0.25),
        control: Control::Number {
            min: 0.,
            max: 1.,
            step: 0.05,
        },
    },
    Setting {
        key: "weather.rain_size",
        label: "Rain streak size",
        description: "Scale the refined streak geometry. Applies immediately.",
        default: Value::Number(1.),
        control: Control::Number {
            min: 0.1,
            max: 2.,
            step: 0.1,
        },
    },
];
impl Plugin for RainRefinement {
    fn id(&self) -> &'static str {
        "rain-refinement"
    }
    fn title(&self) -> &'static str {
        "Weather"
    }
    fn settings(&self) -> &'static [Setting] {
        SETTINGS
    }
    fn register(
        &mut self,
        context: &Context,
        settings: &Settings,
    ) -> Result<(), rekenshi_plugin_sdk::Error> {
        self.rendering = Some(RainRendering::new(context)?);
        context
            .host
            .log("rain-refinement: native streak geometry via shared weather renderer");
        self.settings_changed(context, settings)
    }
    fn settings_changed(
        &mut self,
        _: &Context,
        settings: &Settings,
    ) -> Result<(), rekenshi_plugin_sdk::Error> {
        self.rendering
            .as_ref()
            .ok_or("rain renderer not started")?
            .configure(
                settings.enabled(SETTINGS[0].key),
                settings.number(SETTINGS[1].key) as f32,
                settings.number(SETTINGS[2].key) as f32,
            )
    }
    fn stop(&mut self) -> Result<(), rekenshi_plugin_sdk::Error> {
        self.rendering = None;
        Ok(())
    }
}
