# RE_Kenshi Rust modding

Language for content authored with Kenshi's existing tools and behavior supplied
by the Rust modding platform.

## Language

**Data mod**:
A Kenshi mod containing authored definitions and supported data rules, with any
associated content assets.
_Avoid_: DLL plugin, runtime plugin

**Definition**:
An authored description of a kind of game content and its default properties,
references and supported behavior rules.
_Avoid_: Live object, instance

**Template**:
A definition used as the starting content for creating an instance, such as an
NPC with authored race, equipment and AI choices.
_Avoid_: Instance, spawned character

**Instance**:
A particular item, character or world object that exists in a game world and has
its own identity and changing state.
_Avoid_: Definition, template

**Behavior plugin**:
A mod that supplies runtime logic or engine behavior through the Rust platform.
_Avoid_: Data mod

**Runtime rule**:
A rule evaluated during play that decides when a game operation occurs or how a
game result is calculated, whether authored in FCS or supplied by a plugin.
_Avoid_: DLL-only behavior

**Native provider**:
A plugin that supplies verified game or engine operations for other plugins to
use.
_Avoid_: Content package

**Combined mod**:
A mod whose authored content and runtime plugin cooperate to provide its intended
behavior.
_Avoid_: Plugin-only mod, data-only mod

**Live overlay**:
A reversible runtime override whose removal reveals the current underlying
value, including changes made while the override was active.
_Avoid_: Permanent edit, definition rewrite
