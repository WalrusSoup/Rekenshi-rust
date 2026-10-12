//! Isolated GPU experiment for shared geometry and precomputed placement records.
//!
//! This module is test-only: it owns its device and does not hook game rendering.

use super::shader;
use std::path::Path;
use windows::Win32::Foundation::HMODULE;
use windows::Win32::Graphics::Direct3D::*;
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::Dxgi::Common::*;
use windows::core::s;

/// Three affine rows and a color, explicitly serialized to a 64-byte GPU record.
#[derive(Clone, Copy, Debug)]
struct Instance {
    rows: [[f32; 4]; 3],
    color: [f32; 4],
}

impl Instance {
    fn append(&self, bytes: &mut Vec<u8>) {
        for value in self.rows.iter().flatten().chain(self.color.iter()) {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
    }
}

const INSTANCED_VERTEX: &[u8] = br"
struct Input {
    float3 position : POSITION;
    float4 row0 : INSTANCE0;
    float4 row1 : INSTANCE1;
    float4 row2 : INSTANCE2;
    float4 color : COLOR;
};
struct Output { float4 position : SV_Position; float4 color : COLOR; };
Output main(Input input) {
    Output output;
    float4 local = float4(input.position, 1);
    output.position = float4(dot(input.row0, local), dot(input.row1, local),
                             dot(input.row2, local), 1);
    output.color = input.color;
    return output;
}";

const EXPANDED_VERTEX: &[u8] = br"
struct Output { float4 position : SV_Position; float4 color : COLOR; };
Output main(float3 position : POSITION, float4 color : COLOR) {
    Output output;
    output.position = float4(position, 1);
    output.color = color;
    return output;
}";

const PIXEL: &[u8] = br"
float4 main(float4 position : SV_Position, float4 color : COLOR) : SV_Target {
    return color;
}";

fn buffer(device: &ID3D11Device, bytes: &[u8], bind: D3D11_BIND_FLAG) -> ID3D11Buffer {
    assert!(!bytes.is_empty(), "GPU buffers must have initial contents");
    let desc = D3D11_BUFFER_DESC {
        ByteWidth: u32::try_from(bytes.len()).expect("test buffer fits D3D11 ByteWidth"),
        Usage: D3D11_USAGE_IMMUTABLE,
        BindFlags: bind.0 as u32,
        ..Default::default()
    };
    let data = D3D11_SUBRESOURCE_DATA {
        pSysMem: bytes.as_ptr().cast(),
        ..Default::default()
    };
    let mut result = None;
    // SAFETY: Nonempty byte slice contains ByteWidth initialized bytes and lives
    // through synchronous creation. D3D11 copies them into an owned COM buffer.
    unsafe { device.CreateBuffer(&desc, Some(&data), Some(&mut result)) }
        .expect("create immutable geometry buffer");
    result.expect("D3D11 returned a geometry buffer")
}

fn byte_width(buffer: &ID3D11Buffer) -> u64 {
    let mut desc = D3D11_BUFFER_DESC::default();
    // SAFETY: Live owned COM object writes its descriptor to initialized storage.
    unsafe { buffer.GetDesc(&mut desc) };
    u64::from(desc.ByteWidth)
}

fn input(semantic: windows::core::PCSTR, offset: u32) -> D3D11_INPUT_ELEMENT_DESC {
    D3D11_INPUT_ELEMENT_DESC {
        SemanticName: semantic,
        Format: DXGI_FORMAT_R32G32B32_FLOAT,
        AlignedByteOffset: offset,
        InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
        ..Default::default()
    }
}

fn vertices() -> (Vec<[f32; 3]>, Vec<u32>) {
    // Dense subdivisions intentionally preserve a nontrivial source payload.
    // The missing quadrant makes rotations and mirrored placements observable.
    let mut vertices = Vec::with_capacity(81);
    for y in 0..=8 {
        for x in 0..=8 {
            vertices.push([(x as f32 - 4.) / 32., (y as f32 - 4.) / 32., 0.]);
        }
    }
    let mut indices = Vec::with_capacity(48 * 6);
    for y in 0..8 {
        for x in 0..8 {
            if x >= 4 && y >= 4 {
                continue;
            }
            let a = y * 9 + x;
            indices.extend_from_slice(&[a, a + 1, a + 9, a + 1, a + 10, a + 9]);
        }
    }
    (vertices, indices)
}

fn placements() -> Vec<Instance> {
    (0..64)
        .map(|index| {
            let x = index % 8;
            let y = index / 8;
            let sx = if index % 2 == 0 { -0.75 } else { 0.5 };
            let sy = if index % 3 == 0 { 1. } else { 0.75 };
            let (sin, cos) = match index % 4 {
                0 => (0., 1.),
                1 => (1., 0.),
                2 => (0., -1.),
                _ => (-1., 0.),
            };
            // Dyadic coordinates and quarter-turns permit exact CPU/GPU
            // comparisons without hiding rasterization differences in tolerance.
            Instance {
                rows: [
                    [cos * sx, -sin * sy, 0., -0.875 + x as f32 / 4.],
                    [sin * sx, cos * sy, 0., -0.875 + y as f32 / 4.],
                    [0., 0., 1., 0.5],
                ],
                color: [
                    (index % 4 + 1) as f32 / 4.,
                    (index / 4 % 4 + 1) as f32 / 4.,
                    (index / 16 + 1) as f32 / 4.,
                    1.,
                ],
            }
        })
        .collect()
}

fn floats(values: impl IntoIterator<Item = f32>, bytes: &mut Vec<u8>) {
    for value in values {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
}

struct Draw {
    vertex: ID3D11VertexShader,
    layout: ID3D11InputLayout,
    buffers: Vec<Option<ID3D11Buffer>>,
    strides: Vec<u32>,
    indices: ID3D11Buffer,
    index_count: u32,
    instance_count: Option<u32>,
}

fn draw_resources(
    device: &ID3D11Device,
    game: &Path,
    vertices: &[[f32; 3]],
    indices: &[u32],
    instances: &[Instance],
    instanced: bool,
) -> Draw {
    let code = shader::compile(
        game,
        if instanced {
            INSTANCED_VERTEX
        } else {
            EXPANDED_VERTEX
        },
        true,
    )
    .expect("compile vertex shader");
    let mut elements = vec![input(s!("POSITION"), 0)];
    let mut vertex_bytes = Vec::new();
    let mut instance_bytes = Vec::with_capacity(instances.len() * 64);
    let mut index_bytes = Vec::new();
    if instanced {
        for vertex in vertices {
            floats(*vertex, &mut vertex_bytes);
        }
        for instance in instances {
            instance.append(&mut instance_bytes);
        }
        for index in indices {
            index_bytes.extend_from_slice(&index.to_le_bytes());
        }
        for row in 0..3 {
            elements.push(D3D11_INPUT_ELEMENT_DESC {
                SemanticName: s!("INSTANCE"),
                SemanticIndex: row,
                Format: DXGI_FORMAT_R32G32B32A32_FLOAT,
                InputSlot: 1,
                AlignedByteOffset: row * 16,
                InputSlotClass: D3D11_INPUT_PER_INSTANCE_DATA,
                InstanceDataStepRate: 1,
            });
        }
        elements.push(D3D11_INPUT_ELEMENT_DESC {
            SemanticName: s!("COLOR"),
            Format: DXGI_FORMAT_R32G32B32A32_FLOAT,
            InputSlot: 1,
            AlignedByteOffset: 48,
            InputSlotClass: D3D11_INPUT_PER_INSTANCE_DATA,
            InstanceDataStepRate: 1,
            ..Default::default()
        });
    } else {
        elements.push(D3D11_INPUT_ELEMENT_DESC {
            Format: DXGI_FORMAT_R32G32B32A32_FLOAT,
            ..input(s!("COLOR"), 12)
        });
        for (placement, instance) in instances.iter().enumerate() {
            for position in vertices {
                // Independent reference path expands placements on the CPU.
                let transformed = instance.rows.map(|row| {
                    position[0] * row[0] + position[1] * row[1] + position[2] * row[2] + row[3]
                });
                floats(transformed, &mut vertex_bytes);
                floats(instance.color, &mut vertex_bytes);
            }
            let base = u32::try_from(placement * vertices.len()).expect("test vertex offset fits");
            for index in indices {
                index_bytes.extend_from_slice(&(base + index).to_le_bytes());
            }
        }
    }
    let mut vertex = None;
    let mut layout = None;
    // SAFETY: Compiled bytecode and element descriptors (including static
    // semantic strings) live through creation. COM wrappers own the outputs.
    unsafe {
        device
            .CreateVertexShader(&code, None, Some(&mut vertex))
            .unwrap();
        device
            .CreateInputLayout(&elements, &code, Some(&mut layout))
            .unwrap();
    }
    let mut buffers = vec![Some(buffer(
        device,
        &vertex_bytes,
        D3D11_BIND_VERTEX_BUFFER,
    ))];
    let mut strides = vec![if instanced { 12 } else { 28 }];
    if instanced {
        buffers.push(Some(buffer(
            device,
            &instance_bytes,
            D3D11_BIND_VERTEX_BUFFER,
        )));
        strides.push(64);
    }
    Draw {
        vertex: vertex.unwrap(),
        layout: layout.unwrap(),
        buffers,
        strides,
        index_count: u32::try_from(index_bytes.len() / 4).unwrap(),
        indices: buffer(device, &index_bytes, D3D11_BIND_INDEX_BUFFER),
        instance_count: instanced.then(|| u32::try_from(instances.len()).unwrap()),
    }
}

fn render(device: &ID3D11Device, context: &ID3D11DeviceContext, draw: &Draw) -> Vec<u8> {
    const SIZE: u32 = 256;
    let desc = D3D11_TEXTURE2D_DESC {
        Width: SIZE,
        Height: SIZE,
        MipLevels: 1,
        ArraySize: 1,
        Format: DXGI_FORMAT_R8G8B8A8_UNORM,
        SampleDesc: DXGI_SAMPLE_DESC {
            Count: 1,
            Quality: 0,
        },
        Usage: D3D11_USAGE_DEFAULT,
        BindFlags: D3D11_BIND_RENDER_TARGET.0 as u32,
        ..Default::default()
    };
    let mut texture = None;
    let mut target = None;
    let mut raster = None;
    let mut staging = None;
    // SAFETY: This test owns an independent immediate context. Descriptor sizes,
    // offsets, indices and buffer strides match the initialized data above.
    // The mapping is copied row-by-row using RowPitch, then always unmapped.
    unsafe {
        device
            .CreateTexture2D(&desc, None, Some(&mut texture))
            .unwrap();
        let texture = texture.unwrap();
        device
            .CreateRenderTargetView(&texture, None, Some(&mut target))
            .unwrap();
        let target = target.unwrap();
        device
            .CreateRasterizerState(
                &D3D11_RASTERIZER_DESC {
                    FillMode: D3D11_FILL_SOLID,
                    CullMode: D3D11_CULL_NONE,
                    DepthClipEnable: true.into(),
                    ..Default::default()
                },
                Some(&mut raster),
            )
            .unwrap();
        context.RSSetState(raster.as_ref());
        context.RSSetViewports(Some(&[D3D11_VIEWPORT {
            Width: SIZE as f32,
            Height: SIZE as f32,
            MaxDepth: 1.,
            ..Default::default()
        }]));
        context.OMSetRenderTargets(Some(&[Some(target.clone())]), None);
        context.ClearRenderTargetView(&target, &[0., 0., 0., 0.]);
        context.VSSetShader(&draw.vertex, None);
        context.IASetInputLayout(&draw.layout);
        context.IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
        let offsets = vec![0; draw.buffers.len()];
        context.IASetVertexBuffers(
            0,
            draw.buffers.len() as u32,
            Some(draw.buffers.as_ptr()),
            Some(draw.strides.as_ptr()),
            Some(offsets.as_ptr()),
        );
        context.IASetIndexBuffer(&draw.indices, DXGI_FORMAT_R32_UINT, 0);
        if let Some(count) = draw.instance_count {
            context.DrawIndexedInstanced(draw.index_count, count, 0, 0, 0);
        } else {
            context.DrawIndexed(draw.index_count, 0, 0);
        }
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
        context.CopyResource(&staging, &texture);
        let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
        context
            .Map(&staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped))
            .unwrap();
        let mut result = Vec::with_capacity((SIZE * SIZE * 4) as usize);
        for y in 0..SIZE as usize {
            result.extend_from_slice(std::slice::from_raw_parts(
                mapped.pData.cast::<u8>().add(y * mapped.RowPitch as usize),
                (SIZE * 4) as usize,
            ));
        }
        context.Unmap(&staging, 0);
        result
    }
}

#[test]
#[ignore = "requires Windows hardware D3D11 and REKENSHI_TEST_GAME compiler DLL"]
fn hardware_instancing_matches_expanded_geometry() {
    let game = std::env::var_os("REKENSHI_TEST_GAME").expect("set REKENSHI_TEST_GAME");
    let game = Path::new(&game);
    let mut device = None;
    let mut context = None;
    // SAFETY: No existing device/window is supplied. Writable COM output slots
    // create a separate hardware device whose lifetime is confined to this test.
    unsafe {
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
    }
    let device = device.unwrap();
    let context = context.unwrap();
    let pixel_code = shader::compile(game, PIXEL, false).expect("compile color pixel shader");
    let mut pixel = None;
    // SAFETY: Bytecode is valid compiler output and lives through creation;
    // the returned owned shader is bound only to our independent context.
    unsafe {
        device
            .CreatePixelShader(&pixel_code, None, Some(&mut pixel))
            .unwrap();
        context.PSSetShader(pixel.as_ref(), None);
    }
    let (vertices, indices) = vertices();
    let instances = placements();
    let expanded = draw_resources(&device, game, &vertices, &indices, &instances, false);
    let shared = draw_resources(&device, game, &vertices, &indices, &instances, true);
    let reference = render(&device, &context, &expanded);
    let actual = render(&device, &context, &shared);
    assert_eq!(
        actual, reference,
        "instanced render differs from expanded placements"
    );
    let mut colors = std::collections::BTreeMap::new();
    for pixel in reference.chunks_exact(4).filter(|pixel| pixel[3] == 255) {
        *colors
            .entry([pixel[0], pixel[1], pixel[2]])
            .or_insert(0_usize) += 1;
    }
    assert_eq!(colors.len(), 64, "every distinct placement must be visible");
    assert!(
        colors.values().all(|count| *count >= 32),
        "placements must have visible area"
    );
    let allocation = |draw: &Draw| {
        draw.buffers.iter().flatten().map(byte_width).sum::<u64>() + byte_width(&draw.indices)
    };
    let expanded_bytes = allocation(&expanded);
    let shared_bytes = allocation(&shared);
    assert!(
        shared_bytes * 20 < expanded_bytes,
        "expected over 20x geometry payload reduction: expanded={expanded_bytes}, shared={shared_bytes}"
    );
    println!(
        "64 placements: expanded {expanded_bytes} bytes; shared {shared_bytes} bytes; identical RGBA output, all 64 colors visible"
    );
    // SAFETY: Unbind our owned resources before the device and COM wrappers drop.
    unsafe { context.ClearState() };
}
