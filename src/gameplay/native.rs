//! The Steam 1.0.68 adapter. See docs/research/gameplay-native.md for the
//! independently mapped addresses, ABI evidence, and intentionally absent data.
use crate::{extensions, plugin};
use detour::{Transaction, TypedDetour};
use rekenshi_plugin_api::gameplay::*;
use sha2::{Digest, Sha256};
use std::{
    cell::Cell,
    ffi::c_void,
    sync::{
        OnceLock,
        atomic::{AtomicU64, Ordering},
    },
};
use windows_sys::Win32::System::{
    Diagnostics::Debug::ReadProcessMemory,
    LibraryLoader::{GetModuleFileNameW, GetModuleHandleW},
};

const EXE_SHA256: &str = "a596ab4e407c67b58599c54ffb32dc1bf2b64510cdebd3fa9359ef05a576aeb1";
type Method = unsafe extern "C" fn(usize);
type PositionFn = unsafe extern "C" fn(usize, *mut Position) -> *mut Position;
type PointerFn = unsafe extern "C" fn(usize) -> usize;
type SelectedFn = unsafe extern "C" fn(usize, usize) -> bool;
type Predicate = unsafe extern "C" fn(usize) -> bool;
type OrderFn = unsafe extern "C" fn(usize, usize, i32, usize, bool, bool, *const Position);
type StandingFn = unsafe extern "C" fn(usize, i32, bool);

struct Adapter {
    update: TypedDetour<Method>,
    reset_player: TypedDetour<Method>,
    reset_world: TypedDetour<Method>,
    clear_world: TypedDetour<Method>,
    destroy_player: TypedDetour<Method>,
    destroy_world: TypedDetour<Method>,
    position: PositionFn,
    platoon: PointerFn,
    selected: SelectedFn,
    unconscious: Predicate,
    order: OrderFn,
    standing: StandingFn,
    clear_tasks: Method,
    base: usize,
}
static ADAPTER: OnceLock<Adapter> = OnceLock::new();
static RESET_SERIAL: AtomicU64 = AtomicU64::new(0);
thread_local! { static IN_UPDATE: Cell<bool> = const { Cell::new(false) }; }

pub(super) struct NativeCharacter {
    pub key: [u32; 5],
    pub address: usize,
    pub squad_key: Option<[u32; 5]>,
    pub leader_key: Option<[u32; 5]>,
    pub row: CharacterV1,
}

pub(super) fn capabilities() -> u64 {
    CAP_ROSTER | CAP_MOVE | CAP_STOP | CAP_FOLLOW | CAP_STANDING_ORDER
}

