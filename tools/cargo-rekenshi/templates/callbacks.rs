#![forbid(unsafe_code)]
use rekenshi_plugin_sdk::{Context,Control,Plugin,Setting,Settings,Value};
use std::sync::{Arc,atomic::{AtomicBool,AtomicU64,Ordering}};
#[derive(Default)]
pub struct MyMod { enabled:Arc<AtomicBool>, calls:Arc<AtomicU64> }
impl Plugin for MyMod {
    fn id(&self)->&'static str { "__ID__" }
    fn title(&self)->&'static str { "__ID__" }
    fn settings(&self)->&'static [Setting] { &[Setting {
        key:"__ID__.enabled", label:"Enabled", description:"Enable this mod.",
        default:Value::Toggle(true), control:Control::Toggle,
    }] }
    fn install(&mut self,context:&Context,settings:&Settings)->Result<(),rekenshi_plugin_sdk::Error> {
        self.settings_changed(context,settings)?;
        let enabled=self.enabled.clone();
        let calls=self.calls.clone();
        context.host.hook_ogre_calls(move || {
            if enabled.load(Ordering::Relaxed) {
                calls.fetch_add(1,Ordering::Relaxed);
                // Add your behavior here. This observer preserves the original call.
            }
        })?;
        context.log("Loaded successfully; observing Ogre calls");
        // context.host.hook_ui_frame(move |delta| { /* UI frame behavior */ })?;
        Ok(())
    }
    fn settings_changed(&mut self,context:&Context,settings:&Settings)->Result<(),rekenshi_plugin_sdk::Error> {
        self.enabled.store(settings.enabled("__ID__.enabled"),Ordering::Relaxed);
        context.log(&format!("Enabled: {}; observed calls: {}",settings.enabled("__ID__.enabled"),self.calls.load(Ordering::Relaxed)));
        Ok(())
    }
}
