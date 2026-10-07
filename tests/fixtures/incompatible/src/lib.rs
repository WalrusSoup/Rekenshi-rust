use std::ffi::c_void;
/// # Safety
/// Matches the entry signature but never accesses output; simulates unsupported ABI.
// SAFETY: Unique C export used only by the integration smoke suite.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rekenshi_plugin_entry(_: u32, _: *mut c_void, _: usize) -> i32 {
    2
}
