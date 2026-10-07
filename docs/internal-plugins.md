# Internal plugins in one DLL

This guide covers changes to the bundled core. For a new optional mod, use
the [independent Rust DLL SDK](plugin-sdk.md); no core registration is needed.

The core registration list lives in `src/internal_plugins.rs`:

```rust
Plugins::new()
    .register(ShaderCache)
    .register(HeightmapCache)
    .register(ColliderCache)
    .register(DdsMipSkip)
    .register(CompressedTextures)
    .register(GraphicsBackend)
    .register(rekenshi_dark_ui::DarkUi)
    .register(rekenshi_weather::Weather::new(extensions::allowed))
    .start(root, |id, token| extensions::internal_host(id, token).map_err(Into::into))?;
```

Each built-in feature owns a Rust library under `plugins/<name>/`. The host
links these into `RE_Kenshi_Rust.dll`. Optional behavior and visual effects are
separate DLLs under `samples/`. Default `cargo build --release` builds the host,
CLI tools and separate Kenshi 2 sample; installation of that sample is explicit.
The host coordinates loading and native hook ownership. Weather and dark UI
follow the same crate layout as the caches and graphics backend; all are linked
into this DLL. Dark UI receives a host adapter for verified MyGUI string copying,
hook reservations, diagnostics, and frame readiness. The host forwards weather
interface negotiation and removes rain styles after revoking plugin tokens.
There is no features aggregate crate.

```text
plugins/your-core-plugin/
    Cargo.toml             # Library; dependencies belong to this plugin
    src/lib.rs             # Exports the core plugin type
    src/plugin.rs          # Safe Plugin implementation and settings
    src/native/mod.rs      # Private verified adapter
```

`rekenshi-plugin-runtime` defines `Plugin`, `Plugins`, the Rust `Host`, settings and lifecycle.
It validates dependencies and controls, generates menu rows, applies live changes,
persists snapshots, and stops plugins in reverse order. The runtime forbids unsafe
Rust. Shared engine bindings and ownership helpers live in `crates/native`;
the bounded safe SIC codec lives in `crates/sic`. Graphics owns the backend; `crates/plugin-sdk` supplies a safe `PostProcess`
client to effect DLLs. The SDK wraps the shared runtime and versioned ABI.

## Adding a plugin

For event-driven behavior, start with the [callback quickstart](plugin-callbacks.md):
`context.host.hook_ogre_calls(move || { ... })?` or
`context.host.hook_ui_frame(move |delta_seconds| { ... })?`.
The host owns those detours and the runtime removes callbacks before shutdown.

1. Create a library with `cargo new --lib plugins/your-mod`. Add it to workspace
   members if Cargo has not already done so.
2. Add the runtime with `cargo add -p <your-package> --path crates/plugin-runtime`.
   Implement `Plugin` and declare its `Setting` controls.
3. Keep settings and policy in a module with `#![forbid(unsafe_code)]`. Put any
   necessary, verified native adapter in a private module in the same crate.
4. Add the dependency to the host with `cargo add -p rekenshi-rust --path
   plugins/your-mod`, then import it and add `.register(YourMod)` in
   `src/internal_plugins.rs`. Register dependencies before their dependents.
5. Test the plugin's behavior and failure paths, then run `scripts/verify.ps1`.

`Plugin` requires `id` and `start`; `title`, `requires`, `settings`, `update`
and `stop` have defaults. `update` receives the full validated settings snapshot and
must preserve its previous behavior on error. `stop` must quiesce workers and
disable any plugin-owned native hooks, including after a partially failed `start`.
Managed Host callbacks are removed automatically. Plugin IDs and setting
keys must be unique. Dependency order is checked before any plugin starts.

Declared toggles and numbers appear under **Options → REKENSHI-RUST**, with up to
five controls per page, adjusted to the available height. There is no mod-specific
MyGUI code to add. Checkboxes reflect applied settings; unavailable plugins show
their reason. The native Options TabItem owns every settings widget, so switching
tabs or hiding Options hides them automatically. When Kenshi closes Options it
destroys its tabs; the next opening creates a fresh tab and controls.

