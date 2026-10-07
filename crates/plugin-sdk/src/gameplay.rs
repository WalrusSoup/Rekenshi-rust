//! Owned gameplay snapshots and commands queued for the game update thread.
//!
//! Connect with [`Gameplay::connect`] during plugin startup. [`Gameplay::status`]
//! distinguishes an unsupported adapter from a world that has not loaded yet.
//! Snapshots contain copied values, never borrowed game pointers. Character IDs
//! expire when the world generation changes; use IDs from a fresh snapshot.
//!
//! Commands execute later in the host. Poll every ticket until terminal to reclaim
//! result storage; the host permits 128 outstanding commands per plugin.
//! [`GameplayError`] distinguishes missing services, incompatible tables, numeric
//! host errors, and malformed responses through accessors.
//!
//! ```no_run
//! # fn inspect(context: &rekenshi_plugin_sdk::Context)
//! #     -> Result<(), rekenshi_plugin_sdk::gameplay::GameplayError> {
//! use rekenshi_plugin_sdk::gameplay::{Availability, Gameplay};
//! let gameplay = Gameplay::connect(context)?;
//! if gameplay.status()?.availability == Availability::Ready {
//!     let roster = gameplay.snapshot()?;
//!     context.log(&format!("Observed {} characters", roster.characters.len()));
//! }
//! # Ok(())
//! # }
//! ```
use crate::{
    Context,
    abi::{HostV1, OK, gameplay as raw},
};
pub use raw::{CharacterId, Position, Stats};

