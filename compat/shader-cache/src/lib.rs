//! Independently compiled SDK plugin using the same public registrations as bundled policy.
rekenshi_plugin_sdk::export_plugin!(rekenshi_shader_cache::ShaderCache);
// SAFETY: Unique scalar diagnostic export.
#[unsafe(no_mangle)]
pub extern "C" fn rekenshi_shader_cache_hits() -> u64 {
    rekenshi_shader_cache::hits()
}
