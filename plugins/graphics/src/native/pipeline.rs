use super::{Backend, shader};
use windows::Win32::Graphics::Direct3D::*;
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::Dxgi::Common::*;
use windows::core::Interface;

#[derive(Debug)]
pub enum Pass {
    Lighting,
    Tonemap(ID3D11RenderTargetView, D3D11_TEXTURE2D_DESC),
}
pub fn detect(c: &ID3D11DeviceContext) -> Option<Pass> {
    // SAFETY: Borrowed live context. Getter-created COM references are owned by
    // windows wrappers and released when the temporary arrays leave this scope.
    unsafe {
        let mut views = [None];
        c.OMGetRenderTargets(Some(&mut views), None);
        let target = views[0].take()?;
        let texture: ID3D11Texture2D = target.GetResource().ok()?.cast().ok()?;
        let mut desc = D3D11_TEXTURE2D_DESC::default();
        texture.GetDesc(&mut desc);
        if desc.Width < 640
            || desc.Height < 360
            || desc.SampleDesc.Count != 1
            || desc.ArraySize != 1
            || desc.MipLevels != 1
        {
            return None;
        }
        let mut srvs = [None, None, None];
        c.PSGetShaderResources(0, Some(&mut srvs));
        let format = |v: &Option<ID3D11ShaderResourceView>| -> Option<DXGI_FORMAT> {
            let mut d = D3D11_SHADER_RESOURCE_VIEW_DESC::default();
            v.as_ref()?.GetDesc(&mut d);
            Some(d.Format)
        };
        if desc.Format == DXGI_FORMAT_R11G11B10_FLOAT
            && srvs[0].is_some()
            && format(&srvs[2]) == Some(DXGI_FORMAT_R32_FLOAT)
        {
            return Some(Pass::Lighting);
        }
        if matches!(
            desc.Format,
            DXGI_FORMAT_B8G8R8A8_UNORM | DXGI_FORMAT_R8G8B8A8_UNORM
        ) && format(&srvs[0]) == Some(DXGI_FORMAT_R11G11B10_FLOAT)
        {
            return Some(Pass::Tonemap(target, desc));
        }
        None
    }
}
#[derive(Debug)]
pub struct Gpu {
    device: ID3D11Device,
    width: u32,
    height: u32,
    format: DXGI_FORMAT,
    copy: ID3D11Texture2D,
    srv: ID3D11ShaderResourceView,
    vertex: ID3D11VertexShader,
    effects: Vec<(ID3D11PixelShader, ID3D11Buffer)>,
    sampler: ID3D11SamplerState,
    blend: ID3D11BlendState,
    depth: ID3D11DepthStencilState,
    raster: ID3D11RasterizerState,
}
fn required<T>(value: Option<T>) -> Result<T, String> {
    value.ok_or("D3D11 returned no object".into())
}
fn create(b: &Backend, device: ID3D11Device, desc: &D3D11_TEXTURE2D_DESC) -> Result<Gpu, String> {
    let bytecode = shader::compile(&b.game, shader::VERTEX, true)?;
    // SAFETY: Descriptors obey D3D11 constraints, initial buffers and bytecode
    // slices outlive synchronous creation calls. Native COM owns all GPU memory.
    unsafe {
        let mut vertex = None;
        device
            .CreateVertexShader(&bytecode, None, Some(&mut vertex))
            .map_err(|e| e.to_string())?;
        let copy_desc = D3D11_TEXTURE2D_DESC {
            Width: desc.Width,
            Height: desc.Height,
            MipLevels: 1,
            ArraySize: 1,
            Format: desc.Format,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            Usage: D3D11_USAGE_DEFAULT,
            BindFlags: D3D11_BIND_SHADER_RESOURCE.0 as u32,
            ..Default::default()
        };
        let mut copy = None;
        device
            .CreateTexture2D(&copy_desc, None, Some(&mut copy))
            .map_err(|e| e.to_string())?;
        let copy = required(copy)?;
        let mut srv = None;
        device
            .CreateShaderResourceView(&copy, None, Some(&mut srv))
            .map_err(|e| e.to_string())?;
        let mut effects = vec![];
        for e in &b.effects {
            let mut pixel = None;
            device
                .CreatePixelShader(&e.code, None, Some(&mut pixel))
                .map_err(|e| e.to_string())?;
            let buffer_desc = D3D11_BUFFER_DESC {
                ByteWidth: 64,
                Usage: D3D11_USAGE_IMMUTABLE,
                BindFlags: D3D11_BIND_CONSTANT_BUFFER.0 as u32,
                ..Default::default()
            };
            let data = D3D11_SUBRESOURCE_DATA {
                pSysMem: e.parameters.as_ptr().cast(),
                ..Default::default()
            };
            let mut buffer = None;
            device
                .CreateBuffer(&buffer_desc, Some(&data), Some(&mut buffer))
                .map_err(|e| e.to_string())?;
            effects.push((required(pixel)?, required(buffer)?));
        }
        let sampler_desc = D3D11_SAMPLER_DESC {
            Filter: D3D11_FILTER_MIN_MAG_MIP_LINEAR,
            AddressU: D3D11_TEXTURE_ADDRESS_CLAMP,
            AddressV: D3D11_TEXTURE_ADDRESS_CLAMP,
            AddressW: D3D11_TEXTURE_ADDRESS_CLAMP,
            ComparisonFunc: D3D11_COMPARISON_NEVER,
            MaxLOD: f32::MAX,
            ..Default::default()
        };
        let mut sampler = None;
        device
            .CreateSamplerState(&sampler_desc, Some(&mut sampler))
            .map_err(|e| e.to_string())?;
        let mut blend_desc = D3D11_BLEND_DESC::default();
        blend_desc.RenderTarget[0] = D3D11_RENDER_TARGET_BLEND_DESC {
            BlendEnable: false.into(),
            SrcBlend: D3D11_BLEND_ONE,
            DestBlend: D3D11_BLEND_ZERO,
            BlendOp: D3D11_BLEND_OP_ADD,
            SrcBlendAlpha: D3D11_BLEND_ONE,
            DestBlendAlpha: D3D11_BLEND_ZERO,
            BlendOpAlpha: D3D11_BLEND_OP_ADD,
            RenderTargetWriteMask: D3D11_COLOR_WRITE_ENABLE_ALL.0 as u8,
        };
        let mut blend = None;
        device
            .CreateBlendState(&blend_desc, Some(&mut blend))
            .map_err(|e| e.to_string())?;
        let depth_desc = D3D11_DEPTH_STENCIL_DESC {
            DepthEnable: false.into(),
            DepthWriteMask: D3D11_DEPTH_WRITE_MASK_ZERO,
            DepthFunc: D3D11_COMPARISON_ALWAYS,
            ..Default::default()
        };
        let mut depth = None;
        device
            .CreateDepthStencilState(&depth_desc, Some(&mut depth))
            .map_err(|e| e.to_string())?;
        let raster_desc = D3D11_RASTERIZER_DESC {
            FillMode: D3D11_FILL_SOLID,
            CullMode: D3D11_CULL_NONE,
            DepthClipEnable: true.into(),
            ..Default::default()
        };
        let mut raster = None;
        device
            .CreateRasterizerState(&raster_desc, Some(&mut raster))
            .map_err(|e| e.to_string())?;
        Ok(Gpu {
            device,
            width: desc.Width,
            height: desc.Height,
            format: desc.Format,
            copy,
            srv: required(srv)?,
            vertex: required(vertex)?,
            effects,
            sampler: required(sampler)?,
            blend: required(blend)?,
            depth: required(depth)?,
            raster: required(raster)?,
        })
    }
}