/// Medical readings supported by the character's race and native adapter.
#[derive(Clone, Debug, Default)]
pub struct Health {
    /// Blood medical reading; unavailable fields must not be interpreted.
    pub blood: Option<f32>,
    /// Hunger medical reading; unavailable fields must not be interpreted.
    pub hunger: Option<f32>,
    /// Head medical reading; unavailable fields must not be interpreted.
    pub head: Option<f32>,
    /// Chest medical reading; unavailable fields must not be interpreted.
    pub chest: Option<f32>,
    /// Stomach medical reading; unavailable fields must not be interpreted.
    pub stomach: Option<f32>,
    /// Left arm medical reading; unavailable fields must not be interpreted.
    pub left_arm: Option<f32>,
    /// Right arm medical reading; unavailable fields must not be interpreted.
    pub right_arm: Option<f32>,
    /// Left leg medical reading; unavailable fields must not be interpreted.
    pub left_leg: Option<f32>,
    /// Right leg medical reading; unavailable fields must not be interpreted.
    pub right_leg: Option<f32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Current readiness of the optional native gameplay adapter.
pub enum Availability {
    /// A world and compatible native adapter are available.
    Ready,
    /// No playable world is currently loaded.
    NoWorld,
    /// The installed game build has no compatible native adapter.
    AdapterUnavailable,
    /// A readiness code introduced by a newer host.
    Unknown(u32),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Host-reported feature bits for roster access and command dispatch.
pub struct Capabilities(pub u64);
impl Capabilities {
    /// Report whether the host supports copied character roster observations.
    pub fn roster(self) -> bool {
        self.0 & raw::CAP_ROSTER != 0
    }
    /// Report whether the host supports the requested event or command.
    pub fn supports(self, command: &Command) -> bool {
        self.0 & command.capability() != 0
    }
}
#[derive(Clone, Debug)]
/// Owned gameplay service readiness and capability report.
pub struct Status {
    /// Current service readiness and native adapter availability.
    pub availability: Availability,
    /// Supported roster and command capability bits.
    pub capabilities: Capabilities,
    /// World generation; identifiers from a different generation are stale.
    pub world: u64,
    /// Human-readable diagnostic explaining the service or command state.
    pub message: String,
}
#[derive(Clone, Debug)]
/// Owned squad summary copied from the current world.
pub struct Squad {
    /// Opaque identifier within its world generation.
    pub id: u64,
    /// Display name copied from the current host snapshot.
    pub name: String,
    /// Number of squad members in this snapshot.
    pub size: u32,
    /// World-scoped leader ID, if one is known.
    pub leader: Option<CharacterId>,
}
#[derive(Clone, Debug)]
/// Owned character observation with explicit optional-field availability.
pub struct Character {
    /// Opaque identifier within its world generation.
    pub id: CharacterId,
    /// Display name copied from the current host snapshot.
    pub name: String,
    /// World-space position; use only when the position validity bit is set.
    pub position: Option<Position>,
    /// Optional copied squad information; absent when unsupported or unavailable.
    pub squad: Option<Squad>,
    /// Whether selected, if this state is supported by the native adapter.
    pub selected: Option<bool>,
    /// Whether dead, if this state is supported by the native adapter.
    pub dead: Option<bool>,
    /// Whether unconscious, if supported by the native adapter.
    pub unconscious: Option<bool>,
    /// Combat and attribute readings; use only when stats are marked valid.
    pub stats: Option<Stats>,
    /// Medical readings; each field has its own availability.
    pub health: Option<Health>,
}
#[derive(Clone, Debug)]
/// Consistent owned roster observation scoped to a world generation.
pub struct Snapshot {
    /// World generation; identifiers from a different generation are stale.
    pub world: u64,
    /// Monotonically issued snapshot sequence within this world.
    pub sequence: u64,
    /// Supported roster and command capability bits.
    pub capabilities: Capabilities,
    /// Owned character readings from one consistent host snapshot.
    pub characters: Vec<Character>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Standing-order choices supported by the gameplay command interface.
pub enum StandingOrder {
    /// Avoid initiating combat.
    Passive,
    /// Remain at the current position.
    Hold,
    /// Prefer blocking during combat.
    Block,
    /// Prefer ranged attacks.
    Ranged,
    /// Enable stealth movement.
    Sneak,
}
#[derive(Clone, Copy, Debug)]
/// Owned command validated and queued for game-thread dispatch.
pub enum Command {
    /// Move the character toward the supplied world-space position.
    MoveTo {
        /// World-scoped character receiving this command or represented by this record.
        character: CharacterId,
        /// World-space position; use only when the position validity bit is set.
        position: Position,
    },
    /// Cancel the character current movement or combat order.
    Stop {
        /// World-scoped character receiving this command or represented by this record.
        character: CharacterId,
    },
    /// Issue an attack against the target character.
    Attack {
        /// World-scoped character receiving this command or represented by this record.
        character: CharacterId,
        /// World-scoped target character for attack or follow commands.
        target: CharacterId,
    },
    /// Follow the target character.
    Follow {
        /// World-scoped character receiving this command or represented by this record.
        character: CharacterId,
        /// World-scoped target character for attack or follow commands.
        target: CharacterId,
    },
    /// Toggle one supported standing order for the character.
    StandingOrder {
        /// World-scoped character receiving this command or represented by this record.
        character: CharacterId,
        /// Standing-order discriminant for a standing-order command.
        order: StandingOrder,
        /// Whether this setting or standing order is enabled.
        enabled: bool,
    },
}
impl Command {
    fn capability(self) -> u64 {
        match self {
            Self::MoveTo { .. } => raw::CAP_MOVE,
            Self::Stop { .. } => raw::CAP_STOP,
            Self::Attack { .. } => raw::CAP_ATTACK,
            Self::Follow { .. } => raw::CAP_FOLLOW,
            Self::StandingOrder { .. } => raw::CAP_STANDING_ORDER,
        }
    }
    fn encode(self) -> raw::CommandV1 {
        let mut result = raw::CommandV1::default();
        match self {
            Self::MoveTo {
                character,
                position,
            } => {
                result.kind = raw::MOVE_TO;
                result.character = character;
                result.position = position;
            }
            Self::Stop { character } => {
                result.kind = raw::STOP;
                result.character = character;
            }
            Self::Attack { character, target } => {
                result.kind = raw::ATTACK;
                result.character = character;
                result.target = target;
            }
            Self::Follow { character, target } => {
                result.kind = raw::FOLLOW;
                result.character = character;
                result.target = target;
            }
            Self::StandingOrder {
                character,
                order,
                enabled,
            } => {
                result.kind = raw::STANDING_ORDER;
                result.character = character;
                result.enabled = u32::from(enabled);
                result.order = match order {
                    StandingOrder::Passive => raw::ORDER_PASSIVE,
                    StandingOrder::Hold => raw::ORDER_HOLD,
                    StandingOrder::Block => raw::ORDER_BLOCK,
                    StandingOrder::Ranged => raw::ORDER_RANGED,
                    StandingOrder::Sneak => raw::ORDER_SNEAK,
                };
            }
        }
        result
    }
}
/// A host-issued ticket, scoped to its plugin token. Poll using take_result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CommandTicket(u64);
#[derive(Clone, Debug, PartialEq, Eq)]
/// Progress of an owner-scoped command ticket.
pub enum CommandResult {
    /// The command is queued and its result slot remains allocated.
    Pending,
    /// Native call dispatched; the character may still be travelling or fighting.
    Applied,
    /// The command was rejected; inspect its host code and diagnostic.
    Rejected {
        /// Original host error code describing the rejection.
        code: i32,
        /// Human-readable diagnostic explaining the service or command state.
        message: String,
    },
}
/// A gameplay service failure with a stable classification and diagnostic cause.
#[derive(Clone, Debug)]
pub struct GameplayError {
    kind: Failure,
    detail: crate::Error,
}
#[derive(Clone, Debug, PartialEq, Eq)]
enum Failure {
    Unavailable,
    Incompatible,
    Host(i32),
    InvalidResponse(&'static str),
}
impl GameplayError {
    fn unavailable() -> Self {
        Self {
            kind: Failure::Unavailable,
            detail: crate::Error::unavailable("gameplay"),
        }
    }
    fn incompatible() -> Self {
        Self {
            kind: Failure::Incompatible,
            detail: crate::Error::host("gameplay", crate::abi::INCOMPATIBLE),
        }
    }
    fn host(code: i32) -> Self {
        Self {
            kind: Failure::Host(code),
            detail: crate::Error::host("gameplay", code),
        }
    }
    fn invalid_response(message: &'static str) -> Self {
        Self {
            kind: Failure::InvalidResponse(message),
            detail: crate::Error::new(format!("invalid gameplay response: {message}")),
        }
    }
    /// Report that the optional gameplay service is absent.
    pub fn is_unavailable(&self) -> bool {
        self.kind == Failure::Unavailable
    }
    /// Report that the host negotiated an incompatible gameplay table.
    pub fn is_incompatible(&self) -> bool {
        self.kind == Failure::Incompatible
    }
    /// Return the original host failure code, if the host rejected an operation.
    pub fn host_code(&self) -> Option<i32> {
        match self.kind {
            Failure::Host(code) => Some(code),
            _ => None,
        }
    }
    /// Return the invalid-response description, if host output failed validation.
    pub fn invalid_response_message(&self) -> Option<&'static str> {
        match self.kind {
            Failure::InvalidResponse(message) => Some(message),
            _ => None,
        }
    }
    /// Return the diagnostic backtrace captured when this error was created.
    pub fn backtrace(&self) -> &std::backtrace::Backtrace {
        self.detail.backtrace()
    }
}
impl PartialEq for GameplayError {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind
    }
}
impl Eq for GameplayError {}
impl std::fmt::Display for GameplayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.detail, f)
    }
}
impl std::error::Error for GameplayError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.detail)
    }
}

