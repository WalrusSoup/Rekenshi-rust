# Native symbol discovery

Scan the installed binaries and regenerate the catalog:

```powershell
python -m pip install -r scripts/requirements-symbols.txt
python scripts/scan-symbols.py scan --game 'D:\SteamLibrary\steamapps\common\Kenshi'
```

The scanner reads PE files on disk. It never loads the game DLLs, changes the
install, or needs Kenshi to be closed. Python 3.10+ and `pefile` are required;
an existing `target/re-tools` installation is also supported. On Windows,
system DbgHelp provides demangled declarations. `--no-demangle` produces portable
raw-name output. The scanner makes no network requests.

Default inputs are Kenshi's executable, MyGUI, Ogre, ParticleUniverse, and the
optional KenshiLib DLL. It accepts `kenshi_GOG_x64.exe` when `kenshi_x64.exe` is
absent. The current installed build exposes:

| Binary | Named exports | Ordinal-only exports |
| --- | ---: | ---: |
| kenshi_x64.exe | 214 | 0 |
| MyGUIEngine_x64.dll | 3,598 | 0 |
| OgreMain_x64.dll | 8,192 | 1,137 |
| Plugin_ParticleUniverse_x64.dll | 4,503 | 0 |
| KenshiLib.dll | 7,979 | 0 |

These counts identify this local build, not every version of Kenshi. Each scan
records SHA256 fingerprints. KenshiLib's exports are that DLL's wrappers;
their RVAs are not gameplay function addresses in the executable.

## Search and use

```powershell
python scripts/scan-symbols.py query getParentTechnique --module particle_universe
python scripts/scan-symbols.py query UString --module mygui
python scripts/scan-symbols.py query rain --kind all --limit 50
python scripts/scan-symbols.py query PlayerInterface --module kenshi_lib
```

Search is case-insensitive across decorated names, demangled declarations,
and Rust constant paths. `--kind` selects exports (default), imports,
`rtti_candidates`, or all three. `--module` accepts the Rust module name or the
binary filename. `--catalog` reads an alternate JSON inventory.

Outputs:

- `target/native-symbols/catalog.json`: full exports, imports, delay imports,
  RTTI name candidates, PDB references, PE warnings, file fingerprints, and RVAs.
- `target/native-symbols/summary.md`: counts and debug-file references.
- `crates/native/src/symbols/discovered/*.rs`: generated `&CStr` constants for
  every named export, with demangled declarations and code/data/forwarder kinds.

The JSON/report are local build artifacts. The generated Rust catalog is source
that contributors can browse without owning the game. Every module provides
`FILE`, `SCANNED_SHA256`, `EXPORTED_NAMES` and `ORDINAL_ONLY_EXPORT_COUNT`.
Constants have readable names plus a stable hash suffix of the decorated bytes,
so overloads remain distinct and an address change does not rename a constant.
Generated modules are skipped by rustfmt to preserve exact regeneration output.

For example, the original ParticleRenderer export is now also available as:

```rust
use rekenshi_native::symbols::discovered::particle_universe;

let name = particle_universe::PARTICLE_UNIVERSE_PARTICLE_RENDERER_GET_PARENT_TECHNIQUE_AD2348BC7D00;
```

Existing inspected bindings keep their short names under
`symbols::particle_universe::particle_renderer::GET_PARENT_TECHNIQUE`, etc.
Promote a discovered name into those curated modules when implementing and
verifying its adapter. The generated catalog contains no function pointers,
function signatures, runtime addresses, or automatic hook installation.
Unused constants do not require storage in the shipped DLL.

## Repeatability

```powershell
# Compare against the installed build without writing any files.
python scripts/scan-symbols.py scan --game 'D:\SteamLibrary\steamapps\common\Kenshi' --check

# Tool regression tests; no game required.
python scripts/test-symbols.py

# Scan another DLL separately, without replacing the default Rust catalog.
python scripts/scan-symbols.py scan --game 'D:\SteamLibrary\steamapps\common\Kenshi' --module physx=PhysXCore64.dll --output target/physx-symbols --rust-output target/physx-rust
```

`--module rust_name=file` is repeatable and overrides the default input set.
Paths can be relative to `--game` or absolute. Missing required inputs and
invalid PEs fail before outputs are written. KenshiLib is optional only for the
default scan. Outputs omit scan timestamps and installation paths and have stable
ordering and LF newlines. `--check` returns nonzero for missing or changed output
and leaves files untouched. Regeneration uses the same arguments and demangler;
different system DbgHelp versions can produce different declaration text.

## What discovery establishes

An export name can be looked up with GetProcAddress in its owning module.
Some exports are data, forwarded exports, or ordinal-only entries. Resolving
an address still requires inspecting the ABI, ownership, object layout and
supported binary version before calling or detouring it.

The report retains the actual export entry RVA. For executable entries it also
follows bounded E9 linker jump stubs and reports a candidate body RVA separately.
This is a navigation aid, not proof of a hook site. Unsupported stub forms are
left alone; cycles, truncated bytes and jumps out of code do not claim a body.
All RVAs are specific to the scanned binary and its recorded fingerprint.

Kenshi's executable exports mainly audio APIs. Most gameplay methods are absent
from its export table. Imports describe dependency APIs, while RTTI candidates
name types and are not callable function symbols. Debug records point to
build-time PDB paths; they do not supply those PDBs. The current binaries have
no COFF symbol table. This scanner inventories those records but does not
reconstruct stripped internal names or scan an unavailable PDB. Internal hooks
still need reverse engineering and validated version-specific signatures.
