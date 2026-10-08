# Rust mod SDK

The normal authoring path is a separate Windows x64 DLL. The core supplies hooks,
menu widgets, persistence and rendering services; your DLL supplies behavior.
Players mix mods by copying DLLs into `Kenshi/RE_Kenshi_Rust/plugins/` and restarting.

## Create, build, distribute

From this checkout, with Rust 1.85+ and the MSVC linker installed:

```powershell
cargo install --path tools/cargo-rekenshi --locked
cargo rekenshi new plugin my-mod
cargo rekenshi new plugin my-look --kind graphics
```

`cargo-rekenshi` is an ordinary Cargo subcommand. Installation makes it available
from any directory, including your generated project. The SDK defaults to this
checkout's source location; if you move it, pass
`--sdk-path <new-checkout>/crates/plugin-sdk`. Run `cargo rekenshi --help` for
options. `--path <directory>` chooses an output folder; `--no-build` skips the
release build. The old PowerShell script forwards to the same Rust helper.

Each creation command creates an independent Cargo project in `mods-src/<name>` and builds
its DLL. Edit `src/implementation.rs`, then rebuild:

```powershell
cargo build --release --manifest-path mods-src/my-mod/Cargo.toml
```

Distribute `mods-src/my-mod/target/release/my_mod.dll`. Users need the host and
your DLL, not Rust, Cargo, the SDK or this repository. The SDK is currently a local
path dependency rather than a published crates.io package. Keep this checkout
available when building, or vendor the SDK, runtime and ABI crates with your sources.
The project has its own `[workspace]`; adding a mod requires no root workspace,
core registry or sample selector changes.

`src/lib.rs` only exports your implementation:

```rust
mod implementation;
rekenshi_plugin_sdk::export_plugin!(implementation::MyMod);
```

## A complete callback mod

In `src/implementation.rs`:

```rust
#![forbid(unsafe_code)]
use rekenshi_plugin_sdk::{Context, Error, Plugin, Settings};
use std::sync::{Arc, atomic::{AtomicU64, Ordering}};

#[derive(Default)]
pub struct MyMod { calls: Arc<AtomicU64> }

impl Plugin for MyMod {
    fn id(&self) -> &'static str { "my-mod" }
    fn title(&self) -> &'static str { "My mod" }

    fn install(&mut self, context: &Context, _: &Settings) -> Result<(), Error> {
        let calls = self.calls.clone();
        context.host.hook_ogre_calls(move || {
            calls.fetch_add(1, Ordering::Relaxed);
        })?;
        context.log("My mod loaded");
        Ok(())
    }
}
```

`Default` should be cheap and side-effect free: metadata is read before startup.
`id` must be unique, stable and contain only ASCII letters, digits, hyphens or
underscores. Use `register` for desired operation rules and `install` for native
activation and managed observers. Owned captures require `Send + Sync + 'static`; callbacks may run
concurrently. The [callback guide](plugin-callbacks.md) lists timing and limits.

The frozen entry descriptor requires ASCII ID/title/version metadata; keep titles
short. Settings labels and descriptions support UTF-8. Each DLL can declare up
to 64 controls; the loader accepts up to 64 DLL entries.

## Automatic settings

Override `settings` to return a static slice of declarations:

```rust
use rekenshi_plugin_sdk::{Control, Setting, Value};

fn settings(&self) -> &'static [Setting] {
    &[Setting {
        key: "my-mod.enabled",
        label: "Enabled",
        description: "Count observed Ogre calls.",
        default: Value::Toggle(true),
        control: Control::Toggle,
    }]
}
```

Put that method inside the `Plugin` implementation. Numeric controls use
`Value::Number(default)` with `Control::Number { min, max, step }`.
The host adds controls to **Options -> REKENSHI-RUST** automatically. Keys must
be unique across all loaded mods; namespace them with your plugin ID.

Read initial values in `start`; implement `update(context, settings)` for live
changes. For callbacks, keep enabled state in an `Arc<AtomicBool>` and update
it there; the callback checks the flag. See the complete
[callback-counter sample](../samples/callback-counter/src/counter.rs).
The generator includes a checkbox and update method as a starting point.

Each DLL owns `RE_Kenshi_Rust/settings/<id>.toml`. Missing values use defaults.
Invalid values reject that mod's settings. Changes apply first, then save;
failed saves trigger rollback. Preserve old behavior when `settings_changed` returns an
error. Version and migrate your own keys when changing their meaning.

## Graphics mods

Use `--kind graphics` for a working shader starter. Declare
`requires() -> &'static [&'static str] { &["graphics"] }`, own an
`Option<PostProcess>`, and call:

```rust
let pass = rekenshi_plugin_sdk::PostProcess::new(
    context, include_bytes!("effect.hlsl"), 200, [0.; 16],
)?;
```

