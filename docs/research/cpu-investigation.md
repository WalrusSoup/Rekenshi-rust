# Kenshi CPU investigation: initial evidence

Date: 2026-10-07. Read-only reconnaissance; no gameplay trace or new optimization has been run.

## Questions and status

| Question | Status | Evidence and limits |
| --- | --- | --- |
| Does OGRE support LOD? | Answered: yes | Official OGRE mesh LOD documentation describes generated and manual LODs. This is general engine documentation, not proof of Kenshi's runtime configuration. |
| Does the installed Kenshi renderer contain LOD interfaces? | Answered: yes | Direct PE export inspection of `D:/SteamLibrary/steamapps/common/Kenshi/OgreMain_x64.dll` found `Mesh::getNumLodLevels` at RVA `0x1A72A0`, `Technique::getLodIndex` at `0xBF340`, and two `Material::getNumLodLevels` overloads at `0x156FA0` and `0x156F20`. `OgreMeshLodGenerator_x64.dll` is also present. Presence does not prove runtime use. |
| Do important Kenshi assets have effective LODs? | Unresolved | Need mesh/resource inventory and observation of selected LODs at representative camera distances. No asset coverage claim is made. |
| Where are CPU cycles being wasted? | Unresolved | Need sampled CPU stacks in a representative save. Export names and disassembly cannot establish time spent or call frequency. |
| Can the local tools support the investigation? | Partially answered | WPR and WPA are installed. WPR lists CPU, GPU, DiskIO, and FileIO profiles; CPU profile details include sampled and context-switch stacks. Ghidra 12.1.4 exists at the user-supplied location. OpenJDK 23.0.1 runs from `C:/Users/Jayson/.jdks/openjdk-23.0.1/bin/java.exe`. Headless analysis has not been launched. |

Inspected OgreMain SHA-256:
`7D3888D21398647519F10A89B1F300D8606F761AD159E6021A797B1B9B3DF5F6`.

## Existing work and candidate boundaries

The current [performance documentation](../performance.md) already describes shader,
heightmap, collider, DDS mip, and SIC optimizations. Their presence does not establish
measured gains in a particular save, and they should not be proposed as new features.

The following are hypotheses, not findings of inefficient implementation:

| Candidate | Local discovery evidence | Evidence needed before interception |
| --- | --- | --- |
| Scene traversal and visibility work | `Ogre::SceneManager::_cullPhase01` in `crates/native/src/symbols/discovered/ogre.rs` | Sampled self/inclusive CPU time, object counts, repeated work across camera/shadow passes, and exact visibility semantics. |
| Animation work for distant objects | `Ogre::Entity::_updateAnimation` and `Ogre::InstanceManager::_updateAnimations` in the same inventory | Cost versus visible/animated entity count; determine whether pose updates affect gameplay, attachment transforms, or only rendering. |
| Render preparation and submission | `Ogre::SceneManager::_renderPhase02` and `Ogre::Root::renderOneFrame` in the same inventory | CPU stacks, draw counts, GPU timing, and material/pass distribution. Lower triangle counts alone do not imply fewer draw calls. |
| Zone loading stalls | `ZoneManager::processLoading`, `loadPhase1/2/3`, and `updateMainThread` in `crates/native/src/symbols/discovered/kenshi_lib.rs` | Distinguish CPU execution, disk reads, and thread waits during a fixed travel route; trace phase implementations before assigning work to individual phases. |

Discovery inventories identify possible boundaries. They do not by themselves
prove a callable runtime address, ABI, or active use in the installed executable.

## Tooling limits

REA `binary_session` reported no open target. Its Ghidra provider reports
`GHIDRA_INSTALL_DIR` unset and Java unavailable in the server environment.
The separately located Java runtime exists but has not been configured for REA.
REA also reports that its Windows Ghidra P0 adapter does not admit DLL targets;
OGRE DLL work may require direct Ghidra headless analysis or the repository's
existing PE/Capstone tools. This is an adapter limitation, not a Ghidra inability
to analyze DLLs. No global configuration was changed and no provider restriction
was bypassed.

No Kenshi process was found by the process-name check during reconnaissance.
No trace was recorded, game started, or installed game file changed.