pub(super) fn prepare() -> Result<(), String> {
    if ADAPTER.get().is_some() {
        return Ok(());
    }
    // SAFETY: A null name asks Windows for this process's executable module.
    let module = unsafe { GetModuleHandleW(std::ptr::null()) };
    if module.is_null() {
        return Err("Kenshi executable is unavailable".into());
    }
    let mut path = vec![0u16; 32768];
    // SAFETY: module is live; output is writable for the supplied length.
    let len = unsafe { GetModuleFileNameW(module, path.as_mut_ptr(), path.len() as u32) } as usize;
    if len == 0 || len >= path.len() {
        return Err("Cannot locate Kenshi executable".into());
    }
    use std::os::windows::ffi::OsStringExt;
    let path = std::ffi::OsString::from_wide(&path[..len]);
    let bytes = std::fs::read(path).map_err(|e| format!("Read Kenshi executable: {e}"))?;
    if format!("{:x}", Sha256::digest(bytes)) != EXE_SHA256 {
        return Err("Gameplay requires the verified Steam 1.0.68 executable".into());
    }
    plugin::pin(update_hook as *const ())?;
    let base = module as usize;
    // Probe every hook before reserving or constructing any detour. A modified
    // prologue is rejected even when the on-disk executable still matches.
    for (rva, expected) in [
        (
            0x801630,
            &b"\x48\x8b\xc4\x55\x53\x56\x57\x41\x54\x41\x55\x41\x56\x41\x57\x48"[..],
        ),
        (
            0x7fa4e0,
            &b"\x40\x57\x48\x83\xec\x30\x48\xc7\x44\x24\x20\xfe\xff\xff\xff\x48"[..],
        ),
        (
            0x36cb80,
            &b"\x48\x8b\xc4\x55\x48\x8d\x68\x98\x48\x81\xec\x60\x01\x00\x00\x48"[..],
        ),
        (
            0x36ba80,
            &b"\x40\x53\x48\x83\xec\x20\x48\x8b\xd9\x48\x8b\x89\xb8\x04\x00\x00"[..],
        ),
        (
            0x7fb1e0,
            &b"\x48\x89\x4c\x24\x08\x57\x48\x83\xec\x30\x48\xc7\x44\x24\x20\xfe"[..],
        ),
        (
            0x86d650,
            &b"\x48\x89\x4c\x24\x08\x55\x56\x57\x48\x83\xec\x30\x48\xc7\x44\x24"[..],
        ),
    ] {
        if read::<16>(base + rva)?.as_slice() != expected {
            return Err(format!("Gameplay hook at RVA {rva:#x} was modified"));
        }
    }
    // SAFETY: The complete executable hash and verified prologues establish
    // these Win64 member signatures (RCX=this). Detours live for process life.
    let adapter = unsafe {
        Adapter {
            update: hook(base, 0x801630, update_hook)?,
            reset_player: hook(base, 0x7fa4e0, reset_player_hook)?,
            reset_world: hook(base, 0x36cb80, reset_world_hook)?,
            clear_world: hook(base, 0x36ba80, clear_world_hook)?,
            destroy_player: hook(base, 0x7fb1e0, destroy_player_hook)?,
            destroy_world: hook(base, 0x86d650, destroy_world_hook)?,
            position: std::mem::transmute::<usize, PositionFn>(base + 0x5ce680),
            platoon: std::mem::transmute::<usize, PointerFn>(base + 0x791b10),
            selected: std::mem::transmute::<usize, SelectedFn>(base + 0x7f4d20),
            unconscious: std::mem::transmute::<usize, Predicate>(base + 0x5c9e10),
            order: std::mem::transmute::<usize, OrderFn>(base + 0x5d20d0),
            standing: std::mem::transmute::<usize, StandingFn>(base + 0x5caa50),
            clear_tasks: std::mem::transmute::<usize, Method>(base + 0x5074a0),
            base,
        }
    };
    ADAPTER
        .set(adapter)
        .map_err(|_| "Gameplay adapter initialized twice".to_string())
}

/// # Safety
/// The executable and function's unchanged prologue have already been verified.
unsafe fn hook(base: usize, rva: usize, detour: Method) -> Result<TypedDetour<Method>, String> {
    extensions::reserve_core((base + rva) as *const c_void)?;
    // SAFETY: Caller established the fingerprint and Win64 member ABI.
    unsafe { TypedDetour::new(std::mem::transmute::<usize, Method>(base + rva), detour) }
        .map_err(|e| format!("Gameplay detour {rva:#x}: {e}"))
}

pub(super) fn enable(tx: &mut Transaction<'_>) {
    if let Some(a) = ADAPTER.get() {
        for hook in [
            &a.reset_world,
            &a.clear_world,
            &a.destroy_world,
            &a.destroy_player,
            &a.reset_player,
            &a.update,
        ] {
            tx.enable(hook);
        }
    }
}
pub(super) fn disable(tx: &mut Transaction<'_>) {
    if let Some(a) = ADAPTER.get() {
        for hook in [
            &a.update,
            &a.reset_player,
            &a.destroy_player,
            &a.destroy_world,
            &a.clear_world,
            &a.reset_world,
        ] {
            tx.disable(hook);
        }
    }
}

