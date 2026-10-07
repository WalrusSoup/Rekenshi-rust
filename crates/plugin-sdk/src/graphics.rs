//! Owned client handle: every operation executes in the host's one GPU backend.
use crate::{
    Context,
    abi::{HostV1, OK, graphics::*},
};
/// Owned post-tonemap effect; removal and Drop release its host registration.
#[derive(Debug)]
pub struct PostProcess {
    api: GraphicsV2,
    token: u64,
    handle: Option<u64>,
}
impl PostProcess {
    /// Register an owned post-tonemap shader effect with copied source and parameters.
    ///
    /// # Errors
    /// Returns a failure when service negotiation, validation, or the host operation fails.
    pub fn new(
        context: &Context,
        shader: &[u8],
        priority: i32,
        parameters: [f32; 16],
    ) -> Result<Self, crate::Error> {
        let host = context.host.native();
        let api = api(host)?;
        let descriptor = EffectV1 {
            struct_size: size_of::<EffectV1>() as u32,
            stage: POST_TONEMAP,
            priority,
            reserved: 0,
            hlsl: shader.as_ptr(),
            hlsl_len: shader.len(),
            parameters,
        };
        let mut handle = 0;
        // SAFETY: Negotiated pinned API; descriptor and source remain readable
        // for this call, output is writable, host copies all inputs.
        let result = unsafe { (api.register)(host.plugin_token, &descriptor, &mut handle) };
        if result != OK {
            return Err(crate::Error::host("graphics", result));
        }
        Ok(Self {
            api,
            token: host.plugin_token,
            handle: Some(handle),
        })
    }
    /// Copy sixteen finite parameters into this effect live.
    ///
    /// # Errors
    /// Returns a failure when service negotiation, validation, or the host operation fails.
    pub fn update(&mut self, parameters: [f32; 16]) -> Result<(), crate::Error> {
        let handle = self.handle.ok_or("effect already removed")?;
        // SAFETY: Host copies the sixteen readable parameters synchronously.
        let result =
            unsafe { (self.api.update)(self.token, handle, parameters.as_ptr(), parameters.len()) };
        if result == OK {
            Ok(())
        } else {
            Err(crate::Error::host("graphics", result))
        }
    }
    /// Remove this effect; repeated successful removal is harmless.
    ///
    /// # Errors
    /// Returns a failure when service negotiation, validation, or the host operation fails.
    pub fn remove(&mut self) -> Result<(), crate::Error> {
        if let Some(handle) = self.handle {
            // SAFETY: Negotiated pinned callback, scalar owner/handle contract.
            let result = unsafe { (self.api.remove)(self.token, handle) };
            if result != OK {
                return Err(crate::Error::host("graphics", result));
            }
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
fn api(host: HostV1) -> Result<GraphicsV2, crate::Error> {
    let address = host
        .find_export(c"RE_Kenshi_Rust.dll", c"rekenshi_graphics_api_v2")
        .ok_or_else(|| crate::Error::unavailable("graphics"))?;
    // SAFETY: Named SDK host export has exactly GetApiV2 signature and is pinned.
    let get: GetApiV2 = unsafe { std::mem::transmute(address.as_ptr()) };
    let mut output = std::mem::MaybeUninit::uninit();
    // SAFETY: Writable aligned output and exact version/size.
    let result = unsafe { get(VERSION_V2, output.as_mut_ptr(), size_of::<GraphicsV2>()) };
    if result != OK {
        return Err(crate::Error::host("graphics", result));
    }
    // SAFETY: Successful host negotiation initializes the table.
    let api: GraphicsV2 = unsafe { output.assume_init() };
    if api.struct_size as usize != size_of::<GraphicsV2>() || api.abi_version != VERSION_V2 {
        return Err("incompatible graphics service".into());
    }
    Ok(api)
}
