# Choosing FCS, content tools, or Rust plugins

Status: product scope and proposed author workflow, 2026-10-08. Rust capabilities
below describe the [target platform](plugin-platform-spec.md), not a claim that
those native adapters ship today. See the [example API](plugin-platform-api.md)
and [current SDK](plugin-sdk.md) for that distinction.

## The decision rule

**Use existing content tools when their definitions and supported rules express
the intended change faithfully. Use Rust when the change needs a new runtime
operation, algorithm, interception point, or source of state. Use both when new
content needs new behavior.**

This is a division of responsibilities, not a restriction preventing expert
plugins from implementing native extensions. The baseline SDK should make
runtime extension excellent rather than duplicate content editors.

FCS authors data the game interprets during play. It already supports some
conditional/reactive behavior; dynamic behavior alone is not a reason to require
Rust. Lo-Fi's patch archive records world-state-driven town/squad changes,
dialogue actions and exposed tuning constants. These establish existing
mechanisms, not the availability of every condition/action in the installed FCS.
[Lo-Fi patch archive](https://lofigames.com/patch-notes/).

## Responsibilities

| Tool/workflow | Owns | Reach for it when |
| --- | --- | --- |
| FCS/data mods | Definitions, default properties, references and supported declarative rules | The game exposes the required record, condition, action or constant |
| Existing world/layout tools | Authored placements/layouts and supported map editing | The result is a designed location, interior or persistent authored arrangement |
| External asset tools and existing content workflow | Meshes, textures, skeletons, animations and exported files | You need new art or changes to an asset itself |
| Rust behavior plugins | Live observations, operations, algorithms/state, events, interception and rendering control | Existing data/rules cannot express the behavior or live control |
| Rust native providers | Verified engine bindings and new public capabilities | An engine operation is absent from the current SDK |

FCS is not a 3D modeling application. Lo-Fi's developer guide points to external
modeling/export tools; its animation guide describes linking exported animation
files through FCS records. These historical sources establish the separation;
they are not current tool-installation instructions.
[Developer mod guide](https://www.lofigames.com/phpBB3/viewtopic.php?t=448),
[developer animation guide](https://www.lofigames.com/phpBB3/viewtopic.php?p=69189).

## Decision process

1. State the result, including who it affects and when it changes. Distinguish
   changing a default from changing one existing instance.
2. Check the installed FCS/data schema for the complete rule: conditions,
   actions, lifetime and save/load behavior. A plausibly named field is
   insufficient if unused or semantically different.
3. If supported, author the data/rule there. A DLL adds no value merely by
   rewriting the same default every update.
4. Otherwise, isolate the missing behavior in Rust and reuse the definitions.
   Identify required SDK capabilities; missing ones need verified providers.
5. If content and behavior are both new, package a data mod and plugin with
   explicit references/dependencies and clear ownership of each property.

Do not declare a category impossible in FCS because one implementation was
inconvenient. Conversely, do not require hundreds of fake records, unrelated
faction flags or fragile side effects to emulate a rule that a supported native
extension can express directly. Document the concrete limitation addressed.

## Examples by requested result

The FCS column is a workflow recommendation subject to the installed schema;
the Rust column describes proposed platform scope requiring a verified provider.
This table does not assert that every illustrated native operation exists today.

| Desired change | Preferred workflow | Where Rust contributes |
| --- | --- | --- |
| Add a sword with authored defaults and art | Asset tools as needed, then FCS | No DLL required just to define the sword |
| Make damage depend on custom fatigue history | FCS definition plus Rust if existing rules cannot express it | Track history and intercept the verified damage stage |
| Change an exposed global damage/XP constant | FCS | Only for live/conditional or otherwise unsupported control |
| Define an NPC's race, name, equipment and supported AI package | FCS | No DLL required just to define the NPC |
| Add a town resident or regional squad through supported population rules | FCS plus layout tools where needed | No DLL required solely because NPCs appear during play |
| Spawn an NPC beside a selected character after a new custom encounter rule | FCS template plus Rust | Evaluate the rule, choose a live location and create a native instance |
| Distribute an item through an existing supported dialogue action | FCS | No DLL required for the existing action |
| Transfer a specific live item between containers under a new rule | Existing definition plus Rust where data actions are insufficient | Resolve instance IDs, capacity and native transfer |
| Write dialogue using existing conditions/world states | FCS | No DLL required just for conditional dialogue |
| Make dialogue depend on a new plugin-owned counter | FCS text/tree plus a verified Rust bridge | Persist the counter and expose a condition/action contract |
| Configure a supported world-state town takeover | FCS | No DLL required for an expressible takeover |
| Drive town behavior using a new live supply simulation | FCS town/faction content plus Rust | Maintain custom state and invoke supported world operations |
| Add building art or a designed interior | Asset, FCS and layout tools | No DLL required just to author the content |
| Place an interactive instance at a runtime-selected position | Existing definition plus Rust if no suitable data rule exists | Creation, ownership, collision/navigation and persistence |
| Substitute an asset/material through supported content references | Existing content workflow | No DLL required solely for authored substitution |
| Change material parameters by live state, add postprocessing or replace a matched shader | Rust rendering plugin/provider | GPU control, matching, resource contracts and restoration |
| Hold visual sunset while simulation continues | Rust clock plugin/provider | A verified visual-time override independent of calendar time |
| Add a live inspector, custom automation or engine cache | Rust plugin/provider | Runtime functionality beyond existing content rules |

Defining squads/dialogue does not imply arbitrary immediate spawning or arbitrary
scripting. Check the specific condition, operation, phase and persistence need.
Entire categories such as items, AI and dialogue do not belong exclusively to
either tool.

## Definition versus instance

**Definition:** an authored kind of content, its defaults and references. This
normally belongs in FCS and its content package. A display name is not an identity.

**Instance:** one item/stack, character or placed object in a world. Its owner,
quantity and state can change during play. Rust can act on typed instance IDs
when existing data actions are insufficient.

**Runtime rule:** a decision about when an operation happens or how an engine
result is calculated. Use FCS when supported conditions/actions express it;
use Rust for missing logic or state.

Example: author a medical kit in FCS. An existing dialogue action may distribute
it with no DLL. A custom squad-rescue plugin instead reads injuries, chooses a
medic, and transfers a particular kit when its own policy requires it. The
plugin reuses the definition; it does not import a mesh or redefine the item.

The same distinction applies to NPC templates/spawned characters, building
definitions/placed buildings, and authored AI packages/new decision algorithms.

## Combined mod contract

```text
desert-rescue/
  content/
    desert-rescue.mod           # Definitions and supported data rules
    ...                        # Assets referenced by those definitions
  plugin/
    desert_rescue.dll           # New runtime rescue policy
    rekenshi.toml               # Capabilities and content dependencies
    assets/shaders/...         # If it also supplies shader behavior
```

This is a conceptual distribution layout, not a new game load path. Content
installs through the existing game mod workflow; the DLL uses the host plugin
workflow. A future package tool can install both, but DLL discovery does not
load `.mod` files.

The proposed integration must:

- Resolve definitions by verified record identity/type and data-mod dependency
  where needed. Names/filenames alone are insufficient; aliases map to references.
- Read effective definitions after data-mod load order, preserving compatibility
  patches. Data-mod order and plugin/provider order are separate graphs.
- Explain missing/disabled/incompatible content before native creation.
- Distinguish FCS defaults, reversible live overlays and permanent instance
  mutations. Revocation reveals the current underlying value, not a stale copy.
- Distinguish plugin preferences, per-save plugin state and native save records;
  declare which system persists each property/reference.
- Avoid writing `.mod`/base data files during ordinary runtime operations. An
  instance edit must not accidentally change all instances of its definition.
- Define behavior when the DLL or content part is missing, including save
  references and safe deactivation of dependent behavior.

The existing KenshiLib World States example combines FCS content with a native
plugin to add persisted variables and conditions. It demonstrates an integration
pattern, not compatibility with this SDK/current executable or an implemented
bridge here. [First-party example source](https://github.com/BFrizzleFoShizzle/KenshiLib_Examples/blob/master/WorldStates/README.md).

## Product scope and acceptance

The baseline platform provides typed definition lookup, instance operations,
events/interception, clock/render control, state and native provider facilities.
Authors do not need to recreate their content as Rust structs or replace FCS.
Dialogue/condition bridges are separate capabilities requiring verified adapters;
unknown bridge versions reject explicitly rather than alter dialogue silently.

Every feature/example must identify:

1. What is authored in the content tools.
2. What exceeds existing rules and belongs in Rust.
3. Which loaded records and native capabilities it requires.
4. Which defaults, instances, overlays and saved state it changes.

Release checks include a data-only mod with no DLL, a plugin-only mod reusing
vanilla definitions, and a combined mod with authored definitions/new behavior.
Test missing dependencies, reordered data mods, save/reload, import and removal
of either part. Historical documentation is not a complete current FCS feature
audit; exact behavior needs installed-schema inspection and game validation.
