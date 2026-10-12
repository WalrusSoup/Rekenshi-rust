//! Independent release-host capture against the actual hardware D3D11 runtime.
use std::path::{Path, PathBuf};
use windows::Win32::{
    Foundation::HMODULE,
    Graphics::{Direct3D::*, Direct3D11::*, Dxgi::Common::*},
};

#[test]
#[ignore = "requires release host, installed game modules and a Windows hardware GPU"]
fn resource_inventory_preserves_contents_and_tracks_native_lifetimes() {
    let game = PathBuf::from(std::env::var_os("REKENSHI_TEST_GAME").expect("game directory"));
    let project = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let stage = project.join("target/resource-capture-tests").join(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
            .to_string(),
    );
    let root = stage.join("RE_Kenshi_Rust");
    std::fs::create_dir_all(root.join("plugins")).unwrap();
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
    std::fs::write(
        root.join("plugins.toml"),
        "schema_version=1\ndiscover=false\nplugins=[]\n",
    )
    .unwrap();
    std::fs::write(
        root.join("modpack.toml"),
        "schema_version=1\n[settings.values]\n\"graphics.resource_capture\"=true\n",
    )
    .unwrap();
    rekenshi_native::load_pinned(&stage.join("OgreMain_x64.dll")).unwrap();
    rekenshi_native::load_pinned(&stage.join("MyGUIEngine_x64.dll")).unwrap();
    let module = rekenshi_native::load_pinned(&stage.join("RE_Kenshi_Rust.dll")).unwrap();
    // SAFETY: Pinned release host exports and their documented scalar/byte-slice ABI.
    let (start, stop, set): (
        extern "C" fn(),
        extern "C" fn(),
        unsafe extern "C" fn(*const u8, usize, u32, f64) -> i32,
    ) = unsafe {
        (
            std::mem::transmute::<*const std::ffi::c_void, extern "C" fn()>(
                rekenshi_native::export(module, c"dllStartPlugin").unwrap(),
            ),
            std::mem::transmute::<*const std::ffi::c_void, extern "C" fn()>(
                rekenshi_native::export(module, c"dllStopPlugin").unwrap(),
            ),
            std::mem::transmute::<
                *const std::ffi::c_void,
                unsafe extern "C" fn(*const u8, usize, u32, f64) -> i32,
            >(rekenshi_native::export(module, c"rekenshi_plugin_setting_set").unwrap()),
        )
    };
    start();
    let capture = |enabled: bool| {
        let key = b"graphics.resource_capture";
        // SAFETY: Constant key is readable for length; synchronous call copies it.
        assert_eq!(
            // SAFETY: Constant key is readable for length; synchronous call copies it.
            unsafe { set(key.as_ptr(), key.len(), 1, if enabled { 1. } else { 0. }) },
            0
        );
    };
    // SAFETY: Fixture owns all COM references, descriptors and upload/map memory.
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
        let desc = D3D11_BUFFER_DESC {
            ByteWidth: 4096,
            Usage: D3D11_USAGE_DEFAULT,
            BindFlags: D3D11_BIND_VERTEX_BUFFER.0 as u32,
            ..Default::default()
        };
        let pixels = [0x12345678u32; 1024];
        let mut original = None;
        device
            .CreateBuffer(
                &desc,
                Some(&D3D11_SUBRESOURCE_DATA {
                    pSysMem: pixels.as_ptr().cast(),
                    ..Default::default()
                }),
                Some(&mut original),
            )
            .unwrap();
        let survivor = original.as_ref().unwrap().clone();
        drop(original);
        let mut staging = None;
        device
            .CreateBuffer(
                &D3D11_BUFFER_DESC {
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
        context.UpdateSubresource(&survivor, 0, None, pixels.as_ptr().cast(), 0, 0);
        context.CopyResource(&staging, &survivor);
        context.CopySubresourceRegion(&survivor, 0, 0, 0, 0, &staging, 0, None);
        context.CopySubresourceRegion(&staging, 0, 0, 0, 0, &survivor, 0, None);
        let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
        context
            .Map(&staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped))
            .unwrap();
        assert_eq!(
            std::slice::from_raw_parts(mapped.pData.cast::<u32>(), 1024),
            &pixels
        );
        context.Unmap(&staging, 0);
        let mut retired = None;
        device
            .CreateBuffer(
                &D3D11_BUFFER_DESC {
                    ByteWidth: 2048,
                    ..desc
                },
                None,
                Some(&mut retired),
            )
            .unwrap();
        drop(retired);
        let mut texture1 = None;
        device
            .CreateTexture1D(
                &D3D11_TEXTURE1D_DESC {
                    Width: 17,
                    MipLevels: 1,
                    ArraySize: 2,
                    Format: DXGI_FORMAT_R8G8B8A8_UNORM,
                    Usage: D3D11_USAGE_DEFAULT,
                    BindFlags: D3D11_BIND_SHADER_RESOURCE.0 as u32,
                    ..Default::default()
                },
                None,
                Some(&mut texture1),
            )
            .unwrap();
        let mut texture2 = None;
        device
            .CreateTexture2D(
                &D3D11_TEXTURE2D_DESC {
                    Width: 8,
                    Height: 4,
                    MipLevels: 1,
                    ArraySize: 1,
                    Format: DXGI_FORMAT_R8G8B8A8_UNORM,
                    SampleDesc: DXGI_SAMPLE_DESC {
                        Count: 1,
                        Quality: 0,
                    },
                    Usage: D3D11_USAGE_DEFAULT,
                    BindFlags: D3D11_BIND_SHADER_RESOURCE.0 as u32,
                    ..Default::default()
                },
                None,
                Some(&mut texture2),
            )
            .unwrap();
        let mut texture3 = None;
        device
            .CreateTexture3D(
                &D3D11_TEXTURE3D_DESC {
                    Width: 4,
                    Height: 4,
                    Depth: 2,
                    MipLevels: 1,
                    Format: DXGI_FORMAT_R8_UNORM,
                    Usage: D3D11_USAGE_DEFAULT,
                    BindFlags: D3D11_BIND_SHADER_RESOURCE.0 as u32,
                    ..Default::default()
                },
                None,
                Some(&mut texture3),
            )
            .unwrap();
        let mut invalid = None;
        assert!(
            device
                .CreateBuffer(
                    &D3D11_BUFFER_DESC {
                        ByteWidth: 0,
                        ..desc
                    },
                    None,
                    Some(&mut invalid)
                )
                .is_err()
        );
        device.CreateBuffer(&desc, None, None).unwrap(); // validation-only S_FALSE: no resource.
        context.Flush();
        capture(false);
        let first = reports(&root).pop().unwrap();
        let resources = first["resources"].as_array().unwrap();
        assert!(resources.iter().any(|r| r["description"]["buffer_bytes"]==4096 && r["description"]["usage"]==0), "clone keeps original resource alive");
        assert!(
            !resources
                .iter()
                .any(|r| r["description"]["buffer_bytes"] == 2048),
            "final COM release retires buffer"
        );
        for (kind, bytes) in [("texture1d", 136), ("texture2d", 128), ("texture3d", 32)] {
            assert!(
                resources
                    .iter()
                    .any(|r| r["description"]["kind"] == kind
                        && r["estimated_payload_bytes"] == bytes)
            );
        }
        assert_eq!(first["attachment_failures"], 0);
        assert_eq!(first["untracked_creations"], 0);
        assert_eq!(first["dropped_events"], 0);
        assert_eq!(first["io_errors"], 0);
        assert_eq!(first["activity_tag_failures"], 0);
        assert!(
            resources
                .iter()
                .all(|r| !r["origin_frames"].as_array().unwrap().is_empty())
        );
        let tracked_staging = resources
            .iter()
            .find(|r| r["description"]["usage"] == 3)
            .unwrap();
        assert_eq!(tracked_staging["activity"]["maps"], 1);
        assert_eq!(tracked_staging["activity"]["map_reads"], 1);
        assert_eq!(tracked_staging["activity"]["map_writes"], 0);
        assert_eq!(tracked_staging["activity"]["unmaps"], 1);
        assert_eq!(tracked_staging["activity"]["copy_destination"], 2);
        assert_eq!(tracked_staging["activity"]["copy_source"], 1);
        assert_eq!(
            first["created"], 6,
            "failed and validation-only creation must not count"
        );
        assert_eq!(first["destroyed"], 1);
        assert_eq!(first["live_count"], 5);
        capture(true);
        let mut fresh = None;
        device
            .CreateBuffer(
                &D3D11_BUFFER_DESC {
                    ByteWidth: 1024,
                    ..desc
                },
                None,
                Some(&mut fresh),
            )
            .unwrap();
        let mut fresh_staging = None;
        device
            .CreateBuffer(
                &D3D11_BUFFER_DESC {
                    Usage: D3D11_USAGE_STAGING,
                    BindFlags: 0,
                    CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32,
                    ..desc
                },
                None,
                Some(&mut fresh_staging),
            )
            .unwrap();
        // Both staging buffers have ID 2, in different sessions. The old tag
        // must not increment the new staging record despite its reused ID.
        context
            .Map(&staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped))
            .unwrap();
        context.Unmap(&staging, 0);
        drop(staging);
        drop(survivor);
        drop(texture1);
        drop(texture2);
        drop(texture3);
        context.ClearState();
        context.Flush();
        capture(false);
        let all = reports(&root);
        assert_eq!(all.len(), 2);
        let second = all.last().unwrap();
        assert_eq!(second["created"], 2);
        assert_eq!(second["live_count"], 2);
        assert_ne!(first["session_id"], second["session_id"]);
        assert_eq!(second["resources"][0]["activity"]["maps"], 0);
        assert_eq!(
            second["destroyed"], 0,
            "old-session markers must not retire reused IDs"
        );
        drop(fresh);
        drop(fresh_staging);
    }
    stop();
}
fn reports(root: &Path) -> Vec<serde_json::Value> {
    let mut paths: Vec<_> = std::fs::read_dir(root.join("graphics-captures"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    paths.sort();
    paths
        .into_iter()
        .map(|p| serde_json::from_slice(&std::fs::read(p.join("summary.json")).unwrap()).unwrap())
        .collect()
}
