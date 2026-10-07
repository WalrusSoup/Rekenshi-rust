//! Reuse parsed collider templates while returning a fresh native collection.
#![forbid(unsafe_code)]
use crate::native;
pub use native::hits;
use rekenshi_plugin_runtime::{Context, Control, Plugin, Setting, Settings, Value};
#[derive(Debug)]
pub struct ColliderCache;
const SETTINGS: &[Setting] = &[Setting {
    key: "performance.collider_cache",
    label: "Enabled",
    description: "Cache up to 128 parsed colliders. Applies to new loads; disabling frees templates.",
    default: Value::Toggle(false),
    control: Control::Toggle,
}];
impl Plugin for ColliderCache {
    fn id(&self) -> &'static str {
        "collider-cache"
    }
    fn title(&self) -> &'static str {
        "Collider cache"
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