extern "C" fn update_hook(player: usize) {
    let Some(a) = ADAPTER.get() else {
        return;
    };
    let epoch = RESET_SERIAL.load(Ordering::Acquire);
    // SAFETY: Native caller supplies the original live PlayerInterface.
    unsafe {
        a.update.call(player);
    }
    if RESET_SERIAL.load(Ordering::Acquire) != epoch {
        return;
    }
    IN_UPDATE.with(|active| {
        if active.replace(true) {
            return;
        }
        let _ = std::panic::catch_unwind(|| super::tick(player));
        active.set(false);
    });
}

fn invalidate() {
    RESET_SERIAL.fetch_add(1, Ordering::AcqRel);
    let _ = std::panic::catch_unwind(super::reset);
}
macro_rules! reset_hook {
    ($name:ident, $field:ident) => {
        extern "C" fn $name(object: usize) {
            invalidate();
            if let Some(a) = ADAPTER.get() {
                // SAFETY: The hook preserves the verified original signature
                // and invalidates opaque IDs before native lifetime changes.
                unsafe {
                    a.$field.call(object);
                }
            }
        }
    };
}
reset_hook!(reset_player_hook, reset_player);
reset_hook!(reset_world_hook, reset_world);
reset_hook!(clear_world_hook, clear_world);
reset_hook!(destroy_player_hook, destroy_player);
reset_hook!(destroy_world_hook, destroy_world);

// Copy through the OS so corrupt/unavailable memory yields an error instead of
// creating Rust references to concurrently owned C++ storage. This is not a
// substitute for the callback's game-thread lifetime guarantee.
fn read<const N: usize>(address: usize) -> Result<[u8; N], String> {
    let mut bytes = [0; N];
    copy(address, &mut bytes)?;
    Ok(bytes)
}
fn copy(address: usize, bytes: &mut [u8]) -> Result<(), String> {
    address
        .checked_add(bytes.len())
        .ok_or("Native address overflow")?;
    if address < 0x10000 {
        return Err("Null native object".into());
    }
    let mut copied = 0;
    // SAFETY: -1 is the current-process pseudo handle; bytes and copied are
    // valid writable buffers. ReadProcessMemory checks the source mapping.
    let ok = unsafe {
        ReadProcessMemory(
            -1isize as *mut c_void,
            address as *const c_void,
            bytes.as_mut_ptr().cast(),
            bytes.len(),
            &mut copied,
        )
    };
    if ok == 0 || copied != bytes.len() {
        return Err("Native memory is unavailable".into());
    }
    Ok(())
}
fn ptr(address: usize) -> Result<usize, String> {
    Ok(usize::from_ne_bytes(read(address)?))
}
fn integer(address: usize) -> Result<u32, String> {
    Ok(u32::from_ne_bytes(read(address)?))
}
fn number(address: usize) -> Result<f32, String> {
    let value = f32::from_ne_bytes(read(address)?);
    if value.is_finite() {
        Ok(value)
    } else {
        Err("Non-finite native value".into())
    }
}
fn key(object: usize) -> Result<[u32; 5], String> {
    let bytes = read::<20>(object + 0x60)?;
    Ok(std::array::from_fn(|i| {
        u32::from_ne_bytes(bytes[i * 4..i * 4 + 4].try_into().expect("four bytes"))
    }))
}
fn text(address: usize) -> Result<Text, String> {
    let string = read::<32>(address)?;
    let len = usize::from_ne_bytes(string[16..24].try_into().expect("eight bytes"));
    let capacity = usize::from_ne_bytes(string[24..32].try_into().expect("eight bytes"));
    if len > capacity || len > 65536 || capacity > 16 * 1024 * 1024 {
        return Err("Invalid native string".into());
    }
    let len = len.min(128);
    let mut bytes = [0u8; 128];
    if capacity < 16 {
        if len > 15 {
            return Err("Invalid inline native string".into());
        }
        bytes[..len].copy_from_slice(&string[..len]);
    } else {
        let source = usize::from_ne_bytes(string[..8].try_into().expect("eight bytes"));
        copy(source, &mut bytes[..len])?;
    }
    // A truncated final multibyte sequence is omitted rather than replaced.
    let prefix = match std::str::from_utf8(&bytes[..len]) {
        Err(error) if error.error_len().is_none() => error.valid_up_to(),
        _ => len,
    };
    Ok(Text::new(&String::from_utf8_lossy(&bytes[..prefix])))
}

