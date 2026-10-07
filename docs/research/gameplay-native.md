# Steam 1.0.68 gameplay adapter evidence

Date: 2026-10-06. This records **static observations**, not an observation of a
running saved world. Runtime smoke testing is a separate check.

## Identity and method

The supported root executable is
`D:/SteamLibrary/steamapps/common/Kenshi/kenshi_x64.exe`:

- SHA-256: `a596ab4e407c67b58599c54ffb32dc1bf2b64510cdebd3fa9359ef05a576aeb1`.
- MD5: `8a03c256f0da1555d9cceb939b41530a`.
- PE timestamp: `0x6602d59d`; image size: `0x232d000`; x86-64.
- Preferred image base: `0x140000000`. All addresses below are **RVAs**, added
  to the loaded executable base, so ASLR is supported.

REA's native providers were unavailable: Ghidra lacked configuration/runtime;
the other providers did not provide a usable Windows native session. No binary
session was opened. Existing `target/re-tools/pefile.py` and Capstone were used
to read and disassemble files without executing the game.

The installed `KenshiLib.dll` SHA-256 is
`10a7a7b8a5feef831f39c45bbf2246af327f3560b20f1da376477c4ae6057bd7`.
Its exported functions are six-byte indirect jumps from `FUNC_BEGIN` RVA
`0x156d6`. For an export RVA `e`, its legacy function index is
`(e - 0x156d6) / 6`. The corresponding little-endian u32 in installed
`RE_Kenshi/RVAs/Steam_1.0.65.br` identifies a routine in the compatible legacy
`RE_Kenshi/kenshi_x64.exe` (SHA-256
`504b362cde850d56afb1cea6f5b7b0ee014d9dd7b47e188599d91c804502cd3e`).

These legacy addresses do **not** map to the root executable. Functions were
mapped to the root by comparing decoded instructions while masking relocated
RIP references, relative branches, and relocated switch tables. Candidate
bodies were then inspected for concrete arguments, offsets, and call targets.
The current reference source's generated symbol indices also differ from the
installed DLL, so source indices were not used as the final address authority.

The adapter hashes the loaded executable's file, checks exact in-memory hook
prologues, and uses the resulting fixed root addresses. It does not require
KenshiLib initialization, exports, a legacy launch, or version overrides.

## Native routines

