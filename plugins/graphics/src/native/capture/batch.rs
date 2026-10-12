//! Copied provenance for the fingerprinted Forests batch builders.
//! Reads only their live input queues, before the original builder consumes them.
use detour::{Transaction, TypedDetour};
use rekenshi_plugin_api::HostV1;
use serde::Serialize;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::{cell::RefCell, collections::BTreeMap, ffi::c_void, path::Path};

type Build = unsafe extern "C-unwind" fn(*mut c_void);
type Name = unsafe extern "C" fn(*const c_void) -> *const c_void;
const QUEUE_STRIDE: usize = 0x48;
const MAX_ENTRIES: usize = 1_000_000;
const MAX_SOURCES: usize = 4096;
const MAX_PLACEMENTS: usize = 65_536;
static SERIAL: AtomicU64 = AtomicU64::new(0);
static FAILURES: AtomicU64 = AtomicU64::new(0);
pub(super) fn failures() -> u64 {
    FAILURES.load(Ordering::Relaxed)
}

#[derive(Clone, Debug, Serialize)]
pub(super) struct Placement {
    pub position: [f32; 3],
    pub orientation_wxyz: [f32; 4],
    pub scale: [f32; 3],
    /// Components as stored by the native builder, after its renderer swizzle.
    pub color_components: [f32; 4],
}
fn serialize_placements<S: serde::Serializer>(
    placements: &Arc<[Placement]>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    placements.as_ref().serialize(serializer)
}
#[derive(Clone, Debug, Serialize)]
pub(super) struct Source {
    pub mesh: String,
    /// Source asset material; the SubEntity may override this at runtime.
    pub material: String,
    /// Submesh submissions, not necessarily distinct scene entities.
    pub copies: u64,
    pub vertices_per_copy: u64,
    pub indices_per_copy: u64,
    #[serde(serialize_with = "serialize_placements")]
    pub placements: Arc<[Placement]>,
}
#[derive(Clone, Debug, Serialize)]
pub(super) struct Batch {
    pub build_id: u64,
    pub builder: &'static str,
    /// Expanded vertex counts below describe the original builder's input.
    pub geometry_representation: &'static str,
    pub effective_material: String,
    pub queued_submeshes: u64,
    pub output_vertices: u64,
    pub output_indices: u64,
    pub omitted_submeshes: u64,
    pub placement_schema_version: u32,
    /// Coordinates passed to the builder; no claim about world/batch origin.
    pub placement_space: &'static str,
    pub placement_color_order: &'static str,
    /// Center subtracted from builder input positions when expanding vertices.
    /// This is not the full renderable/ancestor world transform.
    pub batch_origin: [f32; 3],
    pub omitted_placements: u64,
    pub invalid_placements: u64,
    pub sources: Vec<Source>,
}
pub(super) struct Hooks {
    regular: TypedDetour<Build>,
    wind: TypedDetour<Build>,
    name: Name,
    material: Name,
}
thread_local! {
    static CURRENT: RefCell<Option<Batch>> = const { RefCell::new(None) };
}
struct Scope(Option<Batch>);
impl Drop for Scope {
    fn drop(&mut self) {
        CURRENT.with(|current| *current.borrow_mut() = self.0.take());
    }
}
pub(super) fn current() -> Option<Batch> {
    CURRENT.with(|current| current.borrow().clone())
}
pub(super) fn instance_scope() -> impl Drop {
    Scope(CURRENT.with(|current| {
        let previous = current.borrow().clone();
        if let Some(batch) = current.borrow_mut().as_mut() {
            batch.geometry_representation = "shared_mesh_instances";
        }
        previous
    }))
}
impl Hooks {
    pub(super) fn prepare(host: HostV1, game: &Path, ogre: usize) -> Result<Self, String> {
        let base = rekenshi_native::ogre::game_module(game)?;
        let regular = (base + 0xa58930) as *const c_void;
        let wind = (base + 0xa685e0) as *const c_void;
        for address in [regular, wind] {
            host.reserve_hook(std::ptr::NonNull::new(address.cast_mut()).ok_or("batch target")?)
                .map_err(|e| e.to_string())?;
        }
        use rekenshi_native::symbols::discovered::ogre as names;
        let name = rekenshi_native::export(ogre, names::OGRE_RESOURCE_GET_NAME_69388E412170)?;
        let material =
            rekenshi_native::export(ogre, names::OGRE_SUB_MESH_GET_MATERIAL_NAME_9805233571B7)?;
        // SAFETY: Full executable/DLL hashes gate these ABIs. The executable's
        // RTTI vtables identify slot 18 as SubBatch/WindSubBatch::build, void(this).
        // Exported getters return borrowed native strings, copied synchronously.
        unsafe {
            Ok(Self {
                regular: TypedDetour::new(
                    std::mem::transmute::<*const c_void, Build>(regular),
                    build_regular as Build,
                )
                .map_err(|e| e.to_string())?,
                wind: TypedDetour::new(
                    std::mem::transmute::<*const c_void, Build>(wind),
                    build_wind as Build,
                )
                .map_err(|e| e.to_string())?,
                name: std::mem::transmute::<*const c_void, Name>(name),
                material: std::mem::transmute::<*const c_void, Name>(material),
            })
        }
    }
    pub(super) fn patch<'a>(&'a self, tx: &mut Transaction<'a>, enabled: bool) {
        if enabled {
            tx.enable(&self.regular);
            tx.enable(&self.wind);
        } else {
            tx.disable(&self.wind);
            tx.disable(&self.regular);
        }
    }
    unsafe fn scope(&self, receiver: *mut c_void, builder: &'static str) -> Scope {
        let batch = std::panic::catch_unwind(|| {
            // SAFETY: Called only from the exact live builder's entry, with its
            // queue and source meshes still owned by the native caller.
            unsafe {
                snapshot(
                    receiver,
                    builder,
                    |mesh, submesh| self.source(mesh, submesh),
                    |material| {
                        let name = rekenshi_native::ogre::string((self.name)(material))
                            .unwrap_or_default();
                        super::bounded(&name)
                    },
                )
            }
        })
        .ok()
        .flatten();
        if batch.is_none() {
            FAILURES.fetch_add(1, Ordering::Relaxed);
        }
        Scope(CURRENT.with(|current| current.replace(batch)))
    }
    unsafe fn source(&self, mesh: *const c_void, submesh: *const c_void) -> (String, String) {
        // SAFETY: Borrowed parent Mesh and SubMesh are live for this batch build;
        // getters are pinned above, and no native string or pointer is retained.
        unsafe {
            let mesh = rekenshi_native::ogre::string((self.name)(mesh)).unwrap_or_default();
            let material =
                rekenshi_native::ogre::string((self.material)(submesh)).unwrap_or_default();
            (super::bounded(&mesh), super::bounded(&material))
        }
    }
}
unsafe extern "C-unwind" fn build_regular(receiver: *mut c_void) {
    let Some(h) = super::HOOKS.get().and_then(|h| h.batch.as_ref()) else {
        return;
    };
    // SAFETY: Exact live native receiver, signature verified by fingerprint/RTTI.
    let _scope = super::ACTIVE
        .load(Ordering::Acquire)
        .then(|| unsafe { h.scope(receiver, "Forests::BatchedGeometry::SubBatch") });
    // SAFETY: Same pinned native build lane, before the queue is consumed.
    if unsafe { super::super::instancing::try_build(receiver) } {
        return;
    }
    // SAFETY: Forward unchanged; scope restores on native unwinding as well.
    unsafe { h.regular.call(receiver) };
}
unsafe extern "C-unwind" fn build_wind(receiver: *mut c_void) {
    let Some(h) = super::HOOKS.get().and_then(|h| h.batch.as_ref()) else {
        return;
    };
    // SAFETY: Same native base queue layout; distinct verified wind build entry.
    let _scope = super::ACTIVE
        .load(Ordering::Acquire)
        .then(|| unsafe { h.scope(receiver, "Forests::WindBatchedGeometry::WindSubBatch") });
    // SAFETY: Original native arguments and unwind behavior are preserved.
    unsafe { h.wind.call(receiver) };
}

