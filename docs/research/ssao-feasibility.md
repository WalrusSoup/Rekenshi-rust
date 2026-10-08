# SSAO feasibility for Kenshi

Investigated 2026-10-08. Read-only investigation of the installed game and graphics backend; no AO implementation or game-file changes. “SSO” in the request is interpreted as SSAO from the preceding discussion.

## Answer

SSAO means screen-space ambient occlusion: shading that suggests nearby geometry blocking ambient light. It can ground feet, rocks and buildings and add definition to creases. It is not a replacement for sunlight shadows or global illumination. NVIDIA describes depth-buffer sampling as the basis of its HBAO variant. [NVIDIA AO explanation](https://developer.nvidia.com/rendering-technologies/horizon-based-ambient-occlusion-plus)

Adding AO to Kenshi is feasible. Both a historical engine-material mod and a current D3D11 framework demonstrate approaches. This project's existing color-grade service is a useful starting point, but its public shader interface cannot implement geometry-aware SSAO as it stands: it supplies scene color and parameters, with no depth, normals or camera reconstruction data. [Graphics contract](../graphics.md), [C ABI](../../crates/plugin-api/src/graphics.rs)

## What is actually present locally

| Evidence | What it establishes | What it does not establish |
| --- | --- | --- |
| [settings.cfg](D:/SteamLibrary/steamapps/common/Kenshi/settings.cfg), line 63, contains `SSSAO=1` | A setting exists and is saved as enabled | A shader pass is executing or contributing AO |
| [compositors.cfg](D:/SteamLibrary/steamapps/common/Kenshi/data/materials/compositors/compositors.cfg) lists `#SSAO` | The shipped additional SSAO node is commented out in this file | Whether another active mod overrides compositor/material resources |
| [post.compositor](D:/SteamLibrary/steamapps/common/Kenshi/data/materials/compositors/post.compositor) contains node `SSAO` | A dormant pass connects G-buffer normals, depth and scene color | Visual correctness if uncommented |
| [AO.hlsl](D:/SteamLibrary/steamapps/common/Kenshi/data/materials/post/AO.hlsl) and [SSAO_Test.material](D:/SteamLibrary/steamapps/common/Kenshi/data/materials/post/SSAO_Test.material) | Real AO shader/material code ships with the game; the shader uses 16 Poisson samples and multiplies scene color by visibility | A supported, enabled native AO feature |
| [ReShade.ini](D:/SteamLibrary/steamapps/common/Kenshi/ReShade.ini) selects `Kenshi2Wugonorama.ini`; [that preset](D:/SteamLibrary/steamapps/common/Kenshi/Kenshi2Wugonorama.ini) includes `MXAO@MXAO.fx` in `Techniques` | MXAO is configured in the selected preset | ReShade's global toggle, successful compilation, correct depth capture, or visible AO in the current running world |

The earlier “native AO is enabled” conclusion was unsupported. The stronger claim “Kenshi has no native SSAO code whatsoever” would also be inaccurate. This installation contains dormant implementation assets; their existence and `SSSAO=1` do not prove working default AO. Do not claim AO stacking without a live pass comparison.

The original ShaderSSAO author says the mod adds SSAO, runs through the game's graphics engine and supports an option in the Graphics menu. Its changelog describes a reduced-resolution AO target, blur and composite, plus fixes for sky and character-editor behavior. That is historical evidence that the engine can host the effect, not validation on this installation. [ShaderSSAO author page](https://www.nexusmods.com/kenshi/mods/215)

## Existing integration seam and missing work

The [Rust Draw hook](../../plugins/graphics/src/native/mod.rs) invokes the original draw before inspecting it. Its lighting branch records only context identity and a marker. The [pipeline detector](../../plugins/graphics/src/native/pipeline.rs) identifies HDR lighting using an `R11G11B10_FLOAT` target and an `R32_FLOAT` SRV at pixel-shader slot 2, then identifies the LDR resolve. It does not retain the G-buffer or publish a pre-lighting callback.

The local [main compositor](D:/SteamLibrary/steamapps/common/Kenshi/data/materials/compositors/main.compositor) orders main lighting, water, atmosphere fog, fog volumes/particles, and HDR resolve. [Workspace routing](D:/SteamLibrary/steamapps/common/Kenshi/data/materials/compositors/workspace.compositor) puts GUI downstream. Existing [game logs](D:/SteamLibrary/steamapps/common/Kenshi/RE_Kenshi_Rust.log) contain successful world-effect matches at 1280×1024, BGRA8 format 87. This validates the existing resolve seam; it does not validate an earlier AO seam.

The shipped [deferred shader](D:/SteamLibrary/steamapps/common/Kenshi/data/materials/deferred/deferred.hlsl) decodes world-space normals from RGB, treats depth times the far-distance parameter as distance along a normalized view ray, and sums separate environment and sunlight contributions before emissive. A standard hardware-depth linearization formula would therefore be an unsafe visual assumption. Camera corners, normal-space transform, depth validity/sky convention and the live constant-buffer layout must be verified.

There are three increasingly complete routes:

1. **Late screen-space composite:** extend the host to retain depth/normals and camera data, then compute AO at the existing post-tonemap seam. Good for proving reconstruction, but darkening final color also affects fog, direct illumination and emissive. It cannot isolate ambient lighting.
2. **Pre-fog HDR composite:** add an earlier stage and apply AO before atmosphere processing. Better integration with fog and exposure, but multiplying the whole HDR target still darkens sunlight/emissive. Placement relative to water and local-light volumes matters.
3. **Ambient-light integration:** compute AO before the main lighting draw and feed visibility into the environment-light terms. This is the preferred final direction. It needs shader integration as well as a new dispatch/resource contract; it is not a color-grade-only plugin change. The recommendation follows the separate terms visible in the shipped deferred shader.

Dust's actual [SSAO plugin source](https://raw.githubusercontent.com/Bazouz660/Dust/main/effects/ssao/DustSSAO.cpp) performs AO generation before the game's lighting draw using depth, normals and camera data. It binds the resulting AO at slot 8 and a direct-light control texture at slot 9, with a white visibility fallback. Its [host ABI](https://raw.githubusercontent.com/Bazouz660/Dust/main/src/DustAPI.h) includes depth/normal access, pre-fog scene access and explicit handling for AO on point/spot-light volumes. This is concrete source evidence that Kenshi's D3D11 pipeline can support substantially better integration than a late overlay. It does not make Dust effect binaries compatible with this project's different ABI. The [Dust repository](https://github.com/Bazouz660/Dust) advertises GTAO at its lighting stage; this investigation did not install or execute it.

## Smallest useful prototype and acceptance checks

First build host-internal instrumentation, disabled by default, rather than exposing arbitrary native GPU objects to effect DLLs. On the confirmed game context, capture depth, normals and camera reconstruction inputs at lighting. Show grayscale depth, decoded normals and reconstructed position before attempting AO. Confirm camera zoom/rotation, aspect ratio, terrain, characters and sky. A plausible next prototype is a small-radius AO buffer, spatial edge-aware denoise and debug-only final composite. Only after reconstruction works should it be integrated into ambient HDR lighting.

Intel's [XeGTAO integration notes](https://github.com/GameTechDev/XeGTAO) describe depth plus optional normals, an AO output, and separate depth preparation, main and denoise passes. Spatial denoising can work without TAA. Its geometry approximation can over-occlude thin features or produce halos. Its full sample uses DX12/Shader Model 6.3 and is discontinued, so its published performance and build setup should not be treated as a drop-in DX11 implementation or Kenshi timing promise.

Measure GPU time at the user's actual resolution; inspect grass/foliage, silhouette edges, water, fog, sky, interiors, night lighting, UI and character editor. Compare identical camera views with AO off/on and with ReShade MXAO separately disabled to identify each contribution. Verify resize/device recovery, missing-resource bypass and neutral AO fallback. Do not promise a frame-rate cost from another engine's benchmark.

Keep GPU ownership and synchronization in the host, run context work synchronously in the intercepted rendering path, and marshal menu changes without moving native game work to arbitrary threads. A new versioned service must preserve old ABI layouts and DLL allocation ownership. These constraints follow the project's [Rust guidelines](../guidelines/microsoft-rust-guidelines.txt): M-UNSAFE, M-UNSOUND, M-FFI-TRANSLATES, M-ISOLATE-DLL-STATE and M-HOTPATH. This is feasibility research, not a guideline-compliance audit.

Implementation sources also need distinct treatment: the project declares MIT, while [Dust's license](https://raw.githubusercontent.com/Bazouz660/Dust/main/LICENSE) is GPL-3.0-or-later with specific vendor-SDK permissions. Intel publishes XeGTAO under MIT. Studying Dust's integration is useful; copying its implementation requires an explicit licensing decision. ShaderSSAO's author page has separate asset-use terms; no mod assets were downloaded or reused here.

## Verification limits

Checked local source, saved configuration, shipped material/compositor code, existing logs and primary upstream pages on the investigation date. No live GPU capture, visual AO toggle comparison, shader compilation or new effect performance measurement was performed. Active resource overrides, runtime camera constants, alternate render modes and wrapper compatibility remain unverified. Upstream links to `main` are moving references; pin an exact revision before implementation. No claim here establishes currently functioning native AO or ReShade MXAO in the user's rendered frame.
