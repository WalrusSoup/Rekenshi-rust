# Public plugin platform implementation

The current platform implements public desired registrations, operation composition,
provider generations and copied domain events. Built-in policies and independent
DLLs use the same broker. The broader [specification](plugin-platform-spec.md) and
[API sketch](plugin-platform-api.md) remain proposals for capabilities without
verified native bindings. This project is unreleased; preparation is part of the
current SDK contract for every participating DLL.

## Registration, ownership and execution

`rekenshi_platform_api` negotiates an exact-sized Windows x64 C table independently
of `HostV1` and `PluginV1`. Descriptors record namespaced protocol names, schema,
mode, payload bounds, native lane and evidence. Accepting a descriptor does not
certify its evidence. Independent providers can publish new contracts and resolve,
fingerprint, reserve and install native targets without extending a host enum.

Every participating built-in and DLL completes `Plugin::register` before any
provider installs. DLLs export `rekenshi_plugin_prepare` through `export_plugin!`;
the loader rejects missing preparation. `install` activates native providers,
`settings_changed` applies validated preferences, and `stop` releases ownership.
Failed preparation and failed core startup clean pending registrations. The
`start` and `update` trait methods remain aliases for installation and settings
changes; migrated features use `register`, `install` and `settings_changed`.

`Operations::transform` composes synchronous copied inputs/results. Priority,
identity and owner provide deterministic tie breaking. `Registration::order`
adds before/after identities; missing peers impose no edge until registered,
cycles are rejected, and ordering changes after first use require restart.
Duplicate identities, mixed modes and competing exclusive policies fail.
`Operations::manifest` returns stable SHA-256 policy identities/versions and
constraints; generations and process-local tokens are excluded. Authors must
change their registration version when changing policy semantics.

Callbacks keep closures, captures and allocations in their originating DLL.
No Rust trait object, collection or native object crosses the C boundary.
Callbacks execute on the provider's advertised lane and have separately serialized
captures. Broker mutation and recursive dispatch from callbacks fail promptly.
A failed composition discards partial output. Native providers validate replacements
before committing, and preserve their original operation on validation failure.
This does not imply rollback for arbitrary irreversible native operations.

Removal waits for admitted callbacks and is owner-checked and idempotent. Provider
revocation quiesces callbacks before resource retirement. Replacement increments
the generation and rejects schema/mode changes under the same protocol name.
Consumers remain pending across compatible provider recreation. Panics quarantine
an owner across contracts and require restart. Failed deferred observers also
quarantine operations before cleanup. Native state is not presumed reusable after
a panic.

`PostProcess`, `RainRendering`, `DesiredPolicy` and `codecs::Codec` own public desired
rules. They can register before native providers exist. Providers own native objects,
GPU state, stream allocations, recreation and retirement. Scalar policies refresh
only when their composition revision changes; inactive copied operation hooks
avoid payload allocation.

Bounds: 1,024 retained contract names; 8,192 registration handles including cleanup
tombstones over process life; 8,192 lifecycle watches with 64 transitions each;
256 domain-event watches with 1,024 occurrences each; 16 MiB operation payload;
64 KiB synchronous metadata; 512 bytes event metadata and 4 KiB event payload.
Event queues share copied occurrence allocations and report overflow explicitly.
A plugin receives at most 64 deferred records per serial dispatch. Registration
churn can exhaust the process-life budget and then requires restart. High-frequency
particle interception still needs profiling; copied callbacks are opt-in.

## Actual lifecycle and event contracts

`lifecycle_contracts`/`lifecycle_changed` observe operation availability and
revocation. Current status is replayed to a late watcher; revocation suppresses
queued stale readiness. `event_contracts`/`event` receive future copied occurrences,
with the generation at occurrence time. A historical event grants no current
native access. Overflow disables the observer instead of silently losing events.

Ordinary plugin state is serialized with settings and shutdown. Delivery uses
the verified post-PlayerInterface-update point after command dispatch. There is
no public generic tick or scheduler. Headless native fixtures explicitly call
the copied-data dispatch export; an unsupported executable has no automatic
game-lane delivery.

Published gameplay events are roster changes and command completion/rejection,
including rejection on world invalidation. Applied means native command dispatch,
not successful movement or AI completion. The squad inspector takes snapshots
on roster events and sends bounded, coalesced owned JSON to its file worker.
It does not poll the world or depend on a UI frame.

MyGUI observations cover skin attachment, completed initialization and skin
shutdown, with opaque, non-reused IDs and copied skin names. These are skin
lifetime observations, not a universal widget object destructor or UI-ready phase.
Providers revoke their contract generation before their native adapter retires.
World-loaded/unloading/unloaded, save/load/import, pre-destruction device loss,
and whole UI/particle hierarchy readiness/recreation remain unpublished because
the necessary transitions have not been verified.

