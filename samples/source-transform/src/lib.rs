//! Independent compiler-policy example using the same public broker as bundled AO.
use rekenshi_plugin_api::platform::{READY, StatusV1};
use rekenshi_plugin_sdk::{
    Context, Control, Error, Plugin, Setting, Settings, Value,
    operations::{Operations, Registration, SHADER_SOURCE, shader_metadata},
};
use std::sync::atomic::{AtomicU64, Ordering};

static WIDGET_EVENTS: AtomicU64 = AtomicU64::new(0);
static CALLS: AtomicU64 = AtomicU64::new(0);
static LATE_REGISTRATIONS: AtomicU64 = AtomicU64::new(0);
static READY_NOTIFICATIONS: AtomicU64 = AtomicU64::new(0);

#[derive(Default, Debug)]
struct SourceTransform {
    source: Option<Registration>,
    ready_generations: u64,
    rules: Vec<Registration>,
    heightmap: rekenshi_plugin_sdk::policies::DesiredPolicy,
    codec: Option<rekenshi_plugin_sdk::codecs::Codec>,
    effect: Option<rekenshi_plugin_sdk::PostProcess>,
}
impl Plugin for SourceTransform {
    fn id(&self) -> &'static str {
        "source-transform"
    }
    fn settings(&self) -> &'static [Setting] {
        &[
            Setting {
                key: "source_transform.enabled",
                label: "Source transform example",
                description: "Transform only shaders carrying the example marker. Existing shaders require recompilation.",
                default: Value::Toggle(true),
                control: Control::Toggle,
            },
            Setting {
                key: "runtime_policy.enabled",
                label: "Copied native rule examples",
                description: "Fixture/example rules for heightmap, DDS, collider, UI, particles and a custom image codec. Register before native loads.",
                default: Value::Toggle(false),
                control: Control::Toggle,
            },
        ]
    }
    fn event_contracts(&self) -> &'static [&'static str] {
        &[rekenshi_plugin_sdk::adapters::UI_WIDGET]
    }
    fn event(
        &mut self,
        _: &Context,
        _: &str,
        event: &rekenshi_plugin_sdk::events::EventV1,
    ) -> Result<(), Error> {
        if event.data().len() != 12 {
            return Err("invalid widget observation".into());
        }
        WIDGET_EVENTS.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }
    fn lifecycle_contracts(&self) -> &'static [&'static str] {
        &[SHADER_SOURCE]
    }
    fn lifecycle_changed(&mut self, _: &Context, _: &str, status: StatusV1) -> Result<(), Error> {
        if status.phase == READY {
            self.ready_generations += 1;
            READY_NOTIFICATIONS.fetch_add(1, Ordering::Relaxed);
        }
        Ok(())
    }
    fn register(&mut self, context: &Context, settings: &Settings) -> Result<(), Error> {
        if settings.enabled("source_transform.enabled") {
            let source = Operations::new(context)?.transform(
                SHADER_SOURCE,
                "example.source-transform@1",
                -10,
                |metadata, source| {
                    let Some((entry, profile)) = shader_metadata(metadata) else {
                        return Ok(None);
                    };
                    if entry != b"main" || profile != b"ps_5_0" {
                        return Ok(None);
                    }
                    let Ok(text) = std::str::from_utf8(source) else {
                        return Ok(None);
                    };
                    if text.contains("// reject-source-transform") {
                        return Err("sample rejected transformation; preserve original".into());
                    }
                    if !text.contains("// example-source-transform") {
                        return Ok(None);
                    }
                    let anchor = "float4(1,0,0,1)";
                    if text.matches(anchor).count() != 1 {
                        return Ok(None);
                    }
                    CALLS.fetch_add(1, Ordering::Relaxed);
                    Ok(Some(
                        text.replacen(anchor, "float4(0,1,0,1)", 1).into_bytes(),
                    ))
                },
            )?;
            if source.late() {
                LATE_REGISTRATIONS.fetch_add(1, Ordering::Relaxed);
                context.log("source-transform: existing shaders need recompilation or restart");
            }
            self.source = Some(source);
        }
        if settings.enabled("runtime_policy.enabled") && self.rules.is_empty() {
            use rekenshi_plugin_sdk::{adapters, policies, render::ShaderBinding};
            let operations = Operations::new(context)?;
            self.heightmap.set(
                context.host.native(),
                policies::HEIGHTMAP,
                "example.terrain-policy@1",
                Some([1., 0., 0., 0.]),
            )?;
            self.rules.push(operations.transform(
                adapters::HEIGHTMAP_LOAD,
                "example.terrain-open@1",
                0,
                |_, input| {
                    let Ok(path) = std::str::from_utf8(input) else {
                        return Ok(None);
                    };
                    Ok(path
                        .ends_with("redirect.tif")
                        .then(|| path.replace("redirect.tif", "fullmap.tif").into_bytes()))
                },
            )?);
            self.rules.push(operations.transform(
                adapters::HEIGHTMAP_READ,
                "example.terrain-read@1",
                0,
                |_, input| {
                    let result = input
                        .chunks_exact(2)
                        .flat_map(|bytes| {
                            u16::from_le_bytes([bytes[0], bytes[1]])
                                .saturating_add(7)
                                .to_le_bytes()
                        })
                        .collect();
                    Ok(Some(result))
                },
            )?);
            self.rules.push(operations.transform(
                adapters::DDS_MIPS,
                "example.dds-mips@1",
                0,
                |_, _| Ok(Some(2u32.to_le_bytes().to_vec())),
            )?);
            self.rules.push(operations.transform(
                adapters::COLLIDER_LOAD,
                "example.collider@1",
                0,
                |_, _| Ok(Some(policies::encode([1., 2., 2., 2.])?)),
            )?);
            self.rules.push(operations.transform(
                adapters::UI_COLOUR,
                "example.theme@1",
                0,
                |_, input| {
                    let mut colour = policies::decode(input)?;
                    colour[0] = 0.25;
                    Ok(Some(policies::encode(colour)?))
                },
            )?);
            self.rules.push(operations.transform(
                adapters::PARTICLE,
                "example.billboard@1",
                0,
                |_, input| {
                    if input.len() != 40 {
                        return Err("invalid particle record".into());
                    }
                    let mut output = input.to_vec();
                    output[12..16].copy_from_slice(&0.123f32.to_le_bytes());
                    Ok(Some(output))
                },
            )?);
            self.rules.push(operations.transform(
                adapters::SHADER_BINDING,
                "example.binding@1",
                -20,
                |_, code| {
                    if !code
                        .windows(b"ExampleConstants\0".len())
                        .any(|bytes| bytes == b"ExampleConstants\0")
                    {
                        return Ok(None);
                    }
                    Ok(Some(
                        ShaderBinding {
                            identity: 2,
                            slot: 12,
                            parameters: [0.; 4],
                        }
                        .encode()?,
                    ))
                },
            )?);
            self.rules.push(operations.transform(
                adapters::DRAW_PARAMETERS,
                "example.draw@1",
                -20,
                |_, record| {
                    let mut binding = ShaderBinding::decode(record)?;
                    if binding.identity != 2 {
                        return Ok(None);
                    }
                    binding.parameters = [0.25, 0.5, 0.75, 1.];
                    Ok(Some(binding.encode()?))
                },
            )?);
            self.effect=Some(rekenshi_plugin_sdk::PostProcess::new(context,b"float4 main(float4 p:SV_Position,float2 uv:TEXCOORD0):SV_Target{return float4(0.2,0.4,0.6,1);}",0,[0.;16])?);
            self.codec = Some(rekenshi_plugin_sdk::codecs::Codec::new(
                context,
                "rki",
                b"RKI",
                |encoded| {
                    if encoded.len() != 7 || &encoded[..3] != b"RKI" {
                        return Err("invalid example image".into());
                    }
                    Ok(rekenshi_plugin_sdk::codecs::Image {
                        width: 1,
                        height: 1,
                        format: 12,
                        pixels: encoded[3..].to_vec(),
                    })
                },
            )?);
        }
        Ok(())
    }
    fn settings_changed(&mut self, context: &Context, settings: &Settings) -> Result<(), Error> {
        if settings.enabled("runtime_policy.enabled") != !self.rules.is_empty() {
            return Err("runtime rule examples change codec registration; restart with desired setting required".into());
        }
        if settings.enabled("source_transform.enabled") {
            if self.source.is_none() {
                self.register(context, settings)?;
            }
        } else {
            if let Some(source) = self.source.as_mut() {
                source.remove()?;
            }
            self.source = None;
        }
        Ok(())
    }
    fn stop(&mut self) -> Result<(), Error> {
        if let Some(codec) = self.codec.as_mut() {
            codec.remove()?;
        }
        self.codec = None;
        if let Some(effect) = self.effect.as_mut() {
            effect.remove()?;
        }
        self.effect = None;
        for rule in &mut self.rules {
            rule.remove()?;
        }
        self.rules.clear();
        self.heightmap.remove()?;
        if let Some(source) = self.source.as_mut() {
            source.remove()?;
        }
        self.source = None;
        Ok(())
    }
}
rekenshi_plugin_sdk::export_plugin!(SourceTransform);

/// Return the number of matched source transformations for fixture diagnostics.
// SAFETY: Unique scalar-only diagnostic export.
#[unsafe(no_mangle)]
pub extern "C" fn rekenshi_source_transform_calls() -> u64 {
    CALLS.load(Ordering::Relaxed)
}
/// Return the number of registrations that missed an earlier compilation.
// SAFETY: Unique scalar-only diagnostic export.
#[unsafe(no_mangle)]
pub extern "C" fn rekenshi_source_transform_late() -> u64 {
    LATE_REGISTRATIONS.load(Ordering::Relaxed)
}
/// Return copied readiness deliveries; ordinary state is changed only during serial dispatch.
// SAFETY: Unique scalar-only diagnostic export.
#[unsafe(no_mangle)]
pub extern "C" fn rekenshi_source_transform_ready() -> u64 {
    READY_NOTIFICATIONS.load(Ordering::Relaxed)
}

/// Number of deferred widget observations; no native access is granted.
// SAFETY: Unique scalar-only fixture export.
#[unsafe(no_mangle)]
pub extern "C" fn rekenshi_widget_events() -> u64 {
    WIDGET_EVENTS.load(Ordering::Relaxed)
}
