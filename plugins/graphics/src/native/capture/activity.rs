//! Staging usage observed without keeping resources alive or caching pointers.
use super::{ACTIVE, HOOKS, SESSION, store::Operation};
use detour::{Transaction, TypedDetour};
use rekenshi_plugin_api::HostV1;
use std::ffi::c_void;
use std::sync::atomic::Ordering;
use windows::Win32::Graphics::Direct3D11::*;
use windows::core::{GUID, Interface};

pub(super) const TAG: GUID = GUID::from_u128(0x4a510149_d9db_4982_9702_0ba50c357cd1);
type Map = unsafe extern "system" fn(
    *mut c_void,
    *mut c_void,
    u32,
    i32,
    u32,
    *mut D3D11_MAPPED_SUBRESOURCE,
) -> i32;
type Unmap = unsafe extern "system" fn(*mut c_void, *mut c_void, u32);
type Copy = unsafe extern "system" fn(*mut c_void, *mut c_void, *mut c_void);
type Region = unsafe extern "system" fn(
    *mut c_void,
    *mut c_void,
    u32,
    u32,
    u32,
    u32,
    *mut c_void,
    u32,
    *const D3D11_BOX,
);
type Update = unsafe extern "system" fn(
    *mut c_void,
    *mut c_void,
    u32,
    *const D3D11_BOX,
    *const c_void,
    u32,
    u32,
);
pub(super) struct Hooks {
    map: TypedDetour<Map>,
    unmap: TypedDetour<Unmap>,
    copy: TypedDetour<Copy>,
    region: TypedDetour<Region>,
    update: TypedDetour<Update>,
}
impl Hooks {
    pub fn prepare(host: HostV1, context: &ID3D11DeviceContext) -> Result<Self, String> {
        let v = context.vtable();
        let addresses = [
            v.Map as *const c_void,
            v.Unmap as _,
            v.CopyResource as _,
            v.CopySubresourceRegion as _,
            v.UpdateSubresource as _,
        ];
        for address in addresses {
            host.reserve_hook(
                std::ptr::NonNull::new(address.cast_mut()).ok_or("activity hook target")?,
            )
            .map_err(|e| e.to_string())?;
        }
        // SAFETY: Documented context vtable slots establish these exact system
        // ABIs. All hooks forward the arguments unchanged before observation.
        unsafe {
            Ok(Self {
                map: TypedDetour::new(
                    std::mem::transmute::<*const c_void, Map>(addresses[0]),
                    map as Map,
                )
                .map_err(|e| e.to_string())?,
                unmap: TypedDetour::new(
                    std::mem::transmute::<*const c_void, Unmap>(addresses[1]),
                    unmap as Unmap,
                )
                .map_err(|e| e.to_string())?,
                copy: TypedDetour::new(
                    std::mem::transmute::<*const c_void, Copy>(addresses[2]),
                    copy as Copy,
                )
                .map_err(|e| e.to_string())?,
                region: TypedDetour::new(
                    std::mem::transmute::<*const c_void, Region>(addresses[3]),
                    region as Region,
                )
                .map_err(|e| e.to_string())?,
                update: TypedDetour::new(
                    std::mem::transmute::<*const c_void, Update>(addresses[4]),
                    update as Update,
                )
                .map_err(|e| e.to_string())?,
            })
        }
    }
    pub fn patch<'a>(&'a self, tx: &mut Transaction<'a>, enabled: bool) {
        if enabled {
            tx.enable(&self.map);
            tx.enable(&self.unmap);
            tx.enable(&self.copy);
            tx.enable(&self.region);
            tx.enable(&self.update);
        } else {
            tx.disable(&self.update);
            tx.disable(&self.region);
            tx.disable(&self.copy);
            tx.disable(&self.unmap);
            tx.disable(&self.map);
        }
    }
}
fn record(raw: *mut c_void, operation: Operation) {
    let _ = std::panic::catch_unwind(|| {
        if raw.is_null() || !ACTIVE.load(Ordering::Acquire) {
            return;
        }
        // SAFETY: Hook argument is a live borrowed ID3D11Resource. Private data
        // copies into our fixed buffer; no AddRef or native address is retained.
        let Some(child) = (unsafe { ID3D11DeviceChild::from_raw_borrowed(&raw) }) else {
            return;
        };
        let mut tag = [0u8; 16];
        let mut size = tag.len() as u32;
        // SAFETY: Valid borrowed child, GUID and exactly sized writable buffer.
        if unsafe { child.GetPrivateData(&TAG, &mut size, Some(tag.as_mut_ptr().cast())) }.is_err()
            || size != 16
        {
            return;
        }
        let epoch = u64::from_le_bytes(tag[..8].try_into().expect("fixed tag"));
        let id = u64::from_le_bytes(tag[8..].try_into().expect("fixed tag"));
        let store = SESSION.lock().ok().and_then(|s| s.clone());
        if let Some(store) = store {
            if let Ok(mut store) = store.lock() {
                store.activity(epoch, id, operation);
            }
        }
    });
}
unsafe extern "system" fn map(
    this: *mut c_void,
    resource: *mut c_void,
    sub: u32,
    mode: i32,
    flags: u32,
    output: *mut D3D11_MAPPED_SUBRESOURCE,
) -> i32 {
    let Some(h) = HOOKS.get() else {
        return 0x80004005u32 as i32;
    };
    // SAFETY: Forward documented native parameters unchanged.
    let result = unsafe {
        h.activity
            .map
            .call(this, resource, sub, mode, flags, output)
    };
    if result >= 0 {
        record(resource, Operation::Map(mode));
    }
    result
}
unsafe extern "system" fn unmap(this: *mut c_void, resource: *mut c_void, sub: u32) {
    let Some(h) = HOOKS.get() else { return };
    // SAFETY: Forward documented native parameters unchanged.
    unsafe { h.activity.unmap.call(this, resource, sub) };
    record(resource, Operation::Unmap);
}
unsafe extern "system" fn copy(this: *mut c_void, dst: *mut c_void, src: *mut c_void) {
    let Some(h) = HOOKS.get() else { return };
    // SAFETY: Forward documented native parameters unchanged.
    unsafe { h.activity.copy.call(this, dst, src) };
    record(dst, Operation::CopyDestination);
    record(src, Operation::CopySource);
}
unsafe extern "system" fn region(
    this: *mut c_void,
    dst: *mut c_void,
    sub: u32,
    x: u32,
    y: u32,
    z: u32,
    src: *mut c_void,
    source_sub: u32,
    bounds: *const D3D11_BOX,
) {
    let Some(h) = HOOKS.get() else { return };
    // SAFETY: Forward documented native parameters unchanged.
    unsafe {
        h.activity
            .region
            .call(this, dst, sub, x, y, z, src, source_sub, bounds)
    };
    record(dst, Operation::CopyDestination);
    record(src, Operation::CopySource);
}
unsafe extern "system" fn update(
    this: *mut c_void,
    dst: *mut c_void,
    sub: u32,
    bounds: *const D3D11_BOX,
    data: *const c_void,
    pitch: u32,
    depth_pitch: u32,
) {
    let Some(h) = HOOKS.get() else { return };
    // SAFETY: Forward documented native parameters unchanged.
    unsafe {
        h.activity
            .update
            .call(this, dst, sub, bounds, data, pitch, depth_pitch)
    };
    record(dst, Operation::Update);
}