The shared dark theme lives in `src/ui/theme.rs`: charcoal surfaces, light text,
mint checkbox indicators, compact step buttons and wrapped descriptions. Both
built-in and independently loaded plugins use it automatically. The native adapter
tints Kenshi's existing white rectangle skin and uses its transparent text skin;
no additional textures or plugin-specific styling are required.

**Dark game UI** is a separate built-in (`plugins/dark-ui/`). It keeps Kenshi's
original atlas, worn frame artwork, layout, and nine-slice borders. The adapter
classifies actual ResourceSkin resource names during native initialization,
darkens HUD/window chrome and brightens dark neutral text. Coloured status text
keeps its hue; portraits, maps, item art, status fills and icon glyph skins are
excluded from panel tinting. **Frame brightness** controls retained texture detail.
Both settings apply on the UI thread immediately and save in `modpack.toml`.
Checkbox faces retain a brightness floor so their original baked-in tick glyphs
remain legible even with darker frames.

Native colour setters are intercepted below widget layout, including hover and
disabled text states. Original colours are retained for reversible live changes.
Creation/destruction hooks own the lifetime of thread-local address records;
colour changes use the SkinItem base pointer supplied by MyGUI, with no guessed
object offsets. Updating a panel doesn't propagate a new tint to its children.
`scripts/test-dark-ui.py` checks actual native output vertex colours, original
status fill colours, text states, transparency, repeated toggles and address reuse.
Unknown mod skins retain their artwork; extend the reviewed skin list in
`plugins/dark-ui/src/palette.rs` when adding another UI pack. Original game assets are never rewritten.

Kenshi's close handler casts every tab's data to `DatapanelGUI*`. The adapter clones
a verified native pointer holder and sets only the clone's panel pointer to null.
An empty `Any` would fail the game's close contract, and copying a vanilla panel
pointer would share ownership. `src/ui/tab_data.rs` isolates this C++ boundary.
`scripts/test-menu.py` exercises the real MyGUI DLL with a headless hierarchy:
typed close data, repeated hide/reopen and close/recreate, tab switching, live
checkbox clicks, pagination, resizing, and complete GUI reconstruction. Fonts,
skins and Kenshi's input handling still need an in-game visual check.

For an independent graphics DLL, declare `requires() == &["graphics"]` and
own an `Option<rekenshi_plugin_sdk::PostProcess>`. Register once in `start`, update its
parameters in `update`, and remove it in `stop`. The backend owns shader
compilation and GPU state restoration. See `samples/kenshi2-style/src/effect.rs`
and [the graphics interface](graphics.md). New native hooks require verified
signatures and a documented adapter; registering a plugin does not generate bindings.

Decorated C++ export names live in `crates/native/src/symbols.rs`, grouped by DLL
and class. Import the class module in an adapter to keep lookups readable:

```rust
use rekenshi_native::symbols::particle_universe::particle_renderer;

let address = rekenshi_native::export(
    particles,
    particle_renderer::GET_PARENT_TECHNIQUE,
)?;
```

Every constant is a `&CStr`, so termination is checked by Rust and no temporary
string allocation is needed. Add new decorated names to this catalog; the owning
adapter keeps its verified function types, fingerprints, and implementation RVAs.

For discovery, the [repeatable symbol scanner](native-symbols.md) inventories
exports, imports and RTTI candidates from the installed binaries. It generates
all named exports under `symbols::discovered`, with readable declarations and
stable constants. Search with `python scripts/scan-symbols.py query <name>`, then
promote the inspected export into a short curated constant when adding its adapter.

The core plugins are shader-cache, heightmap-cache, collider-cache, dds-mip-skip,
compressed-textures and graphics. Native adapters report unavailable builds
instead of guessing addresses. Collider and DDS need the fingerprinted Steam
1.0.68 executable; those toggles and compressed-textures default off.

