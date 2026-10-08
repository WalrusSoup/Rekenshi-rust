//! Reuse parsed collider templates while returning a fresh native collection.
#![forbid(unsafe_code)]
use crate::native;
pub use native::hits;
use rekenshi_plugin_runtime::{Context, Control, Plugin, Setting, Settings, Value};
#[derive(Debug, Default)]
pub struct ColliderCache {
    policy: rekenshi_plugin_sdk::policies::DesiredPolicy,
}
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
    fn register(
        &mut self,
        context: &Context,
        settings: &Settings,
    ) -> Result<(), rekenshi_plugin_runtime::Error> {
        self.policy.set(
            context.host.native(),
            rekenshi_plugin_sdk::policies::COLLIDER,
            "rekenshi.builtin.collider-cache@1",
            (settings.enabled(SETTINGS[0].key)).then_some([1., 0., 0., 0.]),
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
