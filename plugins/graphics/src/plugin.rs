//! Graphics backend plugin and owned interface for effect plugins.
#![forbid(unsafe_code)]
use crate::native;
use rekenshi_plugin_api::OK;
use rekenshi_plugin_runtime::{Context, Control, Plugin, Setting, Settings, Value};
use rekenshi_plugin_sdk::operations::{Operations, Registration, SHADER_SOURCE, shader_metadata};
#[derive(Debug, Default)]
pub struct GraphicsBackend {
    source: Option<Registration>,
    identity: Option<Registration>,
    draw: Option<Registration>,
    parameters: std::sync::Arc<std::sync::RwLock<[f32; 4]>>,
}
const SETTINGS: &[Setting] = &[
    Setting {
        key: "graphics.ssao",
        label: "Ambient occlusion (experimental)",
        description: "Contact shading in ambient light before fog and UI. Applies immediately. Off by default pending in-game validation.",
        default: Value::Toggle(false),
        control: Control::Toggle,
    },
    Setting {
        key: "graphics.ssao_strength",
        label: "AO strength",
        description: "Amount of contact shading. Zero preserves original lighting.",
        default: Value::Number(0.6),
        control: Control::Number {
            min: 0.,
            max: 2.,
            step: 0.1,
        },
    },
    Setting {
        key: "graphics.ssao_radius",
        label: "AO radius (metres)",
        description: "Neighbourhood size for contact shading. Large radii can produce halos.",
        default: Value::Number(1.5),
        control: Control::Number {
            min: 0.1,
            max: 5.,
            step: 0.1,
        },
    },
    Setting {
        key: "graphics.ssao_debug",
        label: "AO debug view",
        description: "0: normal scene, 1: occlusion, 2: depth (0–100m), 3: world normals. Requires AO enabled; fog still follows.",
        default: Value::Number(0.),
        control: Control::Number {
            min: 0.,
            max: 3.,
            step: 1.,
        },
    },
    Setting {
        key: "graphics.resource_capture",
        label: "Capture graphics resources",
        description: "Record texture/buffer sizes and lifetimes to graphics-captures. Enable before restart for startup coverage. Diagnostic overhead; applies immediately.",
        default: Value::Toggle(false),
        control: Control::Toggle,
    },
    Setting {
        key: "graphics.landscape_instancing",
        label: "Share scenery geometry (experimental)",
        description: "Use shared meshes and per-placement transforms for supported static scenery. Affects newly loaded regions; reload your save after changing. Unsupported batches keep normal rendering.",
        default: Value::Toggle(false),
        control: Control::Toggle,
    },
];

impl Plugin for GraphicsBackend {
    fn id(&self) -> &'static str {
        "graphics"
    }
    fn title(&self) -> &'static str {
        "Graphics"
    }
    fn settings(&self) -> &'static [Setting] {
        SETTINGS
    }
    fn register(
        &mut self,
        context: &Context,
        _: &Settings,
    ) -> Result<(), rekenshi_plugin_runtime::Error> {
        let registration = Operations::new(context)?.transform(
            SHADER_SOURCE,
            "rekenshi.graphics.ambient-occlusion@1",
            0,
            |metadata, source| {
                Ok(shader_metadata(metadata).and_then(|(entry, profile)| {
                    crate::ssao::lighting_shader(source, entry, profile)
                }))
            },
        )?;
        if registration.late() {
            context.log(
                "AO source registration missed compilation; restart required for existing shaders",
            );
        }
        self.source = Some(registration);
        const ID: u64 = 0x72656b656e736169;
        self.identity = Some(Operations::new(context)?.transform(
            rekenshi_plugin_sdk::adapters::SHADER_BINDING,
            "rekenshi.graphics.lighting-identity@1",
            0,
            |_, bytes| {
                let marker = b"REKenshiSsaoConstants\0";
                if bytes.windows(marker.len()).any(|v| v == marker) {
                    Ok(Some(
                        rekenshi_plugin_sdk::render::ShaderBinding {
                            identity: ID,
                            slot: 13,
                            parameters: [0., 1.5, 0.04, 0.],
                        }
                        .encode()?,
                    ))
                } else {
                    Ok(None)
                }
            },
        )?);
        let parameters = std::sync::Arc::clone(&self.parameters);
        self.draw = Some(Operations::new(context)?.transform(
            rekenshi_plugin_sdk::adapters::DRAW_PARAMETERS,
            "rekenshi.graphics.lighting-draw@1",
            0,
            move |_, bytes| {
                let mut binding = rekenshi_plugin_sdk::render::ShaderBinding::decode(bytes)?;
                if binding.identity != ID {
                    return Ok(None);
                }
                binding.parameters = *parameters.read().map_err(|_| "AO policy lock poisoned")?;
                Ok(Some(binding.encode()?))
            },
        )?);
        Ok(())
    }
    fn install(
        &mut self,
        context: &Context,
        settings: &Settings,
    ) -> Result<(), rekenshi_plugin_runtime::Error> {
        native::initialize(context.host.native(), &context.root)?;
        self.settings_changed(context, settings)
    }
    fn settings_changed(
        &mut self,
        context: &Context,
        settings: &Settings,
    ) -> Result<(), rekenshi_plugin_runtime::Error> {
        let enabled = settings.enabled(SETTINGS[0].key);
        let parameters = [
            if enabled {
                settings.number(SETTINGS[1].key) as f32
            } else {
                0.
            },
            settings.number(SETTINGS[2].key) as f32,
            0.04, // Four-centimetre bias prevents numerical self-occlusion.
            if enabled {
                settings.number(SETTINGS[3].key).round() as f32
            } else {
                0.
            },
        ];
        *self
            .parameters
            .write()
            .map_err(|_| "AO policy lock poisoned")? = parameters;
        native::capture::configure(&context.root, settings.enabled("graphics.resource_capture"))?;
        native::instancing::configure(settings.enabled("graphics.landscape_instancing"))?;
        Ok(())
    }
    fn stop(&mut self) -> Result<(), rekenshi_plugin_runtime::Error> {
        if let Some(draw) = self.draw.as_mut() {
            draw.remove()?;
        }
        self.draw = None;
        if let Some(identity) = self.identity.as_mut() {
            identity.remove()?;
        }
        self.identity = None;
        stop()?;
        if let Some(source) = self.source.as_mut() {
            source.remove()?;
        }
        self.source = None;
        Ok(())
    }
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
