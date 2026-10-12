# Runtime shared scenery geometry

The experimental `graphics.landscape_instancing` setting replaces supported
regular Forests batches while their regions load. It reads each live build queue,
shares the source mesh's vertex/index buffers through native metadata clones,
and uploads one 48-byte affine transform per placement. There is no required
baked placement file, world-map crawl or save-specific asset bundle. The option
is off by default; reload your save after changing it to rebuild loaded regions.

The first implementation requires one exact source SubMesh per batch, at least
two placements, white placement colors, positive uniform scale, unit rotation
quaternions, UV0 with no additional texture-coordinate streams, and known
object/shadow HLSL programs. Mixed-source batches, wind, animated meshes and
unsupported materials/layouts use the original builder. This keeps the original
region grouping, scene-node transform, bounds and visibility decisions.

The private material clones preserve textures, fragment programs and named/auto
constants. Their instanced vertex programs compose the instance transform with
the batch's parent exactly once. Shadow variants use the compatible UV0 plus
TEXCOORD1/2/3 matrix layout. Native clear restores the original material and
vertex declaration, removes registered shader/material clones, and releases
instance buffers. Existing optimized batches remain valid after disabling the
setting and retire when their regions unload. Render/clear hooks remain pinned
for that ownership; disabling only stops new replacement builds.

The F11 overlay reports live optimized batches, cumulative placements and
fallbacks. Logs identify each successful build and the first eight preparation
failures. Capture marks instance allocations `shared_mesh_instances`; expanded
input vertex counts must not be interpreted as allocated instance vertices.

The older cache exporter remains an optional diagnostic tool.

## Production-path validation

`scripts/test-instancing-geometry.py` maps the fingerprinted executable without
running its entry point and exercises the actual Rust host hooks in an isolated
native Ogre renderer. Its test-only CRT onexit initialization allows Forests'
lazy static type string to register its destructor; no production CRT state is
patched. The fixture uses the actual object vertex entry with a diagnostic pixel
shader that visualizes normal, tangent, binormal, camera direction and world
position. Its technique-level caster includes basic, RTW and tessellated shaders.

Three placements produce byte-identical 120×64 framebuffers, including 983
colored pixels, through the original expanded and runtime shared paths. All 72
native expanded vertex positions match the shared prototype plus affine stream.
The test checks zero renderer exceptions, draw counts, disabling while instances
remain alive, nonwhite fallback, ordinary outer-clear/recreate, an additional
adversarial same-SubBatch rebuild, provider stop, and native teardown. Retained
constructor metadata keeps only shared prototype bindings; it never contains
expanded copies. Native destruction releases those bindings normally.

Measured fixture geometry buffers: 3,672 bytes expanded versus 1,224 bytes of
shared prototype buffers plus 144 new transform bytes. The prototype was already
resident before either build; this is a synthetic allocation result, not a
measurement of the game's total RAM/VRAM reduction. Full world material/shadow
visual compatibility, game memory residency and frame-time impact remain to be
checked in-game.

The complete `scripts/verify.ps1 -KenshiDir` workflow passed, including the new
native material and runtime geometry fixtures. Final strengthened pixel/lifecycle
run: `target/instancing-geometry-final.log`, report
`target/instancing-geometry-tests/503d3419d8ad4fedb8f51583b9acb03c/report.json`.
Verification log: `target/runtime-instancing-final-verification.log`.

The tested host was installed with the experimental option enabled for the next
Hub test. Fixture, build and installed DLL SHA-256 all match
`DF0B8129D423F2637035395DADECB4E2E249F139380E694ACF3E1B0B44168FD6`.
Previous DLL and settings are preserved in the deployment manifest at
`target/runtime-instancing-deployment.json`. Other user settings were verified
unchanged. No live-game memory saving has been measured for this build yet.

Kenshi's Hub capture attributed 856.44 MiB of live vertex-buffer payload to the
regular and wind Forests batch builders, including 207.30 MiB of rocks and
200.78 MiB from two cactus source meshes. This measures logical buffer payload,
not physical system RAM or dedicated VRAM. The implementation here starts
testing whether shared source geometry plus placement records can replace this
expanded geometry.

## Placement capture and baking

Opt-in `graphics.resource_capture` copies the effective batch material and each submitted submesh's position,
orientation (wxyz quaternion), scale and renderer-native color before the
original builder runs. Capture schema 3 gains additive placement schema 1.
Copies of a captured batch share immutable placement arrays inside the host DLL;
they retain no native mesh pointers. Each build captures at most 65,536 valid
placements and reports omitted and invalid placements explicitly.

