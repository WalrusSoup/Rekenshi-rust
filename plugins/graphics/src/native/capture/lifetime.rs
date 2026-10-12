//! IUnknown marker owned by the resource, with a weak link to copied diagnostics.
use super::{SharedStore, store::Store};
use std::ffi::c_void;
use std::sync::{
    Mutex, Weak,
    atomic::{AtomicBool, AtomicU32, Ordering, fence},
};
use windows::core::{GUID, HRESULT, IUnknown, IUnknown_Vtbl, Interface};

#[repr(C)]
struct Marker {
    table: &'static IUnknown_Vtbl,
    refs: AtomicU32,
    armed: AtomicBool,
    store: Weak<Mutex<Store>>,
    id: u64,
}
static TABLE: IUnknown_Vtbl = IUnknown_Vtbl {
    QueryInterface: query,
    AddRef: add_ref,
    Release: release,
};
pub(super) fn marker(store: &SharedStore, id: u64) -> IUnknown {
    let object = Box::new(Marker {
        table: &TABLE,
        refs: AtomicU32::new(1),
        armed: AtomicBool::new(false),
        store: std::sync::Arc::downgrade(store),
        id,
    });
    // SAFETY: repr(C) begins with the exact IUnknown table. This transfers the
    // initial reference; only TABLE::Release reconstructs the host-owned Box.
    unsafe { IUnknown::from_raw(Box::into_raw(object).cast()) }
}
pub(super) fn arm(marker: &IUnknown) {
    // SAFETY: Only marker() constructs objects passed here; caller owns a live ref.
    unsafe { &*marker.as_raw().cast::<Marker>() }
        .armed
        .store(true, Ordering::Release);
}
unsafe extern "system" fn query(
    this: *mut c_void,
    iid: *const GUID,
    output: *mut *mut c_void,
) -> HRESULT {
    if iid.is_null() || output.is_null() {
        return HRESULT(0x80004003u32 as i32);
    }
    // SAFETY: COM caller supplies a live marker, readable IID and writable output.
    unsafe {
        output.write(std::ptr::null_mut());
        if iid.read() != IUnknown::IID {
            return HRESULT(0x80004002u32 as i32);
        }
        add_ref(this);
        output.write(this);
    }
    HRESULT(0)
}
unsafe extern "system" fn add_ref(this: *mut c_void) -> u32 {
    // SAFETY: COM caller owns a live marker reference.
    unsafe { &*this.cast::<Marker>() }
        .refs
        .fetch_add(1, Ordering::Relaxed)
        + 1
}
unsafe extern "system" fn release(this: *mut c_void) -> u32 {
    // SAFETY: COM caller releases one owned reference; final release uniquely owns Box.
    let refs = unsafe { &*this.cast::<Marker>() }
        .refs
        .fetch_sub(1, Ordering::Release)
        - 1;
    if refs == 0 {
        fence(Ordering::Acquire);
        // Never allow Rust unwinding across the system COM ABI during destruction.
        let _ = std::panic::catch_unwind(|| {
            // SAFETY: Exactly the final reference from marker()'s Box allocation.
            let marker = unsafe { Box::from_raw(this.cast::<Marker>()) };
            if marker.armed.load(Ordering::Acquire) {
                if let Some(store) = marker.store.upgrade() {
                    if let Ok(mut store) = store.lock() {
                        store.retire(marker.id, true);
                    }
                }
            }
        });
    }
    refs
}