## Integration inventory

All native targets retain their existing fingerprints and documented exceptions.
“Before use” below describes the requirement; readiness alone does not establish
that game bootstrap has not already performed the operation.

| Integration and observed target | Earliest safe activation / first use | Lane, native ownership, retirement and recreation | Public facility and evidence limits |
| --- | --- | --- | --- |
| Shader cache: fingerprinted D3DCompiler_43 D3DCompile/D3DCreateBlob | Desired source/cache rules in global preparation; native hook after compiler/renderer fingerprints, before relevant compilation | Compiler caller; copied source before cache lookup; native blob allocation and Release. Detour disable plus callback quiescence before retirement | `shader.source`, `shader.bytecode`, `shader.cache-policy` @1. Bytecode replacements pass native D3DDisassemble validation. Cache hashes transformed source, compiler inputs and policy manifest. Unknown includes bypass caching. `plugins/shader-cache/src/native`; no claim about pre-host bootstrap compiles |
| Graphics/AO: D3D11 CreatePixelShader and Draw | Identity/source/draw rules in preparation, before shader creation/compilation | Render caller; copied private shader metadata, device-local constant buffers, scoped slot restoration; device identity recreates buffers | `render.shader-binding`, `render.draw-parameters` @1 and `shader.source`. Built-in AO uses these same registrations. GPU fixtures cover independent b12 and built-in b13; full in-game AO quality remains experimental |
| Terrain TIFF: open/close, pixel/rectangle in fingerprinted Plugin_Terrain_x64 | Desired loading/read/cache rules before terrain reads; verified native reader before activation | Native loader/read caller; mappings retained by Arc during reads; close removes reader association; eager copy and mapped modes refresh on next read | `terrain.open`, `terrain.read`, `terrain.cache-policy` @1. Replacement file paths bypass mismatched maps. `plugins/heightmap-cache/src/native`; native differential tests include clipping and ownership; no full world teardown claim |
| Collider load/clone/release: executable RVAs 7e53f0/20b860/1d99f0 and NxGetPhysicsSDK | After executable/PhysX verification, before binary collider loading | Native loader; cached NXU templates own release, each load returns an independent clone; scale changes clone the returned collection. Disable, capacity, SDK identity change and stop retire templates | `collider.load`, `collider.cache-policy` @1. No native collection pointer escapes. SDK identity comparison is not a verified pre-destruction notification; existing physics teardown deviation remains |
| DDS listener: executable RVA 82ddc0 and native quality | Desired mip rules before resource opens; native listener after fingerprint validation | Resource caller; copied u32 mip selection; native shared stream replacement; failure rewinds/falls back; retained trampolines | `texture.dds-mips`, `texture.dds-policy` @1. Independent selection works with built-in optimization off. Native differential fixture covers quality levels and allocator ownership |
| FileSystemArchive::open, SIC decode, Ogre Codec registry | Desired selection/decode/codec declarations before provider activation and native lookup | Resource loader; codec vtables pinned, encoded bytes copied, image sizes/formats validated, Ogre allocates streams/ImageData | `texture.select`, `texture.decode`, `texture.sidecar-policy`, `texture.codec-claims`, `texture.codec-decode` @1. Custom `codecs::Codec` demonstrates independent native decoding. Late codec declarations require restart; removal immediately revokes decoding, native slots retire at provider shutdown. Loaded textures are not rewritten |
| ParticleUniverse queue and Ogre billboard injection/dimensions | Desired styles/appearance rules in preparation; fingerprinted renderer bindings before matching queues | Native renderer; scoped TLS, copied initialized billboard scalars, camera fade and default dimensions restored on unwind. No emitter creation or simulation writes | `particle.appearance-policy`, `particle.rain-style`, `particle.billboard` @1. RainRendering uses public composed rules; built-in tone-down uses public policy. Native fixtures verify source preservation and unrelated-effect bypass; no hierarchy-ready claim |
| MyGUI initialize/shutdown, createSkin, SkinItem colour and EditText colour/destructor | Host prepares native plumbing, then activates coordinated hooks; desired themes register before that transaction | UI caller only; provider keeps native borrows in TLS, removes them before teardown/address reuse, restores native colours on disable and clears epochs on stop | `ui.theme-policy`, `ui.skin-policy`, `ui.colour`, `ui.widget-observed` @1. Classification and minimum brightness can be overridden using copied skin metadata. No verified arbitrary ResourceSkin lookup/replacement API is claimed |
| Recognized HDR lighting then LDR tonemap resolve | Desired PostProcess rules before first eligible scene pass; COM resources created there | Render caller; copied HLSL/constants, provider-owned effects and COM state; descriptor/device changes rebuild resources; removal retires copies on next draw or stop | `render.post-tonemap-effects` @1, owned PostProcess, 16 finite parameters. Supports the verified post-tonemap stage only. Format heuristic is not general renderer lifecycle evidence; other stages/reflection/history remain absent |
| Gameplay post-update, native reset/clear/destruct and command dispatch | Executable/prologue verification before enabling hooks; roster availability only after actual capture | Game lane; copied snapshots and world-scoped IDs; invalidate before native reset; queued operations reject when stale | Existing Gameplay snapshot/task service plus `gameplay.roster-changed`, `gameplay.command-completed` @1. `src/gameplay/native.rs` and gameplay-native research; no save/load phase or navigation-success inference |
| Ogre singleton and MyGUI frame observers | Native targets before dependent subscriptions | Synchronous caller/UI lane; managed callback groups quiesce, owner panic requests restart | Existing managed low-level observations remain; migrated ordinary policies do not use them as world readiness or a generic scheduler |
| Color-grade/Kenshi2 style and rain refinement | Owned rules registered during preparation, independent of provider availability | Effects/styles contain copied desired state; provider handles native resources and generation changes | Samples use public PostProcess/RainRendering. DLL builds in `compat/` now use export_plugin and the current preparation contract; directory names do not imply released compatibility obligations |
| Source-transform/runtime rule sample | Preparation before native use; reports missed compiles, rejects late codec changes | Local owned registrations and decoder; copied inputs/results only | `samples/source-transform` demonstrates shader policy and opt-in terrain, mip, collider, UI, particle, codec, identity/draw and postprocess rules through an independent DLL |

