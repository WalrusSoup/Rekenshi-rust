//! Deferred copied domain observations and operation completion on the serial lane.
#[doc(inline)]
pub use rekenshi_plugin_api::platform::EventV1;
/// Roster contents/availability changed after a verified native capture or reset.
/// Payload: world u64, snapshot sequence u64, availability u32, all little-endian.
pub const ROSTER_CHANGED: &str = "rekenshi.gameplay.roster-changed@1";
/// Command finished native dispatch or was rejected, not movement/combat completion.
/// Payload: owner u64, ticket u64, state u32, code i32, all little-endian.
pub const COMMAND_COMPLETED: &str = "rekenshi.gameplay.command-completed@1";
/// Observed MyGUI widget/skin creation or destruction; never general UI readiness.
pub const WIDGET_OBSERVED: &str = "rekenshi.ui.widget-observed@1";
