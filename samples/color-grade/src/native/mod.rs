//! Independent color grading mod using the optional graphics service.
use rekenshi_plugin_api::{ERROR, HostV1, OK, graphics::*};
use serde::Deserialize;
use std::sync::Mutex;
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
struct Session {
    host: HostV1,
    api: GraphicsV1,
    handle: u64,
}
static SESSION: Mutex<Option<Session>> = Mutex::new(None);
pub(super) const SHADER: &[u8] = include_bytes!("../color_grade.hlsl");
pub(super) fn initialize(host: HostV1, settings: Settings) -> Result<(), String> {
    let mut session = SESSION.lock().map_err(|_| "effect lock poisoned")?;
    if !settings.enabled {
        host.write_log("color grading disabled in settings");
        return Ok(());
    }
    if [settings.contrast, settings.saturation, settings.brightness]
        .into_iter()
        .chain(settings.tint)
        .any(|x| !x.is_finite() || !(0.0..=4.0).contains(&x))
    {
        return Err("color settings must be finite and 0..4".into());
    }
    let address = host
        .find_export(c"rekenshi_graphics.dll", c"rekenshi_graphics_api")
        .or_else(|| host.find_export(c"RE_Kenshi_Rust.dll", c"rekenshi_graphics_api"))
        .ok_or("enable graphics service before color-grade in plugins.toml")?;
    // SAFETY: Named service export implements the SDK's negotiated C signature.
    let get: GetApi = unsafe { std::mem::transmute(address.as_ptr()) };
    let mut api = std::mem::MaybeUninit::<GraphicsV1>::uninit();
    // SAFETY: Aligned writable output with exact V1 size. Assume initialized only on OK.
    if unsafe { get(VERSION, api.as_mut_ptr(), size_of::<GraphicsV1>()) } != OK {
        return Err("graphics service unavailable/incompatible".into());
    }
    // SAFETY: Successful negotiation initialized the complete table.
    let api = unsafe { api.assume_init() };
    if api.struct_size as usize != size_of::<GraphicsV1>() || api.abi_version != VERSION {
        return Err("graphics descriptor mismatch".into());
    }
    let mut parameters = [0.; 16];
    parameters[..4].copy_from_slice(&[
        settings.contrast,
        settings.saturation,
        settings.brightness,
        0.,
    ]);
    parameters[4..8].copy_from_slice(&[settings.tint[0], settings.tint[1], settings.tint[2], 1.]);
    let desc = EffectV1 {
        struct_size: size_of::<EffectV1>() as u32,
        stage: POST_TONEMAP,
        priority: settings.priority,
        reserved: 0,
        hlsl: SHADER.as_ptr(),
        hlsl_len: SHADER.len(),
        parameters,
    };
    let mut handle = 0;
    // SAFETY: Descriptor/source/output remain valid through synchronous registration.
    if unsafe { (api.register)(host.plugin_token, &desc, &mut handle) } != OK {
        return Err("color grading registration failed".into());
    }
    host.write_log(&format!(
        "color grading registered: contrast={} saturation={} brightness={} tint={:?}",
        settings.contrast, settings.saturation, settings.brightness, settings.tint
    ));
    *session = Some(Session { host, api, handle });
    Ok(())
}
pub(super) fn stop() -> i32 {
    std::panic::catch_unwind(|| {
        let Ok(mut session) = SESSION.lock() else {
            return ERROR;
        };
        if let Some(s) = session.as_ref() {
            // SAFETY: Service is pinned; loader stops effects before their service.
            // Remove serializes with rendering and releases only this owner's effect.
            if unsafe { (s.api.remove)(s.host.plugin_token, s.handle) } != OK {
                return ERROR;
            }
            s.host.write_log("color grading removed");
        }
        *session = None;
        OK
    })
    .unwrap_or(ERROR)
}
