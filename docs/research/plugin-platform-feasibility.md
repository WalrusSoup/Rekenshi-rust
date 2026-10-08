# Rust plugin platform: feasibility and evidence

Investigated 2026-10-08. Design research supporting the proposed plugin platform, not an implementation or workspace guideline audit. Local contracts and primary upstream sources were inspected. No game execution, native address mapping, shader compilation, save migration, or GPU capture was performed for this research.

## Current foundation versus proposed capabilities

| Area | Evidence available now | Proposed extension; not available merely by naming an API |
| --- | --- | --- |
| Gameplay | Owned player-roster snapshots and bounded commands for movement, stop, follow and standing orders; executable-specific adapter. | Arbitrary entity queries, creation/destruction, stats/inventory mutation, behavior interceptors, world/save lifecycle. |
| Time | Generated discovery catalog includes `GameWorld::setGameSpeed` and `TimeOfDay` methods. These are names, not callable verified bindings. | Calendar mutation, visual solar-time override, separate simulation-rate control and tested event behavior. |
| Rendering | Independent DLLs can register ordered late scene-color passes with live parameters. | Depth/normal/camera resources, geometry/material targeting, shader replacement, public compiler transforms and custom GPU passes. |
| Internal AO | The built-in experimental AO already transforms recognized lighting source before cache hashing, tags resulting shaders and binds a host-owned buffer for tagged draws. | Promote those internal facilities into versioned public contracts and migrate AO through them. |
| Native extension | Named target discovery, pinning and exclusive address reservations exist. | Extensible typed target definitions, coordinated interception, provider services and native-layout tooling. |

Sources: [gameplay contract](../gameplay.md), [adapter evidence](gameplay-native.md), [graphics contract](../graphics.md), [experimental AO](../ssao.md), [hook contract](../hooks.md), [symbol discovery](../native-symbols.md). Documentation contains both historical static-only statements and later live observations; capability acceptance should cite the exact test record rather than infer live coverage from either wording alone.

## Time is several different capabilities

