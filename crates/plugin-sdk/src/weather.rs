//! Owned desired rain style, independent of renderer availability and DLL location.
use crate::{
    Context, Error,
    operations::{Operations, Registration},
    policies,
};
use std::sync::{
    Arc, RwLock,
    atomic::{AtomicU64, Ordering},
};
static SERIAL: AtomicU64 = AtomicU64::new(1);
/// Owned rain rule. Native renderer providers perform geometry and camera fade.
#[derive(Debug)]
pub struct RainRendering {
    registration: Registration,
    values: Arc<RwLock<[f32; 4]>>,
}
impl RainRendering {
    /// Register desired style before its native provider exists.
    /// # Errors
    /// Returns a host registration or negotiation failure.
    pub fn new(context: &Context) -> Result<Self, Error> {
        let values = Arc::new(RwLock::new([0., 1., 1., 0.]));
        let captured = Arc::clone(&values);
        let serial = SERIAL.fetch_add(1, Ordering::Relaxed);
        let registration = Operations::new(context)?.transform(
            policies::RAIN,
            &format!("rekenshi.rain-style.{serial}@1"),
            0,
            move |_, input| {
                let values = *captured.read().map_err(|_| "rain policy poisoned")?;
                if values[0] != 1. {
                    return Ok(None);
                }
                let mut result = policies::decode(input)?;
                result[0] = 1.;
                result[1] = result[1].min(values[1]);
                result[2] = result[2].min(values[2]);
                Ok(Some(policies::encode(result)?))
            },
        )?;
        Ok(Self {
            registration,
            values,
        })
    }
    /// Update live copied style. Visibility is 0..1 and size is 0.1..2.
    /// # Errors
    /// Rejects invalid values or host failures while preserving the prior rule.
    pub fn configure(&self, enabled: bool, visibility: f32, size: f32) -> Result<(), Error> {
        if !visibility.is_finite()
            || !size.is_finite()
            || !(0. ..=1.).contains(&visibility)
            || !(0.1..=2.).contains(&size)
        {
            return Err("invalid rain style".into());
        }
        let previous = std::mem::replace(
            &mut *self.values.write().map_err(|_| "rain policy poisoned")?,
            [f32::from(enabled), visibility, size, 0.],
        );
        if let Err(error) = self.registration.changed() {
            *self.values.write().map_err(|_| "rain policy poisoned")? = previous;
            return Err(error);
        }
        Ok(())
    }
}
