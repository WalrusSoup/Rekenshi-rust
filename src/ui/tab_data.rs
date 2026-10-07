//! Kenshi's Options close contract: each MyGUI tab carries DatapanelGUI* data.
//! Clone the native holder's type, but never share ownership of its panel.
use std::ffi::{CStr, c_void};

pub(super) type Copy = unsafe extern "C" fn(*mut c_void, *const c_void) -> *mut c_void;
pub(super) type Type = unsafe extern "C" fn(*const c_void) -> *const c_void;
pub(super) type Destroy = unsafe extern "C" fn(*mut c_void);

pub(super) struct Api {
    pub copy: Copy,
    pub type_of: Type,
    pub destroy: Destroy,
}

pub(super) struct PanelData<'a> {
    api: &'a Api,
    storage: Box<usize>,
    consumed: bool,
}

impl<'a> PanelData<'a> {
    /// # Safety
    /// The caller owns this live TabControl on its UI thread and has verified
    /// the shipped MyGUI hash. `count` is its native item count. API functions
    /// are from that same pinned DLL; the C++ holder's module stays loaded.
    pub unsafe fn null_for_tabs(
        api: &'a Api,
        tabs: *const c_void,
        count: usize,
    ) -> Result<Self, &'static str> {
        // SAFETY: TabControl's vector begins at 0x580; getItemCount and
        // setItemDataAt bodies verify the 0x58 stride and Any at record+0x50.
        // Microsoft x64 type_info stores its terminated raw name at +0x10.
        unsafe {
            let records = tabs.cast::<u8>().add(0x580).cast::<*const u8>().read();
            let end = tabs.cast::<u8>().add(0x588).cast::<*const u8>().read();
            if records.is_null()
                || count == 0
                || count > 64
                || (end as usize).checked_sub(records as usize) != Some(count * 0x58)
            {
                return Err("Options tab data layout differs");
            }
            for index in 0..count {
                let source = records.add(index * 0x58 + 0x50).cast::<c_void>();
                let type_info = (api.type_of)(source);
                if type_info.is_null()
                    || CStr::from_ptr(type_info.cast::<u8>().add(0x10).cast()).to_bytes()
                        != b".PEAVDatapanelGUI@@"
                {
                    continue;
                }
                let mut data = Self {
                    api,
                    storage: Box::new(0),
                    consumed: false,
                };
                (api.copy)(data.storage.as_mut() as *mut usize as *mut c_void, source);
                let holder = *data.storage as *mut u8;
                if holder.is_null() {
                    return Err("native Options panel holder did not clone");
                }
                // Any::castUnsafe's verified body reads holder+8. Only this
                // independently allocated pointer holder is changed. Native
                // clone/destruction keep RTTI, vtables and allocation in C++.
                holder
                    .add(8)
                    .cast::<*mut c_void>()
                    .write(std::ptr::null_mut());
                return Ok(data);
            }
        }
        Err("Options has no DatapanelGUI* data template")
    }

    /// # Safety
    /// Immediately pass this address as the by-value Any temporary to native
    /// TabControl::addItem, whose verified body destroys it on return/unwind.
    pub unsafe fn transfer(&mut self) -> *mut c_void {
        self.consumed = true;
        (self.storage.as_mut() as *mut usize).cast()
    }
}

impl Drop for PanelData<'_> {
    fn drop(&mut self) {
        if !self.consumed {
            // SAFETY: Storage was native-copy-constructed, or contains an empty
            // Any's null pointer. The native owning destructor runs once.
            unsafe { (self.api.destroy)((self.storage.as_mut() as *mut usize).cast()) };
        }
    }
}
