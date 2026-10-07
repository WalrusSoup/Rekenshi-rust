//! Independent callback example; all authored behavior is in the safe module.
mod counter;
pub use counter::CallbackCounter;
rekenshi_plugin_sdk::export_plugin!(CallbackCounter);
// SAFETY: Scalar-only test diagnostics, no caller preconditions.
#[unsafe(no_mangle)]
pub extern "C" fn rekenshi_counter_calls() -> u64 {
    counter::calls()
}
// SAFETY: Scalar-only test diagnostics, no caller preconditions.
#[unsafe(no_mangle)]
pub extern "C" fn rekenshi_counter_frames() -> u64 {
    counter::frames()
}
// SAFETY: Scalar-only test diagnostics, no caller preconditions.
#[unsafe(no_mangle)]
pub extern "C" fn rekenshi_counter_delta() -> f32 {
    counter::delta()
}
