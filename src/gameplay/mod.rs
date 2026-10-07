//! Copied world snapshots and bounded, owner-scoped game-thread commands.
//! Native pointers exist only inside one update; DLL clients receive opaque IDs.
mod native;

use rekenshi_plugin_api::{ERROR, INCOMPATIBLE, OK, gameplay::*};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::{
        Mutex, OnceLock,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

type Key = [u32; 5];
const MAX_TOTAL_COMMANDS: usize = 8192;
const COMMANDS_PER_TICK: usize = 64;
const COMMAND_LIFETIME: Duration = Duration::from_secs(5);
const CAPTURE_INTERVAL: Duration = Duration::from_millis(100);
static STATE: OnceLock<Mutex<State>> = OnceLock::new();
// Serializes native dispatch with owner cleanup and host shutdown. reset() only
// touches STATE, so a native call that resets the world cannot deadlock this gate.
static DISPATCH: Mutex<()> = Mutex::new(());
static RUNNING: AtomicBool = AtomicBool::new(false);

struct Pending {
    token: u64,
    ticket: u64,
    command: CommandV1,
    queued: Instant,
}
struct State {
    status: StatusV1,
    player: usize,
    sequence: u64,
    last_capture: Option<Instant>,
    next_id: u64,
    next_ticket: u64,
    ids: HashMap<Key, u64>,
    squads: HashMap<Key, u64>,
    rows: Vec<CharacterV1>,
    queue: VecDeque<Pending>,
    results: HashMap<(u64, u64), CommandResultV1>,
}
impl Default for State {
    fn default() -> Self {
        Self {
            status: StatusV1 {
                availability: ADAPTER_UNAVAILABLE,
                world: 1,
                message: Text::new("Gameplay service has not started"),
                ..Default::default()
            },
            player: 0,
            sequence: 0,
            last_capture: None,
            next_id: 1,
            next_ticket: 1,
            ids: HashMap::new(),
            squads: HashMap::new(),
            rows: Vec::new(),
            queue: VecDeque::new(),
            results: HashMap::new(),
        }
    }
}
fn state() -> &'static Mutex<State> {
    STATE.get_or_init(Mutex::default)
}
fn rejection(code: i32, message: &str) -> CommandResultV1 {
    CommandResultV1 {
        state: REJECTED,
        code,
        message: Text::new(message),
    }
}
impl State {
    fn invalidate(&mut self, availability: u32, message: &str, code: i32) {
        self.status.world = self
            .status
            .world
            .checked_add(1)
            .expect("world ID exhausted");
        self.status.availability = availability;
        self.status.message = Text::new(message);
        self.player = 0;
        self.last_capture = None;
        self.rows.clear();
        self.ids.clear();
        self.squads.clear();
        self.next_id = 1;
        self.sequence += 1;
        for pending in self.queue.drain(..) {
            self.results
                .insert((pending.token, pending.ticket), rejection(code, message));
        }
    }
    fn publish(&mut self, player: usize, characters: &mut [native::NativeCharacter]) {
        if self.player != player {
            self.invalidate(NO_WORLD, "World changed", STALE_WORLD);
            self.player = player;
        }
        let keys: HashSet<_> = characters.iter().map(|c| c.key).collect();
        let squads: HashSet<_> = characters.iter().filter_map(|c| c.squad_key).collect();
        self.ids.retain(|key, _| keys.contains(key));
        self.squads.retain(|key, _| squads.contains(key));
        for key in keys {
            self.ids.entry(key).or_insert_with(|| {
                let id = self.next_id;
                self.next_id = self.next_id.checked_add(1).expect("character ID exhausted");
                id
            });
        }
        for key in squads {
            self.squads.entry(key).or_insert_with(|| {
                let id = self.next_id;
                self.next_id = self.next_id.checked_add(1).expect("squad ID exhausted");
                id
            });
        }
        let world = self.status.world;
        self.rows.clear();
        for character in characters {
            character.row.character = CharacterId {
                world,
                id: self.ids[&character.key],
            };
            character.row.squad_id = character
                .squad_key
                .and_then(|key| self.squads.get(&key))
                .copied()
                .unwrap_or(0);
            character.row.squad_leader = character
                .leader_key
                .and_then(|key| self.ids.get(&key))
                .map_or(CharacterId::default(), |id| CharacterId { world, id: *id });
            self.rows.push(character.row);
        }
        self.sequence += 1;
        self.status.availability = READY;
        self.last_capture = Some(Instant::now());
        self.status.message = Text::new("Live player roster available");
    }
    fn validate(&self, command: &CommandV1) -> Result<(), i32> {
        if self.status.availability != READY {
            return Err(UNAVAILABLE);
        }
        if command.character.world != self.status.world {
            return Err(STALE_WORLD);
        }
        let capability = match command.kind {
            MOVE_TO => CAP_MOVE,
            STOP => CAP_STOP,
            ATTACK => CAP_ATTACK,
            FOLLOW => CAP_FOLLOW,
            STANDING_ORDER => CAP_STANDING_ORDER,
            _ => return Err(INVALID_ARGUMENT),
        };
        if self.status.capabilities & capability == 0 {
            return Err(UNSUPPORTED);
        }
        if !self.rows.iter().any(|c| c.character == command.character) {
            return Err(INVALID_CHARACTER);
        }
        if [command.position.x, command.position.y, command.position.z]
            .iter()
            .any(|x| !x.is_finite() || x.abs() > 1_000_000.)
        {
            return Err(INVALID_ARGUMENT);
        }
        if command.kind == STANDING_ORDER {
            if !matches!(
                command.order,
                ORDER_PASSIVE | ORDER_HOLD | ORDER_BLOCK | ORDER_RANGED | ORDER_SNEAK
            ) || command.enabled > 1
            {
                return Err(INVALID_ARGUMENT);
            }
        } else if command.order != 0 || command.enabled != 0 {
            return Err(INVALID_ARGUMENT);
        }
        if matches!(command.kind, ATTACK | FOLLOW) {
            if command.target.world != self.status.world {
                return Err(STALE_WORLD);
            }
            if command.target == command.character
                || !self.rows.iter().any(|c| c.character == command.target)
            {
                return Err(INVALID_CHARACTER);
            }
        } else if command.target != CharacterId::default() {
            return Err(INVALID_ARGUMENT);
        }
        Ok(())
    }
    fn enqueue(&mut self, token: u64, command: CommandV1, now: Instant) -> Result<u64, i32> {
        self.validate(&command)?;
        if self.results.len() >= MAX_TOTAL_COMMANDS
            || self
                .results
                .keys()
                .filter(|(owner, _)| *owner == token)
                .count()
                >= MAX_COMMANDS
        {
            return Err(QUEUE_FULL);
        }
        let ticket = self.next_ticket;
        self.next_ticket = self.next_ticket.checked_add(1).ok_or(QUEUE_FULL)?;
        self.results
            .insert((token, ticket), CommandResultV1::default());
        self.queue.push_back(Pending {
            token,
            ticket,
            command,
            queued: now,
        });
        Ok(ticket)
    }
    fn take_result(&mut self, token: u64, ticket: u64) -> Result<CommandResultV1, i32> {
        let result = *self.results.get(&(token, ticket)).ok_or(UNKNOWN_TICKET)?;
        if result.state != PENDING {
            self.results.remove(&(token, ticket));
        }
        Ok(result)
    }
    fn release(&mut self, token: u64) {
        self.queue.retain(|p| p.token != token);
        self.results.retain(|(owner, _), _| *owner != token);
    }
}

