//! Independent color grading mod using the optional graphics service.
use serde::Deserialize;
#[derive(Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(super) struct Settings {
    pub enabled: bool,
    pub contrast: f32,
    pub saturation: f32,
    pub brightness: f32,
    pub tint: [f32; 3],
    pub priority: i32,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            enabled: true,
            contrast: 1.06,
            saturation: 1.08,
            brightness: 1.0,
            tint: [1.02, 1.0, 0.98],
            priority: 100,
        }
    }
}
pub(super) const SHADER: &[u8] = include_bytes!("../color_grade.hlsl");
