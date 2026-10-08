# In-game debug overlay

Open **Options → REKENSHI-RUST**, page to **Developer diagnostics**, and enable
**Debug overlay**. It appears at the upper left and stays visible when Options
closes. Disable the same checkbox to remove it. **Freeze debug overlay** holds the
displayed snapshot; unfreeze to catch up. These settings apply without restart and
save as `debug.overlay` and `debug.freeze` in `RE_Kenshi_Rust/modpack.toml`.

The overlay shows host status, active built-in/DLL counts, failed DLL count, Ogre
observer calls, shader/collider/sidecar cache hits and terrain reads. Its public
operation summary lists up to seven contracts sorted by counted dispatches, with
provider generation, readiness/revocation, active registration count, observed
dispatch count and the first-use flag. A call count confirms broker dispatch;
it does not prove a shader's visual quality or successful gameplay completion.
Counts accumulate while enabled, pause while disabled, and reset when that
provider publishes a new generation. Some native adapters bypass broker dispatch
when there are no consumers. Freeze holds only the display; counters keep running.

Recent logs include `Context::log`, captured `Logger` messages, host hook startup,
settings applied from Options and plugin failures. The host retains the latest 128
messages, bounded to 200 characters each, and displays the latest 12 with UTC times.
Visible lines are truncated to fit. The existing `RE_Kenshi_Rust.log` remains the
full persistent log. Avoid per-particle/per-draw logging in mods; use the counters
to observe hot operations without flooding the stream.

The native view refreshes about twice per second and after settings clicks on
MyGUI's existing UI callback.
It takes no keyboard or mouse input. Its root is independent of the Options tree;
destroyed widgets are found/recreated by name, with no cached native addresses.
MyGUI owns the widget/layer allocations; disabling destroys the root, and UI-lane
host shutdown retires it before detaching the frame hook. The empty diagnostic
layer is reused until native Gui teardown. The existing MyGUI fingerprint gate
applies; no unsupported native binding is enabled by this setting.

The bounded log ring and toggle atomics share the host's process-lifetime native
adapter state (the retained M-AVOID-STATICS exception): logging and frame entry
points need a common host-local sink. Rust collections and allocator ownership
stay inside the host DLL; MyGUI strings and widgets use its native allocator
(M-ISOLATE-DLL-STATE, M-UNSAFE).

`scripts/test-menu.py` exercises the actual shipped MyGUI DLL: real Options
checkbox activation, copied SimpleText captions, input properties, freeze,
recreation, persistence and UI-lane shutdown/restart. Broker tests verify that
reading diagnostics never executes plugin policy and that disabled counters pause.
These headless native fixtures do not replace visual QA in a running world.

Verification on 2026-10-08: the complete `scripts/verify.ps1 -KenshiDir
'D:\SteamLibrary\steamapps\common\Kenshi'` workflow passed, including the native
overlay regression and existing independent-DLL/GPU/terrain tests. Retained log:
`target/overlay-verification.log`.
