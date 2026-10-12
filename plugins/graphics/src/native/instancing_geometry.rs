//! Native-owned geometry metadata for the fingerprinted regular forest builder.
//! Preparation shares prototype buffers and adds one instance stream. It never
//! changes the live SubBatch until every new resource has been prepared.
use std::{ffi::c_void, marker::PhantomData, ptr::NonNull, rc::Rc};

type CloneData = unsafe extern "C-unwind" fn(*const c_void, bool, *mut c_void) -> *mut c_void;
type Destroy = unsafe extern "C" fn(*mut c_void);
type Find = unsafe extern "C" fn(*const c_void, u32, u16) -> *const c_void;
type ElementType = unsafe extern "C" fn(*const c_void) -> u32;
type Animation = unsafe extern "C-unwind" fn(*mut c_void) -> u32;
type HasSkeleton = unsafe extern "C" fn(*const c_void) -> bool;
type NextIndex = unsafe extern "C" fn(*mut c_void) -> u16;
type AddElement =
    unsafe extern "C-unwind" fn(*mut c_void, u16, usize, u32, u32, u16) -> *const c_void;
type SetBinding = unsafe extern "C-unwind" fn(*mut c_void, u16, *const Shared);
type Manager = unsafe extern "C" fn() -> *mut c_void;
type CreateBuffer =
    unsafe extern "C-unwind" fn(*mut c_void, *mut Shared, usize, usize, u32, bool) -> *mut Shared;
type Write = unsafe extern "C-unwind" fn(*mut c_void, usize, usize, *const c_void, bool);

#[repr(C)]
#[derive(Default)]
struct Shared {
    object: *mut c_void,
    info: *mut c_void,
}

#[derive(Clone, Copy)]
pub(super) struct Bindings {
    clone_vertex: CloneData,
    clone_index: CloneData,
    destroy_vertex: Destroy,
    destroy_index: Destroy,
    delete: Destroy,
    destroy_shared: Destroy,
    clear_index_buffer: Destroy,
    find: Find,
    element_type: ElementType,
    animation: Animation,
    has_skeleton: HasSkeleton,
    next_index: NextIndex,
    add_element: AddElement,
    set_binding: SetBinding,
    manager: Manager,
    create_buffer: CreateBuffer,
}

pub(super) struct Prepared<'a> {
    api: &'a Bindings,
    vertex: Option<NonNull<c_void>>,
    index: Option<NonNull<c_void>>,
    count: u32,
    // Native objects are tied to the game thread; they cannot be transferred.
    _game_thread: PhantomData<Rc<()>>,
}

/// Inert native ownership recipe. Only `restore_and_retire` touches its handles,
/// after the original clear has run on the native game thread. Deliberately no
/// Clone or native Drop: the dispatcher must restore every committed recipe.
pub(super) struct RetiredGeometry {
    api: Bindings,
    original_vertex: usize,
    original_index: usize,
}

