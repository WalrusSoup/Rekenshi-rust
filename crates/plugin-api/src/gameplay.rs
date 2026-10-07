//! Optional live gameplay service. All calls are thread-safe and copy inputs.
//! Records contain no game pointers. IDs expire when the world generation changes.
//! Queue/results share a per-plugin bound; take terminal results to reclaim slots.
/// Version negotiated for this optional service.
pub const VERSION: u32 = 1;
/// Maximum number of characters returned in one roster snapshot.
pub const MAX_CHARACTERS: usize = 4096;
/// Maximum queued and unconsumed command results per plugin.
pub const MAX_COMMANDS: usize = 128;
/// Output capacity is insufficient; retry using the reported count.
pub const BUFFER_TOO_SMALL: i32 = 10;
/// The requested service or native target is unavailable.
pub const UNAVAILABLE: i32 = 11;
/// The adapter does not support the requested command.
pub const UNSUPPORTED: i32 = 12;
/// A supplied identifier belongs to a previous world generation.
pub const STALE_WORLD: i32 = 13;
/// The world-scoped character cannot be resolved.
pub const INVALID_CHARACTER: i32 = 14;
/// The plugin command queue and result slots reached their bound.
pub const QUEUE_FULL: i32 = 15;
/// The ticket is unknown, consumed, or owned by another plugin.
pub const UNKNOWN_TICKET: i32 = 16;
/// The command contains an invalid kind, value, or identifier.
pub const INVALID_ARGUMENT: i32 = 17;
/// Command waited more than five seconds for game-thread dispatch.
pub const EXPIRED: i32 = 18;
/// Readiness code indicating a compatible adapter and loaded world.
pub const READY: u32 = 1;
/// Readiness code indicating that no playable world is loaded.
pub const NO_WORLD: u32 = 2;
/// Readiness code indicating an unsupported native game build.
pub const ADAPTER_UNAVAILABLE: u32 = 3;
/// Capability bit enabling roster.
pub const CAP_ROSTER: u64 = 1;
/// Capability bit enabling move.
pub const CAP_MOVE: u64 = 1 << 1;
/// Capability bit enabling stop.
pub const CAP_STOP: u64 = 1 << 2;
/// Capability bit enabling attack.
pub const CAP_ATTACK: u64 = 1 << 3;
/// Capability bit enabling follow.
pub const CAP_FOLLOW: u64 = 1 << 4;
/// Capability bit enabling standing order.
pub const CAP_STANDING_ORDER: u64 = 1 << 5;
/// Validity bit marking supported position.
pub const VALID_POSITION: u64 = 1;
/// Validity bit marking supported squad.
pub const VALID_SQUAD: u64 = 1 << 1;
/// Validity bit marking supported selected.
pub const VALID_SELECTED: u64 = 1 << 2;
/// Validity bit marking supported dead.
pub const VALID_DEAD: u64 = 1 << 3;
/// Validity bit marking supported unconscious.
pub const VALID_UNCONSCIOUS: u64 = 1 << 4;
/// Validity bit marking supported stats.
pub const VALID_STATS: u64 = 1 << 5;
/// Validity bit marking supported health.
pub const VALID_HEALTH: u64 = 1 << 6;
/// Validity bit marking a supported medical blood.
pub const HEALTH_BLOOD: u32 = 1;
/// Validity bit marking a supported medical hunger.
pub const HEALTH_HUNGER: u32 = 2;
/// Validity bit marking a supported medical head.
pub const HEALTH_HEAD: u32 = 4;
/// Validity bit marking a supported medical chest.
pub const HEALTH_CHEST: u32 = 8;
/// Validity bit marking a supported medical stomach.
pub const HEALTH_STOMACH: u32 = 16;
/// Validity bit marking a supported medical left arm.
pub const HEALTH_LEFT_ARM: u32 = 32;
/// Validity bit marking a supported medical right arm.
pub const HEALTH_RIGHT_ARM: u32 = 64;
/// Validity bit marking a supported medical left leg.
pub const HEALTH_LEFT_LEG: u32 = 128;
/// Validity bit marking a supported medical right leg.
pub const HEALTH_RIGHT_LEG: u32 = 256;
/// State bit marking the character as selected.
pub const FLAG_SELECTED: u64 = 1;
/// State bit marking the character as dead.
pub const FLAG_DEAD: u64 = 1 << 1;
/// State bit marking the character as unconscious.
pub const FLAG_UNCONSCIOUS: u64 = 1 << 2;
/// Command discriminant selecting move to.
pub const MOVE_TO: u32 = 1;
/// Command discriminant selecting stop.
pub const STOP: u32 = 2;
/// Command discriminant selecting attack.
pub const ATTACK: u32 = 3;
/// Command discriminant selecting follow.
pub const FOLLOW: u32 = 4;
/// Command discriminant selecting standing order.
pub const STANDING_ORDER: u32 = 5;
/// Standing-order discriminant selecting passive.
pub const ORDER_PASSIVE: u32 = 1;
/// Standing-order discriminant selecting hold.
pub const ORDER_HOLD: u32 = 2;
/// Standing-order discriminant selecting block.
pub const ORDER_BLOCK: u32 = 3;
/// Standing-order discriminant selecting ranged.
pub const ORDER_RANGED: u32 = 4;
/// Standing-order discriminant selecting sneak.
pub const ORDER_SNEAK: u32 = 5;
/// Command result code indicating queued work.
pub const PENDING: u32 = 0;
/// Command reached native dispatch; movement or combat may still be in progress.
pub const APPLIED: u32 = 1;
/// Command result code indicating failure before or during dispatch.
pub const REJECTED: u32 = 2;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
/// Opaque character identity scoped to one loaded world generation.
pub struct CharacterId {
    /// World generation; identifiers from a different generation are stale.
    pub world: u64,
    /// Opaque identifier within its world generation.
    pub id: u64,
}
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
/// World-space coordinates copied from or supplied to the game.
pub struct Position {
    /// World-space horizontal X coordinate.
    pub x: f32,
    /// World-space vertical coordinate.
    pub y: f32,
    /// World-space horizontal Z coordinate.
    pub z: f32,
}
/// UTF-8 bytes with explicit length; truncation must preserve character boundaries.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Text {
    /// Number of initialized UTF-8 bytes; must not exceed the buffer capacity.
    pub len: u32,
    /// Inline UTF-8 storage; only the first `len` bytes are meaningful.
    pub bytes: [u8; 128],
}
impl Default for Text {
    fn default() -> Self {
        Self {
            len: 0,
            bytes: [0; 128],
        }
    }
}
impl Text {
    /// Copy UTF-8 into inline storage, truncating only at character boundaries.
    ///
    /// # Errors
    /// Returns a failure when service negotiation, validation, or the host operation fails.
    pub fn new(value: &str) -> Self {
        let mut len = value.len().min(128);
        while !value.is_char_boundary(len) {
            len -= 1;
        }
        let mut result = Self::default();
        result.bytes[..len].copy_from_slice(&value.as_bytes()[..len]);
        result.len = len as u32;
        result
    }
}
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
/// Combat and attribute readings supported by the native adapter.
pub struct Stats {
    /// Strength reading from the current character snapshot.
    pub strength: f32,
    /// Dexterity reading from the current character snapshot.
    pub dexterity: f32,
    /// Toughness reading from the current character snapshot.
    pub toughness: f32,
    /// Perception reading from the current character snapshot.
    pub perception: f32,
    /// Melee attack reading from the current character snapshot.
    pub melee_attack: f32,
    /// Melee defence reading from the current character snapshot.
    pub melee_defence: f32,
}
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
/// Medical readings with explicit per-field availability.
pub struct Health {
    /// Per-field HEALTH_* validity, accommodating different race anatomies.
    pub valid: u32,
    /// Blood medical reading; unavailable fields must not be interpreted.
    pub blood: f32,
    /// Hunger medical reading; unavailable fields must not be interpreted.
    pub hunger: f32,
    /// Head medical reading; unavailable fields must not be interpreted.
    pub head: f32,
    /// Chest medical reading; unavailable fields must not be interpreted.
    pub chest: f32,
    /// Stomach medical reading; unavailable fields must not be interpreted.
    pub stomach: f32,
    /// Left arm medical reading; unavailable fields must not be interpreted.
    pub left_arm: f32,
    /// Right arm medical reading; unavailable fields must not be interpreted.
    pub right_arm: f32,
    /// Left leg medical reading; unavailable fields must not be interpreted.
    pub left_leg: f32,
    /// Right leg medical reading; unavailable fields must not be interpreted.
    pub right_leg: f32,
}
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
/// C-layout character snapshot with validity and state bit masks.
pub struct CharacterV1 {
    /// World-scoped character receiving this command or represented by this record.
    pub character: CharacterId,
    /// Display name copied from the current host snapshot.
    pub name: Text,
    /// Bit mask selecting which fields contain supported readings.
    pub valid: u64,
    /// Character state bits; interpret only states marked valid.
    pub flags: u64,
    /// World-space position; use only when the position validity bit is set.
    pub position: Position,
    /// Number of characters currently reported in the squad.
    pub squad_size: u32,
    /// Opaque squad identifier in the current world generation.
    pub squad_id: u64,
    /// Copied squad display name.
    pub squad_name: Text,
    /// Zero ID means no known leader.
    pub squad_leader: CharacterId,
    /// Combat and attribute readings; use only when stats are marked valid.
    pub stats: Stats,
    /// Medical readings; each field has its own availability.
    pub health: Health,
}
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
/// C-layout readiness, world generation, and capability report.
pub struct StatusV1 {
    /// Current service readiness and native adapter availability.
    pub availability: u32,
    /// Reserved for future versions; callers must write zero.
    pub reserved: u32,
    /// Supported roster and command capability bits.
    pub capabilities: u64,
    /// World generation; identifiers from a different generation are stale.
    pub world: u64,
    /// Human-readable diagnostic explaining the service or command state.
    pub message: Text,
}
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
/// C-layout header describing an atomic roster copy.
pub struct SnapshotV1 {
    /// World generation; identifiers from a different generation are stale.
    pub world: u64,
    /// Monotonically issued snapshot sequence within this world.
    pub sequence: u64,
    /// Supported roster and command capability bits.
    pub capabilities: u64,
    /// Number of rows or settings available in the negotiated output.
    pub count: u32,
    /// Reserved for future versions; callers must write zero.
    pub reserved: u32,
}
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
/// C-layout command copied into the host game-thread queue.
pub struct CommandV1 {
    /// Discriminant selecting the command or control representation.
    pub kind: u32,
    /// Standing-order discriminant for a standing-order command.
    pub order: u32,
    /// World-scoped character receiving this command or represented by this record.
    pub character: CharacterId,
    /// World-scoped target character for attack or follow commands.
    pub target: CharacterId,
    /// World-space position; use only when the position validity bit is set.
    pub position: Position,
    /// Standing-order boolean, exactly 0 or 1. All other reserved fields zero.
    pub enabled: u32,
}
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
/// C-layout command progress and rejection diagnostic.
pub struct CommandResultV1 {
    /// Command progress: pending, applied, or rejected.
    pub state: u32,
    /// Original host error code describing the rejection.
    pub code: i32,
    /// Human-readable diagnostic explaining the service or command state.
    pub message: Text,
}
/// Negotiate this service into aligned exact-sized caller-owned output.
pub type GetApi = unsafe extern "C" fn(u32, *mut GameplayV1, usize) -> i32;
#[repr(C)]
#[derive(Clone, Copy, Debug)]
/// Negotiated callbacks for copied gameplay observations and queued commands.
pub struct GameplayV1 {
    /// Size of this C record in bytes; negotiation requires an exact match.
    pub struct_size: u32,
    /// Version of this table; negotiate before invoking its callbacks.
    pub abi_version: u32,
    /// Copy service readiness into caller-owned writable output.
    pub status: unsafe extern "C" fn(u64, *mut StatusV1) -> i32,
    /// Atomic copy. On BUFFER_TOO_SMALL writes header.count only (other header
    /// fields may also be set), and no rows. Null rows allowed only at capacity 0.
    /// Success initializes header and exactly header.count rows, <= capacity.
    pub snapshot: unsafe extern "C" fn(u64, *mut SnapshotV1, *mut CharacterV1, usize) -> i32,
    /// Copies the command; success initializes a nonzero, token-owned ticket.
    pub enqueue: unsafe extern "C" fn(u64, *const CommandV1, *mut u64) -> i32,
    /// Pending does not consume; terminal results are consumed on successful copy.
    /// APPLIED means native dispatch, not completion of movement/combat.
    pub take_result: unsafe extern "C" fn(u64, u64, *mut CommandResultV1) -> i32,
}
