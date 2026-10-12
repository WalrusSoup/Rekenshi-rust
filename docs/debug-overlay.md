# In-game debug overlay

Press **F11** while Kenshi has focus to show or hide the overlay immediately.
Holding the key toggles once. This session toggle does not write the saved
setting; Options still controls startup visibility. Hidden diagnostics stop
accumulating operation counters, while persistent logging continues.
The F11 press/release and focus behavior has a unit regression; the full native
verification workflow passed with this update (`target/f11-verification.log`).

Open **Options → REKENSHI-RUST**, page to **Developer diagnostics**, and enable
**Debug overlay**. It appears at the upper left and stays visible when Options
closes. Disable the same checkbox to remove it. **Freeze debug overlay** holds the
displayed snapshot; unfreeze to catch up. These settings apply without restart and
save as `debug.overlay` and `debug.freeze` in `RE_Kenshi_Rust/modpack.toml`.

For a graphics-memory investigation, enable **Graphics / Capture graphics
resources** (`graphics.resource_capture`). Restart with it enabled to include
startup resources. Each session writes `graphics-captures/<session>/events.jsonl`
and `summary.json` under `RE_Kenshi_Rust`. The summary refreshes every 15 seconds
on the UI lane and when capture is disabled or the host stops. Disable capture to
finish the report; F11 only controls overlay visibility.

The summary ranks live buffers and 1D/2D/3D textures by estimated logical payload,
with dimensions, array/mip/sample counts, numeric DXGI format, usage, bind/CPU
access flags, creation time and a session-local ID. Events record successful
creation and resource-owned COM marker release. Tracking retains no resource
reference or pixel/vertex content. It copies the enclosing synchronous Ogre
resource-load/prepare name when available; this is a correlation, not exclusive ownership.
Native debug names are copied at creation if already assigned. Unnamed resources
stay unnamed. Identical descriptors do not establish duplicate content.

Schema 2 adds a bounded creation stack (up to 32 module-name/relative-address
frames), stable session identity, and staging-resource activity counters. It
observes successful Map, Unmap, CopyResource, CopySubresourceRegion and
UpdateSubresource through the probed context implementation. Copies count source
and destination separately. A private 16-byte session/ID tag identifies staging
resources without caching native pointers. Old-session tags cannot update new
records. Destroy events include the final observed activity. Other context
implementations and operations are outside this coverage; an idle counter does
not establish that a resource is unnecessary. The address-resolution cache is
bounded at 8,192 entries. Creation stacks and per-operation private-data lookup
add diagnostic overhead while capture is enabled.

For a town-loading comparison, copy `summary.json` while stationary outside town,
copy it again after approaching, then copy it after leaving and allowing loading
to settle. Leave capture enabled throughout so IDs remain comparable. Summaries
refresh every 15 seconds. Run `python scripts/compare-graphics-captures.py
<before.json> <after.json>` for added, retired and surviving payload, creation
groups and staging resources with no observed activity during the interval.
Different sessions or out-of-order snapshots are rejected. This tests retention;
it does not measure draw culling, CPU mesh copies or dedicated VRAM residency.

Schema 2 verification passed the full installed-module workflow on 2026-10-08
(`target/resource-origin-verification.log`). The independent hardware GPU fixture
also passed separately (`target/resource-origin-native-test.log`), checking copied
contents, source/destination copy counts, map direction, creation stacks and
same-ID isolation across sessions. `scripts/audit-mesh-assets.py <data-directory>
--output <report-directory>` provides a separate read-only serialized-geometry
inventory. It reports parsing errors and container-length inconsistencies;
embedded LOD absence does not rule out external LOD meshes or engine policies.

Payload estimates include supported block-compressed formats, mip levels, arrays
and multisampling. Unknown formats remain unknown. These are neither system RAM
nor dedicated VRAM measurements: driver alignment, internal metadata, CPU copies
and resource residency are outside this inventory. Earlier resources, swap-chain
buffers, opened/shared resources and devices using another D3D11 implementation
are not covered. Only capture on the probed hardware D3D11 implementation is
claimed. Replacing the diagnostic's private GUID could retire its marker early;
the capture does not do that itself.

