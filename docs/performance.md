# Performance mod

The internal `ShaderCache` plugin implements persistent shader compilation caching
as the first optimization. Its generated menu toggle bypasses/re-enables the
cache live without reinstalling the detour. The compatibility
`rekenshi_shader_cache.dll` uses the same adapter. The implementation studies
[RE_Kenshi's shader cache](https://github.com/BFrizzleFoShizzle/RE_Kenshi/blob/master/ShaderCache.cpp)
and independently implements the documented
[D3DCompile interface](https://learn.microsoft.com/en-us/windows/win32/api/d3dcompiler/nf-d3dcompiler-d3dcompile).
The built-in stack applies it before the graphics service to cache enhancement
shaders too. The size budget remains a startup setting in `shader-cache.toml`;
the old `performance.toml` remains a fallback for existing installations.

Cache keys include a schema tag, compiler/renderer fingerprints, source bytes,
source-name nullness/contents, ordered macro names/values (including null versus
empty), entry point, target profile, flags1/flags2 and include-handler presence.
Records store successful HRESULTs, native bytecode, optional warning blobs,
original compilation duration and ordered include dependency digests. Hit blobs
are allocated by that compiler's D3DCreateBlob and remain owned by native COM.
Compilation failures always come from the original compiler and are not cached.

Only a null include handler or the exactly verified Ogre HLSLIncludeHandler is
eligible. Standard-file sentinel 1 and unfamiliar handlers bypass the cache.
The local renderer's file hash, vtable address, method slots and function-body
hashes must match before an Ogre handler is wrapped. The compiler's actual Open
calls are forwarded with their original arguments/data ownership and recorded;
Close calls return allocations to the original handler. Before using a record,
dependencies are reopened and hashed, then closed with that native handler.
Changed/missing resources trigger ordinary recompilation. This specific handler
ignores include type and ParentData; each hit validation can therefore use
independent Open/Close pairs. These properties were verified by local disassembly
and matched to [Ogre-next 2.0 source](https://github.com/OGRECave/ogre-next/blob/086b31a4bcadf53c37230096e9d3dce953df672f/RenderSystems/Direct3D11/src/OgreD3D11HLSLProgram.cpp).

The handler is read-only, but underlying resource listeners can have side effects;
dependency validation assumes reopening a shader include is acceptable. Unusual
resource-loader mods can disable shader_cache or this DLL. No equivalence between
D3DPreprocess and D3DCompile is assumed. Unknown include handlers keep their
original call ordering and are never wrapped.

Cache files contain SHA-256 integrity checks, bounded TOML metadata and binary
payloads. Reads are capped at 32 MiB; include/source limits cause bypass rather
than unchecked growth. Unique same-directory temporary files are flushed before
atomic replacement. I/O failures fall through to the original compiler. The
size budget defaults to 512 MiB (valid 16..4096): startup trims oldest files and
new writes reserve space atomically. A full cache skips writes. Replacement
accounting is conservative under races and may fill the budget early; restart
recounts actual file sizes. External edits to cache files are detected by checksum,
but concurrently adding unrelated files is outside size accounting.

Logs count hits, misses, bypasses and the sum of original compile durations for
hits. That last number estimates compilation work avoided; it is not a measured
wall-clock improvement. Hashing, include reopening, disk I/O and COM allocation
still cost time. Expect benefit on repeated startup/loading when shaders are
unchanged, not a universal FPS improvement. Shader compilation remains unchanged
when the plugin is disabled.

Current fingerprint gates:

| Module | SHA-256 |
| --- | --- |
| D3DCompiler_43.dll | 44C3A7E330B54A35A9EFA015831392593AA02E7DA1460BE429D17C3644850E8A |
| RenderSystem_Direct3D11_x64.dll | 5F224882DED5B6D3F68F14D6BB4E1762267BBB602B7B9733BD153663D1541BB9 |

Other compiler binaries reject startup; other renderer/include identities bypass
include caching. Private handler RVAs are gated by the full binary fingerprint
and code integrity; no Kenshi executable RVA is assumed. Live 1.0.68 validation
stored real game shader records, including verified native include dependencies.
Native tests compare original compiler results with cache hits, changed macros,
source names/flags, warning blobs, failures, corrupt records, custom include
handlers, standard-file sentinel and a second process reusing the same cache.

The internal [heightmap-cache](heightmap-cache.md) plugin now implements
mapped uncompressed heightmap reads, verified against the installed native TIFF
reader. It defaults off and can be switched live. The other performance plugins are:

| Optimization | Behavior |
| --- | --- |
| Compressed heightmap | Converted assets, codec and verified terrain bindings |
| PhysX collider cache | Cache 128 parsed templates; return an independently owned native clone per hit |
| DDS mip skipping | Read selected DXT mip levels directly at reduced texture quality |
| Custom compressed texture codec | Bounded SIC decoding and optional lossless TGA sidecars |

`OgreDDSCodec2.h` is the interface to the replacement DDS codec; its implementation
and Ogre loading listener belong to `dds-mip-skip`, not `compressed-textures`.
The upstream downsize path seeks past discarded levels, reads the remaining
pixel data in bulk into a native memory stream and rewrites the DDS dimensions
and mip count. It must preserve the game's quality/group rules and delegate
unsupported files to the original loader. Our DDS plugin replaces the verified
game listener's full-file copy with bounded reads of only the selected mip levels.
Power-of-two classic DXT files are eligible; other encodings retain native behavior.
[Codec implementation](https://github.com/BFrizzleFoShizzle/RE_Kenshi/blob/master/OgreDDSCodec2.cpp),
[loading listener](https://github.com/BFrizzleFoShizzle/RE_Kenshi/blob/master/OgreHooks.cpp).

Collider templates are keyed by filename/type and active PhysX SDK. XML, nonbinary
loads, null collections and missing SDKs bypass caching. Each hit calls the native
scale-copy operation at unit scale. LRU eviction, disabling and shutdown release
templates with NXU's native destructor and allocator. Restart the game after changing
collider assets on disk. This cache bounds entry count, not total descriptor memory.

The SIC codec supports BGR, BGRA, R8 and little-endian R16, all seven upstream
filters and both channel layouts. Decoded pixels use Ogre's allocator because
`Image::load` adopts them. Malformed input reports a native Ogre exception.
Files are bounded to 256 MiB of decoded data; unfamiliar stream classes are rejected.
The reserved header byte is ignored for compatibility with legacy writers.

SIC assets work while the pack is mounted. The menu's compressed-textures checkbox
controls automatic TGA sidecar selection. Create a sidecar without replacing the source:

```powershell
& 'D:\SteamLibrary\steamapps\common\Kenshi\RE_Kenshi_Rust\texture-pack.exe' 'PATH\texture.tga' --sidecar
```

The converter verifies the decoded pixels and writes `texture.tga.sic` plus a source
size/mtime manifest. It refuses existing outputs and sidecars larger than their source.
The archive adapter uses valid sidecars for new read-only TGA loads and falls back
to the ordinary loader when absent, malformed or stale. Size/mtime validation does
not detect content replacement that deliberately preserves both. DDS assets are
left intact, including their mip chains. No game textures are converted by installation.

`scripts/test-performance.py` maps a staged executable without invoking its entry
point. It binds imports, constructs a headless Ogre root, compares DDS bytes to the
native listener at five quality levels, tests 56 SIC combinations produced by upstream
code, verifies Image buffer adoption and foreign exception cleanup, and exercises
real PhysX collection copies and mutation isolation. Its GPL reference oracle is
test-only and is never linked into the distributed pack. The harness requires
`pefile` in `target/re-tools`, Visual Studio C++ tools and the upstream checkout
at `target/references/RE_Kenshi`. Performance gains still need timing in real saves;
these checks establish behavior and ownership, not an FPS increase.

`MiscHooks.cpp` also contains features outside the implemented caches:

| Feature | Upstream behavior | Rust pack status |
| --- | --- | --- |
| Cursor visibility fix | Suppresses repeated cursor visibility calls to prevent Windows' display counter drifting | Verified 1.0.68 game call already checks visibility changes; no extra hook added |
| Splash skipping | Advances logo timers | Not implemented |
| RNG fix | Changes random behavior while retaining seeded building/foliage generation paths | Not implemented |
| Launcher mod-list scroll fix | Preserves list scroll/caret across refresh | Not implemented |

The upstream cursor fix hooks `win32u!NtUserShowCursor`, forwarding only when a
stored boolean changes and returning 0/-1 for visible/hidden. Its comment describes
repeated calls accumulating Windows' cursor display counter; it supplies no measured
10,000-call count. The 10,000 bound in `Bugs.cpp` is an emergency-save filename search.
[Upstream cursor hook](https://github.com/BFrizzleFoShizzle/RE_Kenshi/blob/c8dfbc9621b9263863722852f819c4c0f440475a/MiscHooks.cpp#L210-L232),
[Windows counter contract](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-showcursor).

In the fingerprinted root Steam 1.0.68 executable, the direct ShowCursor call is at
RVA `0x82b76e`. At `0x82b763` the caller compares requested visibility with its stored
byte, skips the call when equal, and stores the new value at `0x82b774`. The other
ShowCursor IAT reference is an import thunk at `0xa29af0`; no direct calls, jumps or
absolute pointers to it were found. The root DLL import audit found ShowCursor only
in the executable and dormant RE_Kenshi DLL. This is static evidence for the guarded
game path, not a runtime call-frequency measurement or an audit of future plugins
using dynamically resolved APIs. Additional global deduplication has not been installed.
| Free-camera focus handling | Disables free camera after losing focus and adds its key binding | Not implemented |
| Camera distance | Raises the distance limit | Not implemented |

These are fixes or conveniences, rather than demonstrated steady-state FPS
improvements. The cursor fix does avoid redundant OS calls, but has no benchmark
here. The current mod pack is not a complete RE_Kenshi replacement in feature
coverage. [Miscellaneous hooks](https://github.com/BFrizzleFoShizzle/RE_Kenshi/blob/master/MiscHooks.cpp).

See [the source research](research/performance.md) for concrete upstream algorithms
and compatibility evidence. Collider/mip/texture feature slots appear as
unavailable controls with reasons; they are not working optimizations yet.