## First experiment

1. Use one representative save, fixed camera and settings. Record enabled plugins
   and binary identities. Separate stationary rendering, large-group simulation,
   and travel scenarios instead of combining them into one average.
2. Capture a short WPR CPU/GPU trace (and disk/file activity for travel). WPR is
   system-wide: filter the analysis to the actual Kenshi PID and retain traces locally.
3. Rank CPU stacks and waits, map hot addresses to module-relative offsets, then
   inspect only the costly functions in Ghidra. Inspect asset LOD coverage if
   geometry/rendering measurements make that a promising direction.
4. Implement one behavior-preserving, independently switchable optimization;
   compare original/replacement outputs and native ownership before timing it.
5. Repeat equivalent baseline/treatment runs. Report frame-time distributions,
   CPU cost and loading-pause duration as appropriate, plus visual/gameplay checks.

Rust interception must preserve the Windows x64 ABI, native allocation ownership,
and game-thread constraints. Keep native translation separate from the core
algorithm (`M-FFI-TRANSLATES`); benchmark the hot path (`M-HOTPATH`), justify unsafe
operations (`M-UNSAFE`), and avoid turning diagnosis into per-call logging overhead
(`M-LOG-OVERHEAD`). Use `scripts/verify.ps1` for implementation validation. This
document is not a workspace guideline audit.

## Sources

- [OGRE mesh LOD documentation](https://ogrecave.github.io/ogre/api/latest/meshlod-generator.html)
- [Microsoft WPR command-line options](https://learn.microsoft.com/en-us/windows-hardware/test/wpt/wpr-command-line-options)
- Local PE exports, discovered symbol inventory, tool output, and REA session status as described above.

## Installed terrain storage inventory

Read-only filesystem inventory on 2026-10-07 under
`D:/SteamLibrary/steamapps/common/Kenshi/data/newland`. Sizes are on-disk bytes,
not decoded runtime allocations or a measured per-zone working set.

| Files | Count | Total MiB | Median KiB | Largest KiB |
| --- | ---: | ---: | ---: | ---: |
| `land/fullmap.tif` | 1 | 512.104 | 524394.71 | 524394.71 |
| `land/navtiles/*.hkt` | 3995 | 502.616 | 113.73 | 760.29 |
| `leveldata/**/*.zone` | 672 | 25.035 | 10.35 | 490.32 |
| `land/**/*.dds` | 363 | 1509.552 | 5461.48 | 5461.48 |

The sampled navigation file header identifies Havok `hk_2014.2.0-r1`.
Zone files span the root, Dialogue, Newwworld, and rebirth directories; these
counts are not 672 independent active world chunks. The terrain DDS inventory
includes a Testers directory and copy-named assets. No reachability analysis was
performed, so totals include potentially unused files.

The complete `newland/land` directory is 3298.101 MiB, including PNG maps,
the 153.160 MiB alternate `fullmap.cif`, and miscellaneous files. It is not the
size of one chunk or a required simultaneous allocation. Shared assets also
live outside that directory. Active mods, workshop assets, and save-specific
zone overlays were not inventoried.

The heightmap plus all navigation tile files sum to about 1014.72 MiB on disk.
This makes bounded byte prefetching a plausible experiment, not a demonstrated
optimization. The existing heightmap-cache plugin maps the TIFF on demand and
does not proactively read the entire image (see `docs/heightmap-cache.md`).
Reading ahead cannot by itself remove parsing, native object construction,
GPU upload, or synchronization costs. Windows provides
[PrefetchVirtualMemory](https://learn.microsoft.com/en-us/windows/win32/api/memoryapi/nf-memoryapi-prefetchvirtualmemory)
for mapped ranges; it is a best-effort hint, not a residency guarantee.

Suggested first comparison: the same travel route with the current configuration
versus deliberate heightmap prefetch, recording file/page-fault waits and pause
duration. Do not warm all assets before collecting the initial baseline.
Then consider a bounded upcoming-zone navigation/asset prefetcher if traces show
remaining I/O waits. Native zone activation would require separate lifecycle,
thread, save-state, and population-spawn analysis.
