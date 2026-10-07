//! Optional shader-effect service, separate from the frozen plugin V1 ABI.
//! Effects run after the world HDR resolve and before GUI. Source is copied at
//! registration. No Rust allocations or GPU objects cross DLL ownership boundaries.
/// Version negotiated for this optional service.
pub const VERSION: u32 = 1;
/// Effect stage after world tone mapping and before GUI drawing.
pub const POST_TONEMAP: u32 = 1;
/// Negotiate this service into aligned exact-sized caller-owned output.
pub type GetApi = unsafe extern "C" fn(u32, *mut GraphicsV1, usize) -> i32;
/// Register a copied shader descriptor and initialize the output handle on success.
pub type Register = unsafe extern "C" fn(u64, *const EffectV1, *mut u64) -> i32;
/// Remove an effect using the plugin token and effect handle.
pub type Remove = unsafe extern "C" fn(u64, u64) -> i32;
#[repr(C)]
#[derive(Clone, Copy, Debug)]
/// Negotiated callbacks for registering and removing post-tonemap effects.
pub struct GraphicsV1 {
    /// Size of this C record in bytes; negotiation requires an exact match.
    pub struct_size: u32,
    /// Version of this table; negotiate before invoking its callbacks.
    pub abi_version: u32,
    /// Copy an effect descriptor and return a nonzero owner-scoped handle.
    pub register: Register,
    /// Remove the matching owner-scoped effect after validating its token.
    pub remove: Remove,
}
/// Graphics service version supporting live parameter updates.
pub const VERSION_V2: u32 = 2;
/// Negotiate the graphics V2 table into exact-sized caller-owned output.
pub type GetApiV2 = unsafe extern "C" fn(u32, *mut GraphicsV2, usize) -> i32;
#[repr(C)]
#[derive(Clone, Copy, Debug)]
/// Graphics callback table supporting live parameter updates.
pub struct GraphicsV2 {
    /// Size of this C record in bytes; negotiation requires an exact match.
    pub struct_size: u32,
    /// Version of this table; negotiate before invoking its callbacks.
    pub abi_version: u32,
    /// Copy an effect descriptor and return a nonzero owner-scoped handle.
    pub register: Register,
    /// Remove the matching owner-scoped effect after validating its token.
    pub remove: Remove,
    /// Copy sixteen finite shader parameters into the matching owner-scoped effect.
    pub update: unsafe extern "C" fn(u64, u64, *const f32, usize) -> i32,
}
#[repr(C)]
#[derive(Clone, Copy, Debug)]
/// Caller-owned shader descriptor copied synchronously during registration.
pub struct EffectV1 {
    /// Size of this C record in bytes; negotiation requires an exact match.
    pub struct_size: u32,
    /// Effect insertion stage; this version supports POST_TONEMAP only.
    pub stage: u32,
    /// Effect ordering priority; lower priorities execute first.
    pub priority: i32,
    /// Reserved for future versions; callers must write zero.
    pub reserved: u32,
    /// Readable shader-source bytes; registration copies them before returning.
    pub hlsl: *const u8,
    /// Number of readable shader-source bytes at `hlsl`.
    pub hlsl_len: usize,
    /// Four float4 vectors at PS b0. PS t0 is a copy of the previous LDR result;
    /// s0 is linear clamp. Input float4 SV_Position and float2 TEXCOORD0;
    /// entry main, shader profile ps_5_0, output SV_Target.
    pub parameters: [f32; 16],
}
