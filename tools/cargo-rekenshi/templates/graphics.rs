#![forbid(unsafe_code)]
use rekenshi_plugin_sdk::{Context,Control,Plugin,PostProcess,Setting,Settings,Value};
#[derive(Default)]
pub struct MyMod { effect:Option<PostProcess> }
impl Plugin for MyMod {
    fn id(&self)->&'static str { "__ID__" }
    fn title(&self)->&'static str { "__ID__" }
    fn requires(&self)->&'static [&'static str] { &["graphics"] }
    fn settings(&self)->&'static [Setting] { &[Setting {
        key:"__ID__.enabled", label:"Enabled", description:"Apply the world shader.",
        default:Value::Toggle(true), control:Control::Toggle,
    }] }
    fn start(&mut self,context:&Context,settings:&Settings)->Result<(),rekenshi_plugin_sdk::Error> { self.update(context,settings) }
    fn update(&mut self,context:&Context,settings:&Settings)->Result<(),rekenshi_plugin_sdk::Error> {
        if !settings.enabled("__ID__.enabled") { return self.stop(); }
        let parameters=[1.,1.,1.,1.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.];
        match self.effect.as_mut() {
            Some(effect)=>effect.update(parameters),
            None=>{self.effect=Some(PostProcess::new(context,include_bytes!("effect.hlsl"),200,parameters)?);Ok(())}
        }
    }
    fn stop(&mut self)->Result<(),rekenshi_plugin_sdk::Error> {
        if let Some(effect)=self.effect.as_mut() { effect.remove()?; }
        self.effect=None; Ok(())
    }
}
