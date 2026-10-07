# Writing plugins with callbacks

Independent DLL plugins use the Rust SDK `Host` provided as `context.host`. Register a closure
once in `start`. The host owns the native detour and forwards the original call;
the runtime owns your callback's registration and cleanup.

```rust
use rekenshi_plugin_sdk::{Context, Error, Plugin, Settings};
use std::sync::{Arc, atomic::{AtomicU64, Ordering}};

#[derive(Default)]
pub struct OgreCounter {
    calls: Arc<AtomicU64>,
}

impl Plugin for OgreCounter {
    fn id(&self) -> &'static str { "ogre-counter" }

    fn start(&mut self, context: &Context, _: &Settings) -> Result<(), Error> {
        let calls = self.calls.clone();
        context.host.hook_ogre_calls(move || {
            calls.fetch_add(1, Ordering::Relaxed);
        })?;
        Ok(())
    }
}
```

Add `rekenshi_plugin_sdk::export_plugin!(OgreCounter)` in your cdylib entry
module. The [generator and quickstart](plugin-sdk.md) create that project for you.
`id` and `start` are the only required methods.
`title` defaults to the ID; `update` and `stop` default to doing nothing. Override
them when your plugin has settings, workers, or other resources to manage.
You do not need a native module, a HostV1 table, an export lookup, a function
pointer cast, a hook reservation, a detour transaction, or a callback handle for
these supported callbacks. The shared runtime includes a compiled callback documentation example.

## Available callbacks

| Method | Callback | Timing |
| --- | --- | --- |
| `host.hook_ogre_calls(move || { ... })?` | `Fn()` | After each native `Ogre::Root::getSingletonPtr` call returns |
| `host.hook_ui_frame(move \|delta_seconds\| { ... })?` | `Fn(f32)` | After each native `MyGUI::Gui::frameEvent` returns on the UI thread |

Ogre getter calls are not simulation ticks. UI frames are not graphics draw
events. These are observer callbacks; they preserve the native arguments,
return values and original execution. No native object pointers cross into the
closures. Multiple plugins share each detour, and callbacks run in registration
order for each invocation.

For a UI frame:

```rust
context.host.hook_ui_frame(move |delta_seconds| {
    // Update your owned plugin state using this frame's elapsed time.
})?;
```

For logging, use `context.host.log("started")`. To log from a closure, capture
the copyable `context.host.logger()` handle. Avoid logging on every getter call
or doing blocking work there; the callback runs synchronously on the game caller.
Ogre callbacks can run concurrently. Captures require `Send + Sync + 'static`;
use owned `Arc` state and atomics or locks for mutation. Initialize that state
before registration because callbacks can start immediately.

An unavailable native adapter makes registration return an explanatory error.
Propagating it with `?` reports the plugin as unavailable in its settings rows.
Optional behavior can check `host.supports(Hook::UiFrame)` first.

## Ownership and settings

The runtime removes callbacks after a failed start and before plugin shutdown,
waits for invocations already running, and then calls your `stop`. It removes
all plugin groups before stopping any plugin, so observers cannot run after a
dependency is stopped. Dropping the runtime also detaches callbacks before its
plugin objects are dropped. Register callbacks once rather than registering
again on every settings update.

For a menu toggle, declare a `Setting` and update an `Arc<AtomicBool>` from
`update`. Your captured callback can check that flag before applying its work.
The runtime continues to own the registration when the feature is temporarily
disabled. The [SDK guide](plugin-sdk.md) describes automatic settings and DLL exports.

A callback panic is contained and quarantines all callbacks owned by that plugin,
including callbacks registered for other hook kinds. New registrations and settings
changes for that plugin are rejected. Already-running calls and independent plugins
can finish; the host records and logs a request to restart Kenshi before faulted
plugin state can be reused. SDK callbacks contain the panic inside their owning DLL. Recursive emissions of the same hook
forward native execution but skip nested observers. Different hook kinds can
still compose. Synchronous lifecycle changes inside a callback return an error
instead of waiting for their own stack. Empty subscriber lists avoid locks and
allocations; populated lists use a shared snapshot without per-call Vec allocation.

## Adding another supported hook

For live squad data and character commands, use the [gameplay service](gameplay.md).
Its safe SDK methods copy cached snapshots and enqueue commands; the host owns
native access at the game update hook. No plugin callback or custom detour is
needed to inspect the player roster.

Add the inspected native adapter to the host and expose a typed `Host` method
that dispatches its callback. All plugins then reuse that method. This
concentrates ABI checks, original forwarding and native patch ownership in one
implementation. An export discovered by the [symbol scanner](native-symbols.md)
still needs an inspected adapter before it becomes a callback method.

Hooks that replace a return value or skip the original, such as shader-cache
hits, need a dedicated typed interception interface with explicit forwarding
semantics. The two methods above provide observation. Existing cache adapters
keep their inspected replacement behavior.

`Host::native()` is available for low-level adapters. The SDK negotiates C
tables across DLLs; each Rust closure and trait object stays inside its owning
DLL. You do not write those tables for a normal SDK mod.

## Verify

```powershell
cargo test -p rekenshi-plugin-runtime --locked
cmd /c scripts/build-menu-fixture.cmd
$env:REKENSHI_TEST_GAME = 'D:\SteamLibrary\steamapps\common\Kenshi'
cargo test -p rekenshi-rust --lib --locked managed_callbacks_forward_native_ogre_and_mygui_and_stop_cleanly -- --ignored
```

The native integration test forwards the real Ogre getter and MyGUI frame,
checks closure invocation and frame arguments, exercises recursion, removes
callbacks, and verifies host stop/restart. It creates no game world and writes
no game files. `scripts/verify.ps1 -KenshiDir <install>` includes this test.

`scripts/test-sdk.py` also checks callbacks from a separately compiled DLL,
isolated settings, coexistence with a graphics DLL, and stop/restart.
