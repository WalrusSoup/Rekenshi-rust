//! Graphics backend plugin and owned interface for effect plugins.
#![forbid(unsafe_code)]
use crate::native;
use rekenshi_plugin_api::{HostV1, OK};
use rekenshi_plugin_runtime::{Context, Plugin, Settings};
use std::path::Path;
#[derive(Debug)]
pub struct GraphicsBackend;

/// Owned post-tonemap pass. The backend copies the shader and owns all COM
/// objects; effect plugins receive no native pointers.
#[derive(Debug)]
pub struct PostProcess {
    owner: u64,
    handle: Option<u64>,
}
impl PostProcess {
    pub fn new(
        context: &Context,
        shader: &[u8],
        priority: i32,
        parameters: [f32; 16],
    ) -> Result<Self, rekenshi_plugin_runtime::Error> {
        Ok(Self {
            owner: context.host.native().plugin_token,
            handle: Some(native::register_owned(
                context.host.native().plugin_token,
                shader,
                priority,
                parameters,
            )?),
        })
    }
    pub fn update(&mut self, parameters: [f32; 16]) -> Result<(), rekenshi_plugin_runtime::Error> {
        native::update_effect(
            self.owner,
            self.handle.ok_or("effect already removed")?,
            parameters,
        )
        .map_err(rekenshi_plugin_runtime::Error::from)
    }
    pub fn remove(&mut self) -> Result<(), rekenshi_plugin_runtime::Error> {
        if let Some(handle) = self.handle {
            native::remove_owned(self.owner, handle)?;
            self.handle = None;
        }
        Ok(())
    }
}
impl Drop for PostProcess {
    fn drop(&mut self) {
        let _ = self.remove();
    }
}
impl Plugin for GraphicsBackend {
    fn id(&self) -> &'static str {
        "graphics"
    }
    fn title(&self) -> &'static str {
        "Graphics backend"
    }
    fn start(
        &mut self,
        context: &Context,
        _: &Settings,
    ) -> Result<(), rekenshi_plugin_runtime::Error> {
        native::initialize(context.host.native(), &context.root)
            .map_err(rekenshi_plugin_runtime::Error::from)
    }
    fn update(&mut self, _: &Context, _: &Settings) -> Result<(), rekenshi_plugin_runtime::Error> {
        Ok(())
    }
    fn stop(&mut self) -> Result<(), rekenshi_plugin_runtime::Error> {
        stop()
    }
}
pub fn start_legacy(host: HostV1, root: &Path) -> Result<(), rekenshi_plugin_runtime::Error> {
    native::initialize(host, root).map_err(rekenshi_plugin_runtime::Error::from)
}
pub fn stop() -> Result<(), rekenshi_plugin_runtime::Error> {
    if native::stop() == OK {
        Ok(())
    } else {
        Err("graphics shutdown failed".into())
    }
}
pub use native::api_v2 as export_api_v2;
pub use native::{passes, rekenshi_graphics_api as export_api};
