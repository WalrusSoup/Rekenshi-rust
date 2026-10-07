# RE_Kenshi ecosystem: sample-port catalog

Researched **2026-10-06**. This is a source-backed catalog for designing Rust
samples and shared host capabilities. The rows describe existing authors' mods;
they do **not** mean these mods have been ported or that their DLLs work here.

## Coverage and popularity

“Top” means established, highly subscribed examples plus current plugins with
distinct API needs. The table below orders the inspected mods for which Steam
exposed useful subscriber counts. It is not a complete ecosystem ranking:
untagged mods, Nexus-only downloads, edition differences, and indirect dependencies
make that impossible to infer from one tag search.

Counts are the **visible page snapshots retrieved on 2026-10-06**. Steam/search
caches can lag; these are neither downloads nor independently audited live counts.

| Inspected mod | Visible Steam subscribers | Primary author page |
| --- | ---: | --- |
| Rotate Items | 84,737 | [Workshop](https://steamcommunity.com/sharedfiles/filedetails/?id=3691158692) |
| KenshiExtensionPlugin | 70,147 | [Workshop](https://steamcommunity.com/sharedfiles/filedetails/?id=3638161797) |
| Dust | 68,975 | [Workshop](https://steamcommunity.com/sharedfiles/filedetails/?id=3703458265) |
| BetterBuildingPreview | 36,509 | [Workshop](https://steamcommunity.com/sharedfiles/filedetails/?id=3715230445) |
| Heightmap Fix | 36,307 | [Workshop](https://steamcommunity.com/sharedfiles/filedetails/?id=3426253797) |
| Direct Control | 24,015 | [Workshop](https://steamcommunity.com/workshop/filedetails/?id=3737240806) |
| Fixed Skimmer Ragdoll Physics | 22,494 | [Workshop](https://steamcommunity.com/sharedfiles/filedetails/?id=3553086370) |
| Terrain Fog Overhaul | 18,346 | [Workshop](https://steamcommunity.com/sharedfiles/filedetails/?id=3642416193) |
| Choose Your Beginning Quicker | 16,888 | [Workshop](https://steamcommunity.com/sharedfiles/filedetails/?id=3726744964) |
| Attachment Optimization | 15,746 | [Workshop](https://steamcommunity.com/sharedfiles/filedetails/?id=3731036037) |
| Vital Sense | 14,882 | [Workshop](https://steamcommunity.com/sharedfiles/filedetails/?id=3678097690) |
| Re_Dev | 14,771 | [Workshop](https://steamcommunity.com/sharedfiles/filedetails/?id=3740289989) |
| Conquest | 12,666 | [Workshop](https://steamcommunity.com/sharedfiles/filedetails/?id=3750141111) |
| Cheat Menu | 12,086 | [Workshop](https://steamcommunity.com/sharedfiles/filedetails/?id=3706407417) |
| Organize the Crafting Stations | 9,498 | [Workshop](https://steamcommunity.com/sharedfiles/filedetails/?id=3688602277) |
| BFrizz's Extra Extensions | 8,475 | [Workshop](https://steamcommunity.com/sharedfiles/filedetails/?id=3625832102) |
| Character Highlight | 6,315 | [Workshop](https://steamcommunity.com/sharedfiles/filedetails/?id=3688641949) |
| Better Banking | 5,528 | [Workshop](https://steamcommunity.com/sharedfiles/filedetails/?id=3651996394) |

The accessible [RE_Kenshi Nexus page](https://www.nexusmods.com/kenshi/mods/847)
does not expose its inverse “mods requiring this file” list in the retrieved
document, and its download statistics are blank. The public Steam tag browse also
failed to load through the research tool. Consequently, **all Nexus dependents
and all top-ranked Workshop dependents have not been enumerated**. Individual
author requirements and source repositories below provide the dependency proof;
collection descriptions and user comments are discovery leads, not proof.

## What compatibility means here

This repository supplies its own Rust host SDK and one host DLL. Its external
`HostV1`/`PluginV1` protocol is **our protocol**, not RE_Kenshi's original plugin
loader or KenshiLib's C++ ABI. Similarly named version numbers do not establish
compatibility. Existing native mods below need a source port or a separately
implemented compatibility layer before their DLLs can be supported.

The current SDK has managed Ogre-singleton observer callbacks and MyGUI UI-frame
callbacks. These are not general gameplay or rendering events. Its safe
`PostProcess` interface supplies scene color after tone mapping and before UI;
it does not expose depth, normals, motion vectors, pre-fog lighting, world entities,
or arbitrary game actions. See [callbacks](../docs/plugin-callbacks.md),
[graphics](../docs/graphics.md), and [native symbols](../docs/native-symbols.md).
Export discovery supplies names, not verified object layouts or safe operations.

In the tables, **Needed capability** is our engineering inference from the
author's documented behavior. **Gap** means the feature is not currently exposed
by a comparable safe Rust API; it does not imply that the engine cannot support it.
Caches and the SIC codec solve different problems from mod-file routing.

## Rendering, assets, equipment, and controls

| Mod and primary dependency evidence | Documented behavior | Needed capability / current Rust mapping |
| --- | --- | --- |
| **Dust** — [author README](https://github.com/Bazouz660/Dust#installation) requires RE_Kenshi as its plugin loader. | Native lighting, shadows and post-processing through separately loaded effect DLLs. | Our scene-color `PostProcess` is a partial foundation. Gap: Dust's C API, effect loader, lighting stages, depth/G-buffer, temporal resources and shader-bytecode integration. Existing Dust effects are not binary-compatible. |
| **KenshiExtensionPlugin (KEP)** — [author Workshop](https://steamcommunity.com/sharedfiles/filedetails/?id=3638161797) requires KenshiLib 0.5.0 / RE_Kenshi 0.3.5 or newer. | Broad fixes and extensions; [feature wiki](https://github.com/Lucius64/KenshiExtensionPlugin/wiki/Features) includes equipment slots, crafting, animations, combat and damage calculation. | Gap: many typed gameplay interceptors and data-schema extensions. A callback to an Ogre getter cannot implement KEP. Split prospective Rust ports by feature. |
| **BFrizz's Extra Extensions** — [author Workshop](https://steamcommunity.com/sharedfiles/filedetails/?id=3625832102) requires RE_Kenshi 0.3.0+; mod authors also need FCS_Extended. | [Author wiki](https://github.com/BFrizzleFoShizzle/BFrizz_Extra_Extensions/wiki) documents dialogue conditions/effects and persistent variables used by dialogue/world states. | Gap: dialogue dispatch, variable persistence, world-state predicates and matching FCS definitions. Ordinary plugin configuration is not equivalent to save-state variables. |
| **BetterBuildingPreview** — [author Nexus page](https://www.nexusmods.com/kenshi/mods/1942) declares RE_Kenshi for file overrides. | Replaces `previewbuilding.material` with transparent fills and wireframe edges. | Gap: mod-aware hardcoded file override service. This is an asset/material sample, not a fullscreen post-process plugin. |
| **Heightmap Fix** — [author Workshop](https://steamcommunity.com/sharedfiles/filedetails/?id=3426253797) explicitly requires path overrides for the Workshop edition. | Repaired terrain pixels, holes, spikes and seams; separate [Nexus files](https://www.nexusmods.com/kenshi/mods/1573?tab=files) include a vanilla manual-install edition. | Heightmap-cache is related performance machinery, but does not prove override compatibility. Gap: select mod-provided heightmap by launcher order and route every supported loading mode consistently. |
| **Terrain Fog Overhaul** — [author Workshop](https://steamcommunity.com/sharedfiles/filedetails/?id=3642416193) requires RE_Kenshi path overrides. | Replaces/adds terrain fog volumes through `fogfeatures.dat`. | Gap: mod-file routing. Fog volume data is independent of rain billboards or color grading. Only one winning override can supply that file. |
| **Fixed Skimmer Ragdoll Physics** — [author Workshop](https://steamcommunity.com/sharedfiles/filedetails/?id=3553086370) declares RE_Kenshi required. | Stops the skimmer's excessive ragdoll launch. | Gap: verified ragdoll resource/physics integration. Collider-cache changes mesh cooking cost, not ragdoll behavior. Inspect the package before choosing override versus interceptor implementation. |
| **Rotate Items** — [author Workshop](https://steamcommunity.com/sharedfiles/filedetails/?id=3691158692) requires RE_Kenshi 0.3.1+; [source](https://github.com/jacobericson/KenshiRotate). | Rotates inventory items through a configurable key/middle mouse binding. | Gap: typed inventory drag, item footprint, layout and input operations. UI-frame observation alone does not provide an inventory handle. |
| **Extra Inventory Sections** — [author source](https://github.com/Genpretz/KenshiLib-Plugins) requires RE_Kenshi 0.3.0+; [Nexus](https://www.nexusmods.com/kenshi/mods/1838). | Adds/configures persistent Eyes, Hands and Neck equipment sections. | Gap: equipment-slot schema, inventory widget construction and equip/save semantics; not just additional checkboxes. |
| **Alternative Extra Inventory Sections** — [author Nexus](https://www.nexusmods.com/kenshi/mods/2175) declares RE_Kenshi required and says the original slot plugin is unnecessary. | Replaces the three narrow eye-related slots with a single larger eye slot. | Same equipment and inventory gap. Treat it as an alternative implementation, not a mandatory dependency of the original. |
| **Go Prone** — [author repository](https://github.com/Genpretz/KenshiLib-Plugins) declares RE_Kenshi 0.3.0+ for its plugins. | Lets the player enter a prone stealth-movement position. | Gap: typed input, animation/stance and locomotion operations. |
| **Character Highlight** — [author Workshop](https://steamcommunity.com/sharedfiles/filedetails/?id=3688641949) requires RE_Kenshi 0.3.4+ and links [upstream examples](https://github.com/BFrizzleFoShizzle/KenshiLib_Examples). | Colors highlighted characters by relations/aggression while the highlight key is held; changes shaders. | Gap: character relations and per-object material/shader integration. Scene-color processing cannot identify characters by faction. |
| **Adjustable Character Highlight** — [author Workshop](https://steamcommunity.com/sharedfiles/filedetails/?id=3770099634) requires RE_Kenshi 0.3.1+. | Fork adds configurable highlight radius and dedicated bindings. | Same gap as Character Highlight, plus a reusable settings/input service. Do not run both ports on the same material state. |
| **Attachment Optimization** — [author Workshop](https://steamcommunity.com/sharedfiles/filedetails/?id=3731036037) requires RE_Kenshi; Emkejs Mod Core is recommended. | Hides small equipment/attachment meshes past configurable camera distances. | Gap: typed attachment enumeration and visibility/distance policy, with restoration when near. Independent of shader/heightmap/collider caching. |
| **Direct Control** — [author Workshop](https://steamcommunity.com/workshop/filedetails/?id=3737240806) explicitly requires RE_Kenshi and links [source](https://github.com/smokefoolius/Kenshi-Direct-Control). | WASD movement, first/third-person camera and related controls. | Gap: native character movement, camera/head transforms, input ownership and UI capture. Needs several reusable services, not one generic `Fn()` observer. |
| **Freecam Rebuilt** — [author Workshop](https://steamcommunity.com/sharedfiles/filedetails/?id=3697462928) requires RE_Kenshi 0.3.2+. | Collision-free camera with adjustable movement, roll, smoothing and flashlight. | Gap: camera transform, game input and scene light services. Ogre exports are discovery material for those adapters. |

## UI, gameplay, and mod-development examples

| Mod and primary dependency evidence | Documented behavior | Needed capability / current Rust mapping |
| --- | --- | --- |
| **Organize the Crafting Stations** — [author Workshop](https://steamcommunity.com/sharedfiles/filedetails/?id=3688602277) requires RE_Kenshi; Mod Core integration is optional. | Search/filter crafting recipes and research blueprints. | UI-frame callback is usable for timing. Gap: safe existing-widget access, text-input events and recipe/research view models. |
| **Hidden Faction Relations** — [author Workshop](https://steamcommunity.com/sharedfiles/filedetails/?id=3699367182) requires RE_Kenshi; Mod Core is optional. | Options tab with searchable/sortable faction relations. | Our settings tab demonstrates native UI lifetime handling. Gap: faction enumeration and relation data; general table widgets are not yet a safe plugin service. |
| **Vital Sense** — [author Workshop](https://steamcommunity.com/sharedfiles/filedetails/?id=3678097690) requires RE_Kenshi; Mod Core is optional. | On-screen downed-state and bounty markers, optional character tint. | Gap: character state/bounty access, projection and safe marker widgets; tint also needs per-object rendering. |
| **Choose Your Beginning Quicker** — [author Workshop](https://steamcommunity.com/sharedfiles/filedetails/?id=3726744964) lists RE_Kenshi as required. | Replaces game-start selection with filters and sorting. | Gap: game-start data and a typed new-game window replacement. Avoid treating a frame callback as the complete UI API. |
| **SpeedTweaks** — [author Workshop](https://steamcommunity.com/sharedfiles/filedetails/?id=3728608989) declares a native RE_Kenshi plugin. | Configurable mining, research, production, farming, recovery and related rates. | Gap: typed return/value interceptors for each rate and explicit player/NPC scope. These change results, unlike our observers. |
| **Cheat Menu [WIP]** — [author Workshop](https://steamcommunity.com/sharedfiles/filedetails/?id=3706407417) requires RE_Kenshi. | Health/stats/faction editing, items, teleporting and recruitment. | Gap: entity selection, game actions and commands with object-lifetime checks. The common settings UI is only part of the work. |
| **Re_Dev** — [author Workshop](https://steamcommunity.com/sharedfiles/filedetails/?id=3740289989) requires RE_Kenshi 0.3.5+, KenshiLib 0.5.0+ and KEP 0.17.3+ preload core. | Spawn, dialogue/raid triggers, research/construction completion, camera teleport and NPC inventories. | Gap: shared gameplay services and KEP core compatibility. Build small diagnostic samples per capability before a full developer console. |
| **Emkejs Test Kit** — [author Nexus](https://www.nexusmods.com/kenshi/mods/1912) declares RE_Kenshi; links [source](https://github.com/Emkej/Emkejs-Test-Kit). | Debug panel with inspection, teleporting, spawning and construction completion. | Same gameplay/action gap. Useful reference for proving concrete entity operations and keeping engine work on its proper thread. |
| **Sightlines** — [author Nexus](https://www.nexusmods.com/kenshi/mods/2053) requires RE_Kenshi 0.3.4+. | Ground cones showing hostile vision range and alert state. | Gap: perception data and world-space geometry rendering. Post-processing the final color cannot supply those values. |
| **StatMan** — [author Nexus](https://www.nexusmods.com/kenshi/mods/2052) requires RE_Kenshi 0.3.4+. | Movable, configurable selected-character skill/status bars with per-character layouts. | Gap: selected-character/stat snapshots, safe overlay widgets and stable character identity. |
| **Kenshi Minimap** — [author Nexus](https://www.nexusmods.com/kenshi/mods/2014) requires RE_Kenshi to load its DLL. | Minimap UI, controls and persistent custom markers. | Gap: world/map coordinate services, selection snapshots, map texture/widget access and save-specific persistence. |
| **Japanese IME Input Support** — [author Nexus](https://www.nexusmods.com/kenshi/mods/1930) requires RE_Kenshi 0.3.3+ in its English requirements. | Sends committed IME characters into the active MyGUI text field. | Gap: Windows composition/input handling, focus access and Unicode text injection on the UI thread. |
| **Traders Refuse Irrelevant Items** — [author Nexus](https://www.nexusmods.com/kenshi/mods/2158) requires RE_Kenshi; links [source](https://github.com/Sinuboh/traders-refuse-irrelevant-items). | Rejects sales based on vendor stock rules with exceptions and dialogue feedback. | Gap: a typed sale-decision interceptor, vendor/item data and dialogue commands. |
| **SentientSands** — [author installation documentation](https://www.nexusmods.com/kenshi/mods/1872?tab=docs) requires RE_Kenshi and KenshiLib. | Adds AI-backed dialogue via a native plugin and external AI backend. | Gap: dialogue/entity context, command dispatch and owned asynchronous I/O. Rust can handle I/O, but it still needs verified native game operations. |
| **Hive Mind** — [author Nexus](https://www.nexusmods.com/kenshi/mods/2168) requires RE_Kenshi; Discord/relay is only needed for the shared feed. | In-game command console; optional shared events/chat. | Gap: entity commands, persistent state/events and UI. Keep optional network transport distinct from engine capability. |
| **Corpse Feeder** — [author Nexus](https://www.nexusmods.com/kenshi/mods/2125) requires RE_Kenshi; UWE is recommended, not required. | Player animals seek/consume bodies for nutrition then resume orders. | Gap: AI jobs, corpses, hunger and animation services; simulation update timing is required. |
| **Conquest** — [author Workshop](https://steamcommunity.com/sharedfiles/filedetails/?id=3750141111) identifies its implementation as built with KenshiLib and RE_Kenshi. | Claims settlements after defeat/clear conditions, with ownership restoration and persistent claims. | Gap: settlement/resident queries, faction ownership actions and save/load/import lifecycle. Loader dependency is evidenced by the author implementation statement rather than an explicit Requirements section. |
| **limbregen** — [author Workshop](https://steamcommunity.com/sharedfiles/filedetails/?id=3689978178) requires RE_Kenshi; FCS_Extended is required to edit its added dialogue definitions. | Surgeon dialogue restores missing flesh limbs; adds a missing-limb predicate/effect. | Gap: medical limb operations plus dialogue condition/effect registration. |
| **Wall-B-Gone** — [author Workshop](https://steamcommunity.com/sharedfiles/filedetails/?id=3665666467&searchtext=) requires RE_Kenshi; Mod Core UI is optional. | Hotkey invokes the game's dismantle action for supported player structures. | Gap: input, selected-building access and a typed native dismantle command. |
| **NeedSleep** — [author Workshop](https://steamcommunity.com/sharedfiles/filedetails/?id=3763358143) requires RE_Kenshi 0.3.1+. | Fatigue bar, penalties, collapse and optional autonomous sleep, saved with the game. | Gap: simulation updates, character state, stat/movement interception, jobs and save persistence. UI frame timing is insufficient as a simulation contract. |
| **BBgoat's Boundless Thirst** — [author Workshop](https://steamcommunity.com/sharedfiles/filedetails/?id=3766154146) requires RE_Kenshi and BFrizz's Extra Extensions. | Thirst, water/heat/shade behavior, drinking and health effects with HUD feedback. | Gap: gameplay state and BFrizz variable/dialogue extensions, in addition to UI. Requires the extension dependency, not only the loader. |
| **Better Banking** — [author Workshop](https://steamcommunity.com/sharedfiles/filedetails/?id=3651996394) requires RE_Kenshi and BFrizz's Extra Extensions. | Banker dialogues use custom persistent variables for balances. | Gap: BFrizz's dialogue/world-state variables and money operations. Author says importing resets balances; preserve documented lifecycle semantics in a port. |

## Related projects that are not unconditional runtime dependencies

- **KenshiFP / first person:** the [author README](https://github.com/linguine2552/KenshiFP/blob/main/README.md)
  describes both a standalone Ogre plugin and a separate RE_Kenshi edition. Only
  the latter requires RE_Kenshi. Both demonstrate camera/head transforms, input,
  animation and locomotion needs; neither is a scene-color shader effect.
- **FCS_Extended:** an [editor extension platform](https://github.com/BFrizzleFoShizzle/FCS_extended)
  with its own executable and managed plugin loading, not a runtime graphics
  plugin. [Author release notes](https://github.com/BFrizzleFoShizzle/FCS_extended/releases)
  cover additional definitions and Workshop extension discovery. BFrizz's and
  limbregen's authoring requirements are distinct from playing the installed mods.
- **ReShade presets, including the requested Kenshi 2 style:** a ReShade preset
  configures ReShade effects. It is not automatically an RE_Kenshi plugin.
  RE_Kenshi's [0.3.2 release notes](https://www.nexusmods.com/kenshi/mods/847)
  mention a proxying fix for ReShade, which demonstrates coexistence rather than
  a requirement that every preset uses RE_Kenshi. A Rust sample can reproduce
  selected scene-color operations through our graphics SDK; depth-based effects
  require additional resources. See the graphics sample documentation for the
  selected preset and exactly which operations were implemented.
- **Animated Main Menu — Sunrise Mountains:** the previously circulated
  [Workshop URL](https://steamcommunity.com/sharedfiles/filedetails/?id=3649686859)
  did not return a usable author description during this audit. Its requirement
  and current implementation are unverified here; it is not counted as a confirmed
  dependent just because an old collection called it RE_Kenshi-compatible.

## Turning these references into useful Rust samples

The best first ports exercise one shared capability each:

1. A scene-color graphics sample with explicit parameters and order, using
   `PostProcess`; it should identify its exact render stage and available inputs.
2. A mod-file override sample for a small material file. This unlocks a different
   class of mods from post-processing and should define precedence, validation and
   file paths once in the host.
3. A UI search sample with safe widget lifetime/focus handling and real data from
   the host; the plugin should only provide filtering and presentation.
4. An attachment visibility sample backed by typed camera/attachment snapshots,
   restoring the engine's original visibility policy when disabled.
5. A gameplay interceptor sample, such as a rate multiplier, with an explicit
   original-call/result contract. Observer callbacks should remain observers.

Implement the unsafe adapter centrally, then give samples owned values, typed
handles and callbacks. **There are no implemented ports of the catalogued mods in
this document.** Retain original attribution and inspect each author's applicable
source/asset license before adapting their implementation; documenting a behavior
does not copy its code or assets.

## Repeat this audit

1. Review the [RE_Kenshi author page](https://www.nexusmods.com/kenshi/mods/847)
   and its requirements/dependents panel when accessible. Record unavailable
   panels explicitly instead of treating them as empty.
2. Review the Steam RE_Kenshi tag sorted by subscribers, and the dependency pages
   for [KEP](https://steamcommunity.com/sharedfiles/filedetails/?id=3638161797)
   and [BFrizz's Extra Extensions](https://steamcommunity.com/sharedfiles/filedetails/?id=3625832102).
   Untagged mods need additional searches.
3. Open each author's own description/repository. Record exact required versions,
   optional companions, standalone editions, and source URLs. Collections can
   suggest candidates, but should not establish requirements.
4. Update subscriber snapshots and research date together. Keep rankings scoped to
   measured rows and avoid mixing Nexus downloads with Steam subscriptions.
5. Compare against the current SDK docs/source and mark capabilities separately
   from binary compatibility. A generated export constant is not a completed hook.
