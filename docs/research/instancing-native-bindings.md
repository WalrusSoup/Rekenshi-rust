# Native static-batch instancing seam (2026-10-08)

Read-only PE/export/capstone inspection of shipped Kenshi and Ogre binaries. No live game modifications. REA session was closed; native providers unavailable (Ghidra unconfigured). The findings below are static observations, not an executed replacement renderer.

## Observed capabilities

- OgreMain_x64.dll exports InstanceManager, InstanceBatchHW, InstanceBatchHW_VTF, SceneManager::createInstanceManager, InstanceManager::createInstancedEntity, SceneManager::destroyInstancedEntity, InstanceManager::setBatchesAsStatic.
- createInstanceManager RVA 0x2c9f50 takes `(SceneManager*, const String& name, const String& mesh, const String& group, int technique, size_t instancesPerBatch, uint16 flags, uint16 subMesh)`.
- Native InstanceManager::buildNewBatch at 0x13cb37 dispatches technique 0 ShaderBased, 1 VTF, 2 HWInstancingBasic (branch 0x13cd29), 3 HWInstancingVTF. This independently agrees with upstream Ogre enumeration ordering.
- `HWInstancingBasic` constructed batch has Renderable vtable RVA 0x687fa8 and MovableObject vtable RVA 0x688098. Native getWorldTransforms RVA 0x130760 returns identity; getNumWorldTransforms 0x130790 returns 1.
- HardwareVertexBuffer::getIsInstanceData RVA 0x10d430 reads byte +0x58; getInstanceDataStepRate 0x1083d0 reads size_t +0x60. Private setIsInstanceData RVA 0x1082a0 is called with true by HW build; native HW build writes step rate 1 at buffer+0x60 (0x12fefd).
- D3D renderer `_render` bounds 0x34170..0x36590 in RenderSystem_Direct3D11_x64.dll. It reads **RenderOperation.numberOfInstances at +0x18**, optional global-instancing flag byte +0x21, and passes that count to D3D11 DrawIndexedInstanced at 0x3607d (context vtable +0xa0) or DrawInstanced 0x3631d (+0xa8). Do not confuse +0x20 with instance count.

## Forest integration ownership

- Regular SubBatch vtable RVA 0x174fd08 slot 3 points to getRenderOperation 0xa5b6f0. Writes VertexData* from this+0x58 into op+0, operationType=4 at op+8, useIndices=true at op+0xc, IndexData* from this+0x60 into op+0x10. It leaves instance-count fields untouched.
- SubBatch getWorldTransforms vtable slot 6 / RVA 0xa5b920 copies parent-derived 4x4 matrix, using parent pointer at this+0x78. Replacement must preserve relationship between source instance transforms and parent transform. World-space instance matrices require identity world transform / view-projection semantics; parent-local instance matrices need composition exactly once.
- Native SubBatch destructor RVA 0xa58440..0xa58582 first calls clear 0xa5b4e0, then owns/deletes VertexData at +0x58 and IndexData at +0x60 with Ogre destructors/free. It destroys queue +0x90 and material SharedPtr +0x80, then Renderable base. Scalar-deleting destructor slot 0 / RVA 0xa5b980 calls this destructor, optionally frees receiver.
- **Never assign source mesh-owned VertexData/IndexData directly to these fields.** A replacement must own independent metadata clones sharing underlying buffer SharedPtrs; cloned bindings can own the new instance buffer.
- Queue at +0x90..+0x98 consists of 0x48-byte entries beginning with SubMesh*. Source entries are already copied by capture adapter. The parent is responsible for region scene/culling ownership.
- Only direct caller of BatchedGeometry::build RVA 0xa572d0 is BatchPage::build RVA 0xa33850 at call 0xa33868. No existing alternate hardware-instancing Forest path established.
- One SubBatch may mix multiple different source meshes with compatible material/declaration. A single hardware instancing render operation cannot replace such a batch with one prototype. Initial eligibility must require exactly one source SubMesh/material; mixed batches retain original rendering until render-queue subdivision is implemented.

## Shaders/materials

- Shipped `data/materials/deferred/objects.hlsl` lines 11..14 accepts three float4 matrix rows TEXCOORD1/2/3 under INSTANCED. Lines 43..46 create worldMatrix and transform position. Existing UV consumes TEXCOORD0.
- `common/shadowcaster.hlsl` has INSTANCED branch, but without TEXCOORDS its matrix semantics are TEXCOORD0/1/2; with TEXCOORDS they are TEXCOORD1/2/3. A shared declaration with UV0 therefore needs the TEXCOORDS-compatible caster variant or adjusted declaration. RTW shadows/tessellator also contain INSTANCED code.
- Shader source support is present, but ordinary objects.program and shadows.program do not declare complete instanced object variants. Kenshi generates them natively.
- Generator 0x841a50..0x84390d sees flags in r9d; bit 0x80 emits INSTANCED (test r13b sign at 0x842cbd; append string at 0x842cc8). Its r8 is game asset metadata with property maps at +0x1f8, not a Material SharedPtr, so cannot call it with a simple material clone.
- Reusable native shader-clone helper RVA **0x8409e0..0x840d93** (JMP thunk 0x33d20): inferred ABI `(SharedGpuProgram* out, SharedGpuProgram* consumedByValueTemp, const String* newName, const String* defines, bool argument5)`.
  - Resolves unified delegate, verifies HLSL, clones HighLevelGpuProgram through exported clone, preserves defaults via get/set default parameters virtual methods, applies defines via helper thunk 0x2ca61.
  - No game-singleton reads observed in function body.
  - **Consumes/decrefs and nulls the input SharedPtr temporary** at 0x840ca9..0x840cd7; retain a temporary ref first. Do not pass original Pass's shared member.
  - This helper ABI/defines behavior has not yet been executed in a native fixture. Prefer direct exported APIs if practical.
