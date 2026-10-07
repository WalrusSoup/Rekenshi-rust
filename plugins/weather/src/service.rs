//! Scalar-only service; the host owns native hooks and each plugin's style.
use super::{native, policy::RainStyle};
use rekenshi_plugin_api::{
    ERROR, INCOMPATIBLE, OK,
    weather::{VERSION, WeatherV1},
};
/// Remove a revoked plugin's rain style; repeated removal is harmless.
pub fn release(token: u64) {
    native::set_rain(token, None);
}
/// Negotiate the active renderer's frozen scalar-only weather interface.
///
/// # Safety
/// Accepted requests must supply aligned writable WeatherV1 storage of the exact size.
pub unsafe fn export_api(version: u32, output: *mut WeatherV1, size: usize) -> i32 {
    if version != VERSION || size != size_of::<WeatherV1>() {
        return INCOMPATIBLE;
    }
    if output.is_null() || !native::available() {
        return ERROR;
    }
    // SAFETY: Accepted caller guarantees aligned writable exact-size storage.
    unsafe {
        output.write(WeatherV1 {
            struct_size: size_of::<WeatherV1>() as u32,
            abi_version: VERSION,
            configure_rain,
        });
    }
    OK
}
unsafe extern "C" fn configure_rain(token: u64, enabled: u32, visibility: f32, size: f32) -> i32 {
    if !native::allowed(token) || !native::available() {
        return ERROR;
    }
    match enabled {
        0 => {
            release(token);
            OK
        }
        1 if visibility.is_finite()
            && size.is_finite()
            && (0. ..=1.).contains(&visibility)
            && (0.1..=2.).contains(&size) =>
        {
            if native::set_rain(token, Some(RainStyle { visibility, size })) {
                OK
            } else {
                ERROR
            }
        }
        _ => ERROR,
    }
}