pub(crate) fn prepare() {
    match native::prepare() {
        Ok(()) => {
            let mut state = state()
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            state.status.capabilities = native::capabilities();
            state.invalidate(NO_WORLD, "Waiting for a loaded world", STALE_WORLD);
            crate::logging::write("gameplay adapter prepared; waiting for player update");
        }
        Err(error) => {
            let mut state = state()
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            state.status.capabilities = 0;
            state.invalidate(ADAPTER_UNAVAILABLE, &error, UNAVAILABLE);
            crate::logging::write(&format!("gameplay adapter unavailable: {error}"));
        }
    }
}
pub(crate) fn enable(transaction: &mut detour::Transaction) {
    native::enable(transaction);
}
pub(crate) fn disable(transaction: &mut detour::Transaction) {
    native::disable(transaction);
}
pub(crate) fn start() {
    RUNNING.store(true, Ordering::Release);
}
pub(crate) fn stop() {
    RUNNING.store(false, Ordering::Release);
    let _dispatch = DISPATCH
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Ok(mut state) = state().lock() {
        state.invalidate(ADAPTER_UNAVAILABLE, "Gameplay host stopped", UNAVAILABLE);
        state.status.capabilities = 0;
    }
}
pub(crate) fn release(token: u64) {
    let _dispatch = DISPATCH
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Ok(mut state) = state().lock() {
        state.release(token);
    }
}
pub(super) fn reset() {
    if let Ok(mut state) = state().lock() {
        state.invalidate(
            NO_WORLD,
            "World reset; waiting for player update",
            STALE_WORLD,
        );
    }
}

