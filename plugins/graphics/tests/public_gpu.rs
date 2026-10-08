//! Native GPU parity through an independently loaded policy DLL and the real broker.
#[path = "../src/native/shader.rs"]
mod shader;
use std::path::{Path, PathBuf};
use windows::{
    Win32::{
        Foundation::HMODULE,
        Graphics::{Direct3D::*, Direct3D11::*, Dxgi::Common::*},
    },
    core::Interface,
};
#[test]
#[ignore = "requires release host/sample DLLs, installed game modules and a Windows GPU"]
fn independent_shader_binding_postprocess_and_device_recreation() {
    let game = PathBuf::from(std::env::var_os("REKENSHI_TEST_GAME").expect("game directory"));
    let project = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let stage = project.join("target/public-gpu-tests").join(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
            .to_string(),
    );
    let plugins = stage.join("RE_Kenshi_Rust/plugins");
    std::fs::create_dir_all(&plugins).unwrap();
    for name in [
        "OgreMain_x64.dll",
        "MyGUIEngine_x64.dll",
        "D3DCompiler_43.dll",
        "RenderSystem_Direct3D11_x64.dll",
    ] {
        std::fs::copy(game.join(name), stage.join(name)).unwrap();
    }
    std::fs::copy(
        project.join("target/release/RE_Kenshi_Rust.dll"),
        stage.join("RE_Kenshi_Rust.dll"),
    )
    .unwrap();
    std::fs::copy(
        project.join("target/release/rekenshi_source_transform.dll"),
        plugins.join("rekenshi_source_transform.dll"),
    )
    .unwrap();
    std::fs::create_dir_all(plugins.parent().unwrap().join("settings")).unwrap();
    std::fs::write(
        plugins
            .parent()
            .unwrap()
            .join("settings/source-transform.toml"),
        "schema_version=1\n[settings.values]\n\"runtime_policy.enabled\"=true\n",
    )
    .unwrap();
    std::fs::write(
        plugins.parent().unwrap().join("plugins.toml"),
        "schema_version=1\ndiscover=true\n",
    )
    .unwrap();
    std::fs::write(
        plugins.parent().unwrap().join("modpack.toml"),
        "schema_version=1\n[settings.values]\n",
    )
    .unwrap();
    rekenshi_native::load_pinned(&stage.join("OgreMain_x64.dll")).unwrap();
    rekenshi_native::load_pinned(&stage.join("MyGUIEngine_x64.dll")).unwrap();
    let host = rekenshi_native::load_pinned(&stage.join("RE_Kenshi_Rust.dll")).unwrap();
    // SAFETY: Named scalar-only exports, pinned release host, exact Windows x64 C ABI.
    let (start, stop): (extern "C" fn(), extern "C" fn()) = unsafe {
        (
            std::mem::transmute::<*const std::ffi::c_void, extern "C" fn()>(
                rekenshi_native::export(host, c"dllStartPlugin").unwrap(),
            ),
            std::mem::transmute::<*const std::ffi::c_void, extern "C" fn()>(
                rekenshi_native::export(host, c"dllStopPlugin").unwrap(),
            ),
        )
    };
    start();
    for _ in 0..2 {
        check_device(&stage);
    }
    stop();
    let log = std::fs::read_to_string(stage.join("RE_Kenshi_Rust.log")).unwrap();
    assert!(log.contains("first world effect pass"), "{log}");
}
fn check_device(stage: &Path) {
    let vertex = shader::compile(stage, shader::VERTEX, true).unwrap();
    let pixel=shader::compile_legacy(stage,b"cbuffer ExampleConstants:register(b12){float4 values;};float4 main():SV_Target{return values;}",c"ps_5_0").unwrap();
    // SAFETY: This fixture owns each device/context and all resource lifetimes;
    // descriptors obey D3D11 constraints. Getter references and mapped bytes are retained.
    unsafe {
        let mut device = None;
        let mut context = None;
        D3D11CreateDevice(
            None,
            D3D_DRIVER_TYPE_HARDWARE,
            HMODULE::default(),
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
        let mut vs = None;
        let mut ps = None;
        device
            .CreateVertexShader(&vertex, None, Some(&mut vs))
            .unwrap();
        device
            .CreatePixelShader(&pixel, None, Some(&mut ps))
            .unwrap();
        context.VSSetShader(vs.as_ref(), None);
        context.PSSetShader(ps.as_ref(), None);
        context.IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
        let old = [0.9f32, 0.8, 0.7, 1.];
        let mut previous = None;
        device
            .CreateBuffer(
                &D3D11_BUFFER_DESC {
                    ByteWidth: 16,
                    Usage: D3D11_USAGE_DEFAULT,
                    BindFlags: D3D11_BIND_CONSTANT_BUFFER.0 as u32,
                    ..Default::default()
                },
                Some(&D3D11_SUBRESOURCE_DATA {
                    pSysMem: old.as_ptr().cast(),
                    ..Default::default()
                }),
                Some(&mut previous),
            )
            .unwrap();
        context.PSSetConstantBuffers(12, Some(&[previous.clone()]));
        let texture = |width, height, format, flags| {
            let desc = D3D11_TEXTURE2D_DESC {
                Width: width,
                Height: height,
                MipLevels: 1,
                ArraySize: 1,
                Format: format,
                SampleDesc: DXGI_SAMPLE_DESC {
                    Count: 1,
                    Quality: 0,
                },
                Usage: D3D11_USAGE_DEFAULT,
                BindFlags: flags,
                ..Default::default()
            };
            let mut texture = None;
            device
                .CreateTexture2D(&desc, None, Some(&mut texture))
                .unwrap();
            (texture.unwrap(), desc)
        };
        let render = |target: &ID3D11Texture2D, width, height| {
            let mut view = None;
            device
                .CreateRenderTargetView(target, None, Some(&mut view))
                .unwrap();
            context.OMSetRenderTargets(Some(&[view]), None);
            context.RSSetViewports(Some(&[D3D11_VIEWPORT {
                Width: width as f32,
                Height: height as f32,
                MaxDepth: 1.,
                ..Default::default()
            }]));
            context.Draw(3, 0);
            let mut restored = [None];
            context.PSGetConstantBuffers(12, Some(&mut restored));
            assert_eq!(
                restored[0].as_ref().unwrap().as_raw(),
                previous.as_ref().unwrap().as_raw(),
                "native binding restoration"
            );
        };
        let read = |target: &ID3D11Texture2D, desc: D3D11_TEXTURE2D_DESC| {
            let mut staging = None;
            device
                .CreateTexture2D(
                    &D3D11_TEXTURE2D_DESC {
                        Usage: D3D11_USAGE_STAGING,
                        BindFlags: 0,
                        CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32,
                        ..desc
                    },
                    None,
                    Some(&mut staging),
                )
                .unwrap();
            let staging = staging.unwrap();
            context.CopyResource(&staging, target);
            let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
            context
                .Map(&staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped))
                .unwrap();
            let pixel = std::slice::from_raw_parts(
                mapped
                    .pData
                    .cast::<u8>()
                    .add(16 * mapped.RowPitch as usize + 16 * 4),
                4,
            )
            .to_vec();
            context.Unmap(&staging, 0);
            pixel
        };
        let (small, desc) = texture(
            64,
            64,
            DXGI_FORMAT_R8G8B8A8_UNORM,
            D3D11_BIND_RENDER_TARGET.0 as u32,
        );
        render(&small, 64, 64);
        for (actual, expected) in read(&small, desc).into_iter().zip([64u8, 128, 191, 255]) {
            assert!(
                actual.abs_diff(expected) <= 1,
                "independent binding output {actual} != {expected}"
            );
        }
        let (hdr, _) = texture(
            640,
            360,
            DXGI_FORMAT_R11G11B10_FLOAT,
            (D3D11_BIND_RENDER_TARGET | D3D11_BIND_SHADER_RESOURCE).0 as u32,
        );
        let (depth, _) = texture(
            640,
            360,
            DXGI_FORMAT_R32_FLOAT,
            D3D11_BIND_SHADER_RESOURCE.0 as u32,
        );
        let mut depth_view = None;
        device
            .CreateShaderResourceView(&depth, None, Some(&mut depth_view))
            .unwrap();
        context.PSSetShaderResources(0, Some(&[depth_view.clone(), None, depth_view]));
        render(&hdr, 640, 360);
        let mut hdr_view = None;
        device
            .CreateShaderResourceView(&hdr, None, Some(&mut hdr_view))
            .unwrap();
        let (ldr, desc) = texture(
            640,
            360,
            DXGI_FORMAT_R8G8B8A8_UNORM,
            D3D11_BIND_RENDER_TARGET.0 as u32,
        );
        context.OMSetRenderTargets(None, None);
        context.PSSetShaderResources(0, Some(&[hdr_view, None, None]));
        render(&ldr, 640, 360);
        for (actual, expected) in read(&ldr, desc).into_iter().zip([51u8, 102, 153, 255]) {
            assert!(
                actual.abs_diff(expected) <= 1,
                "independent effect output {actual} != {expected}"
            );
        }
    }
}
