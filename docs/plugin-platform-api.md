# Proposed Rust plugin interface and examples

This is the example companion to the [platform specification](plugin-platform-spec.md).
All `rekenshi` interfaces, manifests, schemas, and commands below are **proposed**.
They are design examples, not code that builds with today's SDK. Domain-specific
capabilities remain unavailable until their native providers are verified. This
document does not supply working game addresses, vanilla record IDs, or shaders
for undocumented engine passes.

For interfaces that compile today, see the
[public operation/lifecycle implementation](plugin-platform-implementation.md)
and the [independent source-transform sample](../samples/source-transform/README.md).
The `rekenshi` sketches below remain proposed and are not aliases for that SDK.

Examples share one authoring model: install registrations once, keep ordinary
mutable plugin state, react to lifecycle/domain events, and let the SDK own registration,
task, and resource cleanup. Each numbered example is independent unless noted.

Item, object and NPC content comes from definitions already loaded by the game or
installed data mods. Mesh/definition authoring stays in the existing content
workflow; these examples instantiate and control that content at runtime. Custom
mesh import is outside the baseline SDK. Plugins can still package shaders,
configuration and inspection/binding profiles.

Use the [tool decision guide](modding-tool-boundary.md) before choosing a DLL.
FCS can already express some reactive rules. These Rust examples demonstrate
runtime extension where existing conditions/actions are insufficient; spawning
or conditional behavior alone does not make a plugin necessary.

## 1. Project and package shape

```text
my-mod/
  Cargo.toml
  Cargo.lock
  rekenshi.toml
  src/lib.rs
  src/behavior.rs
  assets/shaders/desert-grade.hlsl
  bindings/steam-profile.toml     # Only for mods supplying native bindings
  schemas/weather.rks            # Only for mods supplying a custom service
  tests/behavior.rs
```

The future released SDK must be usable without a host source checkout. Pin an
actual released SDK version in Cargo; do not copy a fictional version from this
document. Development against the repository can use a path dependency.

```toml
# rekenshi.toml -- proposed manifest format
[plugin]
id = "example.desert-tools"
name = "Desert tools"
version = "0.1.0"
protocol = "2"              # Proposed new protocol, not current HostV1
architecture = "windows-x64"

[capabilities]
required = ["world.clock.visual@1"]
optional = ["world.spawn.character@1", "render.postprocess.ldr@1"]

[assets]
root = "assets"

[state]
schema = 1
```

Protocol 2 here denotes a proposed separately negotiated extension; it does not
change frozen HostV1/PluginV1 records. Package identity is embedded by the export
macro and validated against the manifest. Asset paths are package relative.

```rust
// src/lib.rs
mod behavior;
rekenshi::export_plugin!(behavior::DesertTools, manifest = "rekenshi.toml");
```

## 2. Core interface shape

The minimum trait and representative registration signatures are:

```rust
pub trait Plugin: Default + Send + 'static {
    fn install(&mut self, install: &mut Install<Self>) -> Result<()>
    where
        Self: Sized;
}

impl<M: Plugin> Install<M> {
    pub fn on_world_loaded(
        &mut self,
        callback: fn(&mut M, &mut Game<'_>, &WorldLoaded) -> Result<()>,
    ) -> Result<()>;

    pub fn on_world_unloading(
        &mut self,
        callback: fn(&mut M, &mut Game<'_>, &WorldUnloading) -> Result<()>,
    ) -> Result<()>;

    pub fn on_event<E: GameEvent>(
        &mut self,
        callback: fn(&mut M, &mut Game<'_>, &E) -> Result<()>,
    ) -> Result<()>;

    pub fn on_settings_changed(
        &mut self,
        callback: fn(&mut M, &mut Game<'_>, &SettingsChanged) -> Result<()>,
    ) -> Result<()>;

    pub fn on_shutdown(
        &mut self,
        callback: fn(&mut M, &Shutdown) -> Result<()>,
    ) -> Result<()>;
}
```

These are signature sketches; method declarations in an `impl` are not standalone
Rust implementations. The export layer keeps Rust state/callbacks local to their
DLL. `Game<'_>` cannot be constructed by consumers or moved to another thread.
One plugin's serial callbacks never concurrently access its `&mut M`.

