//! Heightmap-cache policy. Native TIFF reads share an owned read-only map.
#![forbid(unsafe_code)]
use crate::native;
use rekenshi_plugin_runtime::{Context, Control, Plugin, Setting, Settings, Value};

#[derive(Debug, Default)]
pub struct HeightmapCache {
    policy: rekenshi_plugin_sdk::policies::DesiredPolicy,
}
pub use native::reads;
const SETTINGS: &[Setting] = &[
    Setting {
        key: "performance.heightmap_cache",
        label: "Mapped cache",
        description: "Map the uncompressed heightmap for terrain reads. Applies immediately.",
        default: Value::Toggle(false),
        control: Control::Toggle,
    },
    Setting {
        key: "performance.heightmap_gigacache",
        label: "Gigacache",
        description: "Load the entire heightmap into memory (about 512 MiB). Enabling pauses briefly to load. Works independently of Mapped cache.",
        default: Value::Toggle(false),
        control: Control::Toggle,
    },
];
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
    fn register(
        &mut self,
        context: &Context,
        settings: &Settings,
    ) -> Result<(), rekenshi_plugin_runtime::Error> {
        self.policy.set(
            context.host.native(),
            rekenshi_plugin_sdk::policies::HEIGHTMAP,
            "rekenshi.builtin.heightmap-cache@1",
            (settings.enabled(SETTINGS[0].key) || settings.enabled(SETTINGS[1].key)).then_some([
                settings.enabled(SETTINGS[0].key) as u8 as f32,
                settings.enabled(SETTINGS[1].key) as u8 as f32,
                0.,
                0.,
            ]),
        )
    }
    fn install(
        &mut self,
        context: &Context,
        settings: &Settings,
    ) -> Result<(), rekenshi_plugin_runtime::Error> {
        native::start(
            context.host.native(),
            context.root.parent().ok_or("game directory")?,
        )?;
        self.settings_changed(context, settings)
    }
    fn settings_changed(
        &mut self,
        context: &Context,
        settings: &Settings,
    ) -> Result<(), rekenshi_plugin_runtime::Error> {
        self.register(context, settings)
    }

    fn stop(&mut self) -> Result<(), rekenshi_plugin_runtime::Error> {
        self.policy.remove()?;
        native::stop().map_err(rekenshi_plugin_runtime::Error::from)
    }
}