impl Bindings {
    /// Caller must gate use on the inspected executable/Ogre fingerprints.
    pub(super) fn load(ogre: usize) -> Result<Self, String> {
        use rekenshi_native::symbols::discovered::ogre as n;
        macro_rules! bind {
            ($symbol:ident, $ty:ty) => {{
                let address = rekenshi_native::export(ogre, n::$symbol)?;
                // SAFETY: Exported decorated names encode these Windows x64
                // signatures. The caller gates the private layouts by hash.
                unsafe { std::mem::transmute::<*const c_void, $ty>(address) }
            }};
        }
        Ok(Self {
            clone_vertex: bind!(OGRE_VERTEX_DATA_CLONE_3E08E8481E33, CloneData),
            clone_index: bind!(OGRE_INDEX_DATA_CLONE_C7CF1543B0EE, CloneData),
            destroy_vertex: bind!(OGRE_VERTEX_DATA_DESTROY_EAF469DAE9EE, Destroy),
            destroy_index: bind!(OGRE_INDEX_DATA_DESTROY_5E067AFE0680, Destroy),
            delete: bind!(EXPORT_3_ALLOCATED_OBJECT_V_CATEGORISED_ALLOC_POLICY_00_OGRE_OGRE_SAXPEAX_Z_06F9A1D9F598, Destroy),
            destroy_shared: bind!(OGRE_HARDWARE_VERTEX_BUFFER_SHARED_PTR_DESTROY_2D48BE61CE78, Destroy),
            // SAFETY: Same exported SharedPtr::setNull import called by the
            // pinned native SubBatch::clear at a5b518; void(this) on x64.
            clear_index_buffer: unsafe { std::mem::transmute::<*const c_void, Destroy>(rekenshi_native::export(ogre, c"?setNull@?$SharedPtr@VHardwareIndexBuffer@Ogre@@@Ogre@@QEAAXXZ")?) },
            find: bind!(OGRE_VERTEX_DECLARATION_FIND_ELEMENT_BY_SEMANTIC_FDA289464F2E, Find),
            element_type: bind!(OGRE_VERTEX_ELEMENT_GET_TYPE_CF8B62160C56, ElementType),
            animation: bind!(OGRE_SUB_MESH_GET_VERTEX_ANIMATION_TYPE_684E01BCD944, Animation),
            has_skeleton: bind!(OGRE_MESH_HAS_SKELETON_A20203ACDF89, HasSkeleton),
            next_index: bind!(OGRE_VERTEX_BUFFER_BINDING_GET_NEXT_INDEX_0A4B87B252E1, NextIndex),
            add_element: bind!(OGRE_VERTEX_DECLARATION_ADD_ELEMENT_39648324F6AA, AddElement),
            set_binding: bind!(OGRE_VERTEX_BUFFER_BINDING_SET_BINDING_B86619DA350D, SetBinding),
            manager: bind!(OGRE_HARDWARE_BUFFER_MANAGER_GET_SINGLETON_PTR_DB965ACD3A4E, Manager),
            create_buffer: bind!(OGRE_HARDWARE_BUFFER_MANAGER_CREATE_VERTEX_BUFFER_715E7122E37C, CreateBuffer),
        })
    }

