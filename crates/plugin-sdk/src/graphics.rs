//! Owned desired post-tonemap effects, activated by any public renderer provider.
use crate::{
    Context, Error,
    operations::{Operations, Registration},
};
use std::sync::{
    Arc, RwLock,
    atomic::{AtomicU64, Ordering},
};
/// Public copied effect-list contract. Records: u32 byte length, i32 priority,
/// sixteen finite f32 parameters, then UTF-8 HLSL bytes. At most 32 records.
pub const EFFECTS: &str = "rekenshi.render.post-tonemap-effects@1";
static SERIAL: AtomicU64 = AtomicU64::new(1);
/// Owned effect rule. Provider absence leaves it pending; Drop quiesces its rule.
#[derive(Debug)]
pub struct PostProcess {
    registration: Registration,
    parameters: Arc<RwLock<[f32; 16]>>,
}
impl PostProcess {
    /// Register copied shader policy before native resources exist.
    ///
    /// # Errors
    /// Rejects invalid source, non-finite parameters, ownership conflicts or host failure.
    pub fn new(
        context: &Context,
        shader: &[u8],
        priority: i32,
        parameters: [f32; 16],
    ) -> Result<Self, Error> {
        if shader.is_empty()
            || shader.len() > 256 * 1024
            || std::str::from_utf8(shader).is_err()
            || !parameters.iter().all(|v| v.is_finite())
        {
            return Err("invalid effect source or parameters".into());
        }
        let parameters = Arc::new(RwLock::new(parameters));
        let captured = Arc::clone(&parameters);
        let source = shader.to_vec();
        let serial = SERIAL.fetch_add(1, Ordering::Relaxed);
        let registration = Operations::new(context)?.transform(
            EFFECTS,
            &format!("rekenshi.effect.{serial}@1"),
            priority,
            move |_, input| {
                if decode_effects(input)?.len() >= 32 {
                    return Err("too many effects".into());
                }
                let mut result = input.to_vec();
                result.extend_from_slice(&((68 + source.len()) as u32).to_le_bytes());
                result.extend_from_slice(&priority.to_le_bytes());
                for value in *captured.read().map_err(|_| "effect parameters poisoned")? {
                    result.extend_from_slice(&value.to_le_bytes());
                }
                result.extend_from_slice(&source);
                Ok(Some(result))
            },
        )?;
        context.log(
            "owned post-tonemap effect policy registered; provider activation pending native draw",
        );
        Ok(Self {
            registration,
            parameters,
        })
    }
    /// Update desired finite parameters; the provider rebuilds on its next eligible draw.
    ///
    /// # Errors
    /// Rejects non-finite values or a host quiescence failure, preserving old parameters.
    pub fn update(&mut self, parameters: [f32; 16]) -> Result<(), Error> {
        if !parameters.iter().all(|v| v.is_finite()) {
            return Err("non-finite effect parameters".into());
        }
        let old = std::mem::replace(
            &mut *self
                .parameters
                .write()
                .map_err(|_| "effect parameters poisoned")?,
            parameters,
        );
        if let Err(error) = self.registration.changed() {
            *self
                .parameters
                .write()
                .map_err(|_| "effect parameters poisoned")? = old;
            return Err(error);
        }
        Ok(())
    }
    /// Release desired behavior after admitted callbacks finish.
    ///
    /// # Errors
    /// Host quiescence failure retains ownership.
    pub fn remove(&mut self) -> Result<(), Error> {
        self.registration.remove()
    }
}
/// Provider-side owned decoded effect specification; contains no native resources.
#[derive(Debug)]
pub struct Effect {
    /// Ascending pass order.
    pub priority: i32,
    /// Copied finite shader constants.
    pub parameters: [f32; 16],
    /// Copied UTF-8 shader source.
    pub source: Vec<u8>,
}
/// Validate an entire effect-list transaction before compiling or changing resources.
///
/// # Errors
/// Rejects truncated, excessive, non-finite or malformed effect records.
pub fn decode_effects(mut input: &[u8]) -> Result<Vec<Effect>, Error> {
    let mut result = Vec::new();
    while !input.is_empty() {
        if input.len() < 4 || result.len() == 32 {
            return Err("invalid effect list".into());
        }
        let length =
            u32::from_le_bytes(input[..4].try_into().map_err(|_| "effect length")?) as usize;
        input = &input[4..];
        if !(69..=68 + 256 * 1024).contains(&length) || length > input.len() {
            return Err("invalid effect record".into());
        }
        let record = &input[..length];
        input = &input[length..];
        let priority = i32::from_le_bytes(record[..4].try_into().map_err(|_| "effect priority")?);
        let mut parameters = [0.; 16];
        for (value, bytes) in parameters.iter_mut().zip(record[4..68].chunks_exact(4)) {
            *value = f32::from_le_bytes(bytes.try_into().map_err(|_| "effect parameter")?);
        }
        if !parameters.iter().all(|v| v.is_finite()) || std::str::from_utf8(&record[68..]).is_err()
        {
            return Err("invalid effect values".into());
        }
        result.push(Effect {
            priority,
            parameters,
            source: record[68..].to_vec(),
        });
    }
    Ok(result)
}