/// Copyable connection to the host-owned gameplay service.
#[derive(Clone, Copy, Debug)]
pub struct Gameplay {
    api: raw::GameplayV1,
    token: u64,
}
impl Gameplay {
    /// Negotiate a connection to the optional gameplay service.
    ///
    /// # Errors
    /// Returns a failure when service negotiation, validation, or the host operation fails.
    pub fn connect(context: &Context) -> Result<Self, GameplayError> {
        Self::from_host(context.host.native())
    }
    fn from_host(host: HostV1) -> Result<Self, GameplayError> {
        let address = host
            .find_export(c"RE_Kenshi_Rust.dll", c"rekenshi_gameplay_api")
            .ok_or(GameplayError::unavailable())?;
        // SAFETY: The named host export has this signature and remains pinned.
        let get: raw::GetApi = unsafe { std::mem::transmute(address.as_ptr()) };
        let mut output = std::mem::MaybeUninit::uninit();
        // SAFETY: Exact version and aligned writable table storage.
        check(unsafe {
            get(
                raw::VERSION,
                output.as_mut_ptr(),
                size_of::<raw::GameplayV1>(),
            )
        })?;
        // SAFETY: Successful negotiation initializes the complete table.
        let api: raw::GameplayV1 = unsafe { output.assume_init() };
        if api.struct_size as usize != size_of::<raw::GameplayV1>()
            || api.abi_version != raw::VERSION
        {
            return Err(GameplayError::incompatible());
        }
        Ok(Self {
            api,
            token: host.plugin_token,
        })
    }
    /// Copy service readiness, capabilities, and the current world generation.
    ///
    /// # Errors
    /// Returns a failure when service negotiation, validation, or the host operation fails.
    pub fn status(&self) -> Result<Status, GameplayError> {
        let mut output = raw::StatusV1::default();
        // SAFETY: Negotiated API and writable record, no pointer retained.
        check(unsafe { (self.api.status)(self.token, &mut output) })?;
        Ok(Status {
            availability: match output.availability {
                raw::READY => Availability::Ready,
                raw::NO_WORLD => Availability::NoWorld,
                raw::ADAPTER_UNAVAILABLE => Availability::AdapterUnavailable,
                other => Availability::Unknown(other),
            },
            capabilities: Capabilities(output.capabilities),
            world: output.world,
            message: decode(&output.message)?,
        })
    }
    /// Copy a consistent roster, retrying bounded growth and validating returned data.
    ///
    /// # Errors
    /// Returns a failure when service negotiation, validation, or the host operation fails.
    pub fn snapshot(&self) -> Result<Snapshot, GameplayError> {
        let mut rows = Vec::new();
        for _ in 0..4 {
            let mut header = raw::SnapshotV1::default();
            // SAFETY: All buffers are initialized and writable; capacity is rows.len().
            let code = unsafe {
                (self.api.snapshot)(self.token, &mut header, rows.as_mut_ptr(), rows.len())
            };
            let count = header.count as usize;
            if count > raw::MAX_CHARACTERS {
                return Err(GameplayError::invalid_response("roster exceeds limit"));
            }
            if code == raw::BUFFER_TOO_SMALL {
                if count <= rows.len() {
                    return Err(GameplayError::invalid_response("invalid required capacity"));
                }
                rows.resize(count, raw::CharacterV1::default());
                continue;
            }
            check(code)?;
            if count > rows.len() {
                return Err(GameplayError::invalid_response("roster exceeds buffer"));
            }
            let characters = rows[..count]
                .iter()
                .map(|row| character(row, header.world))
                .collect::<Result<_, _>>()?;
            return Ok(Snapshot {
                world: header.world,
                sequence: header.sequence,
                capabilities: Capabilities(header.capabilities),
                characters,
            });
        }
        Err(GameplayError::invalid_response(
            "roster changed during all copy attempts",
        ))
    }
    /// Validate and queue a command; return its nonzero owner-scoped ticket.
    ///
    /// # Errors
    /// Returns a failure when service negotiation, validation, or the host operation fails.
    pub fn enqueue(&self, command: Command) -> Result<CommandTicket, GameplayError> {
        let input = command.encode();
        if input.kind == raw::MOVE_TO
            && ![input.position.x, input.position.y, input.position.z]
                .iter()
                .all(|v| v.is_finite())
        {
            return Err(GameplayError::host(raw::INVALID_ARGUMENT));
        }
        let mut ticket = 0;
        // SAFETY: Command is readable and output writable for this synchronous copy.
        check(unsafe { (self.api.enqueue)(self.token, &input, &mut ticket) })?;
        if ticket == 0 {
            return Err(GameplayError::invalid_response("zero command ticket"));
        }
        Ok(CommandTicket(ticket))
    }
    /// Pending leaves the ticket active. Applied/Rejected consumes its host result;
    /// subsequent polls return UNKNOWN_TICKET. Applied means native dispatch only.
    pub fn take_result(&self, ticket: CommandTicket) -> Result<CommandResult, GameplayError> {
        let mut output = raw::CommandResultV1::default();
        // SAFETY: Negotiated API, scalar ticket and aligned writable result.
        check(unsafe { (self.api.take_result)(self.token, ticket.0, &mut output) })?;
        match output.state {
            raw::PENDING => Ok(CommandResult::Pending),
            raw::APPLIED => Ok(CommandResult::Applied),
            raw::REJECTED => Ok(CommandResult::Rejected {
                code: output.code,
                message: decode(&output.message)?,
            }),
            _ => Err(GameplayError::invalid_response("unknown command state")),
        }
    }
}
fn check(code: i32) -> Result<(), GameplayError> {
    if code == OK {
        Ok(())
    } else {
        Err(GameplayError::host(code))
    }
}
fn decode(text: &raw::Text) -> Result<String, GameplayError> {
    let bytes = text
        .bytes
        .get(..text.len as usize)
        .ok_or(GameplayError::invalid_response(
            "text length exceeds buffer",
        ))?;
    String::from_utf8(bytes.to_vec()).map_err(|_| GameplayError::invalid_response("invalid UTF-8"))
}
fn character(row: &raw::CharacterV1, world: u64) -> Result<Character, GameplayError> {
    if row.character.world != world || row.character.id == 0 {
        return Err(GameplayError::invalid_response("invalid character ID"));
    }
    let valid = |bit| row.valid & bit != 0;
    if valid(raw::VALID_POSITION)
        && ![row.position.x, row.position.y, row.position.z]
            .iter()
            .all(|v| v.is_finite())
    {
        return Err(GameplayError::invalid_response("nonfinite position"));
    }
    if valid(raw::VALID_STATS)
        && ![
            row.stats.strength,
            row.stats.dexterity,
            row.stats.toughness,
            row.stats.perception,
            row.stats.melee_attack,
            row.stats.melee_defence,
        ]
        .iter()
        .all(|v| v.is_finite())
    {
        return Err(GameplayError::invalid_response("nonfinite stats"));
    }
    Ok(Character {
        id: row.character,
        name: decode(&row.name)?,
        position: valid(raw::VALID_POSITION).then_some(row.position),
        squad: if valid(raw::VALID_SQUAD) {
            Some(Squad {
                id: row.squad_id,
                name: decode(&row.squad_name)?,
                size: row.squad_size,
                leader: (row.squad_leader.id != 0).then_some(row.squad_leader),
            })
        } else {
            None
        },
        selected: valid(raw::VALID_SELECTED).then_some(row.flags & raw::FLAG_SELECTED != 0),
        dead: valid(raw::VALID_DEAD).then_some(row.flags & raw::FLAG_DEAD != 0),
        unconscious: valid(raw::VALID_UNCONSCIOUS)
            .then_some(row.flags & raw::FLAG_UNCONSCIOUS != 0),
        stats: valid(raw::VALID_STATS).then_some(row.stats),
        health: if valid(raw::VALID_HEALTH) {
            Some(health(&row.health)?)
        } else {
            None
        },
    })
}

