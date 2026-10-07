# Graphics effect SDK

The core `GraphicsBackend` provides a shared service to independent effect DLLs.
Use `rekenshi_plugin_sdk::PostProcess` to register embedded HLSL, update sixteen
parameters live and remove your pass. The [SDK generator](plugin-sdk.md) offers
a graphics starter; the [Kenshi 2 sample](../samples/kenshi2-style/README.md)
shows a complete safe implementation. Its declared menu controls appear automatically.

The SDK negotiates `rekenshi_graphics_api_v2` with `GraphicsV2`, including an
owner-checked parameter update callback. Shader buffers and parameters are copied
synchronously; allocation ownership stays in the host. V1 remains unchanged for
legacy clients. New Rust authors should use `PostProcess` instead of hand-writing
export lookups or callback tables.

External V1 DLL plugins use a versioned C service exported from
`RE_Kenshi_Rust.dll`. The compatibility `rekenshi_graphics.dll` also provides it
when the internal registry is absent; load that DLL before effects in `plugins.toml`.
The original HostV1 and PluginV1 layouts remain unchanged. The C SDK is
`crates/plugin-api/src/graphics.rs`; the legacy DLL example is `compat/color-grade`.

1. Resolve `rekenshi_graphics_api` with `HostV1::find_export` from the loaded
   `RE_Kenshi_Rust.dll` (or the compatibility `rekenshi_graphics.dll`). This pins
   the service through the host.
2. Negotiate `VERSION` and the exact `size_of::<GraphicsV1>()`. Rejected requests
   never write the output. An inactive service returns ERROR.
3. Fill `EffectV1` and call `register(host.plugin_token, &descriptor, &mut handle)`.
   Registration compiles and copies the shader source and parameters; no borrowed
   buffers survive that call. It accepts at most 32 effects, source up to 64 KiB,
   finite parameters, reserved=0 and the POST_TONEMAP stage.
4. Call `remove(host.plugin_token, handle)` during stop. A different owner cannot
   remove that handle. Removal serializes against rendering. Code remains pinned.

The pixel shader exports `main`, targets `ps_5_0`, accepts SV_Position and
TEXCOORD0 UV, and returns SV_Target. Bindings:

| Binding | Contract |
| --- | --- |
| PS t0 | Copy of the current LDR scene, including earlier registered effects |
| PS s0 | Linear clamp sampler |
| PS b0 | 64 bytes: descriptor.parameters, four float4 values |

Lower priority runs earlier; registration order breaks ties. Register multiple
effects for a simple ordered multi-pass chain. Each requires a scene copy and a
fullscreen draw. The service owns all shaders, buffers, textures and native COM
references, so the interface never exposes the game context or arbitrary GPU
state to an effect DLL. V1 parameters are static; V2 and the Rust SDK support
live updates through `PostProcess`. Shader compile
failure leaves other registrations intact.

The service detects fullscreen Draw(3/4) calls. It first recognizes HDR lighting
using an R11G11B10_FLOAT target and R32_FLOAT depth at PS t2, then recognizes
HDR-to-BGRA8/RGBA8 resolve on that immediate context. It samples a copy of the
resolved LDR output, rather than incorrectly sampling the original HDR input.
Supported targets have one mip, one array slice and one sample, at least 640×360.
Resources recreate lazily when device, target dimensions or format change.

It preserves every binding it modifies: shader objects and class instances for
VS/PS/GS/HS/DS, render targets/depth view, blend factors/sample mask, depth-stencil,
rasterizer, viewports, input layout/topology and PS t0/s0/b0. Other slots/stages
are not modified. It skips injection if stream-output targets, output-merger UAVs
or predication are active, preserving their hidden state. A runtime GPU error
disables rendering until restart while retaining effect handles for clean removal.
Counters/logs count executed passes, excluding these skips.

This is a new, limited interface inspired by
[Dust's architecture](https://github.com/Bazouz660/Dust#architecture), not its
binary API. It currently provides no G-buffer/depth resources, pre-fog stage,
temporal history, motion vectors, timestamp GPU profiling or shader hot reload.
Existing Dust effect DLLs require a port. The
local Dust/KenshiLib private address maps target 1.0.65, so loading their native
framework unchanged on the local 1.0.68 was not attempted.

Live validation on Kenshi 1.0.68 / NVIDIA RTX 5080 confirmed the world resolve at
1280×1024, format 87 (BGRA8_UNORM), with GUI downstream. A deterministic GPU test
also verifies actual effect pixel values and restoration of captured bindings.
Other renderer builds, wrappers/overlays, AA modes, and graphics hardware have
not been tested. A wrapper exposing a different Draw implementation from the
bootstrap device may require a device-creation capture adapter. No shader hashes
or private Kenshi executable offsets are used in the rendering hook.