Capture is off by default, caps live records at 100,000 and event output at
256 MiB, and reports untracked creations, attachment failures, dropped events and
I/O errors. It adds allocation-time and periodic reporting overhead. The retained
host-local callback state is the native-adapter M-AVOID-STATICS exception; only
opaque COM callbacks cross the DLL boundary, with allocations retired by this DLL
(M-ISOLATE-DLL-STATE). Native tests verify real GPU buffer contents, texture
estimates, failed/validation-only creation, cloned references, final release,
capture restart and retired-session isolation.

Analyze a copied summary with `python scripts/analyze-graphics-capture.py
<capture-directory>`. The tool groups known payload by kind/name and ranks live
resources. Full verification with the installed game modules passed on 2026-10-08
(`target/resource-capture-verification.log`), including the independent release
host's native capture test. A real Kenshi run also produced named asset records,
successful destruction notifications and zero reported tracking/I/O gaps. This
establishes capture operation, not a memory-saving result or a leak diagnosis.

Schema 3 additionally copies source mesh/material names and submission counts
from the live queues entering Kenshi's `Forests::BatchedGeometry::SubBatch` and
`Forests::WindBatchedGeometry::WindSubBatch` builders. It reads their inputs
synchronously, then forwards the original call. A thread-local scope attaches
that copied provenance to resources created during the build and restores any
outer scope on return or native unwinding. It retains no native mesh/entity
references or addresses. A build ID distinguishes repeated builds and prevents
counting the same queue again for every vertex stream. Submesh submissions can
exceed distinct entity counts; these are neither draw nor visible-instance counts.

The private adapter requires executable SHA-256
`A596AB4E407C67B58599C54FFB32DC1BF2B64510CDEBD3FA9359EF05A576AEB1`
and the existing Ogre DLL fingerprint. Executable RTTI associates vtable slot 18
with build entries `0xa58930` (SubBatch) and `0xa685e0` (WindSubBatch). Both native
bodies traverse the vector at receiver `+0x90`/`+0x98` in `0x48`-byte entries,
whose first word is a live SubMesh pointer. Ogre's native `Mesh::createSubMesh`
sets its parent Mesh at `+0x70`; source vertex/index data are at `+8`/`+0x10`,
with counts at data `+0x30`/`+0x18`. Names use fingerprinted exported getters.
These are binary-derived offsets, rather than assumptions about today's C++ STL.
The adapter copies at most 1,000,000 queued entries and 4,096 unique submeshes per
build. Reports expose omitted submissions, snapshot failures and unavailability
(including standalone D3D tests without a mapped game executable).

Use `--mesh-audit <meshes.json>` with the analyzer to show candidate loose source
files for captured mesh names. It apportions each captured non-staging vertex
buffer's payload by source vertex counts only when the source sum equals the
batch's output count and no submissions were omitted. This is an estimated
contribution to expanded buffer payload. Ambiguous basename matches remain
ambiguous, and files outside the audited roots remain unresolved. Geometry not
created by these two Forests builders keeps its existing name/stack provenance.

The saved Hub baseline had 1,246.20 MiB of vertex-buffer payload. Creation paths
inside these verified builders accounted for 726.34 MiB (SubBatch) plus
154.48 MiB (WindSubBatch), or 70.7%. This establishes a large forest-batching
allocation path; it does not identify individual foliage types, prove excessive
polygon density, or establish CPU/VRAM residency. Fresh schema-3 captures supply
the per-mesh evidence needed for those asset questions.

Fresh captures also include additive placement schema 1: owned position, wxyz
orientation, scale and renderer-native color arrays. These are builder input
coordinates, not established world coordinates. Omitted and invalid placements
are reported explicitly. See [the shared geometry prototype](instancing-prototype.md)
for the compact offline instance-cache exporter and hardware differential test.

`scripts/test-batch-capture.py <Kenshi-directory>` maps the fingerprinted
executable without running its entry point, initializes real Ogre/D3D11 with a
hidden 64x64 window, and builds three prefab cube copies through Kenshi's actual
regular Forests builder with different positions, rotations, nonuniform scales
and colors. It compares the geometry and color GPU bytes with capture off
and on, verifies exact source name/counts (72 vertices, 108 indices, 2,304 geometry
bytes plus 288 color bytes), validates the queued transforms against all native
expanded positions and the completed batch transform, and bakes a 192-byte
affine/color payload. It checks reporting gaps and destroys native objects
normally. The independent
child has a timeout; all staged files stay under `target/`. This does not exercise
the wind builder's shader/data path or replace a fresh in-world capture.

