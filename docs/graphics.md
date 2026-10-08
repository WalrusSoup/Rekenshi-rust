# Graphics effect SDK

The host also has an off-by-default [experimental ambient occlusion](ssao.md)
control. It operates in deferred lighting independently of the post-tonemap SDK.

The core `GraphicsBackend` provides a shared service to independent effect DLLs.
Use `rekenshi_plugin_sdk::PostProcess` to register embedded HLSL, update sixteen
parameters live and remove your pass. The [SDK generator](plugin-sdk.md) offers
a graphics starter; the [Kenshi 2 sample](../samples/kenshi2-style/README.md)
shows a complete safe implementation. Its declared menu controls appear automatically.

`PostProcess::new` registers copied desired HLSL and sixteen finite constants
through `rekenshi.render.post-tonemap-effects@1`. It can precede renderer
availability. Updates invalidate its public registration; removal quiesces its
callback. The provider compiles and owns native shader/buffer copies on its next
eligible draw, and retires them on policy removal, recreation or shutdown.
No fixed DLL name or host-private feature call is needed. Separately compiled
providers publish the same contract through the generic broker.

Each effect receives the copied current LDR scene at PS t0, linear clamp sampling
at s0 and its four float4 constants at b0. Ascending priority orders passes;
public registration identities resolve ties. Shader source/bytecode callbacks,
shader identities and scoped draw constants use separate public contracts,
including the built-in AO policy. Other verified stages can be published by
providers under their own protocol names; this backend supplies post-tonemap only.

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
disables the current rendering resources; a changed desired rule rebuilds them.
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
