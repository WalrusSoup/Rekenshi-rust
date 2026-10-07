//! Named, process-local native targets. Numeric IDs cross the C boundary; the
//! Rust enum remains typed. Adding entries never changes the frozen HostV1.
use crate::{HostV1, INCOMPATIBLE, OK};
use std::ffi::c_void;
use std::ptr::NonNull;

/// Version negotiated for this optional service.
pub const VERSION: u32 = 1;
/// The requested service or native target is unavailable.
pub const UNAVAILABLE: i32 = 4;
/// Hook status indicating an unreserved original native address.
pub const AVAILABLE: u32 = 1;
/// Hook status indicating a native address reserved by an owner.
pub const RESERVED: u32 = 2;

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
/// Stable numeric identifiers for inspected native hook signatures.
pub enum HookId {
    /// Original Ogre root singleton getter.
    OgreRootSingleton = 1,
    /// Original MyGUI text-caption setter.
    MyGuiSetCaption = 2,
    /// Original shader compiler entry point.
    D3DCompile = 3,
    /// Original D3D shader-buffer allocator.
    D3DCreateBlob = 4,
    /// Verified live D3D11 context draw implementation.
    D3D11Draw = 5,
}
impl HookId {
    /// All hook identifiers understood by this ABI version.
    pub const ALL: [Self; 5] = [
        Self::OgreRootSingleton,
        Self::MyGuiSetCaption,
        Self::D3DCompile,
        Self::D3DCreateBlob,
        Self::D3D11Draw,
    ];
    /// Return the human-readable native signature name of this hook.
    pub const fn name(self) -> &'static str {
        match self {
            Self::OgreRootSingleton => "Ogre::Root::getSingletonPtr",
            Self::MyGuiSetCaption => "MyGUI::TextBox::setCaption",
            Self::D3DCompile => "D3DCompiler_43::D3DCompile",
            Self::D3DCreateBlob => "D3DCompiler_43::D3DCreateBlob",
            Self::D3D11Draw => "ID3D11DeviceContext::Draw",
        }
    }
    /// Decode a known numeric hook ID; return None for unknown identifiers.
    pub const fn from_raw(id: u32) -> Option<Self> {
        match id {
            1 => Some(Self::OgreRootSingleton),
            2 => Some(Self::MyGuiSetCaption),
            3 => Some(Self::D3DCompile),
            4 => Some(Self::D3DCreateBlob),
            5 => Some(Self::D3D11Draw),
            _ => None,
        }
    }
}

/// Negotiate this service into aligned exact-sized caller-owned output.
pub type GetApi = unsafe extern "C" fn(u32, *mut HooksV1, usize) -> i32;
/// Copy hook information for a known numeric hook ID.
pub type Resolve = unsafe extern "C" fn(u32, *mut HookInfoV1, usize) -> i32;
/// Publish a verified native hook address for the validated owner token.
pub type Publish = unsafe extern "C" fn(u64, u32, *const c_void) -> i32;
#[repr(C)]
#[derive(Clone, Copy, Debug)]
/// Negotiated callbacks for named native hook discovery and publication.
pub struct HooksV1 {
    /// Size of this C record in bytes; negotiation requires an exact match.
    pub struct_size: u32,
    /// Version of this table; negotiate before invoking its callbacks.
    pub abi_version: u32,
    /// Resolve a known hook ID into exact-sized caller-owned output.
    pub resolve: Resolve,
    /// Publish a verified pinned native implementation for a reserved dynamic hook.
    pub publish: Publish,
}
#[repr(C)]
#[derive(Clone, Copy, Debug)]
/// C-layout description of a process-local native hook address and owner.
pub struct HookInfoV1 {
    /// Size of this C record in bytes; negotiation requires an exact match.
    pub struct_size: u32,
    /// Opaque identifier within its world generation.
    pub id: u32,
    /// Original entry address, not a trampoline. Valid only in this process.
    pub address: *const c_void,
    /// Meaningful when status=RESERVED. Token 0 denotes the bootstrap.
    pub owner_token: u64,
    /// Copy service readiness into caller-owned writable output.
    pub status: u32,
    /// Reserved for future versions; callers must write zero.
    pub reserved: u32,
}
impl HookInfoV1 {
    /// Zeroed output placeholder for a negotiation or query.
    pub const EMPTY: Self = Self {
        struct_size: 0,
        id: 0,
        address: std::ptr::null(),
        owner_token: 0,
        status: 0,
        reserved: 0,
    };
    /// Return a non-null original native target address from the validated record.
    ///
    /// # Errors
    /// Returns a failure when service negotiation, validation, or the host operation fails.
    pub fn target(self) -> Result<NonNull<c_void>, i32> {
        NonNull::new(self.address.cast_mut()).ok_or(UNAVAILABLE)
    }
}