pub(super) fn capture(player: usize) -> Result<Vec<NativeCharacter>, String> {
    if !IN_UPDATE.with(Cell::get) {
        return Err("Gameplay reads require the player update callback".into());
    }
    let a = ADAPTER.get().ok_or("Gameplay adapter is unavailable")?;
    let count = integer(player + 0x2b8)? as usize;
    let capacity = integer(player + 0x2bc)? as usize;
    let array = ptr(player + 0x2c0)?;
    if count > capacity || count > MAX_CHARACTERS {
        return Err("Invalid native player roster".into());
    }
    let mut result = Vec::with_capacity(count);
    for i in 0..count {
        let address = ptr(array + i * 8)?;
        let identity = key(address)?;
        let mut row = CharacterV1 {
            name: text(address + 0x18)?,
            ..Default::default()
        };
        // SAFETY: Iterating the live native player's roster inside its update;
        // the hash-pinned signatures copy to Rust-owned output or return bools.
        unsafe {
            (a.position)(address, &mut row.position);
            if (a.selected)(player, address) {
                row.flags |= FLAG_SELECTED;
            }
            if (a.unconscious)(address) {
                row.flags |= FLAG_UNCONSCIOUS;
            }
        }
        if [row.position.x, row.position.y, row.position.z]
            .into_iter()
            .all(f32::is_finite)
        {
            row.valid |= VALID_POSITION;
        }
        row.valid |= VALID_SELECTED | VALID_UNCONSCIOUS | VALID_DEAD;
        if read::<1>(address + 0x5bc)?[0] != 0 {
            row.flags |= FLAG_DEAD;
        }
        // SAFETY: Same live character and verified getter; it resolves the
        // platoon through the game if the directly cached pointer is absent.
        let squad = unsafe { (a.platoon)(address) };
        let mut squad_key = None;
        let mut leader_key = None;
        if squad != 0 {
            let platoon = ptr(squad + 0x78)?;
            squad_key = Some(key(platoon)?);
            row.squad_name = text(platoon + 0x18)?;
            row.squad_size = integer(squad + 0x58)?;
            // Read the designated leader, without getSquadLeader's mutating
            // fallback-selection path; no leader is a legitimate state.
            let leader = ptr(squad + 0xa0)?;
            if leader != 0 {
                leader_key = Some(key(leader)?);
            }
            row.valid |= VALID_SQUAD;
        }
        if let Ok(stats) = stats(address) {
            row.stats = stats;
            row.valid |= VALID_STATS;
        }
        row.health = health(address);
        if row.health.valid != 0 {
            row.valid |= VALID_HEALTH;
        }
        result.push(NativeCharacter {
            key: identity,
            address,
            squad_key,
            leader_key,
            row,
        });
    }
    Ok(result)
}