Keep the handle in plugin state, call `update([f32; 16])` for live parameters,
and `remove()` in `stop`. Disabling your effect should remove its pass.
HLSL is embedded in your DLL; the shared host compiles it and owns GPU resources.
Multiple DLLs compose in priority order. The
[Kenshi 2 style implementation](../samples/kenshi2-style/src/effect.rs) demonstrates
checkboxes, numbers, embedded shaders, live updates and cleanup. See
[the graphics contract](graphics.md) for shader bindings and supported stages.

## Lifecycle and capabilities

The SDK supplies exports, ABI negotiation, panic containment and callback cleanup.
It detaches managed callbacks and waits for in-flight invocations before calling
`stop`. Join your workers and release your own resources in `stop`, including
resources created during a partially failed start. Do not synchronously change
lifecycle or settings from inside a callback. Log with `context.log`; capture
`context.host.logger()` when logging from a closure, and avoid per-call logging.
These messages also appear in the opt-in [in-game debug overlay](debug-overlay.md).

Currently supported safe observers are Ogre root-getter calls and MyGUI UI
frames. The separate [gameplay service](gameplay.md) provides copied squad/character
snapshots and queued commands through `Gameplay::connect(context)`, subject to
native adapter capabilities. See the [squad inspector sample](../samples/squad-inspector/README.md)
for polling from a worker. The graphics service provides LDR
post-tonemap scene color and sixteen parameters, without depth or temporal history.

Special features can contribute an inspected native adapter to the core and a
safe typed SDK method. The [symbol catalog](native-symbols.md) helps discover
candidates, but an export name alone is not a callable binding. Low-level mods
can access `context.host.native()` and the ABI hook catalog; they remain
responsible for verified signatures, reservations and native ownership.

`requires` checks available core or previously started DLL plugin IDs. For
DLL-to-DLL dependencies, put providers earlier in `plugins.toml`; the loader
does not automatically sort a dependency graph. Duplicate IDs or setting keys
are rejected. A disabled explicit entry overrides folder discovery.

## Verify changes

```powershell
cargo test --workspace --all-targets --locked
.\scripts\verify.ps1 -KenshiDir 'D:\SteamLibrary\steamapps\common\Kenshi'
```

The native SDK integration test loads two independently compiled DLLs together,
forwards real Ogre/MyGUI calls, updates a graphics effect, checks settings
isolation and exercises clean stop/restart. Both generated starter types have
also been compiled as independent Cargo projects. Visual changes still need an
in-game world check.

## Rain rendering

Use `RainRendering::new(context)?` during registration to own a desired style.
A public renderer provider activates it when matching native render operations occur. Store it in your plugin and call
`rain.configure(enabled, visibility, size)?` from `settings_changed`. Visibility accepts
0..1, size accepts 0.1..2. Drop the handle in `stop`; owner revocation also disables its rule. No pointers or native detours are needed in your plugin.
See [the rain sample](../samples/rain-refinement/src/plugin.rs). The built-in
weather reduction is independent; multiple rain styles choose the smallest
size and lowest visibility, then run one geometry transformation and fade.

## Public operation providers and lifecycle registrations

Use `Plugin::register` for desired behavior before native use. The host prepares
all participating built-in and DLL compositions before installing any provider. `install` activates providers/resources after that barrier,
`settings_changed` applies preferences, and `stop` cleans up. SDK DLLs must export
the preparation entry generated by `export_plugin!`; this unreleased platform has
no fallback loader path.

`rekenshi_plugin_sdk::operations::Operations` negotiates the public broker. Keep
its returned `Registration` in plugin state; inspect `late()` and remove it during
shutdown. Native callbacks receive copied bytes and separate captured state.
Provider `Contract` descriptors name their schema, lane, evidence and payload
bounds; `Provider` owns the published generation and revokes it before resource
retirement. Third-party contracts need no host enum additions.

`lifecycle_contracts` and `lifecycle_changed` deliver pending copied generation
changes on the serial game lane. An unavailable native gameplay adapter means no
automatic game-lane delivery. Readiness never grants native pointer access or
promises before-first-use activation. Full world/save/device/UI/particle lifecycle
contracts have not been supplied merely by implementing this transport.

See the [independent source-transform DLL](../samples/source-transform/README.md)
and [native integration inventory and remaining work](plugin-platform-implementation.md).


## Copied operations and domain events

`adapters` names the verified native schemas; `policies::DesiredPolicy` owns a
scalar replacement, and `codecs::Codec` owns a custom extension and its copied
image decoder. Declare codecs in `register`; late codec activation requires restart.
Native streams, image metadata and GPU objects stay with their providers.
`Registration::order` expresses optional before/after identities before first use;
cycles reject registration. Policy identity versions contribute to shader cache keys.

`event_contracts` and `event` deliver copied occurrences on ordinary serialized
state. `events` documents gameplay roster/completion records; widget observations
use the `adapters::UI_WIDGET` schema. Queues are bounded and overflow is an error.
No generic tick was added. See the opt-in runtime rules in the source-transform
sample and the [implemented inventory](plugin-platform-implementation.md) for
first-use, ownership and native-evidence limits.
