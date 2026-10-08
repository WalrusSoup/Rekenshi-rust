use std::ffi::{CStr, c_void};
use std::path::Path;
use windows::Win32::Graphics::Direct3D::ID3DBlob;

type Compile = unsafe extern "system" fn(
    *const c_void,
    usize,
    *const i8,
    *const c_void,
    *mut c_void,
    *const i8,
    *const i8,
    u32,
    u32,
    *mut Option<ID3DBlob>,
    *mut Option<ID3DBlob>,
) -> i32;
pub fn compile(game: &Path, source: &[u8], vertex: bool) -> Result<Vec<u8>, String> {
    compile_flags(
        game,
        source,
        if vertex { c"vs_5_0" } else { c"ps_5_0" },
        1 << 15,
    )
}
#[cfg(test)]
pub(super) fn compile_legacy(game: &Path, source: &[u8], target: &CStr) -> Result<Vec<u8>, String> {
    // Kenshi's Ogre shaders use legacy sampler intrinsics under D3D11.
    compile_flags(game, source, target, (1 << 15) | (1 << 12))
}
fn compile_flags(game: &Path, source: &[u8], target: &CStr, flags: u32) -> Result<Vec<u8>, String> {
    let module = rekenshi_native::load_pinned(&game.join("D3DCompiler_43.dll"))?;
    // SAFETY: Documented compiler export signature; DLL pinned for process life.
    let compile: Compile =
        unsafe { std::mem::transmute(rekenshi_native::export(module, c"D3DCompile")?) };
    let mut code = None;
    let mut errors = None;
    // SAFETY: Source slice and C strings live through this synchronous call;
    // writable COM outputs are native-owned and released by windows wrappers.
    let result = unsafe {
        compile(
            source.as_ptr().cast(),
            source.len(),
            c"RE_Kenshi_Rust_effect.hlsl".as_ptr(),
            std::ptr::null(),
            std::ptr::null_mut(),
            c"main".as_ptr(),
            target.as_ptr(),
            flags,
            0,
            &mut code,
            &mut errors,
        )
    };
    if result < 0 {
        return Err(errors.map_or_else(
            || format!("shader compile HRESULT {result:#x}"),
            |blob| {
                // SAFETY: Compiler returned a live ID3DBlob; copy before releasing it.
                unsafe {
                    String::from_utf8_lossy(std::slice::from_raw_parts(
                        blob.GetBufferPointer().cast::<u8>(),
                        blob.GetBufferSize(),
                    ))
                    .trim_end_matches('\0')
                    .to_owned()
                }
            },
        ));
    }
    let code = code.ok_or("compiler returned no bytecode")?;
    // SAFETY: ID3DBlob methods return a valid buffer for its owned lifetime.
    Ok(unsafe {
        std::slice::from_raw_parts(code.GetBufferPointer().cast::<u8>(), code.GetBufferSize())
    }
    .to_vec())
}
pub const VERTEX: &[u8] = b"struct O{float4 p:SV_Position;float2 uv:TEXCOORD0;}; O main(uint id:SV_VertexID){O o;o.uv=float2((id<<1)&2,id&2);o.p=float4(o.uv*float2(2,-2)+float2(-1,1),0,1);return o;}";
