# Live gameplay SDK

`rekenshi_plugin_sdk::Gameplay` exposes owned squad snapshots and a queued command
interface. Plugin workers may read these snapshots and submit commands safely.
The host copies native state and dispatches queued commands at its game update
hook; plugin threads never receive character pointers or C++ containers.

```rust
use rekenshi_plugin_sdk::{Context, Gameplay};
use rekenshi_plugin_sdk::gameplay::{Availability, GameplayError};

fn inspect(context: &Context) -> Result<(), GameplayError> {
    let gameplay = Gameplay::connect(context)?;
    let status = gameplay.status()?;
    if status.availability == Availability::Ready && status.capabilities.roster() {
        let snapshot = gameplay.snapshot()?;
        for character in snapshot.characters {
            context.log(&format!("{}: {:?}", character.name, character.position));
        }
    }
    Ok(())
}
```

Check availability first: `NoWorld` means a world has not been captured;
`AdapterUnavailable` means the native adapter cannot provide data. `connect`
returns an error on older hosts without the optional service. Status messages
explain the current limitation. A snapshot error during a load transition is
normal: retry on a later poll.

The initial native adapter targets the Kenshi 1.0.68 executable with SHA-256
`a596ab4e407c67b58599c54ffb32dc1bf2b64510cdebd3fa9359ef05a576aeb1`.
It uses inspected executable addresses directly and does not require KenshiLib.
Other executable fingerprints report adapter unavailable. Capture and command
dispatch run after `PlayerInterface::update`; world reset/destruction hooks
invalidate IDs. These bindings have been inspected statically; a loaded-world
smoke test remains necessary before claiming live-game validation.

## Snapshot contents

Each `Character` includes an opaque `CharacterId`, name, optional XYZ world
position, optional squad (ID, name, size, leader), selection/death/unconscious
flags, core stats, and health. Stats include strength, dexterity, toughness,
perception, melee attack and melee defence. These are base/raw stats, without
equipment adjustments. Health includes blood, hunger, and head, chest, stomach
and limb values in native units, not normalized percentages. Each health value
is individually optional: animal and modded races may lack particular anatomy.
Optional fields are `None` when unavailable; a missing health field does not mean
zero health. `selected: Some(false)` differs from `selected: None`.

IDs are scoped to `snapshot.world`, which changes across world lifetimes. Discard
saved character and squad IDs when that generation changes. Never turn an ID
into a native pointer or persist it as a save-game identity. Snapshots are owned
copies and remain readable after the game changes. `sequence` identifies the
host capture; it is not a simulation timestamp.

## Commanding player characters

The interface includes `MoveTo`, `Stop`, `Attack`, `Follow` and `StandingOrder`
(passive, hold, block, ranged, sneak). Actual availability is determined by the
native adapter's capability bits. Always check `capabilities.supports(&command)`;
the host also rejects unsupported requests. Commands use explicit character IDs
from the roster and do not require changing the UI selection.

The initial adapter supports movement, stop, following a roster teammate, and
standing orders. `Attack` is represented in the SDK but is unsupported by this
adapter. The roster contains player characters; it is not a nearby-enemy query.

```rust
use rekenshi_plugin_sdk::Gameplay;
use rekenshi_plugin_sdk::gameplay::{
    Command, CommandResult, CommandTicket, GameplayError, Position,
};

fn move_first_character(
    gameplay: &Gameplay,
    destination: Position,
) -> Result<Option<CommandTicket>, GameplayError> {
    let snapshot = gameplay.snapshot()?;
    let Some(character) = snapshot.characters.first() else { return Ok(None); };
    let command = Command::MoveTo { character: character.id, position: destination };
    if !snapshot.capabilities.supports(&command) { return Ok(None); }
    gameplay.enqueue(command).map(Some)
}

// Call once per worker iteration, after checking its shutdown signal.
// Keep `ticket` in worker state; never wait for completion during shutdown.
fn poll_command(
    gameplay: &Gameplay,
    ticket: &mut Option<CommandTicket>,
) -> Result<(), GameplayError> {
    let Some(active) = *ticket else { return Ok(()); };
    match gameplay.take_result(active) {
        Ok(CommandResult::Pending) => {}
        Ok(CommandResult::Applied) => *ticket = None,
        Ok(CommandResult::Rejected { code, message }) => {
            *ticket = None;
            eprintln!("command rejected ({code}): {message}");
        }
        Err(error) => {
            *ticket = None;
            return Err(error);
        }
    }
    Ok(())
}
```

In a plugin, retain tickets in worker state and poll between shutdown checks.
`Applied` means the native call was dispatched; it does not promise the character
reached a destination or finished combat. A successful enqueue is not execution.
Stale worlds, invalid characters and unsupported operations are rejected. The
host bounds queued commands plus uncollected results to 128 per plugin. A terminal
`take_result` consumes the result and frees its slot; `Pending` retains it.
Poll every ticket until terminal, and stop polling once consumed.
Commands waiting over five seconds are rejected as expired when the game next
processes its queue. A paused or stalled update can therefore leave a ticket
pending until updates resume. Stop workers on plugin shutdown instead of waiting
indefinitely for a terminal ticket.

## Read-only inspector sample

Build `cargo build --release -p rekenshi-squad-inspector` and install the resulting
DLL using the [sample instructions](../samples/squad-inspector/README.md). It polls
the copied service every 250 ms and writes `squad-inspector.json` in `Context.root`
(normally `Kenshi/RE_Kenshi_Rust/`)
at most once per second when visible data changes. Capture sequence is omitted
from the JSON so an unchanged roster does not trigger writes each update.
Unavailable fields become JSON `null`, and opaque IDs are decimal strings to
preserve all 64 bits in JavaScript. Read errors become explicit waiting or
unsupported status. File-write errors are logged at most once every 30 seconds.
Readers should retry transient parse/sharing errors while the file is rewritten.

The sample starts one worker, signals and joins it during stop, and never issues
commands automatically. Unit tests verify JSON escaping, ID precision and absent
fields. Runtime adapter support still requires validation inside a loaded Kenshi
world; compilation and service tests alone do not establish live-game behavior.
