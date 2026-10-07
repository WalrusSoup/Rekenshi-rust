//! An original, color-only interpretation of Kenshi 2 Style ReShade.
//! All native rendering and resource ownership stay behind PostProcess.
#![forbid(unsafe_code)]
use rekenshi_plugin_sdk::PostProcess;
use rekenshi_plugin_sdk::{Context, Control, Plugin, Setting, Settings, Value};

pub const SHADER: &[u8] = include_bytes!("style.hlsl");
const ENABLED: &str = "samples.kenshi2_style";

const fn number(
    key: &'static str,
    label: &'static str,
    default: f64,
    min: f64,
    max: f64,
    step: f64,
) -> Setting {
    Setting {
        key,
        label,
        description: "Applies immediately to the world; leaves the interface unchanged.",
        default: Value::Number(default),
        control: Control::Number { min, max, step },
    }
}
const SETTINGS: &[Setting] = &[
    Setting {
        key: ENABLED,
        label: "Kenshi 2 style sample",
        description: "Original comic outlines and desert palette. Color-only; no depth outlines or ambient occlusion.",
        default: Value::Toggle(false),
        control: Control::Toggle,
    },
    number(
        "samples.kenshi2_style.ink",
        "Ink strength",
        0.55,
        0.,
        1.,
        0.05,
    ),
    number(
        "samples.kenshi2_style.ink_width",
        "Ink width (pixels)",
        1.,
        0.5,
        3.,
        0.25,
    ),
    number(
        "samples.kenshi2_style.contrast",
        "Contrast",
        1.06,
        0.5,
        2.,
        0.05,
    ),
    number(
        "samples.kenshi2_style.saturation",
        "Saturation",
        0.9,
        0.,
        2.,
        0.05,
    ),
    number(
        "samples.kenshi2_style.exposure",
        "Exposure (stops)",
        0.,
        -2.,
        2.,
        0.1,
    ),
    number(
        "samples.kenshi2_style.palette",
        "Palette simplification",
        0.15,
        0.,
        1.,
        0.05,
    ),
    number(
        "samples.kenshi2_style.palette_levels",
        "Palette levels",
        16.,
        4.,
        64.,
        1.,
    ),
    number(
        "samples.kenshi2_style.sharpen",
        "Sharpening",
        0.18,
        0.,
        1.,
        0.05,
    ),
    number(
        "samples.kenshi2_style.warmth",
        "Desert warmth",
        0.08,
        0.,
        1.,
        0.05,
    ),
];

#[derive(Default, Debug)]
pub struct Kenshi2Style {
    effect: Option<PostProcess>,
}
impl Plugin for Kenshi2Style {
    fn id(&self) -> &'static str {
        "kenshi2-style"
    }
    fn title(&self) -> &'static str {
        "Sample: Kenshi 2 style"
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
        if !settings.enabled(ENABLED) {
            return self.stop();
        }
        let parameters = [
            settings.number("samples.kenshi2_style.ink") as f32,
            settings.number("samples.kenshi2_style.ink_width") as f32,
            0.035,
            0.06,
            settings.number("samples.kenshi2_style.contrast") as f32,
            settings.number("samples.kenshi2_style.saturation") as f32,
            settings.number("samples.kenshi2_style.exposure") as f32,
            settings.number("samples.kenshi2_style.palette") as f32,
            settings.number("samples.kenshi2_style.palette_levels") as f32,
            settings.number("samples.kenshi2_style.sharpen") as f32,
            settings.number("samples.kenshi2_style.warmth") as f32,
            0.,
            0.,
            0.,
            0.,
            0.,
        ];
        match self.effect.as_mut() {
            Some(effect) => effect.update(parameters),
            None => {
                self.effect = Some(PostProcess::new(context, SHADER, 200, parameters)?);
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
