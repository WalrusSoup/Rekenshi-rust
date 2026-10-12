//! Opt-in D3D11 resource inventory, installed before the game's device creation.
//! Static hooks/state are a host-local native-callback exception (M-AVOID-STATICS).
//! COM carries only opaque IUnknown methods; all Rust ownership stays in this DLL.
mod activity;
mod batch;
mod lifetime;
mod origin;
mod store;
use crate::resources::Description;
use detour::{Threads, Transaction, TypedDetour};
use rekenshi_plugin_api::HostV1;
use std::cell::RefCell;
use std::ffi::c_void;
use std::path::Path;
use std::sync::{
    Arc, Mutex, OnceLock,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;
use windows::Win32::Graphics::{Direct3D::WKPDID_D3DDebugObjectName, Direct3D11::*};
use windows::core::{GUID, Interface};

type Create<D> = unsafe extern "system" fn(
    *mut c_void,
    *const D,
    *const D3D11_SUBRESOURCE_DATA,
    *mut *mut c_void,
) -> i32;
type Load = unsafe extern "C-unwind" fn(*mut c_void, bool);
type Name = unsafe extern "C" fn(*const c_void) -> *const c_void;
type SharedStore = Arc<Mutex<store::Store>>;
struct Hooks {
    buffer: TypedDetour<Create<D3D11_BUFFER_DESC>>,
    texture1: TypedDetour<Create<D3D11_TEXTURE1D_DESC>>,
    texture2: TypedDetour<Create<D3D11_TEXTURE2D_DESC>>,
    texture3: TypedDetour<Create<D3D11_TEXTURE3D_DESC>>,
    load: TypedDetour<Load>,
    prepare: TypedDetour<Load>,
    activity: activity::Hooks,
    name: Name,
    batch: Option<batch::Hooks>,
    batch_unavailable: Option<String>,
}
static HOOKS: OnceLock<Hooks> = OnceLock::new();
static SESSION: Mutex<Option<SharedStore>> = Mutex::new(None);
static ACTIVE: AtomicBool = AtomicBool::new(false);
static CONFIGURATION: Mutex<()> = Mutex::new(());
const TAG: GUID = GUID::from_u128(0x4752e615_5cf8_42fd_b244_7206a20e992d);
thread_local! { static NAME: RefCell<String> = const { RefCell::new(String::new()) }; }

pub(super) fn prepare(
    host: HostV1,
    device: &ID3D11Device,
    context: &ID3D11DeviceContext,
    game: &Path,
) -> Result<(), String> {
    if HOOKS.get().is_some() {
        return Ok(());
    }
    let ogre = rekenshi_native::ogre::initialize(game)?;
    use rekenshi_native::symbols::discovered::ogre as names;
    let load = rekenshi_native::export(ogre.base, names::OGRE_RESOURCE_LOAD_C4A9C3B4E976)?;
    let name = rekenshi_native::export(ogre.base, names::OGRE_RESOURCE_GET_NAME_69388E412170)?;
    let prepare = rekenshi_native::export(ogre.base, names::OGRE_RESOURCE_PREPARE_AE088F7AC37B)?;
    let v = device.vtable();
    let (batch, batch_unavailable) = match batch::Hooks::prepare(host, game, ogre.base) {
        Ok(hooks) => (Some(hooks), None),
        Err(error) => (None, Some(error)),
    };
    let addresses = [
        v.CreateBuffer as *const c_void,
        v.CreateTexture1D as _,
        v.CreateTexture2D as _,
        v.CreateTexture3D as _,
        load,
        prepare,
    ];
    for address in addresses {
        host.reserve_hook(
            std::ptr::NonNull::new(address.cast_mut()).ok_or("resource hook target")?,
        )
        .map_err(|e| e.to_string())?;
    }
    // SAFETY: Device's typed documented Windows COM slots establish each exact
    // signature. Ogre's full DLL hash pins its exported VC100 load/name ABI.
    let hooks = unsafe {
        Hooks {
            buffer: TypedDetour::new(
                std::mem::transmute::<*const c_void, Create<D3D11_BUFFER_DESC>>(addresses[0]),
                create_buffer as Create<D3D11_BUFFER_DESC>,
            )
            .map_err(|e| e.to_string())?,
            texture1: TypedDetour::new(
                std::mem::transmute::<*const c_void, Create<D3D11_TEXTURE1D_DESC>>(addresses[1]),
                create_texture1 as Create<D3D11_TEXTURE1D_DESC>,
            )
            .map_err(|e| e.to_string())?,
            texture2: TypedDetour::new(
                std::mem::transmute::<*const c_void, Create<D3D11_TEXTURE2D_DESC>>(addresses[2]),
                create_texture2 as Create<D3D11_TEXTURE2D_DESC>,
            )
            .map_err(|e| e.to_string())?,
            texture3: TypedDetour::new(
                std::mem::transmute::<*const c_void, Create<D3D11_TEXTURE3D_DESC>>(addresses[3]),
                create_texture3 as Create<D3D11_TEXTURE3D_DESC>,
            )
            .map_err(|e| e.to_string())?,
            load: TypedDetour::new(
                std::mem::transmute::<*const c_void, Load>(load),
                load_resource as Load,
            )
            .map_err(|e| e.to_string())?,
            name: std::mem::transmute::<*const c_void, Name>(name),
            prepare: TypedDetour::new(
                std::mem::transmute::<*const c_void, Load>(prepare),
                prepare_resource as Load,
            )
            .map_err(|e| e.to_string())?,
            activity: activity::Hooks::prepare(host, context)?,
            batch,
            batch_unavailable,
        }
    };
    HOOKS
        .set(hooks)
        .map_err(|_| "resource capture hooks already prepared".to_string())?;
    if HOOKS.get().is_some_and(|h| h.batch.is_some()) {
        super::instancing::prepare(host, game, ogre.base);
    }
    Ok(())
}
pub(super) fn batch_hooks_available() -> Result<(), String> {
    let h = HOOKS.get().ok_or("forest batch hooks not prepared")?;
    if h.batch.is_some() {
        Ok(())
    } else {
        Err(h
            .batch_unavailable
            .clone()
            .unwrap_or_else(|| "forest batch hooks unavailable".into()))
    }
}
pub(super) fn refresh_batch_hooks() -> Result<(), String> {
    let _configuration = CONFIGURATION
        .lock()
        .map_err(|_| "resource configuration lock")?;
    patch(ACTIVE.load(Ordering::Acquire))
}
pub(super) fn instance_build_scope() -> impl Drop {
    batch::instance_scope()
}
fn patch(enabled: bool) -> Result<(), String> {
    let h = HOOKS.get().ok_or("resource capture hooks not prepared")?;
    let mut tx = Transaction::new();
    h.activity.patch(&mut tx, enabled);
    if let Some(batch) = &h.batch {
        batch.patch(&mut tx, enabled || super::instancing::enabled());
    }
    if enabled {
        tx.enable(&h.buffer);
        tx.enable(&h.texture1);
        tx.enable(&h.texture2);
        tx.enable(&h.texture3);
        tx.enable(&h.load);
        tx.enable(&h.prepare);
    } else {
        tx.disable(&h.prepare);
        tx.disable(&h.load);
        tx.disable(&h.texture3);
        tx.disable(&h.texture2);
        tx.disable(&h.texture1);
        tx.disable(&h.buffer);
    }
    // SAFETY: Outside DllMain; transaction suspends threads/relocates live IPs.
    // Hooks and DLL code stay pinned, including outstanding private COM markers.
    unsafe { tx.commit(Threads::All) }.map_err(|e| e.to_string())
}
pub(crate) fn configure(root: &Path, enabled: bool) -> Result<(), String> {
    let _configuration = CONFIGURATION
        .lock()
        .map_err(|_| "resource configuration lock")?;
    if enabled && ACTIVE.load(Ordering::Acquire) {
        return Ok(());
    }
    if !enabled
        && SESSION
            .lock()
            .map_err(|_| "resource session lock")?
            .is_none()
    {
        return Ok(());
    }
    if enabled {
        let store = Arc::new(Mutex::new(store::Store::open(root)?));
        store
            .lock()
            .map_err(|_| "resource capture lock")?
            .snapshot(false)?;
        *SESSION.lock().map_err(|_| "resource session lock")? = Some(store);
        ACTIVE.store(true, Ordering::Release);
        if let Err(error) = patch(true) {
            ACTIVE.store(false, Ordering::Release);
            SESSION.lock().map_err(|_| "resource session lock")?.take();
            return Err(error);
        }
    } else {
        ACTIVE.store(false, Ordering::Release);
        patch(false)?;
        if let Some(store) = SESSION.lock().map_err(|_| "resource session lock")?.take() {
            store
                .lock()
                .map_err(|_| "resource capture lock")?
                .snapshot(true)?;
        }
    }
    Ok(())
}
pub(crate) fn tick() -> Result<(), String> {
    if !ACTIVE.load(Ordering::Acquire) {
        return Ok(());
    }
    let store = SESSION.lock().map_err(|_| "resource session lock")?.clone();
    if let Some(store) = store {
        let mut store = store.lock().map_err(|_| "resource capture lock")?;
        if store.last_report.elapsed() >= Duration::from_secs(15) {
            store.last_report = std::time::Instant::now();
            store.snapshot(false)?;
        }
    }
    Ok(())
}
struct NameScope(String);
impl Drop for NameScope {
    fn drop(&mut self) {
        NAME.with(|name| *name.borrow_mut() = std::mem::take(&mut self.0));
    }
}
unsafe extern "C-unwind" fn load_resource(resource: *mut c_void, background: bool) {
    let Some(h) = HOOKS.get() else { return };
    let _scope = name_scope(resource);
    // SAFETY: Forward live Ogre receiver unchanged with native unwinding allowed.
    unsafe { h.load.call(resource, background) };
}
unsafe extern "C-unwind" fn prepare_resource(resource: *mut c_void, background: bool) {
    let Some(h) = HOOKS.get() else { return };
    let _scope = name_scope(resource);
    // SAFETY: Same-hash Resource::prepare ABI matches Resource::load.
    unsafe { h.prepare.call(resource, background) };
}
fn name_scope(resource: *mut c_void) -> Option<NameScope> {
    let h = HOOKS.get()?;
    std::panic::catch_unwind(|| {
        if !ACTIVE.load(Ordering::Acquire) || resource.is_null() {
            return None;
        }
        // SAFETY: Ogre calls its Resource::load with its live native Resource;
        // same-hash exported getter borrows its std::string only for this copy.
        let bytes =
            unsafe { rekenshi_native::ogre::string((h.name)(resource)) }.unwrap_or_default();
        let name: String = String::from_utf8_lossy(&bytes).chars().take(512).collect();
        Some(NameScope(NAME.with(|previous| previous.replace(name))))
    })
    .ok()
    .flatten()
}
fn bounded(text: &[u8]) -> String {
    String::from_utf8_lossy(text)
        .chars()
        .filter(|c| !c.is_control())
        .take(512)
        .collect()
}
unsafe fn observe(raw: *mut c_void, description: Description) {
    if !ACTIVE.load(Ordering::Acquire) || raw.is_null() {
        return;
    }
    let store = SESSION.lock().ok().and_then(|session| session.clone());
    let Some(store) = store else { return };
    // SAFETY: Successful create returns a live ID3D11Resource, whose vtable begins
    // with ID3D11DeviceChild. Borrowing does not AddRef or retain the resource.
    let Some(child) = (unsafe { ID3D11DeviceChild::from_raw_borrowed(&raw) }) else {
        return;
    };
    let mut bytes = [0u8; 512];
    let mut length = bytes.len() as u32;
    // SAFETY: Bounded writable buffer and native copied private debug-name bytes.
    let debug_name = if unsafe {
        child.GetPrivateData(
            &WKPDID_D3DDebugObjectName,
            &mut length,
            Some(bytes.as_mut_ptr().cast()),
        )
    }
    .is_ok()
    {
        bounded(&bytes[..(length as usize).min(bytes.len())])
    } else {
        String::new()
    };
    let name = NAME.with(|name| name.borrow().clone());
    let staging = description.usage == D3D11_USAGE_STAGING.0;
    let frames = origin::capture();
    let id = store.lock().ok().and_then(|mut store| {
        store.create(description, name, debug_name, frames, batch::current())
    });
    let Some(id) = id else { return };
    if staging {
        let epoch = store.lock().map(|s| s.epoch).unwrap_or(0);
        let mut tag = [0u8; 16];
        tag[..8].copy_from_slice(&epoch.to_le_bytes());
        tag[8..].copy_from_slice(&id.to_le_bytes());
        // SAFETY: COM copies these 16 bytes; tag identifies this capture session
        // and ID only, without retaining any resource or store pointer.
        if unsafe { child.SetPrivateData(&activity::TAG, 16, Some(tag.as_ptr().cast())) }.is_err() {
            if let Ok(mut store) = store.lock() {
                store.tag_failed();
            }
        }
    }
    let marker = lifetime::marker(&store, id);
    // SAFETY: COM holds the marker's reference, not a resource reference. The
    // private GUID is unique to this diagnostic and never intentionally replaced.
    if unsafe { child.SetPrivateDataInterface(&TAG, &marker) }.is_ok() {
        lifetime::arm(&marker);
    } else if let Ok(mut store) = store.lock() {
        store.retire(id, false);
    }
}
macro_rules! creation {
    ($function:ident, $field:ident, $desc:ty) => {
        unsafe extern "system" fn $function(
            device: *mut c_void,
            desc: *const $desc,
            data: *const D3D11_SUBRESOURCE_DATA,
            output: *mut *mut c_void,
        ) -> i32 {
            let Some(h) = HOOKS.get() else {
                return 0x80004005u32 as i32;
            };
            // SAFETY: Forward the documented ABI unchanged, including null validation outputs.
            let result = unsafe { h.$field.call(device, desc, data, output) };
            if result >= 0 && !desc.is_null() && !output.is_null() {
                let _ = std::panic::catch_unwind(|| {
                    // SAFETY: Successful creation leaves readable descriptor and output.
                    unsafe { observe(output.read(), description(desc.read(), !data.is_null())) };
                });
            }
            result
        }
    };
}
creation!(create_buffer, buffer, D3D11_BUFFER_DESC);
creation!(create_texture1, texture1, D3D11_TEXTURE1D_DESC);
creation!(create_texture2, texture2, D3D11_TEXTURE2D_DESC);
creation!(create_texture3, texture3, D3D11_TEXTURE3D_DESC);
trait Describe {
    fn describe(self, initial: bool) -> Description;
}
fn description<D: Describe>(desc: D, initial: bool) -> Description {
    desc.describe(initial)
}
impl Describe for D3D11_BUFFER_DESC {
    fn describe(self, initial: bool) -> Description {
        Description {
            kind: "buffer",
            width: 0,
            height: 0,
            depth: 0,
            array_size: 0,
            mip_levels: 0,
            samples: 0,
            format: 0,
            usage: self.Usage.0,
            bind_flags: self.BindFlags,
            cpu_access_flags: self.CPUAccessFlags,
            misc_flags: self.MiscFlags,
            buffer_bytes: self.ByteWidth,
            structure_stride: self.StructureByteStride,
            initial_data: initial,
        }
    }
}
macro_rules! texture {
    ($ty:ty, $kind:literal, $height:expr, $depth:expr, $array:expr, $samples:expr) => {
        impl Describe for $ty {
            fn describe(self, initial: bool) -> Description {
                Description {
                    kind: $kind,
                    width: self.Width,
                    height: $height(&self),
                    depth: $depth(&self),
                    array_size: $array(&self),
                    mip_levels: self.MipLevels,
                    samples: $samples(&self),
                    format: self.Format.0,
                    usage: self.Usage.0,
                    bind_flags: self.BindFlags,
                    cpu_access_flags: self.CPUAccessFlags,
                    misc_flags: self.MiscFlags,
                    buffer_bytes: 0,
                    structure_stride: 0,
                    initial_data: initial,
                }
            }
        }
    };
}
texture!(
    D3D11_TEXTURE1D_DESC,
    "texture1d",
    |_: &D3D11_TEXTURE1D_DESC| 1,
    |_: &D3D11_TEXTURE1D_DESC| 1,
    |d: &D3D11_TEXTURE1D_DESC| d.ArraySize,
    |_: &D3D11_TEXTURE1D_DESC| 1
);
texture!(
    D3D11_TEXTURE2D_DESC,
    "texture2d",
    |d: &D3D11_TEXTURE2D_DESC| d.Height,
    |_: &D3D11_TEXTURE2D_DESC| 1,
    |d: &D3D11_TEXTURE2D_DESC| d.ArraySize,
    |d: &D3D11_TEXTURE2D_DESC| d.SampleDesc.Count
);
texture!(
    D3D11_TEXTURE3D_DESC,
    "texture3d",
    |d: &D3D11_TEXTURE3D_DESC| d.Height,
    |d: &D3D11_TEXTURE3D_DESC| d.Depth,
    |_: &D3D11_TEXTURE3D_DESC| 1,
    |_: &D3D11_TEXTURE3D_DESC| 1
);