    /// # Safety
    /// Called on the live regular SubBatch build lane of the pinned executable.
    /// `source_submesh` must be its sole live queued source, with no shared mesh
    /// vertices, animation, wind, color override or unsupported shadow path.
    /// The caller validates that each row is the native placement's affine
    /// transform relative to its batch origin and prepares matching shaders.
    pub(super) unsafe fn prepare(
        &self,
        source_submesh: *const c_void,
        rows: &[[f32; 12]],
    ) -> Result<Prepared<'_>, String> {
        if rows.is_empty() || rows.len() > 65_536 || !rows.iter().flatten().all(|v| v.is_finite()) {
            return Err("invalid instance transform array".into());
        }
        // SAFETY: Native source remains owned by its queue caller throughout
        // preparation. Every retained native object is cloned through Ogre.
        unsafe {
            if source_submesh.is_null() {
                return Err("null prototype submesh".into());
            }
            let source = source_submesh.cast::<u8>();
            // SubEntity::getVertexDataForBinding tests SubMesh byte +0 to
            // select shared vertices. Its parent Mesh lives at +70.
            let mesh = source.add(0x70).cast::<*const c_void>().read_unaligned();
            if source.read() != 0
                || mesh.is_null()
                || (self.has_skeleton)(mesh)
                || (self.animation)(source_submesh.cast_mut()) != 0
            {
                return Err("shared or animated prototype geometry is unsupported".into());
            }
            let vertex = source.add(8).cast::<*const c_void>().read_unaligned();
            let index = source.add(0x10).cast::<*const c_void>().read_unaligned();
            if vertex.is_null() || index.is_null() {
                return Err("prototype has no private geometry".into());
            }
            let declaration = vertex
                .cast::<u8>()
                .add(0x10)
                .cast::<*const c_void>()
                .read_unaligned();
            let vertex_count = vertex
                .cast::<u8>()
                .add(0x30)
                .cast::<usize>()
                .read_unaligned();
            let index_count = index
                .cast::<u8>()
                .add(0x18)
                .cast::<usize>()
                .read_unaligned();
            let vertex_start = vertex
                .cast::<u8>()
                .add(0x28)
                .cast::<usize>()
                .read_unaligned();
            let index_start = index
                .cast::<u8>()
                .add(0x10)
                .cast::<usize>()
                .read_unaligned();
            if declaration.is_null() || vertex_count == 0 || index_count == 0 {
                return Err("empty prototype geometry".into());
            }
            if vertex_start != 0 || index_start != 0 {
                return Err("prototype geometry ranges must start at zero".into());
            }
            // objects.hlsl expects POSITION0/NORMAL0 float3 and TANGENT0
            // float3/4. Missing or packed attributes take the original builder.
            // VES_POSITION=1, VES_NORMAL=4, VES_TANGENT=9; FLOAT3/4=2/3.
            for (semantic, types) in [(1, &[2][..]), (4, &[2][..]), (9, &[2, 3][..])] {
                let element = (self.find)(declaration, semantic, 0);
                if element.is_null() || !types.contains(&(self.element_type)(element)) {
                    return Err("prototype position/normal/tangent layout is unsupported".into());
                }
            }
            // VES_TEXTURE_COORDINATES=7; VET_FLOAT2=1 in the pinned Ogre enum.
            let uv = (self.find)(declaration, 7, 0);
            if uv.is_null()
                || (self.element_type)(uv) != 1
                || (1..8).any(|i| !(self.find)(declaration, 7, i).is_null())
            {
                return Err("prototype texture coordinate layout is unsupported".into());
            }
            let mut prepared = Prepared {
                api: self,
                vertex: NonNull::new((self.clone_vertex)(vertex, false, std::ptr::null_mut())),
                index: None,
                count: rows.len() as u32,
                _game_thread: PhantomData,
            };
            let cloned_vertex = prepared
                .vertex
                .ok_or("native vertex clone failed")?
                .as_ptr()
                .cast::<u8>();
            prepared.index = NonNull::new((self.clone_index)(index, false, std::ptr::null_mut()));
            prepared.index.ok_or("native index clone failed")?;
            let declaration = cloned_vertex
                .add(0x10)
                .cast::<*mut c_void>()
                .read_unaligned();
            let binding = cloned_vertex
                .add(0x18)
                .cast::<*mut c_void>()
                .read_unaligned();
            if declaration.is_null() || binding.is_null() {
                return Err("native clone has no declaration or binding".into());
            }
            let stream = (self.next_index)(binding);
            if stream >= 16 {
                return Err("prototype uses too many vertex streams".into());
            }
            let manager = (self.manager)();
            if manager.is_null() {
                return Err("native hardware buffer manager unavailable".into());
            }
            let mut buffer = TemporaryBuffer {
                api: self,
                shared: Shared::default(),
            };
            (self.create_buffer)(manager, &mut buffer.shared, 48, rows.len(), 5, false);
            if buffer.shared.object.is_null() {
                return Err("native instance buffer creation failed".into());
            }
            // Exported getIsInstanceData/getInstanceDataStepRate instructions
            // verify +58 bool/+60 usize. Only this new, exclusively held buffer
            // is modified; source buffers and their flags remain untouched.
            let object = buffer.shared.object.cast::<u8>();
            object.add(0x58).cast::<bool>().write_unaligned(true);
            object.add(0x60).cast::<usize>().write_unaligned(1);
            let table = object.cast::<*const usize>().read();
            let write = std::mem::transmute::<usize, Write>(*table.add(6));
            write(
                buffer.shared.object,
                0,
                std::mem::size_of_val(rows),
                rows.as_ptr().cast(),
                true,
            );
            for i in 0..3 {
                // VET_FLOAT4=3, one 16-byte affine row per TEXCOORD1/2/3.
                (self.add_element)(declaration, stream, i * 16, 3, 7, (i + 1) as u16);
            }
            (self.set_binding)(binding, stream, &buffer.shared);
            Ok(prepared)
        }
    }
}

