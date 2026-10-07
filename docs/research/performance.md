# RE_Kenshi performance research

Research date: 2026-10-06. Source snapshot: [`c8dfbc9621b9263863722852f819c4c0f440475a`](https://github.com/BFrizzleFoShizzle/RE_Kenshi/tree/c8dfbc9621b9263863722852f819c4c0f440475a). The reference checkout is under `target/references/RE_Kenshi`; production code was not changed by this research.

## Recommendation

Implement a separately configurable **persistent shader compilation cache** first. RE_Kenshi already applies this optimization through `D3DCompiler_43.dll!D3DCompile`. This target exposes the documented Direct3D C ABI and does not depend on Kenshi executable RVAs or Visual C++ 2010 container layouts. Cache hits reconstruct native blobs through `D3DCreateBlob`; misses call the original compiler. This saves repeated compilation during startup and material loading. It does not imply higher steady-state FPS. [RE_Kenshi implementation](https://github.com/BFrizzleFoShizzle/RE_Kenshi/blob/c8dfbc9621b9263863722852f819c4c0f440475a/ShaderCache.cpp), [D3DCompile documentation](https://learn.microsoft.com/en-us/windows/win32/api/d3dcompiler/nf-d3dcompiler-d3dcompile), [D3DCreateBlob documentation](https://learn.microsoft.com/en-us/windows/win32/api/d3dcompiler/nf-d3dcompiler-d3dcreateblob).

## What upstream actually optimizes

| Feature | Mechanism and target | Rust implementation implications |
|---|---|---|
| Shader cache | Keys shader source, ordered defines, entry point, target and both flag words; persists bytecode; records include hashes during compilation and reopens includes to invalidate hits. Hooks `D3DCompile`. | Best first feature. Include correctness, warning preservation, compiler identity, corruption handling and concurrency need explicit treatment. [Source](https://github.com/BFrizzleFoShizzle/RE_Kenshi/blob/c8dfbc9621b9263863722852f819c4c0f440475a/ShaderCache.cpp). |
| Fast uncompressed heightmap | Maps `fullmap.tif` read-only and directly samples native 16-bit pixels; replaces `Terrain::getHeight` and `Terrain::getRawData`. | Potential terrain streaming benefit, but requires verified object offsets, TIFF layout, clamping, coordinate conversion and bit-for-bit comparison with vanilla. [Source](https://github.com/BFrizzleFoShizzle/RE_Kenshi/blob/c8dfbc9621b9263863722852f819c4c0f440475a/HeightmapHook.cpp). |
| Compressed heightmap | Uses `CompressToolsLib` to access a preconverted `fullmap.cif`, with streaming or preload modes. Replaces the same terrain methods; upstream recommends compression on HDD and the mapped TIFF on other drives. | Requires a conversion tool, codec compatibility and separate license review; not a small DLL-only feature. [Source](https://github.com/BFrizzleFoShizzle/RE_Kenshi/blob/c8dfbc9621b9263863722852f819c4c0f440475a/HeightmapHook.cpp). |
| PhysX collider cache | Caches collections keyed by filename and type, and returns scale-one copies instead of reloading; explicitly excludes XML. | Requires private game functions and the native SDK/global physics object. The comment giving load RVA `0x7E4850` is not a verified 1.0.68 address. [Source](https://github.com/BFrizzleFoShizzle/RE_Kenshi/blob/c8dfbc9621b9263863722852f819c4c0f440475a/PhysicsHooks.cpp). |
| Skip unused DDS mip reads | Wraps Ogre's loading listener and seeks past discarded mip levels rather than reading the complete DDS before downsizing. Preserves resource-group and `_LO`/`_HI` rules. | Only applies below maximum texture quality; requires native Ogre listener and stream ownership. This is an I/O optimization preserving the selected quality, not a general texture-quality reduction. [Listener source](https://github.com/BFrizzleFoShizzle/RE_Kenshi/blob/c8dfbc9621b9263863722852f819c4c0f440475a/OgreHooks.cpp), [DDS codec](https://github.com/BFrizzleFoShizzle/RE_Kenshi/blob/c8dfbc9621b9263863722852f819c4c0f440475a/OgreDDSCodec2.cpp). |
| SIC texture codec | Registers an extra Ogre codec for `.sic` assets, decoding a custom compressed representation. | Requires converted assets and a codec, not a mesh optimizer or automatic optimization of existing DDS files. [Codec source](https://github.com/BFrizzleFoShizzle/RE_Kenshi/blob/c8dfbc9621b9263863722852f819c4c0f440475a/OgreSICCodec.cpp), [format implementation](https://github.com/BFrizzleFoShizzle/RE_Kenshi/blob/c8dfbc9621b9263863722852f819c4c0f440475a/SIC.cpp). |

No general mesh optimization algorithm was found in the inspected upstream core files. Skipping splash screens is a startup convenience; changing camera distance or game speed is gameplay behavior, not an algorithmic optimization. Avoid advertising these as measured performance improvements. [Miscellaneous hooks](https://github.com/BFrizzleFoShizzle/RE_Kenshi/blob/c8dfbc9621b9263863722852f819c4c0f440475a/MiscHooks.cpp).

## Verified local binary targets

Evidence collected read-only from `D:\SteamLibrary\steamapps\common\Kenshi` with `Get-FileHash -Algorithm SHA256`, VS 2022 `dumpbin /exports`, `/headers`, and PE section inspection. RVAs are corroboration only: resolve named exports at runtime.

| Binary | SHA-256 |
|---|---|
| `D3DCompiler_43.dll` | `44C3A7E330B54A35A9EFA015831392593AA02E7DA1460BE429D17C3644850E8A` |
| Root `kenshi_x64.exe` | `A596AB4E407C67B58599C54FFB32DC1BF2B64510CDEBD3FA9359EF05A576AEB1` |
| `RE_Kenshi/kenshi_x64.exe` | `504B362CDE850D56AFB1CEA6F5B7B0EE014D9DD7B47E188599D91C804502CD3E` |
| `Plugin_Terrain_x64.dll` | `53AFD1E4FAF35020556D33F57007D51C2A9D2330B630E39A67161EE850B7A239` |

Compiler PE timestamp: `0x4BF70BEB`; image size: `0x26F000`; preferred base: `0x180000000`.

| Compiler export | Local RVA | ABI |
|---|---:|---|
| `D3DCompile` | `0x6FFD0` | HRESULT; source pointer/usize, source-name pointer, macro-array pointer, include pointer, entry-point pointer, target pointer, two u32 flag words, code/error blob output pointers. |
| `D3DCreateBlob` | `0x6FFB0` | HRESULT; usize length and blob output pointer. |
| `D3DPreprocess` | `0x71630` | HRESULT; source pointer/usize, source-name pointer, macros, include pointer, expanded-source/error blob output pointers. |

Use Windows `extern "system"` calling conventions. Local `D3DCompile` starts with `48 89 5C 24 10 55 56 57 41 54 41 55 41 56 41 57`; the initial detour span consists of stack saves/pushes rather than calls.

Terrain exports were also verified: `?getHeight@Terrain@@IEAAMAEBVVector3@Ogre@@H@Z` at RVA `0x14430` and `?getRawData@Terrain@@QEBA_KHHHHPEAD@Z` at `0x135B0`. Upstream's object offsets `bounds=0x1B8`, `heightScale=0x1D0`, texture offset `0x1D4`, texture scale `0x1D8` are source assertions, **not independently verified against this DLL**. Do not enable a sampler replacement using those offsets without disassembly and differential tests.

The local TIFF is 536,980,180 bytes; upstream expects a 16,385 × 16,385 16-bit image. Local `settings.cfg` has `texture resolution gimping=0`, so the mip-skipping optimization would currently do no useful work.

Upstream supports 1.0.68 by creating a compatible executable based on 1.0.65; do not assume upstream KenshiLib RVAs map to the root 1.0.68 executable. Export-based compiler hooks avoid this dependence. [Upstream installation documentation](https://github.com/BFrizzleFoShizzle/RE_Kenshi/blob/c8dfbc9621b9263863722852f819c4c0f440475a/README.md#installation).

## Cache correctness requirements

Independently design the cache using the published API. Include compiler SHA-256 and cache format version; length-prefix source, source-name, ordered macro names/definitions, entry point, target and both flags. Preserve null versus empty optional arguments. Successful results can include a diagnostics blob; cache the result code, code bytes and diagnostics, and recreate native blobs on hits. Preserve misses/errors with an unchanged original call. Respect COM ownership and release all temporary blobs. [API contracts](https://learn.microsoft.com/en-us/windows/win32/api/d3dcompiler/nf-d3dcompiler-d3dcompile).

Includes need content invalidation. `D3DPreprocess` can expand source using the actual include handler and macros before hashing. This avoids inferring include paths and naturally processes nesting; hash expanded text alongside the original inputs. Preprocessing failure should bypass caching. This route rereads includes and performs an extra preprocessing pass on misses, so benchmark the cost and verify that handlers tolerate repeat reads. Preserve original compilation inputs on misses. [D3DPreprocess](https://learn.microsoft.com/en-us/windows/win32/api/d3dcompiler/nf-d3dcompiler-d3dpreprocess).

An alternative is recording/replaying a full include tree with parent relationships. Do not copy upstream's assumption that `pParentData` can always be replaced by null: it explicitly describes an Ogre-specific handler, whereas the public include API allows application-defined behavior. Treat include pointer `1` as the standard file handler sentinel, never a vtable pointer. Initially bypass unknown/stateful handlers if their semantics cannot be verified. [Include interface](https://learn.microsoft.com/en-us/windows/win32/api/d3dcommon/nn-d3dcommon-id3dinclude), [include parameter and sentinel](https://learn.microsoft.com/en-us/windows/win32/api/d3dcompiler/nf-d3dcompiler-d3dcompile).

Use bounded records, checksums, atomic file replacement and fallback on corrupt/unwritable cache. Keep cache locks out of original compilation and include callbacks; handle recursion and concurrent identical compiles. Store counters for hits, misses, bypasses, compilation time and cache-read time. These are implementation recommendations, not claims about upstream.

## Validation and performance evidence

1. Compile representative vertex/pixel shaders with the real installed compiler, then compare native output with the first cached call and cache-hit call byte-for-byte, including diagnostics/HRESULT. Restart the test process and verify disk hits.
2. Change source, defines, entry point, target, flags, included file contents and nested includes independently; each must invalidate. Exercise null optionals, standard include sentinel, custom includes, warnings, invalid HLSL and missing includes.
3. Truncate/corrupt cache files, deny cache writes and run concurrent compilation requests; each must return native compiler behavior without crashing or deadlocking.
4. Launch Kenshi twice with cache enabled and verify material rendering and hits. Compare repeated title-screen and identical-save loading times with plugin disabled versus warm cache, controlling filesystem cache and graphics settings. Report measured loading/compile time reductions, not an FPS claim.

## License

RE_Kenshi identifies its core as GPLv3; its `LICENSE` contains GPL version 3. Some bundled files, such as the Ogre DDS codec, have separate notices. Retain the research reference checkout only as reference; independently implement the documented Direct3D API cache instead of copying upstream source into this project's MIT implementation. Any actual upstream code incorporation requires explicit licensing treatment. [README license](https://github.com/BFrizzleFoShizzle/RE_Kenshi/blob/c8dfbc9621b9263863722852f819c4c0f440475a/README.md#license), [LICENSE](https://github.com/BFrizzleFoShizzle/RE_Kenshi/blob/c8dfbc9621b9263863722852f819c4c0f440475a/LICENSE), [Ogre codec notice](https://github.com/BFrizzleFoShizzle/RE_Kenshi/blob/c8dfbc9621b9263863722852f819c4c0f440475a/OgreDDSCodec2.cpp).

## Follow-up: verified Ogre include identity and replay

The local game log identifies Ogre as `2.0.0unstable (Tindalos)`. The matching source family is Ogre-next's `v2-0` branch, inspected at commit `086b31a4bcadf53c37230096e9d3dce953df672f`. Its `Ogre::HLSLIncludeHandler : ID3DInclude` stores only the source-program pointer. `Open` resolves names through the program's resource group with cross-group search enabled, reads the stream as text, allocates `new char[length]`, copies exactly that many bytes and returns them. It ignores include type and parent-data arguments. `Close` deletes that array. This matches the installed DLL's disassembly. [Matching source](https://github.com/OGRECave/ogre-next/blob/086b31a4bcadf53c37230096e9d3dce953df672f/RenderSystems/Direct3D11/src/OgreD3D11HLSLProgram.cpp).

The native shader resource search is not equivalent to resolving relative filesystem paths from `pSourceName`. Use the original include handler to resolve dependencies. The handler itself writes no mutable fields, but Ogre resource-loading listeners or modded archives can introduce their own behavior; the verified identity is not a blanket guarantee for arbitrary third-party resource callbacks.

### Local identity evidence

`RenderSystem_Direct3D11_x64.dll` SHA-256:
`5F224882DED5B6D3F68F14D6BB4E1762267BBB602B7B9733BD153663D1541BB9`.
Timestamp: `0x5CA5F9E4`; image size: `0xBF000`. The only exports are `dllStartPlugin` and `dllStopPlugin`; the include methods are private functions.

MSVC RTTI provides independent identification:

- Type descriptor at RVA `0xAF940`, decorated name `.?AVHLSLIncludeHandler@Ogre@@`.
- Complete-object locator at RVA `0x84B88`: signature `1`, object offset `0`, constructor displacement `0`, type RVA `0xAF940`, class-hierarchy RVA `0x84BB0`, self RVA `0x84B88`.
- Vtable at RVA `0x78808`: first slot `Open` at RVA `0x9E80`, second slot `Close` at RVA `0x9FF0`. The next adjacent pointer is not an additional include method.

Binary disassembly corroborates semantics: `Open` loads `mProgram` from `this+8`, obtains its resource group through virtual slot byte offset `0x110`, calls imported `ResourceGroupManager::getSingleton` through IAT RVA `0x728B8`, then imported `openResource` through IAT `0x728B0` with search-groups `true` and the same program pointer. It invokes stream `getAsString`, allocates and copies bytes. `Close` calls `MSVCR100.dll!operator delete[]` through IAT `0x713D0`, then returns `S_OK`.

Optional exact function-body integrity checks, excluding trailing padding:

| Function | RVA range (end exclusive) | SHA-256 of raw instruction bytes |
|---|---|---|
| Open | `0x9E80..0x9FE3` (355 bytes) | `2D8E79019EA292069853A52037948FAE83DCD73F118DC703FBD64D00AE3B0039` |
| Close | `0x9FF0..0xA004` (20 bytes) | `9725407485E00A61D2F86433B7CB12DDF90C4F009325EE6D38EE6980174BA203` |

These are verified local values, not offsets for other renderer builds. Check the renderer module's full fingerprint, that the input handler vtable equals that module's `base+0x78808`, both slot identities, and unchanged function bodies before selecting the Ogre replay path. Bypass other handlers, including the standard-file sentinel `1`, unless separately implemented and tested. Do not merely accept any function pointer somewhere inside the renderer module.

### Recommended dependency path

Prefer **recording the include events from actual original compilation** over the provisional preprocessing-only approach described earlier. This preserves the compiler's actual preprocessing environment and does not depend on equivalence between `D3DPreprocess` and target-specific compilation.

On a miss, pass a wrapper whose `Open`/`Close` forward to the original handler using its original `this` pointer and original arguments. Record include type, name, byte digest/length, parent identity and the complete Open/Close event order. Assign identifiers to opened buffers; encode a parent as null, original source, or a still-open include identifier. Mark unsupported parent pointers, failed calls, unmatched closes, exceeded bounds or incomplete graphs uncacheable. Do not retain native pointers after the compile returns. The documented include buffer lifetime lasts until `Close`; successful opens are closed before the compiler API returns. [Open contract](https://learn.microsoft.com/en-us/windows/win32/api/d3dcommon/nf-d3dcommon-id3dinclude-open), [Close contract](https://learn.microsoft.com/en-us/windows/win32/api/d3dcommon/nf-d3dcommon-id3dinclude-close).

On a candidate hit, replay the event graph against the **current** recognized handler and current original source pointer. Resolve parent IDs to the newly opened buffers, compare current content length/digest to each recorded dependency, and issue corresponding closes in recorded order. A changed ancestor or dependency invalidates the result; close every buffer already opened before falling back to original compilation. Call original `Close` exactly once for each successful replay `Open`, including failure/early-return cleanup. Do not hold a shared cache mutex while invoking handler methods.

The compiled key still includes original source bytes, optional source name, ordered defines, entry point, target/profile, both flags, compiler fingerprint and cache version. Native warnings and HRESULT remain part of the stored result. Replaying a graph captured from an actual successful compile means profile-specific include branches are captured under the same full compile key.

### Target macro check

`D3DPreprocess` has no entry-point, target-profile or compile-flags parameters, while `D3DCompile` does. Consequently, a preprocess-only key should not assume every compiler version has identical implicit macro/conditional behavior. [Preprocess signature](https://learn.microsoft.com/en-us/windows/win32/api/d3dcompiler/nf-d3dcompiler-d3dpreprocess), [compile signature](https://learn.microsoft.com/en-us/windows/win32/api/d3dcompiler/nf-d3dcompiler-d3dcompile).

Direct probes against the installed compiler tested `#ifdef` plus `#error` for `__SHADER_TARGET_MAJOR`, `__SHADER_TARGET_MINOR`, `__SHADER_TARGET_STAGE`, `__HLSL_VERSION` and `__cplusplus`, with a valid `ps_5_0` shader. Both native `D3DCompile` and `D3DPreprocess` returned `S_OK` with no diagnostics for all five probes: these particular macros are not defined in this DLL under that invocation. This is evidence against claiming a demonstrated target-macro discrepancy here, not proof of equivalence for every profile/flag/compiler. Actual-compile dependency recording avoids relying on that broader assertion.


## Implemented Steam 1.0.68 adapters

The root executable hash is `A596AB4E407C67B58599C54FFB32DC1BF2B64510CDEBD3FA9359EF05A576AEB1`.
Its private bindings are:

| Operation | Root EXE RVA |
| --- | --- |
| loadPhysXResource | `0x7e53f0` |
| NXU scaleCopyCollection | `0x20b860` |
| NXU releaseCollection | `0x1d99f0` |
| DDSListener resourceStreamOpened | `0x82ddc0` |
| Live texture-quality integer | `0x213352c` |

The installed KenshiLib export slots 5999 and 6000 identify the old 1.0.65 load
and clone functions as `0x7e4850` and `0x20b820`. Masked relocation-independent
instruction matching finds the root functions uniquely; disassembly corroborates
collection construction, clone parameters and the engine's native release call.
Release destroys descriptors before calling the executable allocator. The adapter
gets the SDK from the named `PhysXLoader64!NxGetPhysicsSDK` export.

The DDS listener's root vtable is `0x171b3a0`; its linker thunk `0x12346` jumps
to `0x82ddc0`. The original listener reads the entire stream into memory before
calling its downsize helper at `0x82d840`. The new adapter reads the header and
selected levels directly. Native differential tests confirm identical output for
quality levels 0 through 4 on the DXT fixture. GUI, Overlaymaps, PLSM2 and empty
groups bypass optimization; Landscape and _LO./_HI. rules are retained.

Ogre ABI verification uses full DLL SHA-256
`7D3888D21398647519F10A89B1F300D8606F761AD159E6021A797B1B9B3DF5F6`.
VC100 std::string is 32 bytes, SharedPtr is 16 bytes, SharedPtrInfo is 48 bytes,
with an atomic reference count at 0x20. MemoryDataStream is 0x60 bytes.
ImageData is 48 bytes, storing height/width/depth at 8/12/16, size at 24,
mipmap count at 32, flags at 36 and pixel format at 40. Native Image::load
adopts the decoded stream buffer, requiring an Ogre allocation for decoded pixels.
Native shared-reference copying, image adoption/destruction and exception unwind
are covered by the headless integration harness. These layouts are intentionally
confined to the private adapter, rather than exposed as a general C++ interop API.
