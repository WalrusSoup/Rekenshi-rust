//! Optional host-owned rain rendering service. No particle pointers cross DLLs.
/// Version negotiated for this optional service.
pub const VERSION: u32 = 1;
/// Negotiate this service into aligned exact-sized caller-owned output.
pub type GetApi = unsafe extern "C" fn(u32, *mut WeatherV1, usize) -> i32;
/// enabled=0 removes this token's style; enabled=1 copies finite visibility 0..1
/// and size 0.1..2. Multiple styles use the smallest size/visibility.
pub type ConfigureRain = unsafe extern "C" fn(u64, u32, f32, f32) -> i32;
#[repr(C)]
#[derive(Clone, Copy, Debug)]
/// Negotiated callback table for token-owned rain rendering styles.
pub struct WeatherV1 {
    /// Size of this C record in bytes; negotiation requires an exact match.
    pub struct_size: u32,
    /// Version of this table; negotiate before invoking its callbacks.
    pub abi_version: u32,
    /// Copy a token-owned style or remove it when enabled is zero.
    pub configure_rain: ConfigureRain,
}
