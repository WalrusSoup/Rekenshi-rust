//! Provider helpers for copied native operation inputs/results.
//!
//! Native adapters validate transformed data before use and keep original-operation
//! fallback. No native object, allocator, trait object or borrowed Rust state crosses
//! the broker. Input/result transformations must be reversible before native commit.
use crate::{
    Error,
    abi::HostV1,
    operations::{Contract, Operations, Provider},
};

/// Public load/read operation identifiers; metadata is documented by each adapter.
pub const HEIGHTMAP_READ: &str = "rekenshi.terrain.read@1";
/// Terrain TIFF open: UTF-8 filename; i32 native open mode metadata. Replacement paths bypass mismatched cached maps.
pub const HEIGHTMAP_LOAD: &str = "rekenshi.terrain.open@1";
/// Collider request: cache flag and XYZ scale as f32x4; i32 kind then filename metadata.
pub const COLLIDER_LOAD: &str = "rekenshi.collider.load@1";
/// DDS mip selection: u32 little-endian input; filename NUL group metadata.
pub const DDS_MIPS: &str = "rekenshi.texture.dds-mips@1";
/// Texture selection: UTF-8 filename; UTF-8 archive folder metadata.
pub const TEXTURE_SELECT: &str = "rekenshi.texture.select@1";
/// SIC decode result: pixel bytes; width/height/format u32 little-endian metadata.
pub const TEXTURE_DECODE: &str = "rekenshi.texture.decode@1";
/// MyGUI color: four f32 little-endian channels; kind (panel/text) metadata.
pub const UI_COLOUR: &str = "rekenshi.ui.colour@1";
/// MyGUI lifetime observation: created/skin/destroy kind and opaque generation ID.
pub const UI_WIDGET: &str = "rekenshi.ui.widget-observed@1";
/// Skin classification: surface flag and minimum brightness as f32x4; UTF-8 skin metadata.
pub const UI_SKIN: &str = "rekenshi.ui.skin-policy@1";
/// Particle billboard appearance/geometry: copied finite scalars, no native pointer.
pub const PARTICLE: &str = "rekenshi.particle.billboard@1";
/// Successful compilation bytecode: copied DXBC; entry NUL profile metadata.
pub const SHADER_BYTECODE: &str = "rekenshi.shader.bytecode@1";
/// Successful pixel-shader creation: DXBC input; bounded binding specification output.
pub const SHADER_BINDING: &str = "rekenshi.render.shader-binding@1";
/// Tagged draw: copied binding specification; native provider scopes/restores binding.
pub const DRAW_PARAMETERS: &str = "rekenshi.render.draw-parameters@1";

/// Collection of verified native operations owned by one adapter generation.
#[derive(Debug, Default)]
pub struct AdapterOperations {
    providers: Vec<Provider>,
}
impl AdapterOperations {
    /// Whether copied-operation callbacks are active, without allocating payloads.
    pub fn active(&self, name: &str) -> bool {
        self.providers
            .iter()
            .find(|p| p.name() == name)
            .and_then(|p| p.subscribers().ok())
            .is_some_and(|n| n != 0)
    }
    /// Publish each operation only after its native adapter is usable.
    ///
    /// # Errors
    /// Conflicting providers or malformed descriptions abort the entire publication.
    pub fn publish(
        host: HostV1,
        evidence: &str,
        contracts: &[(&str, &str, usize)],
    ) -> Result<Self, Error> {
        let operations = Operations::from_host(host)?;
        let mut providers = Vec::new();
        for (name, schema, limit) in contracts {
            providers.push(operations.publish(Contract::new(name, schema,
                "Synchronous native caller lane; copied bounded bytes; no native access or broker reentry", evidence, *limit)?)?);
        }
        Ok(Self { providers })
    }
    /// Return a transformed copy or None for unchanged/failing/unavailable operations.
    /// Providers must validate the result before committing it to native state.
    pub fn transform(&self, name: &str, metadata: &[u8], input: &[u8]) -> Option<Vec<u8>> {
        let provider = self.providers.iter().find(|p| p.name() == name)?;
        if provider.subscribers().ok()? == 0 {
            return None;
        }
        let result = provider
            .dispatch_bounded(
                metadata,
                input,
                if matches!(name, TEXTURE_SELECT | HEIGHTMAP_LOAD | SHADER_BYTECODE) {
                    provider.limit()
                } else {
                    input.len().max(32).min(provider.limit())
                },
            )
            .ok()?;
        (result != input).then_some(result)
    }
    /// Revision used by providers to refresh live rules without polling callbacks.
    pub fn revision(&self, name: &str) -> u64 {
        self.providers
            .iter()
            .find(|p| p.name() == name)
            .and_then(|p| p.stamp().ok())
            .unwrap_or(0)
    }
    /// Revoke all operations before native resources are retired.
    ///
    /// # Errors
    /// Quiescence failure requires retaining native state and restarting.
    pub fn revoke(&mut self) -> Result<(), Error> {
        for provider in &mut self.providers {
            provider.revoke()?;
        }
        Ok(())
    }
}
