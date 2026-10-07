//! Heightmap-cache policy. Native TIFF reads share an owned read-only map.
#![forbid(unsafe_code)]
use crate::native;
use rekenshi_plugin_runtime::{Context, Control, Plugin, Setting, Settings, Value};

#[derive(Debug)]
pub struct HeightmapCache;
pub use native::reads;
const SETTINGS: &[Setting] = &[Setting {
    key: "performance.heightmap_cache",
    label: "Enabled",
    description: "Map the uncompressed heightmap for terrain reads. Applies immediately.",
    default: Value::Toggle(false),
    control: Control::Toggle,
}];
impl Plugin for HeightmapCache {
    fn id(&self) -> &'static str {
        "heightmap-cache"
    }
    fn title(&self) -> &'static str {
        "Heightmap cache"
    }
    fn settings(&self) -> &'static [Setting] {
        SETTINGS
    }
    fn start(
        &mut self,
        context: &Context,
        settings: &Settings,
    ) -> Result<(), rekenshi_plugin_runtime::Error> {
        native::start(
            context.host.native(),
            context.root.parent().ok_or("game directory")?,
        )?;
        self.update(context, settings)
    }
    fn update(
        &mut self,
        _: &Context,
        settings: &Settings,
    ) -> Result<(), rekenshi_plugin_runtime::Error> {
        native::set_enabled(settings.enabled(SETTINGS[0].key));
        Ok(())
    }
    fn stop(&mut self) -> Result<(), rekenshi_plugin_runtime::Error> {
        native::stop().map_err(rekenshi_plugin_runtime::Error::from)
    }
}
