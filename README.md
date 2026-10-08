# RE_Kenshi Rust

Experimental Windows x64 Ogre plugin using [`detour`](https://github.com/darfink/detour-rs).
The core DLL supplies built-in performance optimizations, an in-game settings
menu, managed hooks and a shared graphics backend. **Optional mods are independent
DLLs** in `Kenshi/RE_Kenshi_Rust/plugins`; players can mix them without rebuilding
the host.

For mod authors, start with the [Rust SDK quickstart](docs/plugin-sdk.md):

```powershell
cargo install --path tools/cargo-rekenshi --locked
cargo rekenshi new plugin my-mod
cargo rekenshi new plugin my-look --kind graphics
```

The generator creates and builds an independent project under `mods-src/`.
Implement `Plugin`, declare controls, and add `export_plugin!(YourMod)`.
The SDK handles DLL exports, callback transport, persistence and cleanup.
Supported hooks accept closures through `context.host.hook_ogre_calls(move || { ... })?`
and `context.host.hook_ui_frame(move |delta| { ... })?`. Graphics mods own a safe
`PostProcess` handle. Adding a DLL mod requires no host registry or MyGUI changes.

The [core contributor guide](docs/internal-plugins.md) covers bundled adapters;
the [performance inventory](docs/performance.md) records supported builds and
remaining RE_Kenshi work. The [sample catalog](samples/README.md) separates
implemented examples from researched ecosystem references.

The proposed [modding platform specification](docs/plugin-platform-spec.md) and
[example Rust interface](docs/plugin-platform-api.md) describe the future extension
system, including built-in/external plugin parity. These are design documents;
their proposed interfaces are not available in the current SDK.
The [implementation record](docs/plugin-platform-implementation.md) describes the
public operation broker, generation notifications and compiler-source migration
now implemented, with an [independent DLL example](samples/source-transform/README.md).
The remaining native migrations and lifecycle evidence are listed explicitly.
The [tool decision guide](docs/modding-tool-boundary.md) explains when to use FCS,
existing content tools, Rust plugins, or a combination.

## Build and verify

Requires Windows x64, stable Rust 1.85+ and the MSVC linker/Windows SDK.

```powershell
cargo build --release
.\scripts\verify.ps1 -KenshiDir 'D:\SteamLibrary\steamapps\common\Kenshi'
```

The default build produces the core and the separate Kenshi 2 sample DLL.
Verification also builds all workspace samples and compatibility fixtures.
It runs formatting, Clippy, workspace tests, installer tests, real Ogre/MyGUI
forwarding, independent DLL lifecycle/settings tests, native cache checks and
GPU pixel/state checks. Fixtures use staged folders and do not modify game
files. Visual behavior still needs an in-game world check.

Individual bootstrap checks:

```powershell
.\target\release\plugin-smoke.exe .\target\release\RE_Kenshi_Rust.dll
.\target\release\plugin-smoke.exe .\target\release\RE_Kenshi_Rust.dll 'D:\SteamLibrary\steamapps\common\Kenshi'
```

## Install and restore

Close Kenshi, then run:

```powershell
.\scripts\install.ps1 -KenshiDir 'D:\SteamLibrary\steamapps\common\Kenshi' -WhatIf
.\scripts\install.ps1 -KenshiDir 'D:\SteamLibrary\steamapps\common\Kenshi' -IncludeKenshi2Sample
```

Without `-IncludeKenshi2Sample`, installation ships the core only. The installer
backs up `Plugins_x64.cfg` under `RE_Kenshi_Rust.bootstrap-backup`, replaces the
active `RE_Kenshi` entry with `RE_Kenshi_Rust` before renderer plugins, and
preserves other engine entries and old files. The original mod's functionality
is absent while it is disabled; this implementation does not promise complete
parity. FCS/data mods remain managed by Kenshi.

Start Kenshi normally. `RE_Kenshi_Rust.log` beside the host records startup,
hook activation and individual mod failures. The title-screen version caption
gets a `RE_Kenshi Rust 0.4.0 loaded | N plugins` indicator. Open
**Options -> REKENSHI-RUST** for built-in and loaded-mod controls. Controls apply
live; built-in choices live in `modpack.toml`, DLL-mod choices in
`settings/<plugin-id>.toml`. Upgrades preserve existing choices.

Enable **Developer diagnostics / Debug overlay** on the same Options tab to inspect
recent host/plugin logs, active plugins, cache counters and public operation
registrations/generations while playing. **Freeze debug overlay** holds the current
snapshot. Both controls apply live and persist; the overlay is off by default and
passes mouse input through. See [debug overlay](docs/debug-overlay.md) for details.

**Dark game UI** darkens Kenshi's original HUD, window frames and panel controls
while retaining their worn metal artwork. Body text becomes light; status colours,
portraits, item art and maps retain their meaning. Its checkbox and **Frame
brightness** setting apply live. It uses native colours and leaves game assets
unchanged; unknown third-party skins retain their own artwork.

To restore the original engine configuration:

```powershell
.\scripts\install.ps1 -KenshiDir 'D:\SteamLibrary\steamapps\common\Kenshi' -Action Restore
```

Restore checks the installed configuration's hash before replacing it. If it
has changed independently, merge the saved original manually. Dormant DLLs,
settings and backups remain available.

## Mix independent mods

Copy a compatible Windows x64 DLL into `RE_Kenshi_Rust/plugins/` and restart.
With `discover = true` (the installer default), unlisted DLLs load automatically.
A missing `plugins.toml` also enables discovery. Explicit entries can disable
a DLL or choose load order:

```toml
schema_version = 1
discover = true

[[plugins]]
file = "my_mod.dll"
enabled = false
```

Configured entries run first in listed order, then unlisted DLLs in filename
order; shutdown reverses that order. Dependencies must already be available
when a mod starts. `discover = false` restricts loading to explicit enabled
entries. Updates preserve that choice, other entries and users' DLL files.
The core starts before external mods. Adding or replacing DLLs requires restart;
code remains pinned for the process lifetime.

The [Rust SDK](docs/plugin-sdk.md) wraps the versioned C tables in
`crates/plugin-api`. Rust traits, closures and allocations stay in their owning
DLL; copied values, function pointers and opaque handles cross the boundary.
Legacy V1 plugins remain supported. Existing KenshiLib, RE_Kenshi and Dust DLLs
require ports or adapters. Native crashes can still affect the game.

## Graphics, performance and native hooks

The core includes shader-cache, heightmap-cache, collider-cache, dds-mip-skip,
compressed-textures and graphics infrastructure. Shader caching defaults on;
other optimization toggles default off. Unsupported adapters show unavailable
instead of applying guessed offsets. See [performance](docs/performance.md).

Color grading and rain refinement are [separate sample DLLs](samples/README.md).
The original [Kenshi 2 style sample](samples/kenshi2-style/README.md) provides
color-based ink, desert grading, palette simplification and sharpening. Its
checkbox starts off on a fresh install; load a world to see it. The graphics
API supplies post-tonemap scene color without depth or temporal history.
Multiple effects compose in priority order. See [graphics](docs/graphics.md).

The bootstrap resolves the installed Ogre's decorated `Root::getSingletonPtr`
export, verifies it exists and checks original forwarding through a trampoline.
This is an inspected ABI assumption, not certification for every engine build.
Original/native replacement behavior needs a dedicated verified adapter.

The [native symbol catalog](docs/native-symbols.md) inventories exports, imports
and RTTI candidates in installed Kenshi, MyGUI, Ogre and ParticleUniverse binaries:

```powershell
python scripts/scan-symbols.py scan --game 'D:\SteamLibrary\steamapps\common\Kenshi'
python scripts/scan-symbols.py query BillboardRenderer
.\target\release\hook-list.exe .\target\release\RE_Kenshi_Rust.dll 'D:\SteamLibrary\steamapps\common\Kenshi'
```

Low-level adapters can use the [named hook catalog](docs/hooks.md) and cooperative
address reservations. IDs are stable; native addresses resolve per process
because of ASLR. A discovered export still needs a verified signature and
ownership contract. Other frameworks detouring the same addresses require
separate coexistence checks.

The bundled **Tone down weather effects** checkbox in Options → REKENSHI-RUST
softens rain, dust storms, ash and particle fog to 35% visibility. It defaults
on, applies live and preserves weather gameplay. The optional rain-refinement
sample uses the shared renderer to add thin streaks and camera fade; see
[weather behavior](docs/weather.md).
