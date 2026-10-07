//! Independent, read-only live squad exporter.
mod inspector;
pub use inspector::SquadInspector;
rekenshi_plugin_sdk::export_plugin!(SquadInspector);
