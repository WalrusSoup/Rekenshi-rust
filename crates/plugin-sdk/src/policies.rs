//! Owned scalar policies shared by bundled features and independent DLLs.
//!
//! Policies register before providers exist. Providers keep native objects local,
//! refresh copied policy only when its composition revision changes, and retire
//! resources on their native lane. Four little-endian finite f32 values form the
//! wire format; domain constants below document their meanings. These are desired
//! rules, not claims that a world, renderer, or UI hierarchy is ready.
use crate::{
    Error,
    abi::HostV1,
    operations::{Contract, Operations, Provider, Registration},
};
use std::sync::{Arc, Mutex, RwLock};

/// Shader cache: enabled (0/1); remaining fields reserved zero.
pub const SHADER_CACHE: &str = "rekenshi.shader.cache-policy@1";
/// Heightmap: mapped (0/1), eager (0/1); remaining fields reserved zero.
pub const HEIGHTMAP: &str = "rekenshi.terrain.cache-policy@1";
/// Collider: cache enabled (0/1); remaining fields reserved zero.
pub const COLLIDER: &str = "rekenshi.collider.cache-policy@1";
/// DDS: optimize (0/1), mip override (-1 uses game quality, otherwise 0..16).
pub const DDS: &str = "rekenshi.texture.dds-policy@1";
/// Texture sidecars: enabled (0/1); remaining fields reserved zero.
pub const TEXTURES: &str = "rekenshi.texture.sidecar-policy@1";
/// Weather: RGB factor and alpha factor (0..1); remaining fields reserved zero.
pub const WEATHER: &str = "rekenshi.particle.appearance-policy@1";
/// UI: enabled (0/1), panel brightness (0.16..0.60); remaining fields reserved zero.
pub const UI: &str = "rekenshi.ui.theme-policy@1";
/// Composed rain style: enabled (0/1), visibility (0..1), streak size (0.1..2), reserved.
pub const RAIN: &str = "rekenshi.particle.rain-style@1";
/// Copied wire schema; contract name determines the four scalar semantics.
pub const SCHEMA: &str = "rekenshi.finite-f32x4-le@1";

/// Encode four finite values without sharing Rust layout or allocation ownership.
pub fn encode(values: [f32; 4]) -> Result<Vec<u8>, Error> {
    if !values.iter().all(|v| v.is_finite()) {
        return Err("non-finite policy".into());
    }
    Ok(values.into_iter().flat_map(f32::to_le_bytes).collect())
}
/// Decode the exact bounded scalar schema, rejecting NaN/infinity and extra bytes.
pub fn decode(bytes: &[u8]) -> Result<[f32; 4], Error> {
    if bytes.len() != 16 {
        return Err("invalid scalar policy size".into());
    }
    let mut values = [0.; 4];
    for (value, chunk) in values.iter_mut().zip(bytes.chunks_exact(4)) {
        *value = f32::from_le_bytes(chunk.try_into().map_err(|_| "policy scalar")?);
    }
    if !values.iter().all(|v| v.is_finite()) {
        return Err("non-finite policy".into());
    }
    Ok(values)
}

/// Exclusive desired rule; dropping it restores provider defaults after quiescence.
#[derive(Debug, Default)]
pub struct DesiredPolicy {
    registration: Option<Registration>,
    values: Arc<RwLock<[f32; 4]>>,
}
impl DesiredPolicy {
    /// Register/update an owned policy; None releases ownership.
    ///
    /// # Errors
    /// Rejects conflicting owners, invalid scalars, poisoned state or host failures.
    pub fn set(
        &mut self,
        host: HostV1,
        name: &str,
        identity: &str,
        values: Option<[f32; 4]>,
    ) -> Result<(), Error> {
        host.platform().map_err(|_| "public platform unavailable")?;
        let Some(values) = values else {
            return self.remove();
        };
        encode(values)?;
        let previous = std::mem::replace(
            &mut *self.values.write().map_err(|_| "policy lock poisoned")?,
            values,
        );
        if let Some(registration) = &self.registration {
            if let Err(error) = registration.changed() {
                *self.values.write().map_err(|_| "policy lock poisoned")? = previous;
                return Err(error);
            }
            return Ok(());
        }
        let captured = Arc::clone(&self.values);
        self.registration =
            Some(
                Operations::from_host(host)?.replace(name, identity, move |_, _| {
                    let values = *captured.read().map_err(|_| "policy lock poisoned")?;
                    Ok(Some(encode(values)?))
                })?,
            );
        Ok(())
    }
    /// Release and quiesce the policy before dropping its captures.
    ///
    /// # Errors
    /// Returns a host quiescence error; the handle remains owned on failure.
    pub fn remove(&mut self) -> Result<(), Error> {
        if let Some(registration) = self.registration.as_mut() {
            registration.remove()?;
        }
        self.registration = None;
        Ok(())
    }
}

/// Native adapter's copied policy source, cached until ownership/settings change.
#[derive(Debug)]
pub struct PolicyProvider {
    provider: Provider,
    cache: Mutex<Option<(u64, [f32; 4])>>,
}
impl PolicyProvider {
    /// Publish after verified hooks are usable; publication conflicts are errors.
    ///
    /// # Errors
    /// Rejects malformed evidence or a competing provider.
    pub fn optional(host: HostV1, name: &str, evidence: &str) -> Result<Option<Self>, Error> {
        let contract = Contract::new(name, SCHEMA,
            "Native provider lane; copied desired policy only, no native pointers or broker reentry", evidence, 16)?.exclusive();
        Ok(Some(Self {
            provider: Operations::from_host(host)?.publish(contract)?,
            cache: Mutex::new(None),
        }))
    }
    /// Publish a composed scalar rule after native bindings activate.
    /// # Errors
    /// Rejects a competing provider or malformed evidence.
    pub fn composed(host: HostV1, name: &str, evidence: &str) -> Result<Self, Error> {
        Ok(Self {
            provider: Operations::from_host(host)?.publish(Contract::new(
                name,
                SCHEMA,
                "Synchronous native caller; copied desired scalar rules; no native access",
                evidence,
                16,
            )?)?,
            cache: Mutex::new(None),
        })
    }
    /// Obtain owned policy or safe provider defaults. This never grants native access.
    /// Callback failure immediately falls back; the broker quarantines panics.
    pub fn values(&self, default: [f32; 4]) -> [f32; 4] {
        let Ok(stamp) = self.provider.stamp() else {
            return default;
        };
        let Ok(mut cache) = self.cache.lock() else {
            return default;
        };
        if let Some((revision, values)) = *cache {
            if revision == stamp {
                return values;
            }
        }
        let result = encode(default)
            .and_then(|input| self.provider.dispatch(&[], &input))
            .and_then(|bytes| decode(&bytes));
        match result {
            Ok(values) => {
                *cache = Some((stamp, values));
                values
            }
            Err(_) => {
                *cache = None;
                default
            }
        }
    }
    /// Revoke and quiesce before native resource retirement.
    ///
    /// # Errors
    /// Returns a host quiescence/generation error.
    pub fn revoke(&mut self) -> Result<(), Error> {
        self.provider.revoke()
    }
}
