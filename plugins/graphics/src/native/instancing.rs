//! Opt-in replacement for the fingerprinted regular Forests batch builder.
//!
//! Native ownership crosses only explicit build/clear callbacks. Process-local
//! callback state is a documented M-AVOID-STATICS exception: host reservations
//! pin these hooks and this DLL until every native batch has been destroyed.
use super::{instancing_geometry as geometry, instancing_material as material};
use detour::{Threads, Transaction, TypedDetour};
use rekenshi_native::ogre::Shared;
use rekenshi_plugin_api::HostV1;
use std::{
    collections::HashMap,
    ffi::c_void,
    path::Path,
    sync::{
        LazyLock, Mutex, OnceLock,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
};

type RenderOperation = unsafe extern "C-unwind" fn(*mut c_void, *mut c_void);
type Clear = unsafe extern "C-unwind" fn(*mut c_void);
struct Hooks {
    render: TypedDetour<RenderOperation>,
    clear: TypedDetour<Clear>,
    geometry: geometry::Bindings,
    material: material::Bindings,
    host: HostV1,
}
struct Live {
    count: u32,
    material: Option<material::RetiredResources>,
    geometry: Option<geometry::RetiredGeometry>,
}
static HOOKS: OnceLock<Result<Hooks, String>> = OnceLock::new();
static CONFIGURATION: Mutex<()> = Mutex::new(());
static LIVE: LazyLock<Mutex<HashMap<usize, Live>>> = LazyLock::new(|| Mutex::new(HashMap::new()));
static ENABLED: AtomicBool = AtomicBool::new(false);
static SERIAL: AtomicU64 = AtomicU64::new(0);
static BUILDS: AtomicU64 = AtomicU64::new(0);
static INSTANCES: AtomicU64 = AtomicU64::new(0);
static FALLBACKS: AtomicU64 = AtomicU64::new(0);
static ERRORS: AtomicU64 = AtomicU64::new(0);

pub(super) fn prepare(host: HostV1, game: &Path, ogre: usize) {
    HOOKS.get_or_init(|| {
        let base = rekenshi_native::ogre::game_module(game)?;
        for address in [base + 0xa5b6f0, base + 0xa5b4e0] {
            host.reserve_hook(
                std::ptr::NonNull::new(address as *mut c_void).ok_or("instance hook target")?,
            )
            .map_err(|e| e.to_string())?;
        }
        // SAFETY: Executable and Ogre full hashes gate the inspected native ABI.
        // SubBatch RTTI and getRenderOperation/clear instructions establish the
        // exact receivers, signatures and ownership lanes used below.
        unsafe {
            Ok(Hooks {
                render: TypedDetour::new(
                    std::mem::transmute::<usize, RenderOperation>(base + 0xa5b6f0),
                    render as RenderOperation,
                )
                .map_err(|e| e.to_string())?,
                clear: TypedDetour::new(
                    std::mem::transmute::<usize, Clear>(base + 0xa5b4e0),
                    clear as Clear,
                )
                .map_err(|e| e.to_string())?,
                geometry: geometry::Bindings::load(ogre)?,
                material: material::Bindings::new(ogre)?,
                host,
            })
        }
    });
}

pub(crate) fn enabled() -> bool {
    ENABLED.load(Ordering::Acquire)
}

pub(crate) fn configure(enabled: bool) -> Result<(), String> {
    let _configuration = CONFIGURATION
        .lock()
        .map_err(|_| "instancing configuration lock")?;
    if enabled == self::enabled() {
        return Ok(());
    }
    if enabled {
        super::capture::batch_hooks_available()?;
        let h = HOOKS
            .get()
            .ok_or("landscape instancing bindings unavailable")?
            .as_ref()
            .map_err(Clone::clone)?;
        let mut tx = Transaction::new();
        tx.enable(&h.render);
        tx.enable(&h.clear);
        // SAFETY: Outside DllMain; suspend/relocate native threads atomically.
        // These hooks remain installed for existing batches even after opt-out.
        unsafe { tx.commit(Threads::All) }.map_err(|e| e.to_string())?;
    }
    ENABLED.store(enabled, Ordering::Release);
    if let Err(error) = super::capture::refresh_batch_hooks() {
        ENABLED.store(!enabled, Ordering::Release);
        return Err(error);
    }
    if let Some(Ok(h)) = HOOKS.get() {
        h.host.write_log(if enabled {
            "Landscape instancing enabled for newly loaded regular scenery batches; existing regions require reload"
        } else {
            "Landscape instancing disabled for new builds; existing instances retire when their regions unload"
        });
    }
    Ok(())
}

/// # Safety
/// Receiver is a live regular SubBatch before its native builder has consumed
/// the queue. Called only on that builder's serialized native owner lane.
pub(super) unsafe fn try_build(receiver: *mut c_void) -> bool {
    if !enabled() {
        return false;
    }
    let Some(Ok(h)) = HOOKS.get() else {
        return false;
    };
    // SAFETY: The caller supplies the pinned builder's live, unchanged queue.
    let result = unsafe { build(h, receiver) };
    match result {
        Ok(true) => true,
        Ok(false) => {
            FALLBACKS.fetch_add(1, Ordering::Relaxed);
            false
        }
        Err(error) => {
            FALLBACKS.fetch_add(1, Ordering::Relaxed);
            if ERRORS.fetch_add(1, Ordering::Relaxed) < 8 {
                h.host
                    .write_log(&format!("Landscape instancing fallback: {error}"));
            }
            false
        }
    }
}

struct Candidate {
    source: *const c_void,
    rows: Vec<[f32; 12]>,
}

/// # Safety
/// Same live regular build receiver/owner-lane contract as `try_build`.
unsafe fn candidate(receiver: *mut c_void) -> Option<Candidate> {
    // SAFETY: Only the inspected live regular builder supplies these offsets.
    // Copies fixed-width queue fields; never retains the trailing opaque word.
    unsafe {
        let receiver = receiver.cast::<u8>();
        if receiver.add(0x68).read() != 0 || receiver.add(0x69).read() != 0 {
            return None;
        }
        let begin = receiver.add(0x90).cast::<usize>().read_unaligned();
        let end = receiver.add(0x98).cast::<usize>().read_unaligned();
        let bytes = end.checked_sub(begin)?;
        if begin == 0 || bytes % 0x48 != 0 || !(2..=65_536).contains(&(bytes / 0x48)) {
            return None;
        }
        let owner = receiver.add(0x78).cast::<*const u8>().read_unaligned();
        if owner.is_null() {
            return None;
        }
        let origin = owner.add(0x174).cast::<[f32; 3]>().read_unaligned();
        let source = (begin as *const *const c_void).read_unaligned();
        if source.is_null() {
            return None;
        }
        let count = bytes / 0x48;
        let mut rows = Vec::new();
        rows.try_reserve_exact(count).ok()?;
        for i in 0..count {
            let entry = (begin + i * 0x48) as *const u8;
            if entry.cast::<*const c_void>().read_unaligned() != source {
                return None;
            }
            rows.push(crate::landscape::affine(
                entry.add(8).cast::<[f32; 3]>().read_unaligned(),
                entry.add(0x14).cast::<[f32; 4]>().read_unaligned(),
                entry.add(0x24).cast::<[f32; 3]>().read_unaligned(),
                entry.add(0x30).cast::<[f32; 4]>().read_unaligned(),
                origin,
            )?);
        }
        Some(Candidate { source, rows })
    }
}

/// # Safety
/// Same native lifetime/owner-lane contract as `try_build`.
unsafe fn build(h: &Hooks, receiver: *mut c_void) -> Result<bool, String> {
    // SAFETY: Caller guarantees the live queue; preparation does not mutate it.
    let Some(candidate) = (unsafe { candidate(receiver) }) else {
        return Ok(false);
    };
    let _capture = super::capture::instance_build_scope();
    // SAFETY: Synchronous native owner lane; source and effective material are
    // live until the original builder returns. RAII drops uncommitted clones.
    unsafe {
        let geometry = h.geometry.prepare(candidate.source, &candidate.rows)?;
        let slot = receiver.cast::<u8>().add(0x80).cast::<Shared>();
        let id = SERIAL.fetch_add(1, Ordering::Relaxed);
        let material = h
            .material
            .prepare(slot, &format!("REKenshi/ForestInstance/{id}"))?;
        // Reserve the entry before any live mutation. Native calls run outside
        // the registry lock; render cannot observe a half-built native object.
        {
            let mut live = LIVE.lock().map_err(|_| "instance registry lock")?;
            if live.contains_key(&(receiver as usize)) {
                return Err("batch already registered".into());
            }
            live.try_reserve(1).map_err(|e| e.to_string())?;
            live.insert(
                receiver as usize,
                Live {
                    count: candidate.rows.len() as u32,
                    material: None,
                    geometry: None,
                },
            );
        }
        let retired = material.commit_into(slot);
        let (count, retired_geometry) = geometry.commit(receiver);
        // No panicking user code runs while holding this registry. A poisoned
        // lock must still permit native retirement, rather than lose ownership.
        let mut live = LIVE
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(entry) = live.get_mut(&(receiver as usize)) {
            entry.material = Some(retired);
            entry.geometry = Some(retired_geometry);
        }
        drop(live);
        BUILDS.fetch_add(1, Ordering::Relaxed);
        INSTANCES.fetch_add(u64::from(count), Ordering::Relaxed);
        h.host.write_log(&format!(
            "Landscape instanced batch {id}: {count} placements, {} transform bytes",
            u64::from(count) * 48
        ));
        Ok(true)
    }
}

unsafe extern "C-unwind" fn render(receiver: *mut c_void, operation: *mut c_void) {
    let Some(Ok(h)) = HOOKS.get() else {
        return;
    };
    // SAFETY: Original native renderable and operation from the engine lane.
    unsafe { h.render.call(receiver, operation) };
    let live = LIVE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let batch = live.get(&(receiver as usize));
    let count = batch.map_or(1, |batch| batch.count as usize);
    // SAFETY: Pinned RenderOperation stores numberOfInstances at +18; its
    // prototype geometry and instance stream remain native-owned until clear.
    // Reset ordinary batches too, in case the caller reuses operation storage.
    unsafe {
        operation
            .cast::<u8>()
            .add(0x18)
            .cast::<usize>()
            .write_unaligned(count)
    };
    if batch.is_some() {
        // SAFETY: +21 is useGlobalInstancingVertexBufferIsAvailable. The pinned
        // renderer otherwise multiplies +18 by its global instance count. Our
        // owned affine stream already supplies the complete placement count.
        unsafe { operation.cast::<u8>().add(0x21).write(0) };
    }
}

unsafe extern "C-unwind" fn clear(receiver: *mut c_void) {
    let Some(Ok(h)) = HOOKS.get() else {
        return;
    };
    let batch = LIVE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .remove(&(receiver as usize));
    let mut retired_geometry = None;
    if let Some(batch) = batch {
        retired_geometry = batch.geometry;
        if let Some(material) = batch.material {
            // SAFETY: Native clear owns this live batch exclusively; restore its
            // retained original material before registered clone resources retire.
            unsafe {
                material.restore_and_retire(receiver.cast::<u8>().add(0x80).cast::<Shared>())
            };
            // SAFETY: Native clear does not reset its cached Technique pointer.
            // Standard enqueue refreshes it; do not retain a freed clone pointer.
            unsafe {
                receiver
                    .cast::<u8>()
                    .add(0xb0)
                    .cast::<usize>()
                    .write_unaligned(0)
            };
        }
    }
    // SAFETY: Forward once. Native clear releases the cloned vertex/index
    // buffers and resets the same native metadata that its destructor owns.
    unsafe { h.clear.call(receiver) };
    if let Some(geometry) = retired_geometry {
        // SAFETY: Original clear has released instance bindings. Restore the
        // exact original declaration before enqueue can rebuild this SubBatch.
        unsafe { geometry.restore_and_retire(receiver) };
    }
}

pub(crate) fn diagnostics() -> String {
    let live = LIVE.lock().map(|v| v.len()).unwrap_or_default();
    format!(
        "Scenery instances {} | live batches {live} | builds {} | placements {} | fallbacks {}",
        if enabled() { "ON" } else { "OFF" },
        BUILDS.load(Ordering::Relaxed),
        INSTANCES.load(Ordering::Relaxed),
        FALLBACKS.load(Ordering::Relaxed)
    )
}