/// Own getter-added references and restore only bindings changed by this service.
/// Arbitrary mod code never touches the context through this API.
struct Saved {
    c: ID3D11DeviceContext,
    vs: Option<ID3D11VertexShader>,
    ps: Option<ID3D11PixelShader>,
    gs: Option<ID3D11GeometryShader>,
    hs: Option<ID3D11HullShader>,
    ds: Option<ID3D11DomainShader>,
    classes: [Vec<Option<ID3D11ClassInstance>>; 5],
    rtvs: [Option<ID3D11RenderTargetView>; 8],
    dsv: Option<ID3D11DepthStencilView>,
    blend: Option<ID3D11BlendState>,
    factor: [f32; 4],
    mask: u32,
    depth: Option<ID3D11DepthStencilState>,
    stencil: u32,
    raster: Option<ID3D11RasterizerState>,
    viewports: Vec<D3D11_VIEWPORT>,
    layout: Option<ID3D11InputLayout>,
    topology: D3D_PRIMITIVE_TOPOLOGY,
    srv: [Option<ID3D11ShaderResourceView>; 1],
    sampler: [Option<ID3D11SamplerState>; 1],
    buffer: [Option<ID3D11Buffer>; 1],
}
impl Saved {
    fn capture(c: &ID3D11DeviceContext) -> Option<Self> {
        // SAFETY: Getter outputs point to initialized writable storage; D3D11
        // returns owned COM references. Preserve shader class instances as well.
        unsafe {
            // Do not execute an injected draw into existing stream-output buffers
            // or UAVs, whose hidden counters cannot be reconstructed by getters.
            let mut so: [Option<ID3D11Buffer>; 4] = Default::default();
            c.SOGetTargets(Some(&mut so));
            let mut uavs: [Option<ID3D11UnorderedAccessView>; 8] = Default::default();
            c.OMGetRenderTargetsAndUnorderedAccessViews(None, None, 0, Some(&mut uavs));
            let mut predicate = None;
            c.GetPredication(Some(&mut predicate), None);
            if so.iter().any(Option::is_some)
                || uavs.iter().any(Option::is_some)
                || predicate.is_some()
            {
                return None;
            }
            let mut s = Saved {
                c: c.clone(),
                vs: None,
                ps: None,
                gs: None,
                hs: None,
                ds: None,
                classes: std::array::from_fn(|_| vec![None; 256]),
                rtvs: Default::default(),
                dsv: None,
                blend: None,
                factor: [0.; 4],
                mask: 0,
                depth: None,
                stencil: 0,
                raster: None,
                viewports: vec![D3D11_VIEWPORT::default(); 16],
                layout: None,
                topology: c.IAGetPrimitiveTopology(),
                srv: [None],
                sampler: [None],
                buffer: [None],
            };
            let mut counts = [256u32; 5];
            c.VSGetShader(
                &mut s.vs,
                Some(s.classes[0].as_mut_ptr()),
                Some(&mut counts[0]),
            );
            c.PSGetShader(
                &mut s.ps,
                Some(s.classes[1].as_mut_ptr()),
                Some(&mut counts[1]),
            );
            c.GSGetShader(
                &mut s.gs,
                Some(s.classes[2].as_mut_ptr()),
                Some(&mut counts[2]),
            );
            c.HSGetShader(
                &mut s.hs,
                Some(s.classes[3].as_mut_ptr()),
                Some(&mut counts[3]),
            );
            c.DSGetShader(
                &mut s.ds,
                Some(s.classes[4].as_mut_ptr()),
                Some(&mut counts[4]),
            );
            for (classes, n) in s.classes.iter_mut().zip(counts) {
                classes.truncate(n.min(256) as usize);
            }
            c.OMGetRenderTargets(Some(&mut s.rtvs), Some(&mut s.dsv));
            c.OMGetBlendState(Some(&mut s.blend), Some(&mut s.factor), Some(&mut s.mask));
            c.OMGetDepthStencilState(Some(&mut s.depth), Some(&mut s.stencil));
            s.raster = c.RSGetState().ok();
            let mut n = 16;
            c.RSGetViewports(&mut n, Some(s.viewports.as_mut_ptr()));
            s.viewports.truncate(n.min(16) as usize);
            s.layout = c.IAGetInputLayout().ok();
            c.PSGetShaderResources(0, Some(&mut s.srv));
            c.PSGetSamplers(0, Some(&mut s.sampler));
            c.PSGetConstantBuffers(0, Some(&mut s.buffer));
            Some(s)
        }
    }
}
impl Drop for Saved {
    fn drop(&mut self) {
        // SAFETY: Context and all captured COM objects remain owned and alive.
        // Unbind our SRV before restoring RTVs, then restore the exact used slots.
        unsafe {
            let c = &self.c;
            c.PSSetShaderResources(0, Some(&[None]));
            c.OMSetRenderTargets(Some(&self.rtvs), self.dsv.as_ref());
            c.VSSetShader(self.vs.as_ref(), Some(&self.classes[0]));
            c.PSSetShader(self.ps.as_ref(), Some(&self.classes[1]));
            c.GSSetShader(self.gs.as_ref(), Some(&self.classes[2]));
            c.HSSetShader(self.hs.as_ref(), Some(&self.classes[3]));
            c.DSSetShader(self.ds.as_ref(), Some(&self.classes[4]));
            c.OMSetBlendState(self.blend.as_ref(), Some(&self.factor), self.mask);
            c.OMSetDepthStencilState(self.depth.as_ref(), self.stencil);
            c.RSSetState(self.raster.as_ref());
            c.RSSetViewports(Some(&self.viewports));
            c.IASetInputLayout(self.layout.as_ref());
            c.IASetPrimitiveTopology(self.topology);
            c.PSSetShaderResources(0, Some(&self.srv));
            c.PSSetSamplers(0, Some(&self.sampler));
            c.PSSetConstantBuffers(0, Some(&self.buffer));
        }
    }
}
pub(super) fn render(
    b: &mut Backend,
    c: &ID3D11DeviceContext,
    target: &ID3D11RenderTargetView,
    desc: &D3D11_TEXTURE2D_DESC,
) -> Result<bool, String> {
    // SAFETY: Synchronous call on the native immediate render context. Resource
    // getters create owned references; our own Draw is protected by reentry guard.
    unsafe {
        let device = c.GetDevice().map_err(|e| e.to_string())?;
        if b.gpu.as_ref().is_none_or(|g| {
            g.device.as_raw() != device.as_raw()
                || g.width != desc.Width
                || g.height != desc.Height
                || g.format != desc.Format
        }) {
            b.gpu = Some(create(b, device, desc)?);
        }
        let Some(_saved) = Saved::capture(c) else {
            return Ok(false);
        };
        let g = b.gpu.as_ref().ok_or("graphics resources unavailable")?;
        let source = target.GetResource().map_err(|e| e.to_string())?;
        c.IASetInputLayout(None);
        c.IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
        c.VSSetShader(&g.vertex, None);
        c.GSSetShader(None, None);
        c.HSSetShader(None, None);
        c.DSSetShader(None, None);
        c.OMSetBlendState(&g.blend, Some(&[1.; 4]), u32::MAX);
        c.OMSetDepthStencilState(&g.depth, 0);
        c.RSSetState(&g.raster);
        c.RSSetViewports(Some(&[D3D11_VIEWPORT {
            Width: desc.Width as f32,
            Height: desc.Height as f32,
            MinDepth: 0.,
            MaxDepth: 1.,
            ..Default::default()
        }]));
        for (pixel, constants) in &g.effects {
            c.PSSetShaderResources(0, Some(&[None]));
            c.OMSetRenderTargets(None, None);
            c.CopyResource(&g.copy, &source);
            c.OMSetRenderTargets(Some(&[Some(target.clone())]), None);
            c.PSSetShader(pixel, None);
            c.PSSetShaderResources(0, Some(&[Some(g.srv.clone())]));
            c.PSSetSamplers(0, Some(&[Some(g.sampler.clone())]));
            c.PSSetConstantBuffers(0, Some(&[Some(constants.clone())]));
            c.Draw(3, 0);
        }
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rekenshi_plugin_api::{HostV1, OK};
    use std::ffi::{c_char, c_void};
    unsafe extern "C" fn log(_: u64, _: *const u8, _: usize) {}
    unsafe extern "C" fn resolve(_: *const c_char, _: *const c_char) -> *const c_void {
        std::ptr::null()
    }
    unsafe extern "C" fn claim(_: u64, _: *const c_void) -> i32 {
        OK
    }
    extern "C" fn count() -> u64 {
        0
    }
    #[test]
    #[ignore = "requires Windows GPU and REKENSHI_TEST_GAME pointing to the compiler DLL"]
    fn actual_pixels_and_game_pipeline_bindings_are_preserved() {
        let source = b"Texture2D scene:register(t0);SamplerState s:register(s0);float4 main(float4 p:SV_Position,float2 uv:TEXCOORD0):SV_Target{return scene.Sample(s,uv)*float4(1,0,0,1);}";
        check_pipeline(source, [0.; 16], &flat([51, 102, 153, 255]), |pixels| {
            assert!((49..=53).contains(&pixels[0]), "red channel={pixels:?}");
            assert_eq!(&pixels[1..4], &[0, 0, 255]);
        });
    }
    fn flat(pixel: [u8; 4]) -> Vec<u8> {
        pixel.repeat(64 * 64)
    }
    #[test]
    #[ignore = "requires Windows GPU and REKENSHI_TEST_GAME pointing to the compiler DLL"]
    fn kenshi2_sample_identity_ink_palette_exposure_and_alpha() {
        let source = include_bytes!("../../../../samples/kenshi2-style/src/style.hlsl");
        let neutral = [
            0., 1., 0.035, 0.06, 1., 1., 0., 0., 16., 0., 0., 0., 0., 0., 0., 0.,
        ];
        let mut input = flat([51, 102, 153, 102]);
        for y in 0..64 {
            for x in 32..64 {
                input[(y * 64 + x) * 4..(y * 64 + x) * 4 + 4]
                    .copy_from_slice(&[204, 204, 204, 102]);
            }
        }
        check_pipeline(source, neutral, &input, |pixels| {
            for (actual, expected) in pixels.iter().zip(&input) {
                assert!(actual.abs_diff(*expected) <= 1);
            }
        });
        let mut ink = neutral;
        ink[0] = 0.75;
        check_pipeline(source, ink, &input, |pixels| {
            let at = |x: usize| &pixels[(32 * 64 + x) * 4..(32 * 64 + x) * 4 + 4];
            assert!(at(32)[0] < 80, "edge should receive ink: {:?}", at(32));
            assert!(
                at(48)[0].abs_diff(204) <= 1,
                "flat field should remain unchanged"
            );
            assert!(pixels.chunks_exact(4).all(|p| p[3] == 102));
        });
        let mut palette = neutral;
        palette[7] = 1.;
        palette[8] = 4.;
        check_pipeline(source, palette, &flat([51, 102, 153, 102]), |pixels| {
            assert_eq!(&pixels[..4], &[85, 85, 170, 102]);
        });
        let mut exposure = neutral;
        exposure[6] = 1.;
        check_pipeline(source, exposure, &flat([51, 102, 153, 102]), |pixels| {
            assert_eq!(&pixels[..4], &[102, 204, 255, 102]);
        });
    }
    fn check_pipeline(
        source: &[u8],
        parameters: [f32; 16],
        input: &[u8],
        check: impl FnOnce(&[u8]),
    ) {
        let game = std::path::PathBuf::from(
            std::env::var_os("REKENSHI_TEST_GAME").expect("game directory"),
        );
        // SAFETY: Test callbacks consume buffers synchronously, return no exports,
        // and treat hook targets as opaque; all callbacks are process-lived.
        let host = unsafe { HostV1::new(1, log, resolve, claim, claim, count) };
        let code = shader::compile(&game, source, false).unwrap();
        let mut backend = Backend {
            host,
            game,
            effects: vec![super::super::Effect {
                token: 1,
                handle: 1,
                priority: 0,
                code,
                parameters,
            }],
            serial: 1,
            gpu: None,
            context: 0,
            saw_lighting: true,
            failed: false,
        };
        // SAFETY: Test owns this immediate context and all resources. Descriptors
        // use documented D3D11 flags; mapped data is read only until Unmap.
        unsafe {
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
            let desc = D3D11_TEXTURE2D_DESC {
                Width: 64,
                Height: 64,
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
            device
                .CreateTexture2D(
                    &desc,
                    Some(&D3D11_SUBRESOURCE_DATA {
                        pSysMem: input.as_ptr().cast(),
                        SysMemPitch: 64 * 4,
                        ..Default::default()
                    }),
                    Some(&mut texture),
                )
                .unwrap();
            let texture = texture.unwrap();
            let mut target = None;
            device
                .CreateRenderTargetView(&texture, None, Some(&mut target))
                .unwrap();
            let target = target.unwrap();
            context.OMSetRenderTargets(Some(&[Some(target.clone())]), None);
            backend.gpu = Some(create(&backend, device.clone(), &desc).unwrap());
            let g = backend.gpu.as_ref().unwrap();
            context.VSSetShader(&g.vertex, None);
            context.PSSetShader(&g.effects[0].0, None);
            context.PSSetConstantBuffers(0, Some(&[Some(g.effects[0].1.clone())]));
            context.OMSetBlendState(&g.blend, Some(&[0.1, 0.2, 0.3, 0.4]), 12345);
            context.IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP);
            let old_viewports = [
                D3D11_VIEWPORT {
                    Width: 32.,
                    Height: 24.,
                    MaxDepth: 1.,
                    ..Default::default()
                },
                D3D11_VIEWPORT {
                    Width: 8.,
                    Height: 16.,
                    MaxDepth: 0.75,
                    ..Default::default()
                },
            ];
            context.RSSetViewports(Some(&old_viewports));
            let pixel = g.effects[0].0.clone();
            let constants = g.effects[0].1.clone();
            let blend = g.blend.clone();
            render(&mut backend, &context, &target, &desc).unwrap();
            let mut restored = None;
            context.PSGetShader(&mut restored, None, None);
            assert_eq!(restored.unwrap().as_raw(), pixel.as_raw());
            let mut cb = [None];
            context.PSGetConstantBuffers(0, Some(&mut cb));
            assert_eq!(cb[0].as_ref().unwrap().as_raw(), constants.as_raw());
            let mut rt = [None];
            context.OMGetRenderTargets(Some(&mut rt), None);
            assert_eq!(rt[0].as_ref().unwrap().as_raw(), target.as_raw());
            let mut restored_blend = None;
            let mut factor = [0.; 4];
            let mut mask = 0;
            context.OMGetBlendState(
                Some(&mut restored_blend),
                Some(&mut factor),
                Some(&mut mask),
            );
            assert_eq!(restored_blend.unwrap().as_raw(), blend.as_raw());
            assert_eq!(factor, [0.1, 0.2, 0.3, 0.4]);
            assert_eq!(mask, 12345);
            assert_eq!(
                context.IAGetPrimitiveTopology(),
                D3D_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP
            );
            let mut viewports = [D3D11_VIEWPORT::default(); 16];
            let mut n = 16;
            context.RSGetViewports(&mut n, Some(viewports.as_mut_ptr()));
            assert_eq!(n, 2);
            assert_eq!(viewports[0].Width, 32.);
            assert_eq!(viewports[1].MaxDepth, 0.75);
            let read_desc = D3D11_TEXTURE2D_DESC {
                Usage: D3D11_USAGE_STAGING,
                BindFlags: 0,
                CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32,
                ..desc
            };
            let mut staging = None;
            device
                .CreateTexture2D(&read_desc, None, Some(&mut staging))
                .unwrap();
            let staging = staging.unwrap();
            context.CopyResource(&staging, &texture);
            let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
            context
                .Map(&staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped))
                .unwrap();
            let mut pixels = Vec::with_capacity(64 * 64 * 4);
            for y in 0..64 {
                pixels.extend_from_slice(std::slice::from_raw_parts(
                    mapped.pData.cast::<u8>().add(y * mapped.RowPitch as usize),
                    64 * 4,
                ));
            }
            context.Unmap(&staging, 0);
            check(&pixels);
        }
    }
}
