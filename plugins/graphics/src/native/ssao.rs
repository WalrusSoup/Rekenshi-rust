//! Public shader identity and scoped four-float pixel constant bindings.
use rekenshi_plugin_sdk::{
    adapters::{self, AdapterOperations},
    render::ShaderBinding,
};
use std::ffi::c_void;
use std::sync::RwLock;
use windows::Win32::Graphics::Direct3D11::*;
use windows::core::{GUID, Interface};

// Provider-owned copied COM metadata, never an exposed native shader address.
const TAG: GUID = GUID::from_u128(0x622905ef_b4f3_4309_a8bc_993bc3fd396a);
static OPERATIONS: RwLock<Option<AdapterOperations>> = RwLock::new(None);
pub(super) fn publish(host: rekenshi_plugin_api::HostV1) -> Result<(), String> {
    *OPERATIONS.write().map_err(|_| "shader binding registry lock")? = Some(AdapterOperations::publish(host,
        "Documented D3D11 CreatePixelShader and Draw vtable methods, hash-gated renderer; COM private metadata copied; scoped pixel constants restore original state; device-specific buffers rebuild on identity change",
        &[(adapters::SHADER_BINDING, "rekenshi.shader-binding-le@1", 16*1024*1024), (adapters::DRAW_PARAMETERS, "rekenshi.shader-binding-le@1", 32)]).map_err(|e| e.to_string())?);
    Ok(())
}
pub(super) fn revoke() -> Result<(), String> {
    if let Some(mut operations) = OPERATIONS
        .write()
        .map_err(|_| "shader binding registry lock")?
        .take()
    {
        operations.revoke().map_err(|e| e.to_string())?;
    }
    Ok(())
}
pub(super) struct Gpu {
    device: ID3D11Device,
    buffer: ID3D11Buffer,
    parameters: [f32; 4],
}
pub(super) struct Binding {
    context: ID3D11DeviceContext,
    previous: [Option<ID3D11Buffer>; 1],
    slot: u32,
}
impl Drop for Binding {
    fn drop(&mut self) {
        // SAFETY: Getter-owned COM objects and context stay alive for this draw.
        unsafe {
            self.context
                .PSSetConstantBuffers(self.slot, Some(&self.previous))
        };
    }
}
/// Caller borrows original creation bytecode and a successful native shader output.
pub(super) unsafe fn tag(code: *const c_void, length: usize, output: *mut c_void) {
    if code.is_null() || output.is_null() || length > 16 * 1024 * 1024 {
        return;
    }
    // SAFETY: Successful creation supplies readable bytecode and a live borrowed COM shader.
    let bytes = unsafe { std::slice::from_raw_parts(code.cast::<u8>(), length) };
    let Ok(operations) = OPERATIONS.read() else {
        return;
    };
    let specification = operations.as_ref().and_then(|operations| {
        operations
            .transform(adapters::SHADER_BINDING, &[], bytes)
            .and_then(|record| ShaderBinding::decode(&record).ok())
    });
    // Native unit fixtures explicitly select their shader; production requires public policy.
    #[cfg(test)]
    let specification = specification.or_else(|| {
        bytes
            .windows(b"REKenshiSsaoConstants\0".len())
            .any(|part| part == b"REKenshiSsaoConstants\0")
            .then_some(ShaderBinding {
                identity: 1,
                slot: 13,
                parameters: [0.; 4],
            })
    });
    let Some(specification) = specification else {
        return;
    };
    let Ok(record) = specification.encode() else {
        return;
    };
    // SAFETY: Native shader is borrowed synchronously; SetPrivateData copies this portable record.
    unsafe {
        if let Some(shader) = ID3D11PixelShader::from_raw_borrowed(&output) {
            let _ = shader.SetPrivateData(&TAG, record.len() as u32, Some(record.as_ptr().cast()));
        }
    }
}
pub(super) fn bind(
    context: &ID3D11DeviceContext,
    resources: &mut Option<Gpu>,
    _fixture_parameters: [f32; 4],
) -> Result<Option<Binding>, String> {
    // SAFETY: Native immediate context is borrowed synchronously; getters own returned references.
    unsafe {
        let mut pixel = None;
        context.PSGetShader(&mut pixel, None, None);
        let Some(pixel) = pixel else {
            return Ok(None);
        };
        let mut bytes = [0u8; 32];
        let mut size = bytes.len() as u32;
        if pixel
            .GetPrivateData(&TAG, &mut size, Some(bytes.as_mut_ptr().cast()))
            .is_err()
            || size != bytes.len() as u32
        {
            return Ok(None);
        }
        let mut spec = ShaderBinding::decode(&bytes).map_err(|e| e.to_string())?;
        let operations = OPERATIONS.read().map_err(|_| "draw policy registry lock")?;
        if let Some(operations) = operations.as_ref() {
            if let Some(record) = operations.transform(adapters::DRAW_PARAMETERS, &[], &bytes) {
                let next = ShaderBinding::decode(&record).map_err(|e| e.to_string())?;
                if next.identity != spec.identity || next.slot != spec.slot {
                    return Err("draw parameters cannot change shader identity or slot".into());
                }
                spec = next;
            }
        }
        #[cfg(test)]
        if operations.is_none() {
            spec.parameters = _fixture_parameters;
        }
        let parameters = spec.parameters;
        let device = context.GetDevice().map_err(|e| e.to_string())?;
        if resources
            .as_ref()
            .is_none_or(|g| g.device.as_raw() != device.as_raw())
        {
            let desc = D3D11_BUFFER_DESC {
                ByteWidth: 16,
                Usage: D3D11_USAGE_DEFAULT,
                BindFlags: D3D11_BIND_CONSTANT_BUFFER.0 as u32,
                ..Default::default()
            };
            let initial = D3D11_SUBRESOURCE_DATA {
                pSysMem: parameters.as_ptr().cast(),
                ..Default::default()
            };
            let mut buffer = None;
            device
                .CreateBuffer(&desc, Some(&initial), Some(&mut buffer))
                .map_err(|e| e.to_string())?;
            *resources = Some(Gpu {
                device,
                buffer: buffer.ok_or("constant buffer unavailable")?,
                parameters,
            });
        }
        let gpu = resources.as_mut().ok_or("draw resources unavailable")?;
        if gpu.parameters != parameters {
            context.UpdateSubresource(&gpu.buffer, 0, None, parameters.as_ptr().cast(), 0, 0);
            gpu.parameters = parameters;
        }
        let mut previous = [None];
        context.PSGetConstantBuffers(spec.slot, Some(&mut previous));
        let binding = Binding {
            context: context.clone(),
            previous,
            slot: spec.slot,
        };
        context.PSSetConstantBuffers(spec.slot, Some(&[Some(gpu.buffer.clone())]));
        Ok(Some(binding))
    }
}
#[cfg(test)]
mod tests {
    use super::super::shader;
    use super::*;