fn health(row: &raw::Health) -> Result<Health, GameplayError> {
    let field = |bit, value: f32| {
        if row.valid & bit == 0 {
            Ok(None)
        } else if value.is_finite() {
            Ok(Some(value))
        } else {
            Err(GameplayError::invalid_response("nonfinite health reading"))
        }
    };
    Ok(Health {
        blood: field(raw::HEALTH_BLOOD, row.blood)?,
        hunger: field(raw::HEALTH_HUNGER, row.hunger)?,
        head: field(raw::HEALTH_HEAD, row.head)?,
        chest: field(raw::HEALTH_CHEST, row.chest)?,
        stomach: field(raw::HEALTH_STOMACH, row.stomach)?,
        left_arm: field(raw::HEALTH_LEFT_ARM, row.left_arm)?,
        right_arm: field(raw::HEALTH_RIGHT_ARM, row.right_arm)?,
        left_leg: field(raw::HEALTH_LEFT_LEG, row.left_leg)?,
        right_leg: field(raw::HEALTH_RIGHT_LEG, row.right_leg)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    static STATE: Mutex<(usize, u32)> = Mutex::new((0, 0));
    #[test]
    fn partial_anatomy_ignores_unavailable_values_and_rejects_nonfinite_readings() {
        let mut row = raw::CharacterV1 {
            character: CharacterId { world: 1, id: 2 },
            valid: raw::VALID_HEALTH,
            health: raw::Health {
                valid: raw::HEALTH_BLOOD | raw::HEALTH_HEAD | (1 << 31),
                blood: 85.0,
                head: 0.0,
                left_arm: f32::NAN,
                ..Default::default()
            },
            ..Default::default()
        };
        let health = character(&row, 1).unwrap().health.unwrap();
        assert_eq!(health.blood, Some(85.0));
        assert_eq!(health.head, Some(0.0));
        assert_eq!(health.left_arm, None);
        assert_eq!(health.right_arm, None);
        row.health.valid |= raw::HEALTH_LEFT_ARM;
        assert!(matches!(
            character(&row, 1),
            Err(ref error) if error.invalid_response_message() == Some("nonfinite health reading")
        ));
        row.valid = raw::VALID_POSITION;
        row.position.x = f32::INFINITY;
        assert!(matches!(
            character(&row, 1),
            Err(ref error) if error.invalid_response_message() == Some("nonfinite position")
        ));
        row.valid = raw::VALID_STATS;
        row.stats.strength = f32::NAN;
        assert!(matches!(
            character(&row, 1),
            Err(ref error) if error.invalid_response_message() == Some("nonfinite stats")
        ));
        row.valid = 0;
        assert!(character(&row, 1).is_ok());
    }
    unsafe extern "C" fn status(_: u64, _: *mut raw::StatusV1) -> i32 {
        OK
    }
    unsafe extern "C" fn snapshot(
        token: u64,
        header: *mut raw::SnapshotV1,
        rows: *mut raw::CharacterV1,
        capacity: usize,
    ) -> i32 {
        assert_eq!(token, 42);
        let mut state = STATE.lock().unwrap();
        state.0 += 1;
        let count = if state.1 == 1 {
            raw::MAX_CHARACTERS + 1
        } else if state.1 == 2 {
            capacity + 1
        } else {
            1
        };
        // SAFETY: Test client supplies valid writable header and rows capacity.
        unsafe {
            header.write(raw::SnapshotV1 {
                world: 7,
                sequence: 5,
                count: count as u32,
                ..Default::default()
            });
        }
        if capacity < count {
            return raw::BUFFER_TOO_SMALL;
        }
        // SAFETY: Capacity is at least one, and rows is writable.
        unsafe {
            rows.write(raw::CharacterV1 {
                character: CharacterId { world: 7, id: 9 },
                name: raw::Text::new("Beep"),
                valid: raw::VALID_SELECTED | (1 << 63),
                flags: raw::FLAG_SELECTED,
                ..Default::default()
            });
        }
        OK
    }
    unsafe extern "C" fn enqueue(_: u64, _: *const raw::CommandV1, ticket: *mut u64) -> i32 {
        // SAFETY: Test client supplies writable ticket storage.
        unsafe {
            ticket.write(13);
        }
        OK
    }
    unsafe extern "C" fn take(_: u64, ticket: u64, output: *mut raw::CommandResultV1) -> i32 {
        assert_eq!(ticket, 13);
        let mut state = STATE.lock().unwrap();
        let result = match state.1 {
            0 => raw::PENDING,
            1 => raw::APPLIED,
            _ => return raw::UNKNOWN_TICKET,
        };
        state.1 += 1;
        // SAFETY: Test client supplies writable result storage.
        unsafe {
            output.write(raw::CommandResultV1 {
                state: result,
                ..Default::default()
            });
        }
        OK
    }
    fn client() -> Gameplay {
        Gameplay {
            token: 42,
            api: raw::GameplayV1 {
                struct_size: size_of::<raw::GameplayV1>() as u32,
                abi_version: 1,
                status,
                snapshot,
                enqueue,
                take_result: take,
            },
        }
    }
    #[test]
    fn transport_retries_bounds_and_result_lifecycle() {
        *STATE.lock().unwrap() = (0, 0);
        let service = client();
        let snapshot = service.snapshot().unwrap();
        assert_eq!(STATE.lock().unwrap().0, 2);
        assert_eq!(snapshot.world, 7);
        assert_eq!(snapshot.characters[0].name, "Beep");
        assert_eq!(snapshot.characters[0].selected, Some(true));
        assert!(snapshot.characters[0].dead.is_none());
        assert!(snapshot.characters[0].position.is_none());
        *STATE.lock().unwrap() = (0, 1);
        assert!(matches!(
            service.snapshot(),
            Err(ref error) if error.invalid_response_message() == Some("roster exceeds limit")
        ));
        *STATE.lock().unwrap() = (0, 2);
        assert!(service.snapshot().is_err());
        assert_eq!(STATE.lock().unwrap().0, 4);
        *STATE.lock().unwrap() = (0, 0);
        let ticket = service
            .enqueue(Command::Stop {
                character: CharacterId { world: 7, id: 9 },
            })
            .unwrap();
        assert_eq!(service.take_result(ticket).unwrap(), CommandResult::Pending);
        assert_eq!(service.take_result(ticket).unwrap(), CommandResult::Applied);
        assert_eq!(
            service.take_result(ticket),
            Err(GameplayError::host(raw::UNKNOWN_TICKET))
        );
    }
    #[test]
    fn commands_and_text_preserve_meaning() {
        let character = CharacterId { world: 1, id: 2 };
        let encoded = Command::StandingOrder {
            character,
            order: StandingOrder::Hold,
            enabled: true,
        }
        .encode();
        assert_eq!(encoded.kind, raw::STANDING_ORDER);
        assert_eq!(encoded.order, raw::ORDER_HOLD);
        assert_eq!(encoded.enabled, 1);
        assert_eq!(encoded.target, CharacterId::default());
        let target = CharacterId { world: 1, id: 3 };
        assert_eq!(
            Command::Attack { character, target }.encode().target,
            target
        );
        assert!(!Capabilities(raw::CAP_ROSTER).supports(&Command::Stop { character }));
        let text = raw::Text::new(&"é".repeat(100));
        assert_eq!(decode(&text).unwrap(), "é".repeat(64));
        let mut invalid = text;
        invalid.len = 129;
        assert!(decode(&invalid).is_err());
    }
    #[test]
    fn old_hosts_report_unavailable() {
        let host = HostV1::stub(42);
        assert!(Gameplay::from_host(host).err().unwrap().is_unavailable());
    }
}
