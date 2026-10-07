# Independent mod samples

The core ships performance optimizations and a shared graphics backend. Optional
visual effects are separate DLLs; users choose which ones to install and can mix
them. Start with the [Rust SDK quickstart](../docs/plugin-sdk.md) or run
`cargo rekenshi new plugin my-mod` after installing the helper with
`cargo install --path tools/cargo-rekenshi --locked`.

| Sample | Build command | DLL in `target/release` |
| --- | --- | --- |
| [Kenshi 2 style](kenshi2-style/README.md) | `cargo build --release -p rekenshi-kenshi2-style` | `rekenshi_kenshi2_style.dll` |
| [Color grade](color-grade/README.md) | `cargo build --release -p rekenshi-color-grade-dll` | `rekenshi_color_grade_sample.dll` |
| [Rain refinement](rain-refinement/README.md) | `cargo build --release -p rekenshi-rain-refinement` | `rekenshi_rain_refinement.dll` |
| [Callback counter](callback-counter/src/counter.rs) | `cargo build --release -p rekenshi-callback-counter` | `rekenshi_callback_counter.dll` |

Copy only the desired DLLs into `Kenshi/RE_Kenshi_Rust/plugins/` and restart.
Installer defaults enable folder discovery. No registry entry or `samples.toml`
is needed. Explicit disabled entries in `plugins.toml` override discovery.
Controls appear under **Options -> REKENSHI-RUST**; each DLL owns
`RE_Kenshi_Rust/settings/<plugin-id>.toml`.

Default `cargo build --release` builds the host and the separate Kenshi 2 DLL.
`./scripts/install.ps1 -KenshiDir <game> -IncludeKenshi2Sample` installs both;
without that switch it installs just the core. The style checkbox starts off
on a fresh install. Enable it and load a world to see the effect. Color grade
and rain refinement start enabled when their DLLs are installed.

The [RE_Kenshi mod catalog](re-kenshi-mods.md) documents 38 researched ecosystem
references, dependency evidence and capabilities needed for ports. It does not
claim those mods are implemented or binary compatible. The
[implementation inventory](builtin-effects.md) lists what actually ships.

## Upgrade from bundled effects

The retired `samples.toml` selector is ignored. Migration moves old visual keys
from `modpack.toml` into each owning DLL's settings file and preserves the first
original as `modpack.toml.before-dll-plugins`. Existing per-mod values win, and
performance choices survive. Install the corresponding sample DLL to use those
saved settings. Removing a DLL leaves its settings available for reinstalling.

## Author another mod

Create an independent project with the generator, implement `Plugin`, and add
`export_plugin!(YourMod)`. Do not edit `src/internal_plugins.rs` for an optional
DLL. See [managed callbacks](../docs/plugin-callbacks.md) and
[graphics](../docs/graphics.md). Native extensions still need verified signatures
and ownership rules; discovered symbols alone cannot create safe bindings.