    #[test]
    #[ignore = "requires shipped Kenshi shader assets and compiler DLL"]
    fn shipped_lighting_shader_compiles_with_ssao() {
        fn expand(path: &std::path::Path, common: &std::path::Path) -> String {
            String::from_utf8_lossy(&std::fs::read(path).unwrap())
                .lines()
                .map(|line| {
                    if let Some(name) = line
                        .trim()
                        .strip_prefix("#include \"")
                        .and_then(|v| v.strip_suffix('"'))
                    {
                        let local = path.parent().unwrap().join(name);
                        expand(
                            &if local.is_file() {
                                local
                            } else {
                                common.join(name)
                            },
                            common,
                        )
                    } else {
                        format!("{line}\n")
                    }
                })
                .collect()
        }
        let game = std::path::PathBuf::from(std::env::var_os("REKENSHI_TEST_GAME").unwrap());
        let materials = game.join("data/materials");
        let source = expand(
            &materials.join("deferred/deferred.hlsl"),
            &materials.join("common"),
        );
        let patched = crate::lighting_shader(source.as_bytes(), b"main_fs", b"ps_5_0")
            .expect("recognized game shader");
        for defines in ["", "#define CSM 1\n", "#define RTW 1\n"] {
            let source = [
                b"#define main_fs main\n".as_slice(),
                defines.as_bytes(),
                &patched,
            ]
            .concat();
            for profile in [c"ps_4_0", c"ps_5_0"] {
                let code = shader::compile_legacy(&game, &source, profile).unwrap();
                assert!(
                    code.windows(b"REKenshiSsaoConstants\0".len())
                        .any(|v| v == b"REKenshiSsaoConstants\0")
                );
            }
        }
    }

    #[test]
    #[ignore = "requires Windows GPU and REKENSHI_TEST_GAME compiler DLL"]
    fn tagged_shader_binding_restores_game_constants() {
        let game = std::path::PathBuf::from(std::env::var_os("REKENSHI_TEST_GAME").unwrap());
        let source = b"cbuffer REKenshiSsaoConstants:register(b13){float4 params;};float4 main():SV_Target{return params;}";
        let code = shader::compile(&game, source, false).unwrap();
        // SAFETY: Test owns this device/context and all descriptors/output storage.
        unsafe {
            use windows::Win32::Graphics::Direct3D::*;
            let mut device = None;
            let mut context = None;
            D3D11CreateDevice(
                None,
                D3D_DRIVER_TYPE_HARDWARE,
                windows::Win32::Foundation::HMODULE::default(),
                D3D11_CREATE_DEVICE_FLAG(0),
                Some(&[D3D_FEATURE_LEVEL_11_0]),
                D3D11_SDK_VERSION,
                Some(&mut device),
                None,
                Some(&mut context),
            )
            .unwrap();
            let device = device.unwrap();
            let context = context.unwrap();
            let mut pixel = None;
            device
                .CreatePixelShader(&code, None, Some(&mut pixel))
                .unwrap();
            let pixel = pixel.unwrap();
            context.PSSetShader(&pixel, None);
            let mut resources = None;
            assert!(bind(&context, &mut resources, [0.; 4]).unwrap().is_none());
            tag(code.as_ptr().cast(), code.len(), pixel.as_raw());
            let desc = D3D11_BUFFER_DESC {
                ByteWidth: 16,
                Usage: D3D11_USAGE_DEFAULT,
                BindFlags: D3D11_BIND_CONSTANT_BUFFER.0 as u32,
                ..Default::default()
            };
            let mut previous = None;
            device
                .CreateBuffer(&desc, None, Some(&mut previous))
                .unwrap();
            context.PSSetConstantBuffers(13, Some(&[previous.clone()]));
            let binding = bind(&context, &mut resources, [0.6, 1.5, 0.04, 0.])
                .unwrap()
                .unwrap();
            let mut current = [None];
            context.PSGetConstantBuffers(13, Some(&mut current));
            assert_ne!(
                current[0].as_ref().unwrap().as_raw(),
                previous.as_ref().unwrap().as_raw()
            );
            drop(binding);
            context.PSGetConstantBuffers(13, Some(&mut current));
            assert_eq!(
                current[0].as_ref().unwrap().as_raw(),
                previous.as_ref().unwrap().as_raw()
            );
        }
    }
}