| Meaning | Legacy RVA | Verified root RVA | Corroboration |
| --- | --- | --- | --- |
| `PlayerInterface::update()` | `0x800a80` | `0x801630` | Called by root `GameWorld::mainLoop_GPUSensitiveStuff` at `0x788d9b`; caller loads `this->player` at `+0x580`. |
| `PlayerInterface::clearAndReset()` | `0x7f9940` | `0x7fa4e0` | At `0x7fa5aa` writes zero to roster count `+0x2b8`; clears current platoon next. |
| `GameWorld::resetGame()` | `0x36ca40` | `0x36cb80` | Calls player reset through thunk `0xcdc4` at `0x36ccf5`. |
| `GameWorld::_clearAndDestroyGameWorldStuff()` | `0x36b940` | `0x36ba80` | Calls player reset at `0x36ba9c`. |
| `PlayerInterface::~PlayerInterface()` | `0x7fa640` | `0x7fb1e0` | GameWorld destructor calls its thunk at `0x86d792`. |
| `GameWorld::~GameWorld()` | `0x86c780` | `0x86d650` | Releases player and zeros `world+0x580`. |
| const player roster getter | `0x7f18f0` | `0x7f2490` | `lea rax,[rcx+0x2b0]; ret`. |
| `Character::getPosition()` | `0x5cdbf0` | `0x5ce680` | RCX is character, **RDX is explicit result storage**; returns result pointer in RAX. Handles carried/unloaded positions, falling back to `+0x48/+0x4c/+0x50`. |
| `Character::getPlatoon()` | `0x790f70` | `0x791b10` | Reads `+0x658`; fallback resolves through native faction. |
| `PlayerInterface::isObjectSelected()` | `0x7f4180` | `0x7f4d20` | RCX player, RDX object; native bool return. |
| `Character::isUnconcious()` | `0x5c9380` | `0x5c9e10` | Includes medical `+0x5b9/+0x5bc` and native state checks. |
| `Character::isDead()` | `0x620b20` | `0x6215b0` | `movzx eax,byte ptr [rcx+0x5bc]; ret`. |
| `Character::getStats()` | `0xdee40` | `0xdee80` | Returns pointer at `+0x450`. |
| `ActivePlatoon::getName()` | `0x411d60` | `0x412100` | Reads platoon at `+0x78`, returns its string at `+0x18`. |
| `ActivePlatoon::getSquadSize()` | `0x419b90` | `0x419f30` | Reads u32 at `+0x58`. |
| `ActivePlatoon::getSquadLeader()` | `0x791ff0` | `0x792b90` | Checks designated leader `+0xa0`, then backup `+0xa8`, then selects fallback. Adapter reads designated leader to avoid this getter's fallback mutation. |
| `Character::addOrder(...)` | `0x5d1640` | `0x5d20d0` | Argument and handle-copy sequence described below. |
| `Character::setStandingOrder(...)` | `0x5c9fc0` | `0x5caa50` | RCX character, EDX enum, R8B on/off; switch accepts 0 through 17. |
| `PlayerInterface::stopCharactersMovement()` | `0x7f58d0` | `0x7f6470` | Per-character dispatch at `0x7f6589..0x7f65b3` described below. |

All hook methods have the `void(this*)` Win64 signature. Snapshot capture and
command dispatch run after player update's original body. Reset/clear/destructor
hooks invalidate IDs **before** invoking originals. A reset serial prevents a
player update that crossed a reset from publishing the old world again. No
native object address leaves the host API.

The update call comes from the game's main GPU-sensitive loop; native player
input commands also originate in player update. This establishes a much better
ownership point than a renderer callback or an arbitrary background thread.
Static inspection cannot itself demonstrate absence of every possible race in
all mods, or prove pathfinding completion after an order is dispatched.

## Copied layouts

The roster's `lektor<Character*>` at player `+0x2b0` has an eight-byte allocator
base, count u32 `+8`, capacity u32 `+12`, and array pointer `+16`. Root reset and
roster-use instructions corroborate player `+0x2b8` and `+0x2c0`. Count is bounded
to SDK `MAX_CHARACTERS`; every pointer/primitive copy uses ReadProcessMemory and
checks its complete byte count.

Character and Platoon derive from RootObjectBase. Their handle at `+0x58` starts
with a vtable; the identity tuple at `+0x60` is five u32 values: type, container,
container serial, index, serial. The root addOrder body copies these exact five
fields from a target at `0x5d2229..0x5d2248`. The host additionally assigns a
world generation and opaque IDs, invalidated on every reset/lifetime hook.

Names are read at RootObjectBase `+0x18`. **The actual native string is 32 bytes,
not the 40-byte size implied by some reconstructed headers.** At root
`RootObjectBase::getName` `0xd3ce0`, instruction `0xd3cf9` passes `this+0x18` to
the string copy routine via linker thunk `0x280a1`. Its body `0x69bc0` reads
source length at `+0x10` (`0x69bd4`), capacity at `+0x18` (`0x69c50`), and heap
pointer at `+0` (`0x69c57`) when capacity is at least 16. Capacity below 16 means
inline bytes. No C++ string is constructed or destroyed in Rust. Names are
bounded, copied, and truncated at UTF-8 boundaries.

Raw/base stats are read from CharStats offsets:

