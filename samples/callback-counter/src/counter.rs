//! This is the entire plugin implementation; the SDK owns hook registration.
#![forbid(unsafe_code)]
use rekenshi_plugin_sdk::{Context, Control, Plugin, Setting, Settings, Value};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
static CALLS: AtomicU64 = AtomicU64::new(0);
static FRAMES: AtomicU64 = AtomicU64::new(0);
static DELTA: AtomicU32 = AtomicU32::new(0);
static ENABLED: AtomicBool = AtomicBool::new(false);
pub(crate) fn calls() -> u64 {
    CALLS.load(Ordering::Relaxed)
}
pub(crate) fn frames() -> u64 {
    FRAMES.load(Ordering::Relaxed)
}
pub(crate) fn delta() -> f32 {
    f32::from_bits(DELTA.load(Ordering::Relaxed))
}
#[derive(Default, Debug)]
pub struct CallbackCounter;
impl Plugin for CallbackCounter {
    fn id(&self) -> &'static str {
        "callback-counter"
    }
    fn title(&self) -> &'static str {
        "Callback counter sample"
    }
    fn settings(&self) -> &'static [Setting] {
        &[Setting {
            key: "callback_counter.enabled",
            label: "Count callbacks",
            description: "Observe Ogre calls and UI frames without replacing engine behavior.",
            default: Value::Toggle(true),
            control: Control::Toggle,
        }]
    }
    fn install(
        &mut self,
        context: &Context,
        settings: &Settings,
    ) -> Result<(), rekenshi_plugin_sdk::Error> {
        self.settings_changed(context, settings)?;
        context.host.hook_ogre_calls(|| {
            if ENABLED.load(Ordering::Relaxed) {
                CALLS.fetch_add(1, Ordering::Relaxed);
            }
        })?;
        if context.host.supports(rekenshi_plugin_sdk::Hook::UiFrame) {
            context.host.hook_ui_frame(|delta| {
                if ENABLED.load(Ordering::Relaxed) {
                    FRAMES.fetch_add(1, Ordering::Relaxed);
                    DELTA.store(delta.to_bits(), Ordering::Relaxed);
                }
            })?;
        }
        Ok(())
    }
    fn settings_changed(
        &mut self,
        _: &Context,
        settings: &Settings,
    ) -> Result<(), rekenshi_plugin_sdk::Error> {
        ENABLED.store(
            settings.enabled("callback_counter.enabled"),
            Ordering::Relaxed,
        );
        Ok(())
    }
    fn stop(&mut self) -> Result<(), rekenshi_plugin_sdk::Error> {
        ENABLED.store(false, Ordering::Relaxed);
        Ok(())
    }
}
