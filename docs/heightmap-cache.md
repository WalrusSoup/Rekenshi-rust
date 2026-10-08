# Heightmap cache

`HeightmapCache` is an independent internal Rust plugin. Its
`performance.heightmap_cache` toggle defaults to false and applies live through
**Options → REKENSHI-RUST**. It serves reads from a read-only
mapping of `data/newland/land/fullmap.tif`, without a converted asset or a second
on-disk cache. Windows loads pages as they are accessed; the implementation does
not preload the entire 512 MiB image.

## Gigacache

**Options → REKENSHI-RUST → Heightmap cache → Gigacache** enables
`performance.heightmap_gigacache` (default false). It copies the complete
uncompressed `fullmap.tif` into an owned buffer before switching reads to it.
The installed file requires about 512 MiB of additional committed memory;
Windows may also retain mapped file pages in its cache and can page out the
owned buffer under memory pressure. This is eager loading, not locked physical RAM.

Gigacache works independently of **Mapped cache**. With both on, Gigacache takes
priority. Turning it off releases the copy after in-flight reads finish and
returns to mapped reads if Mapped cache is on, or native reads otherwise.
Enabling performs synchronous loading and may pause the menu/game; enable it
before traveling. Saved settings preload on plugin startup. Repeated setting
updates reuse the existing copy. Shutdown releases it.

Allocation is fallible and files over 1 GiB are rejected. Preparation failure
leaves the previous read mode intact. The copy is prepared outside the reader
registry lock, so existing native reads can continue during loading. The same
file mapping and write/delete sharing restrictions remain alive with each copy.
No texture, navigation, or live zone state is preloaded. No gameplay performance
gain is claimed without a travel benchmark.

The safe plugin module owns policy. Its private adapter hooks the installed
terrain DLL's TIFF reader beneath `Terrain::getHeight` and `getRawData`.
Consequently, the native code still handles world coordinates, height scaling,
terrain cache hits, reader locking and rectangle clipping. Whole in-bounds
rectangles copy mapped rows; clipped/invalid/oversized requests forward to native
code, preserving its output and return count. No Terrain field layout is used.

The adapter tracks successful native read-only opens by canonical path and checks
reader dimensions, sample count and bit depth. Other TIFFs always bypass, even
if their dimensions match. Tracking remains active while the setting is disabled,
so a reader opened with caching off can use the map when toggled on. Native close
removes the reader association. Mapping references survive in-flight reads;
shutdown disables hooks and releases session ownership. Existing readers after
a full host stop/restart use vanilla reads until reopened.

The mapped file handle denies Windows write/delete sharing for its lifetime,
preventing ordinary writes or truncation while mapped. The parser bounds all
accesses and accepts classic little-endian, unsigned, uncompressed, single-channel
16-bit TIFF strips. It checks offsets, byte counts and row lengths. Unsupported
formats, assets or terrain binaries make the plugin unavailable with a reason.
BigTIFF, tiled/compressed TIFF and RE_Kenshi's converted CIF codec are unsupported.

## Verified native bindings

`Plugin_Terrain_x64.dll` SHA-256:
`53AFD1E4FAF35020556D33F57007D51C2A9D2330B630E39A67161EE850B7A239`.
Private RVAs are used only after that entire binary matches, then its module is
pinned and targets reserved before patching.

| RVA | Verified Windows x64 signature |
| --- | --- |
| `0x20880` | `Open(filename: *const char, mode: i32) -> reader*`; mode 1 is read-only |
| `0x21c40` | `Close(reader*) -> void`; closes the reader's native FILE |
| `0x21640` | `ReadPixel(reader*, x: i32, y: i32, out: void*) -> usize` |
| `0x21800` | `ReadRectangle(reader*, x, y, width, height: i32, out: void*) -> usize` |

Native pixel/rectangle methods seek before every read. Their return values count
samples, not bytes. Inspection of the caller and reader bodies verifies these
signatures and the 40-byte reader's width/height/bits/samples at 8/12/16/20.
The native deleting destructor at `0x143f0` is used only by the integration test
to release its own native allocation. No constructor/destructor is substituted.

## Validation

The ignored native test uses the actual installed 16,385 × 16,385 TIFF and DLL:
1,024 scattered pixel results, whole rectangles and clipped edge rectangles are
compared to original trampolines, including return counts and untouched output.
The comparison runs in both mapped and Gigacache modes. It also verifies reuse,
switching modes with an in-flight owner, copy release, and eager shutdown.
It checks disable, reader close, hook shutdown and restart. The staged host suite
also checks live toggles, a different TIFF with matching dimensions, persisted
settings and startup with unavailable feature slots.

```powershell
$env:REKENSHI_TEST_GAME = 'D:\SteamLibrary\steamapps\common\Kenshi'
cargo test -p rekenshi-heightmap-cache --lib -- --include-ignored
```

This validates correctness on the pinned binary. Loading-time/FPS improvements
have not been benchmarked; the optimization removes native seek/read work for
eligible heightmap accesses. Collider caching and texture loading are separate
plugins and are not implemented by this sampler.