| Field | Offset | `StatsEnumerated` | Root getStatRef switch destination |
| --- | --- | --- | --- |
| strength | `0x80` | 1 | `0x88420a` |
| dexterity | `0x88` | 18 | `0x884252` |
| toughness | `0x90` | 21 | `0x88426a` |
| perception | `0x8c` | 24 | `0x88427a` |
| melee attack | `0x120` | 2 | `0x8841ea` |
| melee defence | `0x124` | 19 | `0x88425a` |

`CharStats::getStatRef` root `0x884190` uses jump table `0x8842cc`; each listed
destination explicitly returns the corresponding field. Values are **not**
adjusted for gear, injuries, or standing orders.

MedicalSystem begins at character `+0x458`. Blood is medical `+0x70`, hunger
`+0x60`: root medical GUI body `0x88a8f0` reads blood at `0x88aa69` and hunger at
`0x88dd11`; bloodloss update `0x646140` repeatedly reads/writes `+0x70`.
Anatomy count/array are `+0x198/+0x1a0`, independently read by the indexed
getPart root `0x2aa600` and limb getPart root `0x657590`. Each health part has
type at `+8`, side at `+0x20`, flesh at `+0x40`, stun at `+0x44`, jury rigging at
`+0x4c`. Root `updateDerivedHealths` `0x643c20` calculates
`flesh - fleshStun + juryRigging` before normalizing by maximum health.
The adapter exposes that effective current amount for head and identified
left/right arms/legs, plus blood and hunger. Missing anatomy has its validity
bit clear. Chest/stomach are deliberately unavailable: both use torso type,
and a robust distinction for custom race anatomy was not established.

Hunger uses the native raw units: multiply by 100 to compare it with Kenshi's
HUD nutrition number. During the parent task's manually launched root 1.0.68
smoke test, approximately `2.97` in the snapshot corresponded to `297` in the
game UI. This adapter preserves the raw value; consumers choose display units.

## Commands

`Character::addOrder` receives character in RCX, destination building in RDX,
TaskType in R8D, subject object in R9, then stack arguments shift, clear, and a
Vector3 reference. The root body reads those stack slots, copies subject handle
fields, and forwards to the character's AI task system. Move uses TaskType 29
(`MOVE_CUS_ORDERED`), null building/subject, shift=false, clear=true. Follow uses
TaskType 44 (`FOLLOW_PLAYER_ORDER`), a currently resolved player teammate as
subject, shift=false, clear=true. Queued orders therefore replace the actor's
current player order. Position components must be finite and at most 1e6 in
absolute magnitude; the game still determines reachability.

Stop reproduces the per-character sequence observed at root `0x7f6589`:

1. Read Character movement `+0x640`, invoke its vtable slot `+0x98` (halt).
2. Read Character AI `+0x650`, then task system `+0x20`; call linker thunk
   `0x49c15`, whose verified body is root `0x5074a0` (clears pending/current
   player tasks). The adapter calls the body.
3. Read Character body `+0x648`, invoke vtable slot `+0x60` (`_endAction`).

This preserves derived native implementations. Both virtual code pointers are
required to belong to the verified executable text region. Standing-order
mapping is block=11, hold=12, passive=13, ranged=17; sneak uses native enum3 for
on and enum4 for off. Targets and actors are resolved from the fresh roster on
the game update thread. Attack is not advertised: this service does not yet
resolve arbitrary nearby non-player targets into lifetime-safe entity IDs.

`APPLIED` means the native method was invoked, not that the character arrived,
stopped permanently, accepted a context-sensitive order, or finished combat.

## Verification boundary

Focused unit tests exercise native inline/heap string copying, UTF-8
truncation, malformed capacity and unreadable source pointers, anatomy validity
and effective health calculation, and rejection outside the update callback.
`cargo check -p rekenshi-rust --lib` and these focused tests passed. Full
workspace checks and live smoke observations are reported by the parent task.
Do not describe static mapping or synthetic-memory tests as a successful
live-world call. Other executable fingerprints fail closed until mapped and
verified separately.