The upstream reconstructed [GameWorld header](https://raw.githubusercontent.com/BFrizzleFoShizzle/KenshiLib/RE_Kenshi_mods/Include/kenshi/GameWorld.h) separates pause, game speed, frame multiplier, timestamps and hour length. The [TimeOfDay header](https://raw.githubusercontent.com/BFrizzleFoShizzle/KenshiLib/RE_Kenshi_mods/Include/kenshi/util/TimeOfDay.h) describes a type storing one double with conversion and comparison operations. `TimeOfDay::setTime` therefore does not establish a global world-clock setter. These headers are discovery references, not verified contracts for our current executable.

The specification should distinguish:

- Monotonic wall time for workers and timeouts.
- Simulation delta and pause/rate policy for AI, movement and physics.
- Absolute calendar time for hunger, crops, schedules, raids and deadlines.
- Visual solar/environment time for sky, sun and lighting.

This separation is a design requirement inferred from the distinct engine mechanisms. Calendar jumps need explicit forward/backward and missed-event policies; a sky override must say whether the calendar changes. Reading a timestamp, setting visual noon, pausing simulation, accelerating the calendar, and advancing the world by a day require separate verified capabilities. No global time setter or decoupled solar-time adapter was verified here.

## Native spawning is more than allocating an object

Upstream [RootObjectFactory.h](https://raw.githubusercontent.com/BFrizzleFoShizzle/KenshiLib/RE_Kenshi_mods/Include/kenshi/RootObjectFactory.h) declares general creation, building/item creation, random character/squad creation, unloaded variants and a main-thread update queue. Signatures depend on records, factions, containers, home buildings, save state and sometimes towns and squad state. This is useful evidence that native creation paths exist; it does not establish persistence or a usable binding on our supported game build.

A proposed spawn API should require a typed record/template, location, ownership and persistence policy. It must return completion only after native registration succeeds, distinguishing request accepted, created, streamed into the world and failed. A render-only mesh is a different resource from an interactive building with collision, pathfinding, ownership and save state. Inventory item creation is different from placing an item physically in the world. NPC creation must document squad/faction/AI/inventory initialization and loaded-zone requirements.

Runtime handles must remain generation-scoped. Save references need a separately verified durable identity and a resolver that can report unloaded, missing or mismatched records. Idempotent restoration keys must prevent duplicate spawning after reload. Native-save entities, plugin-reconstructed entities and session-only entities need distinct policies; restoring a plugin sidecar must never also recreate an entity already restored by the engine. Import/new game/removed plugin behavior requires acceptance tests. Destruction must use verified native removal paths, not Rust deallocation of native objects. These are proposed contracts; native save registration and all spawning paths remain unverified.

Product scope clarification: the baseline platform instantiates existing game/data-mod definitions. Definition/mesh authoring remains in the existing content workflow; custom mesh import is not a baseline SDK feature. The render-only/native-object distinction above remains a technical constraint, not a requirement to add a mesh-loading API.

## Shader controls need several interfaces

The existing [graphics service](../graphics.md) supplies LDR scene color, one sampler and a parameter buffer after tonemapping. It cannot identify NPCs, modify their materials, replace terrain shading or obtain geometry merely from a fullscreen image. The [built-in AO implementation](../ssao.md) already demonstrates a distinct internal source transformation and tagged-draw binding path, making it a valuable parity test for the proposed public SDK.

The proposed public system should separately support:

1. Parameter/material overrides against verified semantic parameters.
2. Targeted source transformations with recognized source/variant identity, transform ordering and strict anchor validation.
3. Complete source or bytecode replacement with input/output/resource-layout compatibility checks.
4. Additional render passes with declared resource reads/writes, stage/timing and resource lifetime.
5. An explicit native rendering extension for authors implementing new adapters.

These are distinct contracts. Whole replacements can be exclusive while compatible transforms compose; both need diagnostics naming the participants. Reflection can inspect compiled HLSL metadata, but cannot prove shader semantics or visual correctness. The actual installed compiler and reflection interface still need verification. [Microsoft D3DReflect reference](https://learn.microsoft.com/en-us/windows/win32/api/d3dcompiler/nf-d3dcompiler-d3dreflect).

Dust's first-party [C API](https://raw.githubusercontent.com/Bazouz660/Dust/main/src/DustAPI.h) exposes render stages, pre/post callbacks, camera information, depth/normal/albedo resources and resolution callbacks. Its [pipeline detector](https://raw.githubusercontent.com/Bazouz660/Dust/main/src/PipelineDetector.cpp) captures G-buffer resources from specific observed bindings and detects HDR lighting/fog/tonemapping. This establishes an implementation precedent, not compatibility with our DLL ABI or proof for every renderer variant. The earlier pinned [Dust investigation](dust-ssao.md) records shader transformation details; some pinned source URLs were unavailable through the browser during this follow-up, so those details were not independently re-fetched here. Moving branch references should be pinned before implementation. Study does not authorize copying code; retain source provenance and resolve the project's licensing decision before reuse. [Dust license](https://raw.githubusercontent.com/Bazouz660/Dust/main/LICENSE).

## GPU lifecycle and ownership are mandatory contracts

- D3D11 devices and device contexts have different threading rules. Immediate-context operations require coordinated access; host render callbacks should execute synchronously on their verified owner thread, and frame-scoped resource views must not escape to workers. [Microsoft threading introduction](https://learn.microsoft.com/en-us/windows/win32/direct3d11/overviews-direct3d-11-render-multi-thread-intro).
- Sampling a view that overlaps an active output binding causes D3D11 to bind null to the shader-resource slot. A render graph must track hazards and provide copies/ping-pong resources where needed. It must not promise in-place arbitrary texture reads/writes. [PSSetShaderResources reference](https://learn.microsoft.com/en-us/windows/win32/api/d3d11/nf-d3d11-id3d11devicecontext-pssetshaderresources).
- All direct and indirect back-buffer references must be released for swap-chain resize. SDK-held views and context bindings therefore participate in resize shutdown/recreation. [ResizeBuffers reference](https://learn.microsoft.com/en-us/windows/win32/api/dxgi/nf-dxgi-idxgiswapchain-resizebuffers).
- Device removal invalidates the resource set; reset handlers must recreate device-dependent resources and invalidate history rather than retain old handles. Host recovery remains limited by what the game itself can recover. [Microsoft device-removal guidance](https://learn.microsoft.com/en-us/windows/uwp/gaming/handling-device-lost-scenarios).

Depth encoding, coordinate systems, normals, color space, alpha, sample count, camera cuts and history validity belong in resource descriptors. A nominal `depth` texture is insufficient: the existing [SSAO research](ssao-feasibility.md) identifies a radial-distance convention in the shipped deferred source, which must be verified before adopting a generic hardware-depth reconstruction formula. State restoration, multi-plugin order and cache/source-transform identity are testable platform responsibilities.

## Third-party providers without a host fork

KenshiLib's [first-party development instructions](https://github.com/BFrizzleFoShizzle/KenshiLib#developing-re_kenshikenshilib-plugins) require its historical C++ toolchain and recommend its shared hook mechanism. Existing RE_Kenshi [installation documentation](https://github.com/BFrizzleFoShizzle/RE_Kenshi/blob/c8dfbc9621b9263863722852f819c4c0f440475a/README.md#installation) describes a compatible executable for supported 1.0.68 installations; that is distinct from this project's root executable adapter. Our [adapter evidence](gameplay-native.md) records address and even reconstructed string-layout differences. Neither headers, a matching marketing version nor an exported wrapper name prove ABI compatibility.

Providers should publish namespaced, versioned capability contracts plus exact module fingerprints, signature/layout evidence, execution phase and unsupported-build diagnostics. Cross-DLL service discovery must use explicit schema IDs and versioned C tables, not Rust `TypeId`, trait objects, `Any`, `String` or `Vec` ownership transfer. The ergonomic Rust facade can use ordinary Rust values internally, copying/encoding at its DLL boundary and retaining allocation ownership. This follows **M-ISOLATE-DLL-STATE** and **M-FFI-TRANSLATES** in the [local Microsoft guideline snapshot](../guidelines/microsoft-rust-guidelines.txt).

Safe consumer methods need providers whose unsafe construction contract establishes ABI, object lifetime, aliasing, thread affinity and callback validity; an unsafe provider cannot be made sound simply by returning an `Ok` status. Unsafe native registration belongs where misuse can cause UB (**M-UNSAFE**, **M-UNSAFE-IMPLIES-UB**). Arbitrary addresses and guessed layouts should remain research extensions until evidence and tests justify a supported adapter. Unsupported targets fail closed; no blanket guarantee covers arbitrary game versions or other detouring frameworks.

## Acceptance evidence to require

Before describing a capability as supported: verify exact binary identity and ABI, test the service boundary, execute the operation in game, and record the tested lifecycle cases. Spawning requires save/reload/streaming/import tests; clock control requires calendar/simulation/visual separation tests; shader changes require real-stage detection, variant coverage, resize/device resource handling and two-plugin conflict tests. Run a substantial built-in through the same public DLL boundary to establish parity. The current built-in AO is a concrete renderer candidate; a small NPC/object encounter plus time control is a concrete gameplay candidate. This research does not mark any of those proposed capabilities complete.