`GameEvent` identifies a provider-verified typed event contract, such as
`CharacterDamaged` or `SelectionChanged`. Register it with
`install.on_event::<CharacterDamaged>(Self::character_damaged)?`. Copied events
carry occurrence/world identity; delivery is on the serial game lane when safe.
An event describes something that happened; an interceptor changes the native
operation synchronously. Event payloads and callbacks remain local Rust values
behind the portable transport, not Rust types shared directly between DLLs.

The proposal has no public tick callback or scheduled-update API. Lifecycle
events establish when resources/world state are ready, specific events trigger
reactions, and task completion reports requested operations. Renderer/UI readiness
uses its own typed lifecycle contracts and scoped services. The host's internal
game-update hook executes queued work; authors do not subscribe to that loop.

The [public hook inventory and activation contract](plugin-platform-spec.md#public-hook-coverage-and-activation-contract)
defines the required coverage, including native loading/compilation hooks,
renderer/UI/particle lifecycles, entity/gameplay events and persistence phases.
Those registrations must be accessible to independent plugins and bundled
features alike; the small interface sketch above is not the complete catalog.

Choose the registration by what the mod needs to do:

| Author intent | Register during installation | Activation/execution |
| --- | --- | --- |
| Cache, transform or replace a native operation | Typed operation interceptor | Before its first relevant use; then synchronously at that operation |
| Keep an effect, theme or particle style enabled | Owned rule/effect handle | Provider activates when resources exist and handles recreation |
| Manage readiness-sensitive native resources directly | Provider-specific ready/invalidation callbacks with scoped lane access | At verified subsystem-generation transitions |
| Initialize or clear per-world behavior | World loaded/unloading/unloaded callbacks | Once per applicable world-generation transition |
| React to a gameplay change | Typed domain event subscription | At a documented safe delivery point after observation |

Ready is not synonymous with before-first-use. Source transforms and codecs must
be committed before compilation/loading begins. Ready subscriptions can deliver
the current subsystem generation to late subscribers, but interceptors cannot
replay earlier native operations. Registration reports whether existing resources
can be updated or reloaded, or whether a restart is required. Invalidation also
revokes access immediately; deferred notifications never grant access to a
destroyed native resource.

## 3. A complete plugin: keep visual lighting at sunset

This is a complete proposed plugin body, including settings and cleanup. Its
required capability must implement visual time independently of calendar time.

```rust
use rekenshi::{
    Game, Install, Plugin, Result, SettingsChanged,
    WorldLoaded, WorldUnloading,
};
use rekenshi::clock::{TimeOfDay, VisualTimeLease};
use rekenshi::settings::SettingKey;

#[derive(Default)]
pub struct DesertTools {
    enabled: Option<SettingKey<bool>>,
    sunset: Option<VisualTimeLease>,
}

impl Plugin for DesertTools {
    fn install(&mut self, install: &mut Install<Self>) -> Result<()> {
        self.enabled = Some(install.settings().toggle(
            "sunset",
            "Keep sunset lighting",
            false,
        )?);
        install.on_world_loaded(Self::world_loaded)?;
        install.on_world_unloading(Self::world_unloading)?;
        install.on_settings_changed(Self::settings_changed)?;
        Ok(())
    }
}

impl DesertTools {
    fn apply(&mut self, game: &mut Game<'_>) -> Result<()> {
        let key = self.enabled.ok_or_else(|| {
            rekenshi::Error::configuration("sunset setting was not installed")
        })?;
        let enabled = game.settings().get(key)?;
        if !game.world_status().is_ready() {
            return Ok(());
        }
        if enabled && self.sunset.is_none() {
            self.sunset = Some(game.clock().override_visual_time(
                TimeOfDay::from_hms(18, 30, 0)?,
            )?);
        } else if !enabled {
            self.sunset.take();
        }
        Ok(())
    }

    fn world_loaded(&mut self, game: &mut Game<'_>, _: &WorldLoaded) -> Result<()> {
        self.apply(game)
    }

    fn settings_changed(
        &mut self,
        game: &mut Game<'_>,
        _: &SettingsChanged,
    ) -> Result<()> {
        self.apply(game)
    }

    fn world_unloading(
        &mut self,
        _: &mut Game<'_>,
        _: &WorldUnloading,
    ) -> Result<()> {
        self.sunset.take();
        Ok(())
    }
}
```

This mod reacts to world lifecycle and settings; its owned lease supplies continuous control.
Setting keys are automatically namespaced by plugin ID. A visual lease is
world-scoped, owner-checked and exclusive unless the provider defines compatible
sharing. Acquisition can return `Conflict` naming another owner. Drop requests
revocation at the correct phase; the host also revokes it after plugin failure.
Native lighting resumes at its current time, rather than the installation hour.

## 4. Calendar edits, simulation speed, and pause

```rust
use rekenshi::clock::{CalendarJumpPolicy, GameTimestamp, SimulationRate};

// In a game callback; each operation requires its own capability.
let timestamp = GameTimestamp::from_day_and_time(
    42,
    rekenshi::clock::TimeOfDay::from_hms(8, 0, 0)?,
)?;
let task = game.clock().set_calendar(
    timestamp,
    CalendarJumpPolicy::RejectIfScheduledEventsWouldBeSkipped,
)?;
game.tasks().on_complete(task, |game, result| {
    let change = result?;
    game.log().info("calendar changed", &change);
    Ok(())
})?;

// Store these leases in plugin state while control should remain active.
let speed = game.clock().override_simulation_rate(SimulationRate::new(0.5)?)?;
let pause = game.clock().pause("Photo mode")?;
```

`CalendarJumpPolicy` values are supported operations, not options an adapter can
ignore. If the native scheduler cannot determine skipped events, the first
request fails as unsupported. Rate control does not promise an independent
calendar rate. Dropping `pause` must preserve player pause and other pause leases;
dropping `speed` releases this plugin's rate ownership. Neither task completion
nor a visual sky override implies a simulated passage through intervening days.

## 5. Queries, character orders, stats, and health

```rust
use rekenshi::character::{StandingOrder, Stat};
use rekenshi::ai::Order;
use rekenshi::query::CharacterQuery;

let selected = game.characters().query(
    CharacterQuery::player().selected().limit(32),
)?;

for character in selected.items() {
    if character.health().blood_fraction().is_some_and(|x| x < 0.25) {
        let task = game.characters().order(
            character.id(),
            Order::Standing(StandingOrder::Passive(true)),
        )?;
        game.tasks().on_complete(task, |game, result| {
            game.log().info("passive order dispatch", &result?);
            Ok(())
        })?;
    }
}

// Separate mutation capability; recalculates affected derived state.
let task = game.characters().set_base_stat(actor, Stat::Strength, 40.0)?;
game.tasks().on_complete(task, |_, result| { result?; Ok(()) })?;
```

`blood_fraction()` is optional and requires verified maximum/current semantics;
it must not normalize today's raw blood readings against a guessed maximum.
An actual rescue mod also tracks its previous order, applies only on transitions,
and restores under an explicit player/other-mod arbitration policy. This short
fragment demonstrates operations, not a complete rescue policy.

All observations are copied snapshots with a revision. `actor` must be a fresh
typed character ID obtained from a supported query. Foreign-world or streamed-out
IDs reject at dispatch. Setting a base stat differs from applying a reversible
modifier to an effective stat. Permanent mutations and owned modifiers use
separate methods; handle drop never reverses a permanent stat edit implicitly.

## 6. Place an existing item in the world

```rust
use rekenshi::spawn::{ItemSpawn, Persistence, Placement};

// The alias resolves a loaded game/data-mod item definition, not a mesh file.
let medkit = game.definitions().item_alias("starter-medkit")?;
let specification = ItemSpawn::from_template(medkit)
    .quantity(1)?
    .placement(Placement::project_to_ground(location))
    .persistence(Persistence::NativeSave)
    .build()?;
let task = game.spawns().item(specification)?;
game.tasks().on_complete(task, |game, result| {
    let items = result?; // Native item/stack IDs, including any stack splitting.
    game.log().info("world items registered", &items);
    Ok(())
})?;
```

The definition supplies the item's model and supported properties. Native world
registration supplies pickup/interaction and save behavior. Giving an item directly
to an inventory is a separate operation shown in section 10; transferring an
existing item must not create a second copy. Changing the item's content definition
or adding a new model is done through the content workflow, not this spawn call.

## 7. Interactive objects and buildings

```rust
use rekenshi::spawn::{ObjectSpawn, Persistence, Placement};

// "supply-crate" is an alias the mod maps to an existing data definition.
let template = game.definitions().object_alias("supply-crate")?;
let specification = ObjectSpawn::from_template(template)
    .placement(Placement::project_to_ground(location))
    .owner(player_faction)
    .persistence(Persistence::NativeSave)
    .build()?;

let task = game.spawns().object(specification)?;
game.tasks().on_complete(task, |game, result| {
    let object = result?;
    game.log().info("interactive object registered", &object);
    Ok(())
})?;
```

The completed ID denotes a native game object, not a scene node. The template's
supported behavior determines collision, storage, interaction, and placement
constraints. Building placement is a separate builder with settlement ownership,
blueprint/completed state, terrain and navigation policy. It must not equate
adding a collider with updating the game's navigation system.

## 8. Spawn an NPC and a squad

```rust
use rekenshi::spawn::{CharacterSpawn, Persistence, Placement};

let template = game.definitions().character_alias("desert-guide")?;
let faction = game.definitions().faction_alias("guide-faction")?;

let specification = CharacterSpawn::from_template(template)
    .name("Nara")
    .faction(faction)
    .placement(Placement::reachable_near(location, 10.0)?)
    .persistence(Persistence::NativeSave)
    .build()?;

let task = game.spawns().character(specification)?;
game.tasks().on_complete(task, |game, result| {
    let npc = result?;
    game.log().info("guide ready", &npc);
    Ok(())
})?;
```

The aliases above are supplied by this mod's validated definition mapping, not
claimed vanilla Kenshi IDs. `CharacterSpawn` also supports capability-specific
race/body settings, AI package, equipment, inventory, base stats and squad
membership. Unsupported options reject before native dispatch where possible.

```rust
use rekenshi::spawn::{MemberSpec, SquadSpawn};

let plan = SquadSpawn::new()
    .name("Desert escorts")
    .faction(faction)
    .placement(Placement::reachable_near(location, 20.0)?)
    .member(MemberSpec::from_template(guide_template).leader())
    .member(MemberSpec::from_template(guard_template).count(2)?)
    .persistence(Persistence::NativeSave)
    .build()?;
let task = game.spawns().squad(plan)?;
```

The squad operation documents whether it is all-or-cleaned-up or returns partial
results with member failures. It must not claim general rollback if native
factories cannot provide it. NPC control requires actual native AI/task
integration, not filling an allocated character structure in Rust.

## 9. Create once across saves; explicitly despawn

```rust
use rekenshi::save::SaveKey;
use rekenshi::spawn::{MissingEntityPolicy, RemovalPolicy};

let key = SaveKey::new("guide-npc")?; // Namespaced to this plugin and save.
let task = game.spawns().ensure_character(
    key,
    specification,
    MissingEntityPolicy::ReportMissing,
)?;
game.tasks().on_complete(task, |game, result| {
    let outcome = result?; // Existing or newly created; includes CharacterId.
    game.log().info("guide resolved", &outcome);
    Ok(())
})?;

// Later, after an explicit mod action and with a verified removal provider:
let task = game.characters().despawn(
    npc,
    RemovalPolicy::RejectIfReferencedOrCarryingItems,
)?;
```

`ensure_character` creates only if this save has never recorded the key. If the
recorded character was killed, deleted or is unresolved, `ReportMissing` does not
respawn it. Separate explicit policies can recreate or retire a missing record.
An unloaded entity is not automatically missing. Saving/reloading returns an
existing character rather than cloning it. Dropping `CharacterId` never despawns.
No persistence support is advertised until save/import identity is verified.

## 10. Inventory, factions, AI, and custom gameplay

```rust
use rekenshi::inventory::{CapacityPolicy, ItemStack};
use rekenshi::ai::{Order, OrderConflictPolicy};

let medkit = game.definitions().item_alias("starter-medkit")?;
let task = game.inventory().give(
    npc,
    ItemStack::new(medkit, 2)?,
    CapacityPolicy::RejectIfFull,
)?;

// A second operation is submitted after item creation succeeds when order matters.
let task = game.ai().issue(
    npc,
    Order::Follow(player_character),
    OrderConflictPolicy::ReplaceCurrentPlayerOrder,
)?;
```

Each operation has independent capabilities and errors. `give` cannot silently
drop excess items on the ground. APIs for faction membership, recruitment, money,
construction, crafting, dialogue and research follow the same typed operation
model; they require their own native adapters and save semantics. Data-definition
registration is separate from changing a live instance.

## 11. Add a postprocess shader and adjust parameters

```rust
use rekenshi::render::{Effect, EffectSpec, Stage};

// During installation; may prepare while waiting for a render device.
let shader = install.assets().hlsl("shaders/desert-grade.hlsl")?;
let effect: Effect = install.render().effect(
    EffectSpec::new("desert-grade")
        .stage(Stage::PostTonemapBeforeUi)
        .shader(shader)
        .read_scene_color()
        .float("ExposureStops", 0.0)
        .float("Saturation", 0.85)
        .build()?,
)?;

// Store `effect` in plugin state, then apply on a settings/game callback:
effect.parameters().set_float("ExposureStops", 0.5)?;
effect.set_enabled(true)?;
```

This evolves today's LDR `PostProcess` into a named, reflected parameter layout.
It does not require a game world during installation. Changes publish immutable
parameter revisions; the render thread uploads them at a safe point. `Effect`
owns registration, not the underlying game device. Removal/Drop safely revokes it.

Example asset for this particular proposed fullscreen-pass ABI:

```hlsl
Texture2D<float4> SceneColor : register(t0);
SamplerState SceneSampler : register(s0);
cbuffer ModParameters : register(b0) {
    float ExposureStops;
    float Saturation;
    float2 Padding;
};

float4 main(float4 position : SV_Position, float2 uv : TEXCOORD0) : SV_Target {
    float4 color = SceneColor.Sample(SceneSampler, uv);
    color.rgb *= exp2(ExposureStops);
    float luminance = dot(color.rgb, float3(0.2126, 0.7152, 0.0722));
    color.rgb = lerp(luminance.xxx, color.rgb, Saturation);
    return color;
}
```

The stage descriptor must specify whether its sampled values are linear or
display-encoded; this sample's grading is illustrative in the declared sampled
space, not a claim of physically correct scene-linear exposure after tonemapping.
The proposed pass preserves input alpha. It is not a replacement for a game's
geometry shader and cannot access depth or normals by implication.

## 12. Adjust an existing material

```rust
use rekenshi::render::material::{MaterialSelector, ParameterOverride};

// Selector and parameter come from a verified provider/material inspection.
let selector = MaterialSelector::from_profile("desert-rock-materials")?;
let roughness = install.render().materials().override_parameter(
    selector,
    ParameterOverride::float("SurfaceRoughness", 0.7),
)?;
```

`SurfaceRoughness` is an example semantic parameter, not a claim that Kenshi
exposes that name. Inspection may discover no such parameter; the operation must
then fail. A material provider can map a semantic parameter to verified native
shader constants. Texture replacement, sampler overrides, and per-instance
material clones have separate validated descriptors. Store the returned override
to retain it; revocation reveals the current underlying material values.

## 13. Overwrite a native shader

```rust
use rekenshi::render::shader::{ReplacementSpec, ShaderSelector};

// Captured selector profile records original bytecode/stage/layout and scope.
let selector = ShaderSelector::from_profile(
    install.assets().file("profiles/terrain-lighting.shader.json")?,
)?;
let source = install.assets().hlsl("shaders/terrain-lighting.hlsl")?;

let replacement = install.render().shaders().replace(
    ReplacementSpec::new("terrain-lighting")
        .selector(selector)
        .source(source)
        .expected_matches(1)
        .preserve_input_output_contract()
        .preserve_resource_contract()
        .build()?,
)?;
```

This is complete substitution of a matched native shader, not another fullscreen
pass. The profile must cover actual variants; one observed shader may correspond
to several materials. Exact match count is relative to the profile's declared
capture domain/lifecycle, not a universal assumption that every shader is loaded
at startup. New variants remain inactive until validated. `replacement.status()`
reports waiting, matched, ambiguous, incompatible, conflicted, or active.

The safe provider only exposes replacement where interface and downstream
resource contracts are enforceable. Reflection checks do not establish shader
semantics. Unconstrained replacement uses an explicitly unsafe native provider.
Two exclusive replacements for the same scope return a conflict naming both mods.
Shader reload preserves the old shader when compilation or validation fails.

## 14. Transform shader source and add bindings: built-in AO parity

```rust
use rekenshi::render::shader::{SourceTransformSpec, TransformOrder};

let profile = install.assets().file("profiles/environment-lighting.transform.json")?;
let helper = install.assets().hlsl("shaders/ao-helper.hlsl")?;
let transform = install.render().shaders().transform_source(
    SourceTransformSpec::from_profile(
        "environment-ao",
        profile,
    )?
    .insert_hlsl(helper)
    .order(TransformOrder::before_shader_cache())
    .require_exact_anchors()
    .reserve_constant_buffer("AmbientOcclusion", 16)
    .build()?,
)?;

// Logical name maps to a validated free slot; no arbitrary universal b13 promise.
transform.parameters().set_float("Strength", 0.6)?;
```

The profile defines exact supported sources/variants and edits, marker placement,
resource slots, parameter layout and binding lifetime. The transformed source,
macros, compiler flags/profile and transform versions participate in the cache key.
Unknown source variants or occupied bindings do not get patched. Failure policy
is explicit: keep last valid version, use verified original source, or deactivate.
Multiple transforms revalidate anchors and interfaces against the evolving source;
priority alone cannot establish that two source patches compose correctly.

This must replace the special Rust-only route currently used by experimental AO.
An external AO DLL should be able to compile recognized variants, tag successful
outputs, bind parameters only to tagged draws, and restore bindings through public
facilities. Its native renderer provider can be a dependency; its behavior must
not require a private host import.

## 15. Depth-aware effects and render plans

```rust
use rekenshi::render::{EffectSpec, Resource, Stage};

let shader = install.assets().hlsl("shaders/contact-outline.hlsl")?;
let effect = install.render().effect(
    EffectSpec::new("contact-outline")
        .stage(Stage::BeforeTonemap)
        .shader(shader)
        .read(Resource::SceneColor)
        .read(Resource::Depth)
        .read(Resource::WorldNormals)
        .write_intermediate("outlined-hdr")
        .publish_scene_color("outlined-hdr")
        .build()?,
)?;
```

The manifest requires corresponding stage/resource capabilities. Resource
descriptors include encoding, color space, camera conventions and validity.
The graph publishes the named output as scene color for downstream stages, owns
copies/ping-pong targets and rejects unresolved resources/cycles.
Temporal effects additionally request bounded history with reset conditions.
This example is unavailable on today's color-only postprocess interface.

## 16. Intercept combat behavior and replace a decision

```rust
use rekenshi::combat::{DamageCalculation, DamageResult};
use rekenshi::hooks::HookOrder;

// Only a provider-verified transformable damage contract exposes this target.
install.hooks().transform_result(
    DamageCalculation,
    HookOrder::named("reduce-player-damage"),
    |call, result: &mut DamageResult| {
        if call.victim().is_player_owned() {
            result.scale_physical_damage(0.75)?;
        }
        Ok(())
    },
)?;
```

The provider defines whether this is raw damage, armour-adjusted damage, or a
different stage. Methods validate legal result ranges and preserve native
ownership. This callback must not capture `&mut self`; it can read a typed
published setting/configuration handle. It runs synchronously on the target's
declared lane and may be concurrent.

```rust
use rekenshi::ai::{CanStartAttack, AttackDecision};

let rule = install.settings().toggle("peaceful", "Peaceful player squad", false)?;
install.hooks().replace(
    CanStartAttack,
    HookOrder::named("peaceful-player"),
    move |call, original| {
        if rule.current()? && call.attacker().is_player_owned() {
            Ok(AttackDecision::Deny)
        } else {
            original.call(call)
        }
    },
)?;
```

The binding must explicitly permit skipping its original and returning Deny.
`original` is call-scoped and consumed when called; it cannot be cloned, stored,
or called twice. Replacement ownership is exclusive. Observation/transform
composition follows the binding's documented ordering. Constructor/destructor
targets do not inherit the ability to skip native execution.

## 17. Add a new native binding without modifying the host

```rust
use rekenshi::native::{BindingSpec, EntryPoint, NativeContract};

// Files contain the author's inspected build evidence; no real address is invented.
let profile = install.assets().file("bindings/steam-profile.toml")?;
let descriptor = BindingSpec::from_profile(profile, "vendor.example.custom-decision")?
    .entry(EntryPoint::verified_module_relative())
    .contract(NativeContract::from_profile())
    .build()?;

type NativeDecision = unsafe extern "C" fn(*mut std::ffi::c_void, u32) -> bool;

// SAFETY: The provider author must establish the profile's ABI, live object,
// thread, module identity, unwind policy and lifetime contracts before shipping.
let binding = unsafe {
    install.native().bind_function::<NativeDecision>(descriptor)?
};
```

`NativeDecision` is a hypothetical signature illustrating explicit unsafe
registration, not the signature of a discovered Kenshi function. A safe consumer
interface requires an adapter that proves its own invariants. Binding a function
does not permit safe callers to pass any object address or call it on workers.
An unsafe call method requires documented preconditions; public provider
facilities must permit implementing scoped safe domain operations on top.

The same public module offers managed function-entry detours, verified vtable
substitutions, and expected-byte patches. Raw native interceptors remain unsafe
where typed argument validation is impossible. Patch descriptors include module
identity, exact expected bytes, canonical ownership range, restoration checks,
and thread/quiescence contracts. Merely filling in a pattern is not verification.

## 18. Provide a service from one plugin to another

Proposed small service schema:

```text
service org.example.weather.control version 1 {
    lane game;
    message RainStyle { visibility: f32; width_scale: f32; }
    command set_rain(style: RainStyle) -> unit;
    query status() -> WeatherStatus;
}
```

The generator emits local Rust clients/handler traits plus portable C transport.
Imports below refer to generated bindings, not shared Rust objects across DLLs.

```rust
// Provider installation:
install.services().provide::<weather::ControlV1>(WeatherProvider::new())?;

// Consumer installation:
let weather = install.services().require::<weather::ControlV1>()?;

// Consumer game callback:
let task = weather.set_rain(game, weather::RainStyle {
    visibility: 0.35,
    width_scale: 0.2,
})?;
```

Both plugins can build independently. Rust traits, allocations and closures stay
in their DLL; only encoded portable records/handles cross. `require` negotiates
the schema/version and dependency. Provider shutdown revokes the client; calls
return provider unavailable instead of using freed state. Native provider
publication is unsafe where it establishes invariants used by safe consumers;
ordinary portable application-service registration need not be unsafe.

## 19. Workers, UI, input, and audio

```rust
// Copy an observation before scheduling work; never capture Game or native views.
let snapshot = game.characters().query(query)?.into_owned();
let task = game.workers().spawn("analyze-squad", move |cancel| {
    analyze_owned_snapshot(snapshot, cancel)
})?;
game.tasks().on_complete(task, |game, result| {
    game.log().info("squad analysis", &result?);
    Ok(())
})?;
```

`Task<T>` supports completion callbacks on the serial plugin lane. Callbacks own
their captures and cannot retain `Game<'_>`. Cancellation is cooperative, tracked
by the SDK, and bounded shutdown reports uncooperative work without freeing its
code. A worker may submit a queued command through a thread-safe client, but
cannot obtain the scoped game token synchronously.

```rust
// During installation; these are distinct optional provider capabilities.
let panel = install.ui().panel("squad-tools", "Squad tools")?;
let binding = install.input().action("photo-mode", "Toggle photo mode")?;
let sound = install.audio().sound("sounds/beacon.ogg")?;
```

Panel creation does not expose persistent native MyGUI pointers; UI lifecycle
events rebuild views. Input actions declare focus and consumption rules and can
conflict with other actions. Audio assets require an actually supported decoder/
backend; a `.ogg` path in this example is not proof of engine loading support.
These handles configure resources; typed UI/input/audio callbacks provide their
own lane tokens and queue game operations rather than calling native game state.

## 20. Plugin save state and migration

```rust
// During installation. `EncounterState` supplies a bounded codec and migrations.
let state = install.save().register::<EncounterState>("encounter", 2)?;

// In a save-aware game callback:
let mut encounter = game.save().read(state)?;
encounter.times_met += 1;
game.save().write(state, &encounter)?;
```

Save codecs return owned local Rust values and portable bytes at the DLL seam.
State fields containing entities use persistent references, not `CharacterId`.
Migration from schema 1 to 2 runs on a copy before replacing stored state. Writes
stage against the current save session; they do not claim an immediate native save
to disk. A settings file with an NPC pointer is never a substitute for save state.

## 21. Develop and test without launching the game

```rust
use rekenshi::testing::{ClockFixture, TestHost};
use std::time::Duration;

#[test]
fn sunset_override_preserves_calendar_and_releases_on_disable() -> rekenshi::Result<()> {
    let mut host = TestHost::new();
    host.provide_clock(ClockFixture::independent_visual_and_calendar());
    host.load::<DesertTools>("rekenshi.toml")?;
    host.open_world("desert-fixture")?;
    host.settings().set("example.desert-tools.sunset", true)?;

    let before = host.clock().calendar();
    host.step_simulation(Duration::from_secs(1))?;
    assert_eq!(host.clock().visual_time().hour(), 18);
    assert_eq!(host.clock().calendar(), before.advance_seconds(1)?);

    host.settings().set("example.desert-tools.sunset", false)?;
    host.step_simulation(Duration::from_secs(1))?;
    assert_eq!(host.clock().visual_time(), host.clock().native_visual_time());
    Ok(())
}
```

Other fake-host scenarios inject region-unloaded spawn failures, stale entity
generations, missing definitions, provider removal, full queues, settings rollback,
shader compilation failure, and save-schema migration. These tests validate mod
policy through its real facade. They do not validate a native clock setter,
constructor address, render-stage detector or save mapping in the actual game.

## 22. Development commands and inspection

All commands in this block are target product syntax, not today's command set.

```powershell
cargo rekenshi new plugin desert-tools --template gameplay
cargo rekenshi check --game-profile steam-local
cargo rekenshi test
cargo rekenshi build
cargo rekenshi deploy --game-profile steam-local
cargo rekenshi logs --follow
cargo rekenshi inspect capabilities
cargo rekenshi inspect entities --selected
cargo rekenshi inspect hooks --owners
cargo rekenshi capture shaders --game-profile steam-local
cargo rekenshi inspect shaders --profile profiles/terrain-lighting.shader.json
cargo rekenshi bindings check bindings/steam-profile.toml --game-profile steam-local
cargo rekenshi service generate schemas/weather.rks
cargo rekenshi package
```

Runtime inspection commands require a local versioned diagnostic connection to
the running host. Offline checks inspect files and manifests only; they cannot
enumerate live entities or promise callable ABI validation. Shader capture shows
source/bytecode availability honestly and never claims arbitrary source recovery.
Diagnostics are read-only by default; mutation experiments are explicit operations
on a selected development world. Shader/data reload is supported only where
prepared swaps exist. Rebuilding native Rust DLL code initially requires restart.

Today `cargo rekenshi new plugin` supports `callbacks` and `graphics` starters,
and the current workflow is documented in [the existing SDK guide](plugin-sdk.md).

## 23. The same code bundled or external

```rust
// Proposed composition interface for built-in packaging:
host.bundle::<WeatherPlugin>(include_str!("weather/rekenshi.toml"))?;
host.bundle::<DarkUiPlugin>(include_str!("dark-ui/rekenshi.toml"))?;

// Independent packaging, in each plugin's own DLL:
rekenshi::export_plugin!(WeatherPlugin, manifest = "rekenshi.toml");
```

The examples above are alternatives, not two registrations for the same plugin.
Host composition owns packaging only. WeatherPlugin and DarkUiPlugin consume the
same capability/provider interfaces, hook broker, configuration, tasks and scoped
tokens either way. First-party native providers can also be packaged separately.

Parity means actual independently compiled DLL fixtures preserve ordering,
settings, cleanup, rendering and native behavior. The cache/graphics/UI/weather
implementations must not bypass this with private host globals. See the acceptance
matrix and migration sequence in the [specification](plugin-platform-spec.md).
