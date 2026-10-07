//! Optional SDK services. V1 plugin and host layouts remain frozen.
use std::ffi::c_void;
/// Version negotiated for this optional service.
pub const VERSION: u32 = 1;
/// Event emitted after forwarding the original Ogre root getter.
pub const OGRE_CALLS: u32 = 1;
/// Event emitted after forwarding a MyGUI frame with elapsed seconds.
pub const UI_FRAME: u32 = 2;
/// Invoke a plugin-owned callback with retained userdata and frame delta.
pub type Event = unsafe extern "C" fn(*mut c_void, f32);
/// Negotiate this service into aligned exact-sized caller-owned output.
pub type GetApi = unsafe extern "C" fn(u32, *mut ServicesV1, usize) -> i32;
#[repr(C)]
#[derive(Clone, Copy, Debug)]
/// Negotiated host callbacks for dependencies, persistence paths, and managed events.
pub struct ServicesV1 {
    /// Size of this C record in bytes; negotiation requires an exact match.
    pub struct_size: u32,
    /// Version of this table; negotiate before invoking its callbacks.
    pub abi_version: u32,
    /// Copy the plugin persistence path into the caller bounded UTF-8 output.
    pub root: unsafe extern "C" fn(u64, *mut u8, usize, *mut usize) -> i32,
    /// Check whether the named UTF-8 dependency is active for this plugin.
    pub dependency: unsafe extern "C" fn(u64, *const u8, usize) -> i32,
    /// Report whether the requested callback event is available on this host.
    pub supports: extern "C" fn(u32) -> bool,
    /// Userdata remains plugin-owned until unsubscribe returns OK. Callback
    /// must contain panics; unsubscribe waits for in-flight host calls.
    pub subscribe: unsafe extern "C" fn(u64, u32, Event, *mut c_void, *mut u64) -> i32,
    /// Disable a subscription and wait for in-flight callbacks before returning OK.
    pub unsubscribe: extern "C" fn(u64, u64) -> i32,
}
/// Negotiate the independent plugin settings table into caller-owned output.
pub type GetUi = unsafe extern "C" fn(u32, *mut PluginUiV1, usize) -> i32;
#[repr(C)]
#[derive(Clone, Copy, Debug)]
/// Independent plugin callbacks for enumerating and editing settings.
pub struct PluginUiV1 {
    /// Size of this C record in bytes; negotiation requires an exact match.
    pub struct_size: u32,
    /// Version of this table; negotiate before invoking its callbacks.
    pub abi_version: u32,
    /// Return the number of setting rows supplied by this plugin.
    pub count: extern "C" fn() -> u32,
    /// Copy one setting into aligned exact-sized caller-owned output.
    pub row: unsafe extern "C" fn(u32, *mut SettingV1, usize) -> i32,
    /// Apply a UTF-8 setting key, control kind, and finite value synchronously.
    pub set: unsafe extern "C" fn(*const u8, usize, u32, f64) -> i32,
}
#[repr(C)]
#[derive(Clone, Copy, Debug)]
/// C-layout setting row containing bounded NUL-terminated UTF-8 display text.
pub struct SettingV1 {
    /// Size of this C record in bytes; negotiation requires an exact match.
    pub struct_size: u32,
    /// Discriminant selecting the command or control representation.
    pub kind: u32,
    /// Plugin group title displayed in the settings menu.
    pub title: [u8; 96],
    /// Stable unique setting key used for persistence and updates.
    pub key: [u8; 128],
    /// Control label displayed in the settings menu.
    pub label: [u8; 96],
    /// Explanation of the setting shown to the user.
    pub description: [u8; 512],
    /// Current failure description, or empty storage when the plugin is available.
    pub error: [u8; 512],
    /// Default control value; toggles use exactly zero or one.
    pub default_value: f64,
    /// Current validated setting value.
    pub value: f64,
    /// Inclusive lower bound of the numeric control.
    pub min: f64,
    /// Inclusive upper bound of the numeric control.
    pub max: f64,
    /// Positive increment used by the numeric control.
    pub step: f64,
}
impl Default for SettingV1 {
    fn default() -> Self {
        Self {
            struct_size: size_of::<Self>() as u32,
            kind: 0,
            title: [0; 96],
            key: [0; 128],
            label: [0; 96],
            description: [0; 512],
            error: [0; 512],
            default_value: 0.,
            value: 0.,
            min: 0.,
            max: 0.,
            step: 0.,
        }
    }
}