struct TemporaryBuffer<'a> {
    api: &'a Bindings,
    shared: Shared,
}
impl Drop for TemporaryBuffer<'_> {
    fn drop(&mut self) {
        // SAFETY: Native createVertexBuffer constructed this stack SharedPtr;
        // setBinding retains its own native reference before this one releases.
        unsafe { (self.api.destroy_shared)((&mut self.shared as *mut Shared).cast()) };
    }
}
impl Prepared<'_> {
    /// # Safety
    /// Receiver is the same live regular SubBatch being prepared on this thread.
    /// Its old +58/+60 metadata is exclusively owned, with no expanded buffers
    /// submitted yet. Caller must patch render operation instance count and use
    /// a compatible material before allowing this object to render.
    pub(super) unsafe fn commit(mut self, receiver: *mut c_void) -> (u32, RetiredGeometry) {
        // SAFETY: Caller guarantees receiver ownership and render coordination.
        // Ogre clones use the same destructors/category1 allocator as the
        // pinned SubBatch destructor a58440; native ownership transfers once.
        let retired = unsafe {
            let receiver = receiver.cast::<u8>();
            let vertex_slot = receiver.add(0x58).cast::<*mut c_void>();
            let index_slot = receiver.add(0x60).cast::<*mut c_void>();
            let old_vertex = vertex_slot.read_unaligned();
            let old_index = index_slot.read_unaligned();
            vertex_slot.write_unaligned(self.vertex.take().expect("prepared vertex").as_ptr());
            index_slot.write_unaligned(self.index.take().expect("prepared index").as_ptr());
            // Original build's completion clears the trivially destructible
            // QueuedMesh vector and marks +68 built. The native clear/reset
            // path then owns releasing both cloned bindings and index buffer.
            let begin = receiver.add(0x90).cast::<usize>().read_unaligned();
            receiver.add(0x98).cast::<usize>().write_unaligned(begin);
            receiver.add(0x68).cast::<bool>().write_unaligned(true);
            RetiredGeometry {
                api: *self.api,
                original_vertex: old_vertex as usize,
                original_index: old_index as usize,
            }
        };
        (self.count, retired)
    }
}
impl RetiredGeometry {
    /// # Safety
    /// Invoke exactly once on the same live SubBatch and native game thread,
    /// immediately after its original clear has released the instance buffers.
    /// No render or rebuild may occur between native clear and restoration.
    pub(super) unsafe fn restore_and_retire(self, receiver: *mut c_void) {
        // SAFETY: The dispatcher retains the original metadata exclusively.
        // Native clear leaves declarations intact, so restoring that metadata
        // prevents instance elements leaking into subsequent ordinary builds.
        unsafe {
            let receiver = receiver.cast::<u8>();
            let vertex_slot = receiver.add(0x58).cast::<*mut c_void>();
            let index_slot = receiver.add(0x60).cast::<*mut c_void>();
            let instance_vertex = vertex_slot.read_unaligned();
            let instance_index = index_slot.read_unaligned();
            let original_vertex = self.original_vertex as *mut u8;
            let original_index = self.original_index as *mut u8;
            if !original_vertex.is_null() {
                // The constructor's retained bindings share prototype buffers;
                // they contain no expanded allocations. The original builder
                // needs them to enumerate streams/strides on a later rebuild.
                // Clearing them here would leave declaration-only streams.
                original_vertex.add(0x28).cast::<usize>().write_unaligned(0);
                original_vertex.add(0x30).cast::<usize>().write_unaligned(0);
            }
            if !original_index.is_null() {
                (self.api.clear_index_buffer)(original_index.cast());
                original_index.add(0x10).cast::<usize>().write_unaligned(0);
                original_index.add(0x18).cast::<usize>().write_unaligned(0);
            }
            vertex_slot.write_unaligned(original_vertex.cast());
            index_slot.write_unaligned(original_index.cast());
            if !instance_vertex.is_null() {
                (self.api.destroy_vertex)(instance_vertex);
                (self.api.delete)(instance_vertex);
            }
            if !instance_index.is_null() {
                (self.api.destroy_index)(instance_index);
                (self.api.delete)(instance_index);
            }
        }
    }
}
impl Drop for Prepared<'_> {
    fn drop(&mut self) {
        // SAFETY: Uncommitted clones have exclusive ownership here. Committed
        // clones were taken and now belong to the native SubBatch destructor.
        unsafe {
            if let Some(vertex) = self.vertex.take() {
                (self.api.destroy_vertex)(vertex.as_ptr());
                (self.api.delete)(vertex.as_ptr());
            }
            if let Some(index) = self.index.take() {
                (self.api.destroy_index)(index.as_ptr());
                (self.api.delete)(index.as_ptr());
            }
        }
    }
}
