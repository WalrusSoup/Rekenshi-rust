# Graphics integration research

Investigated 2026-10-06. Dust source snapshot: [`365c903515d957904ee447605541005697cc5fce`](https://github.com/Bazouz660/Dust/tree/365c903515d957904ee447605541005697cc5fce). Source links below use `main` for readability; findings refer to this checked revision.

## Practical recommendation

Implement a host-owned D3D11 effects service and a native Rust color-grading plugin first. Hook game render draws, identify the tonemap pass from GPU resources, and run an effect immediately afterward, before UI composition. This follows Dust's successful integration seam without needing KenshiLib's game executable addresses. An effect at Present changes already-composited UI and cannot claim the same placement. This is an implementation recommendation inferred from Dust's actual [pipeline detector](https://github.com/Bazouz660/Dust/blob/main/src/PipelineDetector.cpp) and [draw hook](https://github.com/Bazouz660/Dust/blob/main/src/D3D11Hook.cpp).

Supporting existing Dust binaries is a separate compatibility feature. No `Dust.dll` currently exists under the user's installed Kenshi directory, and simply calling its startup against the current executable is unsafe: its KenshiLib dependency currently only recognizes supported 1.0.65 executable hashes. Local installation observations and upstream compatibility details follow below.

## What Dust actually does

Dust splits its rendering framework from independent effect DLLs. The framework identifies deferred lighting and tonemap passes, supplies GPU resource/state helpers, and dispatches effect callbacks. Its current actual header advertises **API 9**; the README's API 8 description is stale. Consult [DustAPI.h](https://github.com/Bazouz660/Dust/blob/main/src/DustAPI.h), [EffectLoader.cpp](https://github.com/Bazouz660/Dust/blob/main/src/EffectLoader.cpp), and [PipelineDetector.cpp](https://github.com/Bazouz660/Dust/blob/main/src/PipelineDetector.cpp).

The architecture does not replace Ogre. `D3D11Hook::Install()` creates a temporary hardware D3D11 device/context/swapchain, reads function addresses from COM vtables, and installs inline hooks through KenshiLib. It discovers Draw, DrawIndexed, shader/resource creation and binding methods, Map/Unmap, Present/Present1, and ResizeBuffers. It separately hooks `D3DCompile` before Ogre compiles shaders. The current source also injects motion vectors and integrates vendor temporal-AA runtimes; that is substantially larger than a basic post-tonemap filter. [Source: D3D11Hook.cpp](https://github.com/Bazouz660/Dust/blob/main/src/D3D11Hook.cpp).

Dust's first confirmed pipeline detection selects the game's actual device/context. Capturing the first arbitrary D3D11 call can select an overlay's separate device and cause cross-device resource binding failures. Its current Present filter uses top-level calls and initialized GUI-swapchain affinity; comments explicitly reject render-thread and device-pointer identity as unreliable under NVIDIA Smooth Motion. A Rust implementation should confirm its rendering context from pass detection before retaining GPU objects and treat swapchain identification separately. [Source: confirmed-device capture and IsGamePresent in D3D11Hook.cpp](https://github.com/Bazouz660/Dust/blob/main/src/D3D11Hook.cpp).

### Pass identification

The following are source observations, not guaranteed signatures for every future engine update. Validate them with real render logs and fail closed when absent. [Source: PipelineDetector.cpp](https://github.com/Bazouz660/Dust/blob/main/src/PipelineDetector.cpp).

| Stage | Dust's GPU-state signature | Captured resources |
| --- | --- | --- |
| Deferred lighting | RTV `R11G11B10_FLOAT`; PS SRV slot 2 `R32_FLOAT`; PS SRV slot 0 bound | Albedo slot 0, normals slot 1, depth slot 2, HDR RTV |
| Fog | After lighting; HDR RTV; alpha blend enabled; slot 0 `R32_FLOAT` | The pre-fog HDR snapshot is managed separately |
| Tonemap | After lighting; RTV `B8G8R8A8_UNORM` or `R8G8B8A8_UNORM`; full-resolution texture; slot 0 `R11G11B10_FLOAT` | LDR RTV |

Tonemap detection's lower bound is 640×360 to reject bloom/luminance downsamples. Effects may run before and/or after the matched game draw; framework-generated fullscreen draws must bypass detection recursively. Per-frame detection flags reset before the next game's render loop. [Source: PipelineDetector.cpp](https://github.com/Bazouz660/Dust/blob/main/src/PipelineDetector.cpp), [D3D11Hook.cpp](https://github.com/Bazouz660/Dust/blob/main/src/D3D11Hook.cpp).

## Exact existing-Dust loader requirements

The exported startup symbol is **`?startPlugin@@YAXXZ`**, with Windows x64 signature `void(void)`. Both `Dust.dll` and `DustBoot.dll` export it as C++ linkage. Neither exposes a public stop callback. Their DllMain attach logic pins the DLL permanently because KenshiLib trampolines cannot be removed. Their termination paths signal passthrough before teardown; explicit hot-unload is inappropriate. [Sources: Dust dllmain.cpp](https://github.com/Bazouz660/Dust/blob/main/src/dllmain.cpp), [DustBoot.cpp](https://github.com/Bazouz660/Dust/blob/main/boot/DustBoot.cpp).

Dust's manifest requests `PreloadPlugins=["DustBoot.dll"]` and `Plugins=["Dust.dll"]`. Original RE_Kenshi invokes preload DLLs after `GameWorld::initModsList` populates the available-mod list; postload DLLs come from active mods. It adds plugin search directories before `LoadLibraryW` and resolves the decorated startup symbol. A compatibility implementation must preserve DLL/resource directories and early boot versus later startup, rather than assuming a `rekenshi_plugin_entry` export. [Sources: Dust manifest](https://github.com/Bazouz660/Dust/blob/main/mod/RE_Kenshi.json), [RE_Kenshi Plugins.cpp](https://github.com/BFrizzleFoShizzle/RE_Kenshi/blob/master/Plugins.cpp).

DustBoot hooks DXGI factory swapchain creation early and saves the real swapchain/HWND. It exports `DustBoot_GetSwapChain() -> IDXGISwapChain*`, `DustBoot_GetHWND() -> HWND`, `DustBoot_IsHooked() -> bool`, and crash-report coordination functions. Dust has later discovery/recovery paths when Boot misses creation, but this does not eliminate the game-loop/RVA dependency. [Source: DustBoot.cpp](https://github.com/Bazouz660/Dust/blob/main/boot/DustBoot.cpp).

Dust startup loads effect descriptors/configs first, then hooks `GameWorld::_NV_mainLoop_GPUSensitiveStuff(GameWorld*, float)` to reset render-frame state and supply camera access, installs its early PSSM hook, and finally installs D3D11 hooks. The shader-compile hook and early shadow configuration must precede creation/compilation of corresponding resources. Startup returns `void`, so a loader cannot treat return as proof that all hooks succeeded; Dust logs contain the actual success/failure evidence. [Source: dllmain.cpp](https://github.com/Bazouz660/Dust/blob/main/src/dllmain.cpp).

### KenshiLib compatibility blocker in this installation

Local read-only inspection found:

- Root game executable: `D:\SteamLibrary\steamapps\common\Kenshi\kenshi_x64.exe`, MD5 **`8A03C256F0DA1555D9CCEB939B41530A`**.
- Existing installed `KenshiLib.dll` exports `?Init@KenshiLib@@YA_NXZ`, `?GetRealAddress@KenshiLib@@YA_JPEAX@Z`, and `?AddHook@KenshiLib@@YA?AW4HookStatus@1@PEAX0PEAPEAX@Z`.
- Installed RVA files are only `RE_Kenshi\RVAs\Steam_1.0.65.br` and `GOG_1.0.65.br`, each 24,044 bytes.
- A separate legacy compatible `RE_Kenshi\kenshi_x64.exe` exists; its use would require an explicit compatibility-launch implementation and validation, not silently applying its offsets to the root executable.

Upstream KenshiLib revision `b61f9515e44fce3dafd6459eed3bbf041850c022` recognizes MD5 **`df4a5a7ef8a29deb24b70e7b7f4a222a`** for Steam 1.0.65 and **`213349bf76a0a758067f9ed0aef2ab01`** for GOG 1.0.65. `KenshiLib::Init()` calls `InitRVAs()`, rejects unknown executable hashes, reads `RE_Kenshi/RVAs/<platform>_<version>.br` relative to CWD, requires its exact generated-table length, builds absolute function pointers, and initializes MinHook. `GetRealAddress` depends on that initialized table. Do not force the 1.0.65 map onto a different executable. [Sources: Kenshi.cpp](https://github.com/KenshiReclaimer/KenshiLib/blob/b61f9515e44fce3dafd6459eed3bbf041850c022/Source/kenshi/Kenshi.cpp), [Functions.cpp](https://github.com/KenshiReclaimer/KenshiLib/blob/b61f9515e44fce3dafd6459eed3bbf041850c022/Source/core/Functions.cpp).

Dust uses raw verified game offsets for camera traversal (`GameWorld+0x580`, `PlayerInterface+0x30`, `CameraClass+0x68`) because compiling reconstructed C++ structures with VS2022 does not preserve the game's VS2010 standard-library layouts. A Rust graphics service should avoid these internals until individually version-gated. [Source: CameraAccess.cpp](https://github.com/Bazouz660/Dust/blob/main/src/CameraAccess.cpp).

## Stable graphics API requirements

The current Rust `HostV1` provides logging, export resolution, and hook reservations; it supplies no render events/resources. Add a separate versioned graphics service/table rather than changing V1's exact-size contract in place. This recommendation is based on the local [`crates/plugin-api/src/lib.rs`](../../crates/plugin-api/src/lib.rs) contract and [Dust's C interface](https://github.com/Bazouz660/Dust/blob/main/src/DustAPI.h).

A useful first service needs scalar ABI/version sizes, opaque COM handles, explicit stage/priority, init/device-reset/render/shutdown callbacks, resource ownership rules, registration/unregistration, and a graphics-ready state. Render callbacks must run synchronously on the game render thread; pointers are borrowed for the call unless explicitly AddRef'd. Track active callbacks before unregister/shutdown and hold modules while trampolines remain callable. This is an engineering recommendation inferred from Dust's lifecycle and resource management, not a claim that its exact API can be reused without full implementation. [Sources: DustAPI.h](https://github.com/Bazouz660/Dust/blob/main/src/DustAPI.h), [EffectLoader.cpp](https://github.com/Bazouz660/Dust/blob/main/src/EffectLoader.cpp), [D3D11Hook.h](https://github.com/Bazouz660/Dust/blob/main/src/D3D11Hook.h).

Native `DustEffectCreate` is `int(DustEffectDesc*)`, returns zero on success, and fills a caller-zeroed descriptor. API 9 uses pure C pointers/scalars, C enum storage, C `int` callback results, and plugin-owned settings pointers. Its host logging callback is C **variadic**, which requires special care in Rust. Boolean settings point to native C++ `bool` storage, not a four-byte BOOL. Declaring Rust bindings is feasible; claiming Dust plugin compatibility requires implementing every used helper and stage, including early configuration and post-light-volume dispatch. [Source: DustAPI.h](https://github.com/Bazouz660/Dust/blob/main/src/DustAPI.h).

Calculated Windows x64 default-alignment layouts for the current header (must assert against a native compiler before shipping bindings):

| Type | Size | Significant offsets |
| --- | --- | --- |
| DustCameraData | 128 | valid 0, right 4, up 16, forward 28, position 40, inverseView 52, nearZ 116 |
| DustFrameContext | 168 | device 0, context 8, point 16, timing 20, width 24, height 28, frameIndex 32, camera 40 |
| DustSettingDesc | 64 | name 0, type 8, valuePtr 16, min 24, max 28, iniKey 32, enumLabels 40, description 48, perfImpact 56, flags 60 |
| DustEffectDesc | 168 | version 0, name 8, point 16, Init 24, Shutdown 32, Resize 40, pre 48, post 56, enabled 64, settings 72, count 80, changed 88, save 96, load 104, timingPtr 112, flags 120, section 128, directory 136, priority 144, earlyConfig 152, postLightVolumes 160 |
| DustHostAPI | 168 | version 0; 20 function pointers begin at 8 |

### Implementation safety and verification

`OMGetRenderTargets` and `PSGetShaderResources` increment interface reference counts: every inspection must release these references. `Draw` supports a fullscreen triangle generated from `SV_VertexID`, allowing an effect pass without a game vertex-buffer ABI. Before ResizeBuffers, release retained swapchain backbuffers and views; re-create effect render targets after dimensions/device changes. [Microsoft OMGetRenderTargets](https://learn.microsoft.com/en-us/windows/win32/api/d3d11/nf-d3d11-id3d11devicecontext-omgetrendertargets), [PSGetShaderResources](https://learn.microsoft.com/en-us/windows/win32/api/d3d11/nf-d3d11-id3d11devicecontext-psgetshaderresources), [Draw](https://learn.microsoft.com/en-us/windows/win32/api/d3d11/nf-d3d11-id3d11devicecontext-draw), [ResizeBuffers](https://learn.microsoft.com/en-us/windows/win32/api/dxgi/nf-dxgi-idxgiswapchain-resizebuffers).

Recommended concrete first enhancement: post-tonemap color grade with exposure/contrast/saturation controls. Copy the matched output to a separate SRV texture, render back to the matched RTV, preserve original alpha, and save/restore every touched D3D11 binding. Include a thread-local recursion guard, deterministic priority, game-context identity, resize/device-loss paths, and disabled-by-default configuration while validating. Verify with an offscreen D3D11 render test showing identity settings preserve pixels and nonidentity settings change them, followed by live game-stage logs and screenshots. These are implementation recommendations, not guarantees of measured visual quality or FPS improvement.

## Licensing

Dust declares GPL-3.0-or-later with a GPL section 7 vendor SDK exception for NVIDIA DLSS/NGX and AMD FSR. Its header is in that repository and no separate permissive header license was found. KenshiLib includes GPLv3. Preserve attribution/license/source obligations if copying or adapting their implementation; independently implemented ABI service concepts can remain distinct from copied source. Redistribution of vendor runtimes has separate terms. [Dust LICENSE](https://github.com/Bazouz660/Dust/blob/main/LICENSE), [Dust COPYING](https://github.com/Bazouz660/Dust/blob/main/COPYING), [KenshiLib LICENSE](https://github.com/KenshiReclaimer/KenshiLib/blob/master/LICENSE).

## Follow-up: concrete color-grade implementation seam

### Exact hook targets

Dust's temporary device yields implementation addresses from these zero-based COM vtable indices. On Windows x64 use `extern "system"` with the explicit object pointer first. [Source: D3D11Hook.cpp](https://github.com/Bazouz660/Dust/blob/main/src/D3D11Hook.cpp).

| Object / method | Vtable index | Complete ABI shape |
| --- | --- | --- |
| ID3D11DeviceContext::Draw | 13 | `void(context*, u32 vertexCount, u32 startVertex)` |
| ID3D11DeviceContext::DrawIndexed | 12 | `void(context*, u32 indexCount, u32 startIndex, i32 baseVertex)` |
| IDXGISwapChain::Present | 8 | `HRESULT(chain*, u32 syncInterval, u32 flags)` |
| IDXGISwapChain::ResizeBuffers | 13 | `HRESULT(chain*, u32 count, u32 width, u32 height, i32 format, u32 flags)` |
| IDXGISwapChain1::Present1 | 22 | `HRESULT(chain*, u32 syncInterval, u32 flags, const PRESENT_PARAMETERS*)` |

Dust's native stage detector operates in **Draw**, and only considers `vertexCount == 3 || vertexCount == 4`. Its DrawIndexed callbacks handle geometry/light-volume modifications rather than calling the fullscreen-pass detector. Therefore Draw is the necessary first hook; DrawIndexed is optional future compatibility for other fullscreen-quad implementations and must not be assumed to occur in this installed renderer. Dust calls original Draw once, then post-stage callbacks. A Rust guard must bypass its own `Draw(3,0)` to prevent recursive detection. [Source: HookedDraw and HookedDrawIndexed in D3D11Hook.cpp](https://github.com/Bazouz660/Dust/blob/main/src/D3D11Hook.cpp).

### Installed Ogre compositor evidence

Read-only inspection of this installation confirms the architecture statically:

- [main.compositor](/D:/SteamLibrary/steamapps/common/Kenshi/data/materials/compositors/main.compositor) creates `Lighting_HDR/rt_hdr` at target resolution in `PF_R11G11B10_FLOAT`. `Resolve_HDR` writes `HDR/Composite` into `rt_input`, supplying `rt_hdr` as input 0, bloom as input 1, and adapted luminance as input 2.
- [post.compositor](/D:/SteamLibrary/steamapps/common/Kenshi/data/materials/compositors/post.compositor) defines ping-pong postprocessing buffers in `PF_A8R8G8B8` without FSAA.
- [workspace.compositor](/D:/SteamLibrary/steamapps/common/Kenshi/data/materials/compositors/workspace.compositor) routes the world pipeline `Resolve_HDR -> Debug -> GUI`, so immediately after the Composite draw is before MyGUI.
- [hdr.material](/D:/SteamLibrary/steamapps/common/Kenshi/data/materials/post/hdr.material) defines the Composite pixel shader from `hdrfp4.hlsl`, entry `Composite_fp4`, target `ps_4_0`; it disables depth checking/writing and hardware culling. [hdrfp4.hlsl](/D:/SteamLibrary/steamapps/common/Kenshi/data/materials/post/hdrfp4.hlsl) binds the HDR scene as texture register 0.
- [gui.compositor](/D:/SteamLibrary/steamapps/common/Kenshi/data/gui/gui.compositor) defines **Main_Menu as GUI-only**. The title screen has no deferred-lighting/tonemap pipeline. A graphics plugin can register successfully at the menu while its world shader has never executed.

This matches Dust's expected HDR-input/LDR-output conditions, but is **not runtime evidence that the exact D3D11 view formats and Draw vertex counts have fired on executable 1.0.68**. No running Kenshi process or graphics instrumentation log was present during this investigation. The live check must load a world and log matched draw count, RTV/SRV formats, context identity, resolution, and stage callback count. A menu screenshot alone cannot validate post-tonemap processing.

**Subsequent live confirmation:** after leaving character creation and entering the world, the installed Rust graphics backend logged `[1791312446] ... first world effect pass: 1280x1024 format=87 before GUI` in [RE_Kenshi_Rust.log](/D:/SteamLibrary/steamapps/common/Kenshi/RE_Kenshi_Rust.log). Format 87 is `DXGI_FORMAT_B8G8R8A8_UNORM`. The source's effect-dispatch path requires a detected lighting signature, then the same context's LDR target with HDR SRV0, so this provides actual runtime confirmation of those conditions for this executable/session. It does not independently prove visual correctness or effects' performance cost. Source checked locally: `plugins/graphics/src/lib.rs` and `pipeline.rs`.

### Source versus destination and alias handling

At the original tonemap draw, PS SRV 0 is the **HDR input**, not the final LDR image. A color-grade pass must first let the original game draw complete, then copy the **destination LDR RTV resource** into a separate sampleable texture. Sample that copy while writing back into the original LDR RTV. Reading original SRV 0 would bypass the game's exposure, bloom, and tonemapping; sampling the destination resource while binding it as an RTV would create a read/write conflict. [Sources: PipelineDetector.cpp](https://github.com/Bazouz660/Dust/blob/main/src/PipelineDetector.cpp), [HostGetSceneCopy in EffectLoader.cpp](https://github.com/Bazouz660/Dust/blob/main/src/EffectLoader.cpp), [ClarityPostExecute](https://github.com/Bazouz660/Dust/blob/main/effects/clarity/DustClarity.cpp).

Dust's scene copy derives dimensions and format from the underlying RTV's Texture2D. It allocates a distinct one-mip, one-slice, sample-count-1 DEFAULT texture with SHADER_RESOURCE bind flags, creates its SRV, and CopyResources the source. It re-creates on size or format changes. For a minimal Rust path explicitly reject multisampled/array/mip/subrectangle targets until supported; preserve the **RTV view format** when creating the copy SRV, including typeless-resource handling, rather than assuming the resource format always equals the view format. Preserve target channel semantics through D3D11 views; do not manually swizzle BGRA. [Source: EffectLoader.cpp](https://github.com/Bazouz660/Dust/blob/main/src/EffectLoader.cpp).

Suggested minimal HLSL behavior: use a fullscreen triangle from SV_VertexID, use SV_Position integer coordinates with `Texture2D.Load` for exact matching pixels, apply exposure/contrast/saturation in the existing LDR domain, preserve source alpha, and clamp output to [0,1]. Identity parameters should reproduce the UNORM pixels. `Load` avoids changing sampler state and avoids half-texel/interpolation ambiguity. This is an independent implementation recommendation; it is an artistic LDR grade, not a replacement physically based HDR tonemapper. [Microsoft Draw / SV_VertexID](https://learn.microsoft.com/en-us/windows/win32/api/d3d11/nf-d3d11-id3d11devicecontext-draw).

### Save and restore the state actually touched

Dust's state block captures IA input layout/topology, VS+CB0, PS+SRVs0–3+samplers0–1+CB0, four RTVs+DSV, blend state/factor/mask, depth-stencil state/reference, rasterizer state, and one viewport. Each getter's COM references are released after restoration. Its Clarity renderer sets its own fullscreen shaders/topology/layout, no-cull rasterizer, no-depth depth state, full viewport, no-blend state, resources and constant buffer, then unbinds temporary SRVs before restoring. [Sources: D3D11StateBlock.cpp](https://github.com/Bazouz660/Dust/blob/main/src/D3D11StateBlock.cpp), [ClarityRenderer.cpp](https://github.com/Bazouz660/Dust/blob/main/effects/clarity/ClarityRenderer.cpp).

A robust Rust grade should snapshot **all state it will alter**, without assuming Dust's partial snapshot is complete for arbitrary plugins. At minimum preserve IA layout/topology; VS/PS and any class instances; PS SRV0+CB0; all previously bound RTVs/DSV if replacing them; blend factor/sample mask; depth/stencil reference; rasterizer state; and all viewports. If clearing GS/HS/DS to guarantee an independent triangle pipeline, snapshot and restore those shaders and their class instances too. Vertex/index buffers need not change when the shader uses SV_VertexID. Avoid altering samplers if using Load. Restore temporary SRVs to null first, restore outputs, then original inputs to avoid implicit conflict unbinding. Use explicit no-depth/no-blend/no-cull state objects (null depth state has depth enabled). These are implementation recommendations derived from D3D11 state semantics and the source's touched-binding list; verify against the debug layer and an offscreen pixel/state-preservation test.

### Installed renderer device and draw dispatch

Read-only `dumpbin /imports` and `/disasm` inspection of [RenderSystem_Direct3D11_x64.dll](/D:/SteamLibrary/steamapps/common/Kenshi/RenderSystem_Direct3D11_x64.dll), SHA256 `5F224882DED5B6D3F68F14D6BB4E1762267BBB602B7B9733BD153663D1541BB9`, found the `D3D11CreateDevice` IAT at RVA `0x72B10`. All three direct callsites (RVAs `0x2A21F`, `0x2AA48`, `0x3894A`) explicitly zero `r9d` before the call. On Windows x64, the fourth argument is Flags, so this renderer requests flags **0**, not SINGLETHREADED. No creation-flag mismatch with a flags-0 temporary device was indicated.

Its render-dispatch block calls context `Draw` through vtable offset `0x68` (slot 13) at RVA `0x3632C`, with VertexCount from the render operation and StartVertexLocation as the third argument. Nearby branches handle DrawInstanced at offset `0xA8` and DrawAuto at `0x130`. These disassembly observations establish that normal Draw is a legitimate installed-renderer path; the later world-pass log confirms the existing flags-0 discovered target actually fires in this game session.