The fingerprinted queue constructor at executable RVA `0xa58590` copies the
fields into 0x48-byte entries: SubMesh pointer +0, position +8, orientation
+0x14, scale +0x24, color +0x30, opaque native field +0x40. The opaque field is
not exported. Queue positions are the inputs passed to native `addEntity`.
The native builder recenters expanded vertices. Capture also copies its batch
origin from the verified parent geometry field, without invoking scene-node
getters during construction. The native fixture checks all expanded positions
against the queued affine transforms after applying the completed batch's
output transform. The input matrices retain their original builder coordinates;
subtract the batch origin when reproducing the original batch-local vertices.
Ancestor transforms and streaming world origins are not assumed to be a
persistent world coordinate system.

Bake a fresh captured summary with:

```powershell
python scripts/export-instance-cache.py <capture-directory> <new-output-file.instances>
python scripts/test-instance-cache.py
```

The exporter rejects missing, incomplete, conflicting, non-finite and unsupported
placement records. It counts a build only once even if several vertex streams
repeat its metadata. It writes one indexed file with a 24-byte little-endian
header (`REKINST1`, u32 version, u32 JSON length, u64 record count), JSON source
and build indices (including batch origins), then 64-byte records containing three affine float4 rows and
the four captured color components. A SHA-256 checksum covers the record payload;
the metadata includes the source summary checksum. The reader checks lengths,
source ranges, finite values and payload integrity.

Source material names describe the SubMesh asset, which may differ from a live
SubEntity override. The separately copied effective batch material identifies
the native binding for this session; it is not a persistent material content
fingerprint.

These files are prototype artifacts. Build IDs are session-local chunks, not
persistent region identifiers. They do not yet encode resolved world origins,
mesh content fingerprints, active mod/save compatibility or unload ownership.
No production renderer automatically reads them. They contain neither source
geometry nor collision/navigation data.

## Hardware experiment

```powershell
$env:REKENSHI_TEST_GAME = 'D:\SteamLibrary\steamapps\common\Kenshi'
cargo test -p rekenshi-graphics hardware_instancing_matches_expanded_geometry -- --ignored --nocapture
```

The test owns an isolated hardware D3D11 device. It compares an independently
CPU-expanded indexed mesh with a shared mesh and 64 precomputed affine/color
records rendered using `DrawIndexedInstanced`. Placements include quarter-turn
rotation, nonuniform scale, mirrors and distinct colors. It requires identical
RGBA output and visible pixels for all 64 colors, then compares actual buffer
descriptors. The fixture measured 218,880 bytes expanded versus 6,220 bytes
shared, including vertex, index and instance buffers. This is a synthetic result,
not a measured reduction in Kenshi's memory usage.

The test uses a simple color shader and disables face culling to cover mirrored
placements. It does not establish game material, normal/tangent, shadow, wind,
visibility, region-unload or performance compatibility.

`python scripts/test-native-instancing.py <Kenshi-directory>` separately exercises
the shipped Ogre HWBasic backend with compiled affine HLSL programs. Three cube
entities share a 768-byte prototype vertex buffer and 144-byte matrix buffer;
Ogre makes one prototype copy per manager. Static scene nodes provide distinct
positions, rotations and nonuniform scales. Two real Ogre frames upload all three
48-byte affine matrices and render 36 faces in one batch. The fixture checks the
uploaded rows against each node's completed native transform, verifies entity
replacement reuses the buffers, and exercises normal entity/manager/renderer
teardown. Its report records `rendered_frame: true` and native face/batch counts.
The pinned backend requires scene-manager instancing culling method 1; method 0
left the visible-instance list empty in the regression. This native fixture does
not establish game material/shadow compatibility or compare Ogre frame pixels.

## Native integration boundary

The shipped Ogre DLL exports `InstanceManager`, `InstanceBatchHW` and static
instance batches. The installed object and shadow shader sources include an
`INSTANCED` path; Kenshi's material define generator also emits that token.
This establishes an existing backend to investigate, rather than a need to
replace the entire renderer. It does not establish a Forests replacement.

Before enabling a live replacement, verify the Forest renderable's scene/world
transform, its material and every shadow pass, and the teardown path when its
region unloads. Reuse the original spatial grouping/culling. Expanded buffers
must be avoided at build time; adding instanced draws alongside the original
allocations would not deliver the intended memory saving.

The existing Windows x64 fingerprinted private ABI and host-local callback-state
exception remain in force (M-UNSAFE, M-DOCUMENTED-MAGIC, M-ISOLATE-DLL-STATE,
M-AVOID-STATICS). The synthetic GPU experiment itself adds no live hooks; the
runtime option uses the pinned build/render/clear hooks described above. Rust
ownership remains within the host DLL. CPU/GPU memory residency and in-world
frame-time improvements remain unmeasured.
