//! Copied shader identity and scoped pixel-stage constant binding contracts.
//!
//! A shader-creation interceptor returns this record only for positively identified
//! bytecode. The native provider tags that COM object, then composes parameters on
//! matching draws, creates device-local buffers and restores the previous binding.
//! It never exposes a native COM pointer. Identity values must be stable namespaced
//! hashes chosen by the author; slot ownership is negotiated by source policy.
use crate::Error;

/// Four-float pixel constant binding; 32-byte little-endian wire record.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShaderBinding {
    /// Stable author-defined shader policy identity; zero is invalid.
    pub identity: u64,
    /// Pixel-stage constant slot, 0..13. Never occupy an existing shader declaration.
    pub slot: u32,
    /// Finite parameters copied into a provider-owned 16-byte buffer.
    pub parameters: [f32; 4],
}
impl ShaderBinding {
    /// Encode portable bytes, validating identity, slot and scalar bounds.
    ///
    /// # Errors
    /// Rejects invalid identity/slot or non-finite parameters.
    pub fn encode(self) -> Result<Vec<u8>, Error> {
        if self.identity == 0 || self.slot > 13 || !self.parameters.iter().all(|p| p.is_finite()) {
            return Err("invalid shader binding".into());
        }
        Ok([
            self.identity.to_le_bytes().as_slice(),
            self.slot.to_le_bytes().as_slice(),
            &[0; 4],
            &crate::policies::encode(self.parameters)?,
        ]
        .concat())
    }
    /// Decode the exact bounded schema, including reserved zero bytes.
    ///
    /// # Errors
    /// Rejects invalid size/reserved bytes or unsupported slot/parameters.
    pub fn decode(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() != 32 || bytes[12..16] != [0; 4] {
            return Err("invalid shader binding record".into());
        }
        let binding = Self {
            identity: u64::from_le_bytes(bytes[..8].try_into().map_err(|_| "shader identity")?),
            slot: u32::from_le_bytes(bytes[8..12].try_into().map_err(|_| "shader slot")?),
            parameters: crate::policies::decode(&bytes[16..])?,
        };
        binding.encode()?;
        Ok(binding)
    }
}

#[doc(inline)]
pub use crate::graphics::{EFFECTS, Effect, decode_effects};