## Activation and remaining evidence blockers

The global barrier spans participating built-ins and all currently discovered DLLs.
It cannot observe game bootstrap before host startup or uncoordinated third-party
native loading. `late()` detects earlier broker-observed uses only. Shader transforms
and identity selectors cannot retroactively change compiled/created shaders. Codec
activation consumes declarations once before lookup; late changes explicitly require
restart. Eager terrain copying now occurs on the next eligible read after a preference
change, and may pause that caller while copying.

Specific unsupported bindings remain: verified full world loading phases, native
save/import identity and completion, physics SDK pre-destruction, device loss before
resource teardown, UI/particle hierarchy boundaries, arbitrary native reskinning,
and additional postprocess stages. No placeholder contracts advertise these. FCS
and asset tools retain definition/asset authoring responsibilities. Time, spawning,
AI and persistence in the broader proposal are not implemented by these seams.

## Validation and guideline limits

On 2026-10-08, the complete `scripts/verify.ps1 -KenshiDir
'D:\SteamLibrary\steamapps\common\Kenshi'` workflow exited successfully, including
formatting, warnings-denied Clippy, workspace tests, documentation, release builds,
installation tests and the native/independent-DLL fixtures below. The retained run
log is `target/platform-verification-final.log`. It includes the regression for
retiring a callback-faulted owner at serial dispatch without pending events.

The verification workflow is `scripts/verify.ps1 -KenshiDir ...`. Behavioral broker
tests cover pending first use, late registration, semantic order/cycles, stable
policy manifests, composition failure, exclusive conflicts, compatible recreation,
stale generation rejection, event/lifecycle overflow, owner authorization, reentry,
panic quarantine and in-flight shutdown. Installation tests check desired registration
before any provider and cleanup after failed registration.

Native scripts exercise installed compiler/Ogre/MyGUI/ParticleUniverse/terrain/PhysX
binaries, real native allocation/adoption/release, original-operation fallback,
settings and restart. Independent DLL cases cover copied terrain opens/reads, DDS,
colliders, custom codec pixels, UI colours/skin lifetime events and particles.
`plugins/graphics/tests/public_gpu.rs` uses the real loaded host and independent
policy DLL to verify scoped constants, pixel output, postprocess output and two
devices. Existing GPU cases retain built-in AO pixel/state validation. None of
these fixtures establishes loaded-world save/import behavior or universal hardware
support; in-game visual QA and performance capture remain necessary.

Retained guideline deviations: the safe registry uses pointer-free C descriptor
records and transport statuses rather than a separate native core/error model
(M-FFI-TRANSLATES, M-ERRORS-CANONICAL-STRUCTS); SDK clients receive structured Error.
Process-pinned native detour/codec state and DLL-local guards retain the existing
M-AVOID-STATICS exception. Panic containment requires restart (M-PANIC-CONTINUATION).
Allocator isolation and documented unsafe lifetimes apply M-ISOLATE-DLL-STATE,
M-UNSAFE and M-UNSOUND. Behavioral tests apply M-TAUTOLOGICAL-TESTS. This is not a
complete workspace guideline audit; Miri does not validate Windows detours or COM.