pub(super) fn tick(player: usize) {
    if !RUNNING.load(Ordering::Acquire) {
        return;
    }
    let Ok(_dispatch) = DISPATCH.try_lock() else {
        return;
    };
    if !RUNNING.load(Ordering::Acquire) {
        return;
    }
    // UI consumers need a timely snapshot, not a full anatomy copy per render
    // frame. A queued command always requests a fresh roster before dispatch.
    if let Ok(state) = state().lock() {
        if state.player == player
            && state.queue.is_empty()
            && state
                .last_capture
                .is_some_and(|time| time.elapsed() < CAPTURE_INTERVAL)
        {
            return;
        }
    }
    let mut characters = match native::capture(player) {
        Ok(characters) => characters,
        Err(error) => {
            if let Ok(mut state) = state().lock() {
                state.invalidate(NO_WORLD, &error, UNAVAILABLE);
            }
            return;
        }
    };
    let pending: Vec<_> = {
        let Ok(mut state) = state().lock() else {
            return;
        };
        state.publish(player, &mut characters);
        let count = state.queue.len().min(COMMANDS_PER_TICK);
        state.queue.drain(..count).collect()
    };
    for pending in pending {
        if !crate::extensions::allowed(pending.token) {
            continue;
        }
        let validation = if pending.queued.elapsed() > COMMAND_LIFETIME {
            Err(EXPIRED)
        } else {
            state()
                .lock()
                .map_err(|_| ERROR)
                .and_then(|s| s.validate(&pending.command))
        };
        let result = validation.and_then(|()| {
            let actor = characters
                .iter()
                .find(|c| c.row.character == pending.command.character)
                .ok_or(INVALID_CHARACTER)?;
            let target = characters
                .iter()
                .find(|c| c.row.character == pending.command.target)
                .map(|c| c.address);
            native::execute(&pending.command, actor.address, target)
        });
        let result = match result {
            Ok(()) => CommandResultV1 {
                state: APPLIED,
                code: OK,
                message: Text::new(
                    "Native order dispatched; task completion is controlled by the game",
                ),
            },
            Err(code) => rejection(
                code,
                match code {
                    STALE_WORLD => "World changed before command execution",
                    EXPIRED => "Command expired before a game update",
                    INVALID_CHARACTER => "Character or target is no longer in the player roster",
                    UNSUPPORTED => "Command is not supported by this adapter",
                    _ => "Game command rejected",
                },
            ),
        };
        if let Ok(mut state) = state().lock() {
            if let Some(slot) = state.results.get_mut(&(pending.token, pending.ticket)) {
                *slot = result;
            }
        }
    }
}