fn stats(character: usize) -> Result<Stats, String> {
    let s = ptr(character + 0x450)?;
    Ok(Stats {
        strength: number(s + 0x80)?,
        dexterity: number(s + 0x88)?,
        toughness: number(s + 0x90)?,
        perception: number(s + 0x8c)?,
        melee_attack: number(s + 0x120)?,
        melee_defence: number(s + 0x124)?,
    })
}
fn health(character: usize) -> Health {
    let medical = character + 0x458;
    let mut h = Health::default();
    if let Ok(value) = number(medical + 0x70) {
        h.blood = value;
        h.valid |= HEALTH_BLOOD;
    }
    if let Ok(value) = number(medical + 0x60) {
        h.hunger = value;
        h.valid |= HEALTH_HUNGER;
    }
    let Ok(count) = integer(medical + 0x198) else {
        return h;
    };
    let Ok(parts) = ptr(medical + 0x1a0) else {
        return h;
    };
    if count > 128 {
        return h;
    }
    for i in 0..count as usize {
        let Ok(part) = ptr(parts + i * 8) else {
            continue;
        };
        let (Ok(kind), Ok(side), Ok(flesh), Ok(stun), Ok(rig)) = (
            integer(part + 8),
            integer(part + 0x20),
            number(part + 0x40),
            number(part + 0x44),
            number(part + 0x4c),
        ) else {
            continue;
        };
        let value = flesh - stun + rig;
        if !value.is_finite() {
            continue;
        }
        let (destination, bit) = match (kind, side) {
            (3, _) => (&mut h.head, HEALTH_HEAD),
            (2, 1) => (&mut h.left_arm, HEALTH_LEFT_ARM),
            (2, 2) => (&mut h.right_arm, HEALTH_RIGHT_ARM),
            (1, 1) => (&mut h.left_leg, HEALTH_LEFT_LEG),
            (1, 2) => (&mut h.right_leg, HEALTH_RIGHT_LEG),
            _ => continue,
        };
        *destination = value;
        h.valid |= bit;
    }
    h
}

pub(super) fn execute(command: &CommandV1, actor: usize, target: Option<usize>) -> Result<(), i32> {
    if !IN_UPDATE.with(Cell::get) {
        return Err(UNAVAILABLE);
    }
    let a = ADAPTER.get().ok_or(UNAVAILABLE)?;
    // SAFETY: Parent resolves actor/target only against this update's freshly
    // captured player roster. Calls preserve inspected Win64 signatures.
    unsafe {
        match command.kind {
            MOVE_TO => {
                if ![command.position.x, command.position.y, command.position.z]
                    .into_iter()
                    .all(|v| v.is_finite() && v.abs() <= 1_000_000.)
                {
                    return Err(INVALID_ARGUMENT);
                }
                (a.order)(actor, 0, 29, 0, false, true, &command.position);
            }
            FOLLOW => {
                let target = target.ok_or(INVALID_CHARACTER)?;
                if target == actor {
                    return Err(INVALID_ARGUMENT);
                }
                let mut position = Position::default();
                (a.position)(target, &mut position);
                (a.order)(actor, 0, 44, target, false, true, &position);
            }
            STOP => {
                let movement = ptr(actor + 0x640).map_err(|_| UNAVAILABLE)?;
                let ai = ptr(actor + 0x650).map_err(|_| UNAVAILABLE)?;
                let tasks = ptr(ai + 0x20).map_err(|_| UNAVAILABLE)?;
                let body = ptr(actor + 0x648).map_err(|_| UNAVAILABLE)?;
                let halt = virtual_method(a, movement, 0x98)?;
                let end = virtual_method(a, body, 0x60)?;
                halt(movement);
                (a.clear_tasks)(tasks);
                end(body);
            }
            STANDING_ORDER => {
                if command.enabled > 1 {
                    return Err(INVALID_ARGUMENT);
                }
                let native = match command.order {
                    ORDER_PASSIVE => 13,
                    ORDER_HOLD => 12,
                    ORDER_BLOCK => 11,
                    ORDER_RANGED => 17,
                    ORDER_SNEAK => {
                        if command.enabled != 0 {
                            3
                        } else {
                            4
                        }
                    }
                    _ => return Err(UNSUPPORTED),
                };
                (a.standing)(actor, native, command.enabled != 0);
            }
            _ => return Err(UNSUPPORTED),
        }
    }
    Ok(())
}