/// Copy this handle into plugin state once during start. Subsequent queries call
/// the host's cached table directly, without per-query export lookup.
#[derive(Clone, Copy, Debug)]
pub struct HookCatalog {
    api: HooksV1,
    token: u64,
}
impl HostV1 {
    /// Negotiate a cached connection to the optional native hook catalog.
    ///
    /// # Errors
    /// Returns a failure when service negotiation, validation, or the host operation fails.
    pub fn hooks(&self) -> Result<HookCatalog, i32> {
        let address = self
            .find_export(c"RE_Kenshi_Rust.dll", c"rekenshi_hooks_api")
            .ok_or(UNAVAILABLE)?;
        // SAFETY: Named versioned host export implements this SDK signature;
        // find_export pins its module before returning the address.
        let get: GetApi = unsafe { std::mem::transmute(address.as_ptr()) };
        let mut output = std::mem::MaybeUninit::<HooksV1>::uninit();
        // SAFETY: Exact V1 size and aligned writable table output.
        let result = unsafe { get(VERSION, output.as_mut_ptr(), size_of::<HooksV1>()) };
        if result != OK {
            return Err(result);
        }
        // SAFETY: A successful negotiated call initializes the entire table.
        let api = unsafe { output.assume_init() };
        if api.abi_version != VERSION || api.struct_size as usize != size_of::<HooksV1>() {
            return Err(INCOMPATIBLE);
        }
        Ok(HookCatalog {
            api,
            token: self.plugin_token,
        })
    }
}
impl HookCatalog {
    /// Copy and validate the current native target and owner of a named hook.
    ///
    /// # Errors
    /// Returns a failure when service negotiation, validation, or the host operation fails.
    pub fn info(&self, id: HookId) -> Result<HookInfoV1, i32> {
        let mut info = HookInfoV1::EMPTY;
        // SAFETY: Pinned negotiated callback, known numeric ID and writable output.
        let result = unsafe { (self.api.resolve)(id as u32, &mut info, size_of::<HookInfoV1>()) };
        if result != OK {
            return Err(result);
        }
        if info.struct_size as usize != size_of::<HookInfoV1>()
            || info.id != id as u32
            || info.reserved != 0
            || !matches!(info.status, AVAILABLE | RESERVED)
            || info.address.is_null()
        {
            return Err(INCOMPATIBLE);
        }
        Ok(info)
    }
    /// Gets a named original entry. Reservation and detour installation remain
    /// explicit; callers must honor that target's documented native ABI.
    pub fn target(&self, id: HookId) -> Result<NonNull<c_void>, i32> {
        self.info(id)?.target()
    }

    /// Publish a dynamically discovered vtable target after reserving it through
    /// HostV1. Currently only D3D11Draw accepts provider publication.
    /// # Safety
    /// target must be the real implementation of id's documented signature,
    /// in a pinned module. Never publish a trampoline or a plugin callback.
    pub unsafe fn publish(&self, id: HookId, target: NonNull<c_void>) -> Result<(), i32> {
        // SAFETY: Caller guarantees the target ABI; host checks live ownership
        // and restricts publication to dynamic slots, never overwriting builtins.
        match unsafe { (self.api.publish)(self.token, id as u32, target.as_ptr()) } {
            OK => Ok(()),
            e => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ids_and_c_layout_are_stable() {
        for id in HookId::ALL {
            assert_eq!(HookId::from_raw(id as u32), Some(id));
            assert!(!id.name().is_empty());
        }
        for id in [0, 6, u32::MAX] {
            assert_eq!(HookId::from_raw(id), None);
        }
        if cfg!(target_pointer_width = "64") {
            assert_eq!(size_of::<HooksV1>(), 24);
            assert_eq!(size_of::<HookInfoV1>(), 32);
        }
        assert_eq!(HookInfoV1::EMPTY.target(), Err(UNAVAILABLE));
    }
}
