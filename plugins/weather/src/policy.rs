//! Pure rendering policy; native ownership stays in the adapter.
#![forbid(unsafe_code)]
#[derive(Clone, Copy)]
pub(crate) struct RainStyle {
    pub visibility: f32,
    pub size: f32,
}
impl RainStyle {
    pub fn dimensions(self, width: f32, height: f32) -> (f32, f32) {
        (
            (width * 0.18).min(0.12) * self.size,
            (height * 0.45).min(1.8) * self.size,
        )
    }
    pub fn brightness(self, distance: Option<f32>) -> f32 {
        let fade = distance.map_or(1., |d| {
            let t = ((d - 3.) / 9.).clamp(0., 1.);
            t * t * (3. - 2. * t)
        });
        self.visibility * fade
    }
    pub fn brightness_at(
        self,
        mut position: [f32; 3],
        camera: Option<[f32; 3]>,
        transform: Option<[f32; 16]>,
    ) -> f32 {
        if let Some(m) = transform {
            position = [
                m[0] * position[0] + m[1] * position[1] + m[2] * position[2] + m[3],
                m[4] * position[0] + m[5] * position[1] + m[6] * position[2] + m[7],
                m[8] * position[0] + m[9] * position[1] + m[10] * position[2] + m[11],
            ];
        }
        let distance = camera.map(|camera| {
            position
                .iter()
                .zip(camera)
                .map(|(p, c)| (p - c).powi(2))
                .sum::<f32>()
                .sqrt()
        });
        self.brightness(distance)
    }
}
pub(crate) fn is_rain(material: &[u8]) -> bool {
    [
        b"kenshi_rain1".as_slice(),
        b"Kenshi_rain_basic",
        b"Kenshi_rain_basic_red",
        b"Kenshi_Rain_RED",
        b"Kenshi_black_rain",
    ]
    .iter()
    .any(|name| material.eq_ignore_ascii_case(name))
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum WeatherKind {
    Rain,
    Additive,
    Alpha,
}
/// Exact vanilla template names keep shared smoke materials out of fire effects.
pub(super) fn classify(material: &[u8], template: &[u8]) -> Option<WeatherKind> {
    if is_rain(material) {
        return Some(WeatherKind::Rain);
    }
    let weather = [
        b"kenshi_weather_dust1".as_slice(),
        b"kenshi_weather_dust_point1",
        b"kenshi_weather_ash1",
        b"kenshi_weather_ash2",
        b"kenshi_weather_ash3",
        b"kenshi_sandstorm_wall1",
        b"Permanent-dust-storm",
        b"DesertCloudStorm",
        b"Desert-dust-swirls",
        b"Desert-Sand-Strings",
        b"DesertDetritus01",
        b"fogislands",
        b"islandfog",
        b"kenshi_twister_dust1",
        b"kenshi_twister_dust2",
        b"Twister-Large",
        b"Twister_chuff_Desert",
    ]
    .iter()
    .any(|name| template.eq_ignore_ascii_case(name));
    if !weather {
        return None;
    }
    // These native materials use additive/colour blending, which ignores alpha.
    let additive = [
        b"kenshi_smoke2_add".as_slice(),
        b"kenshi_dustparticle1",
        b"kenshi_dustparticle2",
        b"DesertCloudStorm_Mite",
    ]
    .iter()
    .any(|name| material.eq_ignore_ascii_case(name));
    Some(if additive {
        WeatherKind::Additive
    } else {
        WeatherKind::Alpha
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn curtains_become_streaks_and_camera_fade_is_smooth() {
        let style = RainStyle {
            visibility: 0.25,
            size: 1.,
        };
        assert_eq!(style.dimensions(1., 10.), (0.12, 1.8));
        assert_eq!(style.brightness(Some(0.)), 0.);
        assert_eq!(style.brightness(Some(3.)), 0.);
        assert_eq!(style.brightness(Some(7.5)), 0.125);
        assert_eq!(style.brightness(Some(12.)), 0.25);
        assert_eq!(style.brightness(None), 0.25);
        let translated = [
            1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 20., 0., 0., 0., 1.,
        ];
        assert_eq!(
            style.brightness_at([0., 0., 0.], Some([0., 0., 17.]), Some(translated)),
            0.
        );
        assert_eq!(
            style.brightness_at([0., 0., 0.], Some([0., 0., 0.]), Some(translated)),
            0.25
        );
        assert!(is_rain(b"Kenshi_Rain_RED"));
        assert!(!is_rain(b"Kenshi-rainsplash_Basepart"));
        assert!(!is_rain(b"MyRainSmoke"));
    }
}

#[cfg(test)]
mod classification_tests {
    use super::*;
    #[test]
    fn shared_smoke_only_changes_for_weather_templates() {
        assert_eq!(
            classify(b"kenshi_smoke2_add", b"kenshi_weather_dust1"),
            Some(WeatherKind::Additive)
        );
        assert_eq!(
            classify(b"kenshi_smoke2_add", b"kenshi_twister_fire1"),
            None
        );
        assert_eq!(
            classify(b"DustMite", b"Permanent-dust-storm"),
            Some(WeatherKind::Alpha)
        );
        assert_eq!(classify(b"DustMite", b"Dustkick01"), None);
        assert_eq!(
            classify(b"Kenshi_mist_cloud", b"fogislands"),
            Some(WeatherKind::Alpha)
        );
        assert_eq!(
            classify(b"Kenshi-rainsplash_Basepart", b"Kenshi_Rain_RED"),
            None
        );
    }
}