/// Negotiate the optional gameplay service without changing frozen host tables.
/// # Safety
/// For a supported version/size, output points to writable aligned GameplayV1.
// SAFETY: Unique versioned C export; inputs and outputs have fixed C layouts.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rekenshi_gameplay_api(
    version: u32,
    output: *mut GameplayV1,
    size: usize,
) -> i32 {
    if version != VERSION || size != size_of::<GameplayV1>() {
        return INCOMPATIBLE;
    }
    if output.is_null() {
        return ERROR;
    }
    // SAFETY: Caller supplies exact-sized writable table storage.
    unsafe {
        output.write(GameplayV1 {
            struct_size: size as u32,
            abi_version: VERSION,
            status,
            snapshot,
            enqueue,
            take_result,
        });
    }
    OK
}
unsafe extern "C" fn status(token: u64, output: *mut StatusV1) -> i32 {
    std::panic::catch_unwind(|| {
        if output.is_null() || !crate::extensions::allowed(token) {
            return ERROR;
        }
        let Ok(state) = state().lock() else {
            return ERROR;
        };
        // SAFETY: Caller supplies writable StatusV1 storage.
        unsafe {
            output.write(state.status);
        }
        OK
    })
    .unwrap_or(ERROR)
}
unsafe extern "C" fn snapshot(
    token: u64,
    header: *mut SnapshotV1,
    rows: *mut CharacterV1,
    capacity: usize,
) -> i32 {
    std::panic::catch_unwind(|| {
        if header.is_null()
            || (capacity > 0 && rows.is_null())
            || capacity > MAX_CHARACTERS
            || !crate::extensions::allowed(token)
        {
            return ERROR;
        }
        let Ok(state) = state().lock() else {
            return ERROR;
        };
        if state.status.availability != READY {
            return UNAVAILABLE;
        }
        let value = SnapshotV1 {
            world: state.status.world,
            sequence: state.sequence,
            capabilities: state.status.capabilities,
            count: state.rows.len() as u32,
            reserved: 0,
        };
        // SAFETY: Caller supplies writable header; rows are copied only when
        // capacity suffices. Lock makes header and rows one coherent snapshot.
        unsafe {
            header.write(value);
            if capacity < state.rows.len() {
                return BUFFER_TOO_SMALL;
            }
            if !state.rows.is_empty() {
                std::ptr::copy_nonoverlapping(state.rows.as_ptr(), rows, state.rows.len());
            }
        }
        OK
    })
    .unwrap_or(ERROR)
}
unsafe extern "C" fn enqueue(token: u64, command: *const CommandV1, ticket: *mut u64) -> i32 {
    std::panic::catch_unwind(|| {
        if command.is_null() || ticket.is_null() {
            return ERROR;
        }
        let Ok(mut state) = state().lock() else {
            return ERROR;
        };
        // Checked under STATE to order enqueue against revocation and cleanup.
        if !RUNNING.load(Ordering::Acquire) || !crate::extensions::allowed(token) {
            return UNAVAILABLE;
        }
        // SAFETY: Caller supplies readable command storage; copy retains no references.
        match state.enqueue(token, unsafe { command.read() }, Instant::now()) {
            Ok(id) => {
                // SAFETY: Caller supplies writable ticket output.
                unsafe {
                    ticket.write(id);
                }
                OK
            }
            Err(code) => code,
        }
    })
    .unwrap_or(ERROR)
}
unsafe extern "C" fn take_result(token: u64, ticket: u64, output: *mut CommandResultV1) -> i32 {
    std::panic::catch_unwind(|| {
        if output.is_null() || !crate::extensions::allowed(token) {
            return ERROR;
        }
        let Ok(mut state) = state().lock() else {
            return ERROR;
        };
        match state.take_result(token, ticket) {
            Ok(result) => {
                // SAFETY: Caller supplies writable result storage.
                unsafe {
                    output.write(result);
                }
                OK
            }
            Err(code) => code,
        }
    })
    .unwrap_or(ERROR)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn character(serial: u32) -> native::NativeCharacter {
        native::NativeCharacter {
            key: [1, 2, 3, 4, serial],
            address: serial as usize,
            squad_key: Some([5, 6, 7, 8, 9]),
            leader_key: Some([1, 2, 3, 4, 1]),
            row: CharacterV1::default(),
        }
    }
    fn ready() -> State {
        let mut state = State::default();
        state.status.capabilities =
            CAP_ROSTER | CAP_MOVE | CAP_STOP | CAP_FOLLOW | CAP_STANDING_ORDER;
        state.publish(100, &mut [character(1), character(2)]);
        state
    }
    fn move_command(state: &State) -> CommandV1 {
        CommandV1 {
            kind: MOVE_TO,
            character: state.rows[0].character,
            position: Position {
                x: 10.,
                y: 0.,
                z: 20.,
            },
            ..Default::default()
        }
    }
    #[test]
    fn ids_survive_reordering_but_not_despawn_or_world_reset() {
        let mut s = ready();
        let first = s.rows[0].character;
        let squad = s.rows[0].squad_id;
        s.publish(100, &mut [character(2), character(1)]);
        assert_eq!(s.rows[1].character, first);
        assert_eq!(s.rows[1].squad_id, squad);
        assert_eq!(s.rows[0].squad_leader, first);
        s.publish(100, &mut [character(2)]);
        s.publish(100, &mut [character(1), character(2)]);
        assert_ne!(s.rows[0].character, first);
        let before_reset = s.rows[0].character;
        s.invalidate(NO_WORLD, "reset", STALE_WORLD);
        s.publish(100, &mut [character(1)]);
        assert_ne!(s.rows[0].character.world, before_reset.world);
    }
    #[test]
    fn reset_rejects_pending_commands_even_when_player_address_is_reused() {
        let mut s = ready();
        let command = move_command(&s);
        let ticket = s.enqueue(1, command, Instant::now()).unwrap();
        s.invalidate(NO_WORLD, "reset", STALE_WORLD);
        s.publish(100, &mut [character(1)]);
        assert_eq!(s.take_result(1, ticket).unwrap().code, STALE_WORLD);
        assert_eq!(s.validate(&command), Err(STALE_WORLD));
        assert!(s.queue.is_empty());
    }
    #[test]
    fn tickets_are_owner_scoped_and_terminal_results_reclaim_capacity() {
        let mut s = ready();
        let command = move_command(&s);
        let ticket = s.enqueue(1, command, Instant::now()).unwrap();
        assert_eq!(s.take_result(2, ticket).unwrap_err(), UNKNOWN_TICKET);
        assert_eq!(s.take_result(1, ticket).unwrap().state, PENDING);
        assert_eq!(s.take_result(1, ticket).unwrap().state, PENDING);
        for _ in 1..MAX_COMMANDS {
            s.enqueue(1, command, Instant::now()).unwrap();
        }
        assert_eq!(s.enqueue(1, command, Instant::now()), Err(QUEUE_FULL));
        assert!(s.enqueue(2, command, Instant::now()).is_ok());
        s.invalidate(NO_WORLD, "reset", STALE_WORLD);
        assert_eq!(s.take_result(1, ticket).unwrap().state, REJECTED);
        assert_eq!(s.take_result(1, ticket).unwrap_err(), UNKNOWN_TICKET);
        s.release(1);
        assert!(s.results.keys().all(|(owner, _)| *owner == 2));
    }
    #[test]
    fn commands_reject_invalid_numbers_targets_and_unsupported_capabilities() {
        let s = ready();
        let mut command = move_command(&s);
        command.position.x = f32::NAN;
        assert_eq!(s.validate(&command), Err(INVALID_ARGUMENT));
        command = move_command(&s);
        command.kind = ATTACK;
        assert_eq!(s.validate(&command), Err(UNSUPPORTED));
        command.kind = FOLLOW;
        command.target = command.character;
        assert_eq!(s.validate(&command), Err(INVALID_CHARACTER));
        command.target = s.rows[1].character;
        assert_eq!(s.validate(&command), Ok(()));
        command.kind = STANDING_ORDER;
        command.target = CharacterId::default();
        command.order = ORDER_HOLD;
        command.enabled = 2;
        assert_eq!(s.validate(&command), Err(INVALID_ARGUMENT));
    }
}