fn virtual_method(a: &Adapter, object: usize, slot: usize) -> Result<Method, i32> {
    let table = ptr(object).map_err(|_| UNAVAILABLE)?;
    let method = ptr(table + slot).map_err(|_| UNAVAILABLE)?;
    // Native class implementations (including derived animal variants) must
    // belong to the fingerprinted executable's executable .text section.
    if !(a.base + 0x1000..a.base + 0x1673000).contains(&method) {
        return Err(UNSUPPORTED);
    }
    // SAFETY: Inspected native vtable slot is a void member call with only this.
    Ok(unsafe { std::mem::transmute::<usize, Method>(method) })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn word(bytes: &mut [u8], offset: usize, value: usize) {
        bytes[offset..offset + 8].copy_from_slice(&value.to_ne_bytes());
    }
    fn float(bytes: &mut [u8], offset: usize, value: f32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_ne_bytes());
    }

    #[test]
    fn native_name_copies_inline_and_heap_strings() {
        let mut string = [0u8; 32];
        string[..4].copy_from_slice(b"Beep");
        word(&mut string, 16, 4);
        word(&mut string, 24, 15);
        let result = text(string.as_ptr() as usize).unwrap();
        assert_eq!(&result.bytes[..result.len as usize], b"Beep");
        let name = b"A longer squad name stored outside the native string";
        word(&mut string, 0, name.as_ptr() as usize);
        word(&mut string, 16, name.len());
        word(&mut string, 24, name.len());
        let result = text(string.as_ptr() as usize).unwrap();
        assert_eq!(&result.bytes[..result.len as usize], name);
    }

    #[test]
    fn native_name_rejects_bad_capacity_and_unreadable_pointer() {
        let mut string = [0u8; 32];
        word(&mut string, 16, 16);
        word(&mut string, 24, 15);
        assert!(text(string.as_ptr() as usize).is_err());
        word(&mut string, 0, 1);
        word(&mut string, 24, 32);
        assert!(text(string.as_ptr() as usize).is_err());
        assert!(read::<16>(0).is_err());
        assert!(read::<16>(usize::MAX).is_err());
    }

    #[test]
    fn native_name_truncates_at_a_utf8_boundary() {
        let name = format!("{}猫", "a".repeat(127));
        let mut string = [0u8; 32];
        word(&mut string, 0, name.as_ptr() as usize);
        word(&mut string, 16, name.len());
        word(&mut string, 24, name.len());
        let result = text(string.as_ptr() as usize).unwrap();
        assert_eq!(result.len, 127);
        assert!(result.bytes[..127].iter().all(|b| *b == b'a'));
    }

    #[test]
    fn anatomy_copies_only_present_verified_parts() {
        let mut character = vec![0u8; 0x700];
        float(&mut character, 0x458 + 0x60, 250.);
        float(&mut character, 0x458 + 0x70, 95.);
        let mut arm = [0u8; 0x68];
        arm[8..12].copy_from_slice(&2u32.to_ne_bytes());
        arm[0x20..0x24].copy_from_slice(&1u32.to_ne_bytes());
        float(&mut arm, 0x40, 70.);
        float(&mut arm, 0x44, 10.);
        float(&mut arm, 0x4c, 5.);
        let parts = [arm.as_ptr() as usize];
        character[0x5f0..0x5f4].copy_from_slice(&1u32.to_ne_bytes());
        word(&mut character, 0x5f8, parts.as_ptr() as usize);
        let health = health(character.as_ptr() as usize);
        assert_eq!(health.valid, HEALTH_BLOOD | HEALTH_HUNGER | HEALTH_LEFT_ARM);
        assert_eq!(health.left_arm, 65.);
        assert_eq!(health.blood, 95.);
        assert_eq!(health.hunger, 250.);
    }

    #[test]
    fn gameplay_cannot_read_or_dispatch_outside_native_update() {
        assert!(capture(0).is_err());
        assert_eq!(execute(&CommandV1::default(), 0, None), Err(UNAVAILABLE));
    }
}