/// # Safety
/// Receiver must be a live SubBatch/WindSubBatch from the pinned executable,
/// entered on its native build lane. Its queue and source meshes must be live
/// and unmodified for this synchronous copy; name callbacks must not mutate
/// the borrowed meshes or material.
unsafe fn snapshot(
    receiver: *mut c_void,
    builder: &'static str,
    mut names: impl FnMut(*const c_void, *const c_void) -> (String, String),
    mut material_name: impl FnMut(*const c_void) -> String,
) -> Option<Batch> {
    // SAFETY: All offsets are established by the pinned builders' instructions:
    // queue at +90, vector end +98, 48-byte QueuedMesh with first word SubMesh*.
    // Ogre Mesh::createSubMesh writes parent +70; both builders read source
    // VertexData +8/vertexCount +30 and IndexData +10/indexCount +18.
    unsafe {
        let receiver = receiver.cast::<u8>();
        let begin = receiver.add(0x90).cast::<usize>().read_unaligned();
        let end = receiver.add(0x98).cast::<usize>().read_unaligned();
        let bytes = end.checked_sub(begin)?;
        if bytes % QUEUE_STRIDE != 0 || bytes / QUEUE_STRIDE > MAX_ENTRIES {
            return None;
        }
        let count = bytes / QUEUE_STRIDE;
        if count > 0 && begin == 0 {
            return None;
        }
        let vertex = receiver.add(0x58).cast::<*const u8>().read_unaligned();
        let index = receiver.add(0x60).cast::<*const u8>().read_unaligned();
        if vertex.is_null() || index.is_null() {
            return None;
        }
        // SAFETY: SubBatch's live owner is at +78. The outer geometry builder
        // writes its bounds center at +174 before calling either sub-builder.
        // Regular build a595bd and wind build a6860a read that same center.
        let parent = receiver.add(0x78).cast::<*const u8>().read_unaligned();
        if parent.is_null() {
            return None;
        }
        let batch_origin = parent.add(0x174).cast::<[f32; 3]>().read_unaligned();
        if !batch_origin.iter().all(|v| v.is_finite()) {
            return None;
        }
        // SAFETY: The verified SubBatch constructor stores an owned Material
        // SharedPtr at +80; its object is a live Ogre Resource during build.
        let material = receiver.add(0x80).cast::<*const c_void>().read_unaligned();
        if material.is_null() {
            return None;
        }
        let mut batch = Batch {
            build_id: SERIAL.fetch_add(1, Ordering::Relaxed) + 1,
            builder,
            geometry_representation: "expanded_vertices",
            effective_material: material_name(material),
            queued_submeshes: count as u64,
            output_vertices: vertex.add(0x30).cast::<u64>().read_unaligned(),
            output_indices: index.add(0x18).cast::<u64>().read_unaligned(),
            omitted_submeshes: 0,
            placement_schema_version: 1,
            placement_space: "builder_input",
            placement_color_order: "renderer_native",
            batch_origin,
            omitted_placements: 0,
            invalid_placements: 0,
            sources: Vec::new(),
        };
        let mut sources: BTreeMap<usize, (Source, Vec<Placement>)> = BTreeMap::new();
        let mut captured_placements = 0;
        for i in 0..count {
            let entry = (begin + i * QUEUE_STRIDE) as *const u8;
            let submesh = entry.cast::<*const u8>().read_unaligned();
            if !sources.contains_key(&(submesh as usize)) {
                if submesh.is_null() || sources.len() >= MAX_SOURCES {
                    batch.omitted_submeshes += 1;
                    batch.omitted_placements += 1;
                    continue;
                }
                let mesh = submesh.add(0x70).cast::<*const c_void>().read_unaligned();
                let vertex = submesh.add(8).cast::<*const u8>().read_unaligned();
                let index = submesh.add(0x10).cast::<*const u8>().read_unaligned();
                if mesh.is_null() || vertex.is_null() || index.is_null() {
                    batch.omitted_submeshes += 1;
                    batch.omitted_placements += 1;
                    continue;
                }
                let (mesh_name, material) = names(mesh, submesh.cast());
                sources.insert(
                    submesh as usize,
                    (
                        Source {
                            mesh: mesh_name,
                            material,
                            copies: 0,
                            vertices_per_copy: vertex.add(0x30).cast::<u64>().read_unaligned(),
                            indices_per_copy: index.add(0x18).cast::<u64>().read_unaligned(),
                            placements: Arc::from([]),
                        },
                        Vec::new(),
                    ),
                );
            }
            let (source, placements) = sources.get_mut(&(submesh as usize))?;
            source.copies += 1;
            if captured_placements == MAX_PLACEMENTS {
                batch.omitted_placements += 1;
                continue;
            }
            // SAFETY: SubBatch::addSubEntity at RVA a58590 copies these fixed
            // float arrays into its owned 0x48-byte queue entry. No pointer in
            // the trailing opaque word (+40) is read or retained.
            let placement = Placement {
                position: entry.add(8).cast::<[f32; 3]>().read_unaligned(),
                orientation_wxyz: entry.add(0x14).cast::<[f32; 4]>().read_unaligned(),
                scale: entry.add(0x24).cast::<[f32; 3]>().read_unaligned(),
                color_components: entry.add(0x30).cast::<[f32; 4]>().read_unaligned(),
            };
            if !placement
                .position
                .iter()
                .chain(&placement.orientation_wxyz)
                .chain(&placement.scale)
                .chain(&placement.color_components)
                .all(|value| value.is_finite())
                || placement.orientation_wxyz.iter().all(|value| *value == 0.)
            {
                batch.invalid_placements += 1;
                continue;
            }
            placements.push(placement);
            captured_placements += 1;
        }
        batch.sources = sources
            .into_values()
            .map(|(mut source, placements)| {
                source.placements = placements.into();
                source
            })
            .collect();
        Some(batch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn expanded_queue_counts_submesh_copies_and_restores_nested_scope() {
        let mut vertex = [0usize; 8];
        vertex[6] = 300;
        let mut index = [0usize; 4];
        index[3] = 900;
        let mut submesh = [0usize; 16];
        submesh[1] = vertex.as_ptr() as usize;
        submesh[2] = index.as_ptr() as usize;
        submesh[14] = 1; // Names callback receives this opaque parent identity only.
        let mut queue = [[0usize; QUEUE_STRIDE / 8]; 3];
        for item in &mut queue {
            item[0] = submesh.as_ptr() as usize;
            // SAFETY: These arrays own the full queue entry; fixed float fields
            // are within its bounds and are intentionally unaligned at +14.
            unsafe {
                let entry = item.as_mut_ptr().cast::<u8>();
                entry
                    .add(8)
                    .cast::<[f32; 3]>()
                    .write_unaligned([1., 2., 3.]);
                entry.add(0x14).cast::<[f32; 4]>().write_unaligned([0.5; 4]);
                entry
                    .add(0x24)
                    .cast::<[f32; 3]>()
                    .write_unaligned([2., 3., 4.]);
                entry
                    .add(0x30)
                    .cast::<[f32; 4]>()
                    .write_unaligned([0.25, 0.5, 0.75, 1.]);
            }
        }
        let mut output_vertex = [0usize; 8];
        output_vertex[6] = 900;
        let mut output_index = [0usize; 4];
        output_index[3] = 2700;
        let mut receiver = [0usize; 24];
        let mut parent = [0usize; 49];
        // SAFETY: Owned fixture storage covers the unaligned native center.
        unsafe {
            parent
                .as_mut_ptr()
                .cast::<u8>()
                .add(0x174)
                .cast::<[f32; 3]>()
                .write_unaligned([4., 5., 6.]);
        }
        receiver[15] = parent.as_ptr() as usize;
        receiver[16] = 1; // Opaque material identity for the fixture callback.
        receiver[11] = output_vertex.as_ptr() as usize;
        receiver[12] = output_index.as_ptr() as usize;
        receiver[18] = queue.as_ptr() as usize;
        receiver[19] = receiver[18] + std::mem::size_of_val(&queue);
        // SAFETY: Owned arrays model precisely the copied layout; callback does
        // not dereference opaque identity or mutate the borrowed inputs.
        let batch = unsafe {
            snapshot(
                receiver.as_mut_ptr().cast(),
                "fixture",
                |_, _| ("grass.mesh".into(), "grass".into()),
                |_| "effective grass".into(),
            )
        }
        .unwrap();
        assert_eq!(batch.queued_submeshes, 3);
        assert_eq!(batch.output_vertices, 900);
        assert_eq!(batch.sources.len(), 1);
        assert_eq!(batch.sources[0].copies, 3);
        assert_eq!(batch.sources[0].vertices_per_copy, 300);
        assert_eq!(batch.batch_origin, [4., 5., 6.]);
        assert_eq!(batch.effective_material, "effective grass");
        assert_eq!(batch.sources[0].placements.len(), 3);
        assert_eq!(batch.sources[0].placements[0].position, [1., 2., 3.]);
        assert_eq!(batch.sources[0].placements[0].orientation_wxyz, [0.5; 4]);
        assert_eq!(batch.sources[0].placements[0].scale, [2., 3., 4.]);
        assert_eq!(
            batch.sources[0].placements[0].color_components,
            [0.25, 0.5, 0.75, 1.]
        );
        assert_eq!(batch.invalid_placements, 0);
        assert_eq!(batch.omitted_placements, 0);
        let cloned = batch.clone();
        assert!(Arc::ptr_eq(
            &batch.sources[0].placements,
            &cloned.sources[0].placements
        ));
        let outer = Scope(CURRENT.with(|c| c.replace(Some(batch))));
        {
            let _inner = Scope(CURRENT.with(|c| c.replace(None)));
            assert!(current().is_none());
        }
        assert_eq!(current().unwrap().sources[0].copies, 3);
        drop(outer);
        assert!(current().is_none());
        // SAFETY: Owned queue entry; NaN must be reported and excluded from JSON.
        unsafe {
            queue[1]
                .as_mut_ptr()
                .cast::<u8>()
                .add(8)
                .cast::<f32>()
                .write_unaligned(f32::NAN);
        }
        // SAFETY: Same live synthetic native objects and non-mutating callback.
        let invalid = unsafe {
            snapshot(
                receiver.as_mut_ptr().cast(),
                "fixture",
                |_, _| ("grass.mesh".into(), "grass".into()),
                |_| "effective grass".into(),
            )
        }
        .unwrap();
        assert_eq!(invalid.sources[0].copies, 3);
        assert_eq!(invalid.sources[0].placements.len(), 2);
        assert_eq!(invalid.invalid_placements, 1);
        assert_eq!(invalid.omitted_placements, 0);
        let mut oversized = vec![queue[0]; MAX_PLACEMENTS + 1];
        receiver[18] = oversized.as_mut_ptr() as usize;
        receiver[19] = receiver[18] + std::mem::size_of_val(oversized.as_slice());
        // SAFETY: Owned full synthetic queue remains live throughout the copy.
        let limited = unsafe {
            snapshot(
                receiver.as_mut_ptr().cast(),
                "fixture",
                |_, _| ("grass.mesh".into(), "grass".into()),
                |_| "effective grass".into(),
            )
        }
        .unwrap();
        assert_eq!(limited.sources[0].copies, (MAX_PLACEMENTS + 1) as u64);
        assert_eq!(limited.sources[0].placements.len(), MAX_PLACEMENTS);
        assert_eq!(limited.omitted_placements, 1);
        assert_eq!(limited.invalid_placements, 0);
        receiver[19] += 1;
        // SAFETY: Same owned arrays; malformed queue length is rejected before reads.
        assert!(
            // SAFETY: Same owned arrays; malformed length is rejected before reads.
            unsafe {
                snapshot(
                    receiver.as_mut_ptr().cast(),
                    "fixture",
                    |_, _| unreachable!(),
                    |_| unreachable!(),
                )
            }
            .is_none()
        );
    }
}
