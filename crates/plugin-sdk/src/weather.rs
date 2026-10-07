//! Safe owned rain style; the shared host adapter handles the native hooks.
use crate::{
    Context,
    abi::{OK, weather::*},
};
/// Owned thin rain streaks and camera fade for this plugin.
///
/// Drop removes this plugin's style. The built-in weather visibility switch remains independent.
#[derive(Debug)]
pub struct RainRendering {
    api: WeatherV1,
    token: u64,
}
impl RainRendering {
    /// Connect an owned rain style to the optional host weather service.
    ///
    /// # Errors
    /// Returns a failure when service negotiation, validation, or the host operation fails.
    pub fn new(context: &Context) -> Result<Self, crate::Error> {
        let host = context.host.native();
        let address = host
            .find_export(c"RE_Kenshi_Rust.dll", c"rekenshi_weather_api")
            .ok_or_else(|| crate::Error::unavailable("weather"))?;
        // SAFETY: Named pinned SDK export has the versioned GetApi signature.
        let get: GetApi = unsafe { std::mem::transmute(address.as_ptr()) };
        let mut output = std::mem::MaybeUninit::uninit();
        // SAFETY: Aligned writable output with the exact requested version/size.
        let result = unsafe { get(VERSION, output.as_mut_ptr(), size_of::<WeatherV1>()) };
        if result != OK {
            return Err(crate::Error::host("weather", result));
        }
        // SAFETY: Accepted API request initializes this complete table.
        let api: WeatherV1 = unsafe { output.assume_init() };
        if api.struct_size as usize != size_of::<WeatherV1>() || api.abi_version != VERSION {
            return Err("incompatible weather service".into());
        }
        Ok(Self {
            api,
            token: host.plugin_token,
        })
    }
    /// Apply a rain style live. visibility is 0..1 and streak size is 0.1..2.
    ///
    /// # Errors
    /// Returns a weather host status if values are invalid or the adapter rejects the update.
    pub fn configure(&self, enabled: bool, visibility: f32, size: f32) -> Result<(), crate::Error> {
        // SAFETY: Negotiated pinned scalar-only callback validates all inputs.
        let result =
            unsafe { (self.api.configure_rain)(self.token, u32::from(enabled), visibility, size) };
        if result == OK {
            Ok(())
        } else {
            Err(crate::Error::host("weather", result))
        }
    }
}
impl Drop for RainRendering {
    fn drop(&mut self) {
        let _ = self.configure(false, 0., 1.);
    }
}
