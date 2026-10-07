//! Combined menu for built-in optimizations and independently loaded DLL mods.
#![forbid(unsafe_code)]
use rekenshi_plugin_runtime::{Row, Value};
pub(crate) fn rows() -> Vec<Row> {
    let mut rows = crate::internal_plugins::rows();
    rows.extend(crate::extensions::rows());
    rows
}
pub(crate) fn set(key: &str, value: Value) -> Result<(), String> {
    rekenshi_plugin_runtime::HookEvents::check_shutdown()?;
    if crate::internal_plugins::rows()
        .iter()
        .any(|r| r.setting.key == key)
    {
        crate::internal_plugins::set(key, value)
    } else {
        crate::extensions::set(key, value)
    }
}
