//! Color-grading policy and its declarative menu controls.
#![forbid(unsafe_code)]
use crate::native;
use rekenshi_plugin_api::{HostV1, OK};
use rekenshi_plugin_sdk::{Context, Control, Plugin, Setting, Settings, Value};
use std::path::Path;
#[derive(Default, Debug)]
pub struct ColorGrade {
    effect: Option<rekenshi_plugin_sdk::PostProcess>,
}
const SETTINGS: &[Setting] = &[
    Setting {
        key: "graphics.color_grade",
        label: "Color grading",
        description: "Grade the world before the interface is drawn. Applies immediately.",
        default: Value::Toggle(true),
        control: Control::Toggle,
    },
    Setting {
        key: "graphics.contrast",
        label: "Contrast",
        description: "World contrast. Applies immediately.",
        default: Value::Number(1.06),
        control: Control::Number {
            min: 0.,
            max: 4.,
            step: 0.05,
        },
    },
    Setting {
        key: "graphics.saturation",
        label: "Saturation",
        description: "World saturation. Applies immediately.",
        default: Value::Number(1.08),
        control: Control::Number {
            min: 0.,
            max: 4.,
            step: 0.05,
        },
    },
    Setting {
        key: "graphics.brightness",
        label: "Brightness",
        description: "World brightness. Applies immediately.",
        default: Value::Number(1.),
        control: Control::Number {
            min: 0.,
            max: 4.,
            step: 0.05,
        },
    },
];
impl Plugin for ColorGrade {
    fn id(&self) -> &'static str {
        "color-grade"
    }
    fn title(&self) -> &'static str {
        "Graphics"
    }
    fn requires(&self) -> &'static [&'static str] {
        &["graphics"]
    }
    fn settings(&self) -> &'static [Setting] {
        SETTINGS
    }
    fn start(
        &mut self,
        context: &Context,
        settings: &Settings,
    ) -> Result<(), rekenshi_plugin_sdk::Error> {
        self.update(context, settings)
    }
    fn update(
        &mut self,
        context: &Context,
        settings: &Settings,
    ) -> Result<(), rekenshi_plugin_sdk::Error> {
        let mut config: native::Settings =
            rekenshi_native::settings(&context.root.join("color-grade.toml"))?;
        config.enabled = settings.enabled("graphics.color_grade");
        config.contrast = settings.number("graphics.contrast") as f32;
        config.saturation = settings.number("graphics.saturation") as f32;
        config.brightness = settings.number("graphics.brightness") as f32;
        if config
            .tint
            .iter()
            .any(|x| !x.is_finite() || !(0.0..=4.0).contains(x))
        {
            return Err("tint must be finite and 0..4".into());
        }
        if !config.enabled {
            return self.stop();
        }
        let mut parameters = [0.; 16];
        parameters[..4].copy_from_slice(&[
            config.contrast,
            config.saturation,
            config.brightness,
            0.,
        ]);
        parameters[4..8].copy_from_slice(&[config.tint[0], config.tint[1], config.tint[2], 1.]);
        match self.effect.as_mut() {
            Some(effect) => effect.update(parameters),
            None => {
                self.effect = Some(rekenshi_plugin_sdk::PostProcess::new(
                    context,
                    native::SHADER,
                    config.priority,
                    parameters,
                )?);
                Ok(())
            }
        }
    }
    fn stop(&mut self) -> Result<(), rekenshi_plugin_sdk::Error> {
        if let Some(effect) = self.effect.as_mut() {
            effect.remove()?;
        }
        self.effect = None;
        Ok(())
    }
}
pub fn start_legacy(host: HostV1, root: &Path) -> Result<(), rekenshi_plugin_sdk::Error> {
    native::initialize(
        host,
        rekenshi_native::settings(&root.join("color-grade.toml"))?,
    )
    .map_err(rekenshi_plugin_sdk::Error::from)
}
pub fn stop() -> Result<(), rekenshi_plugin_sdk::Error> {
    if native::stop() == OK {
        Ok(())
    } else {
        Err("color grading shutdown failed".into())
    }
}