Color grading, rain refinement and Kenshi 2 style are independent DLLs under
`samples/`. Only installed DLLs supply their controls. See
[sample installation and migration](../samples/README.md),
[our inventory](../samples/builtin-effects.md), [weather](weather.md) and
[performance](performance.md).

## Settings and lifecycle

`RE_Kenshi_Rust/modpack.toml` retains its original filename and keys for upgrades.
It stores declared settings under `[settings.values]`, with `schema_version = 1`.
Missing fields take their defaults. Unknown fields, wrong types, nonfinite or
out-of-range numbers and unsupported schemas reject registry startup with a log.
A failed plugin is shown as unavailable; its dependents are not started.

Live changes apply to the owning plugin, then save through a same-directory file
replacement. A save failure rolls the plugin back to the previous snapshot.
A rollback failure marks it unavailable and preserves its native resources until
shutdown. Plugins stop in reverse order, including partially initialized plugins.
Hook reservations are released only after successful shutdown; a failed shutdown
retains the registry and its process-pinned code.

Internal tokens occupy a separate range beginning at 1001. External DLLs load in
configured order after internal plugins. The graphics service is exported from
`RE_Kenshi_Rust.dll` for effects using the external interface.

The installer migrates the former shader/graphics/color-grade DLL entries into
internal settings, preserving performance choices. Former built-in visual settings move into `settings/<id>.toml`; the first
original is preserved in `modpack.toml.before-dll-plugins`. It backs
up the original list to `plugins.toml.before-modpack`; unrelated entries and files
survive updates. Migration also recognizes `rekenshi_performance.dll` and keeps
the `performance.shader_cache` key. Shader budgets move to `shader-cache.toml`,
preserving `performance.toml` values. Cache budgets, RGB tint and effect priority
remain startup settings in the legacy TOML files.

New exports are `rekenshi_internal_plugins` and `rekenshi_plugin_setting_set`.
`rekenshi_pack_layers` and `rekenshi_pack_setting_set` retain the original ABI
as compatibility aliases.

## Optional external DLL support

The versioned external `HostV1`/`PluginV1` interface remains available for third-party
DLLs, with module pinning and cooperative hook ownership. The Rust SDK gives independent DLLs the same `Plugin` and managed `Host`
interfaces by negotiating additional C service tables. Rust traits, closures
and allocations stay inside their owning DLL.

Older shader/graphics/color-grade DLL adapters live under `compat/`. The external
diagnostics example lives in `examples/dll-plugins/diagnostics`. They are outside
the default build and installation. `cargo build --workspace --release` also
builds them for compatibility tests. Fresh `plugins.toml` enables folder discovery and has an empty explicit
list; existing external config and files survive updates.

## Verification

Runtime tests cover registration order, dependency failures, duplicate IDs/keys,
failed starts, invalid settings, persistence and failed-save rollback.
`test-modpack.py` checks native startup, live shader/heightmap bypass/re-enable,
foreign-map passthrough, sample style changes, availability reporting and restart.
It also checks the new exports against their compatibility aliases.
`test-performance.py` starts the core plugins against a staged executable,
compares DDS output with the native listener, validates SIC formats and ownership,
and tests independent PhysX collection copies. `test-weather.py` captures rain
submissions through the real detours with its separate rain DLL installed. The optional external DLL suite and actual
GPU pixel/state checks still run. Installer checks verify core-only and explicit sample installation, settings
migration, third-party file preservation and exact configuration restoration.
`test-sdk.py` loads independently compiled callback and graphics DLLs together.

**Weather effects** is bundled in the host from `plugins/weather/`. The **Tone down
weather effects** checkbox (`weather.tone_down`, default true) scales additive
rain/dust brightness and alpha-blended storm/ash/fog opacity to 35%. Native
particle templates distinguish weather from fire using the same smoke material.
The weather crate implements the two particle-render hooks; the host supplies
plugin-token authorization and coordinates startup/shutdown. The optional rain-refinement DLL
uses the SDK's owned `RainRendering` client to add thin streaks and proximity
fade without installing competing detours. Disabling the checkbox restores the
base visibility while leaving that optional rain style independent.