The overlay shows host status, active built-in/DLL counts, failed DLL count, Ogre
observer calls, shader/collider/sidecar cache hits and terrain reads. Its public
operation summary lists up to seven contracts sorted by counted dispatches, with
provider generation, readiness/revocation, active registration count, observed
dispatch count and the first-use flag. A call count confirms broker dispatch;
it does not prove a shader's visual quality or successful gameplay completion.
Counts accumulate while enabled, pause while disabled, and reset when that
provider publishes a new generation. Some native adapters bypass broker dispatch
when there are no consumers. Freeze holds only the display; counters keep running.

Recent logs include `Context::log`, captured `Logger` messages, host hook startup,
settings applied from Options and plugin failures. The host retains the latest 128
messages, bounded to 200 characters each, and displays the latest 12 with UTC times.
Visible lines are truncated to fit. The existing `RE_Kenshi_Rust.log` remains the
full persistent log. Avoid per-particle/per-draw logging in mods; use the counters
to observe hot operations without flooding the stream.

The native view refreshes about twice per second and after settings clicks on
MyGUI's existing UI callback.
At startup it waits for the named skin resources and a usable native font before
creating captioned text. A frame callback can occur before font registration;
it does not establish readiness.
It takes no keyboard or mouse input. Its root is independent of the Options tree;
destroyed widgets are found/recreated by name, with no cached native addresses.
MyGUI owns the widget/layer allocations; disabling destroys the root, and UI-lane
host shutdown retires it before detaching the frame hook. The empty diagnostic
layer is reused until native Gui teardown. The existing MyGUI fingerprint gate
applies; no unsupported native binding is enabled by this setting.

The bounded log ring and toggle atomics share the host's process-lifetime native
adapter state (the retained M-AVOID-STATICS exception): logging and frame entry
points need a common host-local sink. Rust collections and allocator ownership
stay inside the host DLL; MyGUI strings and widgets use its native allocator
(M-ISOLATE-DLL-STATE, M-UNSAFE).

`scripts/test-menu.py` exercises the actual shipped MyGUI DLL: real Options
checkbox activation, copied SimpleText captions, input properties, freeze,
recreation, persistence and UI-lane shutdown/restart. Broker tests verify that
reading diagnostics never executes plugin policy and that disabled counters pause.
The native fixture also runs before font registration, registers the font later,
and executes Kenshi's exact font-getter/setter slots after activation. This covers
the 2026-10-08 startup crash: the shipped EditText font getter returns `mFont + 8`
without checking null, and Kenshi's font refresh dereferenced that invalid string
after the overlay created text too early. The crash dump identified
`FontManager::getByName` at RVA `0xd5eb0`, reached from Kenshi RVAs
`0x3e7fd7`/`0x3e7fe3`. The regression failed before the resource gate and passed
after it. Fixture calls use the inspected native SkinItem base offset, not the
current compiler's C++ object layout.
These headless native fixtures do not replace visual QA in a running world.

The experimental Graphics option **Share scenery geometry** reports live
optimized scenery batches, cumulative instance builds/placements and fallbacks
in the F11 overlay. Its logs confirm successful runtime replacement builds and
the first eight preparation failures. Changing this option affects new region
builds; reload the save to rebuild scenery already loaded. Disabling it keeps
existing instances valid until they unload and stops new replacements.

Verification on 2026-10-08: the complete `scripts/verify.ps1 -KenshiDir
'D:\SteamLibrary\steamapps\common\Kenshi'` workflow passed, including the native
overlay regression and existing independent-DLL/GPU/terrain tests. Retained log:
`target/crash-investigation/verification.log`. The final focused native run is
`target/crash-investigation/final-native-check.log`. The fixed host was installed
with a matching SHA-256 and the overlay enabled. A real Kenshi startup reached the
title screen, logged resource readiness and overlay creation, and remained running
past the previous crash point; the user confirmed the debug logs were visible.
