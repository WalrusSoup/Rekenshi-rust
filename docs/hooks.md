# Named native hook catalog

For bundled plugins, use the [managed callback API](plugin-callbacks.md):
`context.host.hook_ogre_calls(move || { ... })?` and
`context.host.hook_ui_frame(move |delta_seconds| { ... })?`.
The details below describe the lower-level native catalog used by inspected
adapters and separate DLL plugins.

`HostV1::hooks()` negotiates the optional `rekenshi_hooks_api` service once. Store
its `HookCatalog` in plugin state; later `target(HookId)`/`info(HookId)` calls use
that function table directly, without repeated export lookup in the plugin.
HostV1 and PluginV1 retain their original binary layouts. Existing V1 mods still
load; new mods requiring this optional service reject gracefully on older hosts.

| ID | Rust name | Resolution / native signature on Windows x64 |
| --- | --- | --- |
| 1 | OgreRootSingleton | OgreMain_x64 exported Root::getSingletonPtr; `extern "C" fn() -> *mut c_void` |
| 2 | MyGuiSetCaption | Fingerprinted MyGUI export thunk followed to verified body; `unsafe extern "C" fn(widget: *mut c_void, ustring: *const c_void)` |
| 3 | D3DCompile | Fingerprinted D3DCompiler_43 export; documented D3DCompile ABI |
| 4 | D3DCreateBlob | Same fingerprinted compiler; `unsafe extern "system" fn(usize, *mut *mut ID3DBlob) -> HRESULT` |
| 5 | D3D11Draw | Published by graphics service from typed native context vtable; `unsafe extern "system" fn(context: *mut c_void, vertex_count: u32, start_vertex: u32)` |

The UI implementation publishes after its binary and native-string ABI checks.
The host preloads the game-bundled compiler only after fingerprint verification,
then fills compiler entries through GetProcAddress. Successful exported lookups
are cached, including startup stop/restart in the same process. An initially
missing module is not cached as permanently absent; lookup can retry when it
loads. Draw cannot be found through GetProcAddress: the graphics service first
discovers its real vtable method, reserves it, then publishes that named slot.

Only D3D11Draw accepts external publication, and the provider must have a live
host token and already own the address reservation. A published slot cannot be
replaced with a different address during the process. Publication is an unsafe
contract: a native provider must supply the real entry with the correct ABI,
not its callback or trampoline. Native mods are trusted process code, not a
security boundary. Builtin targets cannot be overwritten through that service.

`HookInfoV1` exposes the original entry address and a momentary ownership snapshot.
AVAILABLE means unreserved; RESERVED includes owner_token (0 means bootstrap).
A snapshot does not claim ownership: always call `HostV1::reserve_hook` before
patching and handle HOOK_BUSY. The bootstrap retains ownership of its own targets
even while stopped. Plugin ownership releases after successful shutdown.

The address is the callable entry, which may already be detoured. It is NOT an
unhooked trampoline. A detour callback must call its own `TypedDetour::call` to
forward, rather than calling the catalog address recursively. Enum lookup does
not install a detour or erase native signature/object-lifetime requirements.
For graphics effects use the graphics service's shared POST_TONEMAP stage;
multiple effects should not compete to patch Draw independently.

Across DLLs, IDs/statuses are plain u32 and tables are repr(C). Unknown IDs, ABI
versions and sizes reject without modifying outputs. Rust's `HookId::from_raw`
validates an integer before constructing an enum. IDs are append-only; future
contracts needing different arguments require a new versioned interface.

Use `HookId::ALL` to inspect the supported catalog. `hook-list` prints availability,
addresses and owners against local native DLLs, running startup/shutdown but no
game world. When run against an installed host it starts the enabled native mods,
which may perform their normal cache/diagnostics I/O. The tool does not edit the
game's plugin configuration.

Addresses stay in memory and are resolved again for every new process.
[Windows ASLR](https://learn.microsoft.com/en-us/cpp/build/reference/dynamicbase-use-address-space-layout-randomization)
can change DLL bases; persisting absolute pointers would be incorrect. Plugin
authors use stable names while the host manages this per-launch work.