- Material cloning must preserve pass properties, textures, parameters, colouring, and shadow material configuration. Only vertex transformation changes. Need validate instanced colour/tangent/normal output against original nonuniform scale/rotation behavior and originals' vertex colour multiplication.

## Existing game instancing paths (not Forest)

- Generic entity-to-instance converter RVA 0x930c90..0x93108a requires one submesh, creates/reuses InstanceManager, transfers parent scene node, copies rendering distance/visibility, destroys original entity through game helper. Used via thunk 0xf1d7 from calls 0x934371 (conditional arg7 boolean) and 0x9347f9. Original pointer mutation/asset callback ownership not suitable to call blindly.
- Caller 0x816bd8 lies in NestBatcher::createItemEntityCallback (bounds 0x8169b0..0x816d7e; explicit identifying log). Caller 0x43a4c0 belongs HarpoonManager_ path. These prove native feature use but not forest eligibility.

## Minimum useful prototype

1. Hidden native Ogre InstanceManager fixture, HWInstancingBasic=2, real mesh and HLSL material; verify distinct translated/rotated/scaled instances, one prototype vertex/index allocation and 48-byte matrices.
2. Match original output pixels or vertex transforms; execute D3D instanced draw, not only build buffers.
3. Initial Forest replacement limited to regular, one-source, opaque object shader, compatible declaration/colour cases. Preserve parent bounds/culling and native destruction through owned metadata clones. Verify shadows separately.
4. Reject unsupported shader/declaration/colour/mixed-source/wind cases and call original builder; no fake setting reporting an optimization without replacing allocations.
5. Verify region deletion/recreation and teardown before live installation. No safe live replacement yet established by this inspection alone.

## Native frame corroboration

`scripts/test-native-instancing.py` now executes the shipped Ogre HWBasic backend
in an isolated hidden renderer. Static scene nodes with distinct translations,
quarter-turn rotations and nonuniform scales share one 768-byte cube vertex
buffer and one 144-byte affine buffer. Two `Root::renderOneFrame` calls upload
the expected three matrices and report exactly 36 rendered faces in one batch.
Entity replacement reuses both buffers; normal instance/manager/renderer teardown
passes. The fixture does not map or invoke the game executable, install hooks,
or modify the running game.

The native regression initially produced all-zero instance data and zero faces.
Scene-manager instancing culling method 0 left the instance list empty despite
correct visibility masks, world bounds, camera position and attached entities.
Changing the creation argument to method 1 was sufficient: the minimized passing
fixture needs no manual bounds update, forced render-queue injection, or manual
compositor execution. This is evidence for the pinned backend/fixture, not a claim
that every scene using method 0 is broken.

Verification: `python scripts/test-native-instancing.py
'D:/SteamLibrary/steamapps/common/Kenshi'` passed; log
`target/native-instancing-frame-final.log`, report
`target/native-instancing-tests/ae53b339f14343b7a21a43a5d4a65dd5/report.json`.
Ogre frame pixels and native game material/shadow variants remain unverified.

## Runtime replacement ownership

The optional Rust dispatcher now hooks the regular build, getRenderOperation
and clear lanes on these exact fingerprints. Geometry preparation uses native
VertexData/IndexData clone(false), keeps the original metadata for restoration,
and adds a native 48-byte instance stream with divisor one. Clear preserves
VertexDeclaration contents; regular build reuses that declaration at
0xa58b73..0xa58d38. Consequently the dispatcher restores the original metadata
after native clear and destroys the instance metadata, rather than leaving its
TEXCOORD1/2/3 elements in a future ordinary build.

The native clear path leaves the selected technique at +0xb0 untouched; standard
enqueue refreshes it at 0xa5b6aa..0xa5b6b5. The dispatcher zeros that cached pointer
when its private material retires. For optimized render operations it sets
instance count +0x18 and clears global-instance flag +0x21. D3D _render otherwise
multiplies explicit instance count by global count at 0x3421c..0x3422d.

Ogre resource manager map lookup/add/remove and Resource::load use native
recursive mutexes. This establishes map and same-resource load exclusion;
inspection did not establish an OS thread identity for every Forest caller or
arbitrary-thread renderer compatibility. Native fixture coverage and live game
validation must be kept distinct.

Runtime corroboration now includes the actual Rust hooks: byte-identical native
expanded/instanced frames using object vertex outputs for normal/tangent/binormal,
camera direction and world position, exact native geometry/affine correspondence,
opt-out, outer clear/recreate, adversarial same-object reuse, provider stop and
clean teardown. Fixture report:
`target/instancing-geometry-tests/503d3419d8ad4fedb8f51583b9acb03c/report.json`.
The additional same-object test preserves constructor prototype bindings during
restoration. In the observed vanilla lifecycle, SubBatch::clear's only executable
direct caller is its destructor; BatchedGeometry::clear deletes each SubBatch
through scalar-deleting vslot 0 and clears its map. Normal streaming therefore
constructs new SubBatches, rather than directly requeueing cleared ones.
