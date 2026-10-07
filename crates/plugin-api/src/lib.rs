//! Versioned, Windows x64 C interface for independent RE_Kenshi Rust DLL mods.
//! Only scalars, byte arrays, pointers, and C function pointers cross DLLs.
//! Allocation ownership stays with the DLL that allocated the data.
//! Lifecycle callbacks must not reenter dllStartPlugin/dllStopPlugin.
#![deny(missing_docs)]
use std::ffi::{CStr, c_char, c_void};
pub mod gameplay;
pub mod graphics;
pub mod hooks;
pub mod services;
pub mod weather;

/// Frozen plugin and host C table version.
pub const ABI_VERSION: u32 = 1;
/// Operation completed and all promised output was initialized.
pub const OK: i32 = 0;
/// Operation failed; output must not be interpreted as successful.
pub const ERROR: i32 = 1;
/// Requested table size or ABI version is unsupported.
pub const INCOMPATIBLE: i32 = 2;
/// The native address is already reserved by another owner.
pub const HOOK_BUSY: i32 = 3;
/// NUL-terminated DLL export name used to negotiate plugin metadata.
pub const ENTRY_EXPORT: &[u8] = b"rekenshi_plugin_entry\0";

/// Negotiated plugin descriptor export using caller-owned output storage.
pub type Entry = unsafe extern "C" fn(u32, *mut PluginV1, usize) -> i32;
/// Plugin startup callback receiving a trusted process-lived host table.
pub type Start = unsafe extern "C" fn(*const HostV1) -> i32;
/// Plugin cleanup callback; quiesce resources and contain Rust panics.
pub type Stop = unsafe extern "C" fn() -> i32;
/// Synchronous logging callback accepting token-scoped readable UTF-8 bytes.
pub type Log = unsafe extern "C" fn(u64, *const u8, usize);
/// Resolve a named export in a mapped module and pin its executable lifetime.
pub type ResolveExport = unsafe extern "C" fn(*const c_char, *const c_char) -> *const c_void;
/// Reserve or release an opaque native address for a validated owner token.
pub type ClaimHook = unsafe extern "C" fn(u64, *const c_void) -> i32;
/// Return the cumulative count of forwarded Ogre calls.
pub type HookCalls = extern "C" fn() -> u64;

/// The host supplies a stable table before calling start. Valid for process life.
/// Functions may be used by plugin threads; stop must quiesce those threads.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct HostV1 {
    /// Size of this C record in bytes; negotiation requires an exact match.
    pub struct_size: u32,
    /// Version of this table; negotiate before invoking its callbacks.
    pub abi_version: u32,
    /// Host-issued owner token; zero is reserved for the bootstrap.
    pub plugin_token: u64,
    log: Log,
    resolve_export: ResolveExport,
    claim_hook: ClaimHook,
    release_hook: ClaimHook,
    ogre_hook_calls: HookCalls,
}

impl HostV1 {
    /// Construct a host table from process-lived callbacks with verified contracts.
    ///
    /// # Safety
    /// All callbacks must implement the V1 contracts for every token and valid
    /// input, support concurrent calls, and remain mapped for process life.
    /// `resolve_export` must return null or a pinned export with the signature
    /// and complete initialization behavior specified by its module and name.
    /// Logging must consume the supplied bytes synchronously; hook operations
    /// must treat targets as opaque addresses and validate ownership tokens.
    ///
    /// Safe callers cannot invent a trusted resolver:
    /// ```compile_fail
    /// use rekenshi_plugin_api::HostV1;
    /// extern "C" fn log(_: u64, _: *const u8, _: usize) {}
    /// extern "C" fn resolve(_: *const std::ffi::c_char, _: *const std::ffi::c_char)
    ///     -> *const std::ffi::c_void { 1usize as *const _ }
    /// extern "C" fn hook(_: u64, _: *const std::ffi::c_void) -> i32 { 0 }
    /// extern "C" fn calls() -> u64 { 0 }
    /// let host = HostV1::new(1, log, resolve, hook, hook, calls);
    /// ```
    pub const unsafe fn new(
        plugin_token: u64,
        log: Log,
        resolve_export: ResolveExport,
        claim_hook: ClaimHook,
        release_hook: ClaimHook,
        ogre_hook_calls: HookCalls,
    ) -> Self {
        Self {
            struct_size: size_of::<Self>() as u32,
            abi_version: ABI_VERSION,
            plugin_token,
            log,
            resolve_export,
            claim_hook,
            release_hook,
            ogre_hook_calls,
        }
    }

    /// Create a harmless host for testing Rust logic without native services.
    ///
    /// Logging is discarded, exports are unavailable, reservations return ERROR,
    /// and the native call count is zero. No supplied executable address is used.
    #[cfg(feature = "test-util")]
    pub const fn stub(plugin_token: u64) -> Self {
        extern "C" fn log(_: u64, _: *const u8, _: usize) {}
        extern "C" fn resolve(_: *const c_char, _: *const c_char) -> *const c_void {
            std::ptr::null()
        }
        extern "C" fn hook(_: u64, _: *const c_void) -> i32 {
            ERROR
        }
        extern "C" fn calls() -> u64 {
            0
        }
        // SAFETY: These process-lived callbacks never dereference inputs or
        // return executable addresses; they reject all native operations.
        unsafe { Self::new(plugin_token, log, resolve, hook, hook, calls) }
    }

    /// Return the host's cumulative count of forwarded native Ogre calls.
    pub fn ogre_hook_calls(&self) -> u64 {
        (self.ogre_hook_calls)()
    }

    /// Copy a host table after validating its negotiated version and size.
    ///
    /// # Errors
    /// Returns ERROR for null input or INCOMPATIBLE for an unsupported layout.
    ///
    /// # Safety
    /// pointer must refer to an aligned, readable HostV1 for this call. The host
    /// must supply function pointers obeying the V1 contracts, mapped for process life.
    pub unsafe fn read(pointer: *const Self) -> Result<Self, i32> {
        if pointer.is_null() {
            return Err(ERROR);
        }
        // SAFETY: The caller guarantees valid, aligned table storage. Copying
        // retains only scalars and function pointers, never borrowed allocations.
        let host = unsafe { pointer.read() };
        if host.abi_version != ABI_VERSION || host.struct_size as usize != size_of::<Self>() {
            return Err(INCOMPATIBLE);
        }
        Ok(host)
    }

    /// Send readable UTF-8 diagnostic bytes through the trusted host logger.
    pub fn write_log(&self, message: &str) {
        // SAFETY: V1 Log accepts this valid slice for the synchronous call and
        // never retains its pointer. Unknown tokens are handled by the host.
        unsafe { (self.log)(self.plugin_token, message.as_ptr(), message.len()) };
    }

    /// Finds an export in an already loaded module, which the host then pins.
    /// No unloaded module is implicitly loaded by this operation.
    pub fn find_export(&self, module: &CStr, name: &CStr) -> Option<std::ptr::NonNull<c_void>> {
        // SAFETY: Both C strings remain valid for the synchronous lookup.
        std::ptr::NonNull::new(
            unsafe { (self.resolve_export)(module.as_ptr(), name.as_ptr()) }.cast_mut(),
        )
    }

    /// Reserve the actual implementation address before installing any detour.
    /// This coordinates ownership; it does not install a hook or check its ABI.
    pub fn reserve_hook(&self, target: std::ptr::NonNull<c_void>) -> Result<(), i32> {
        // SAFETY: The host treats target as an opaque address and does not dereference it.
        match unsafe { (self.claim_hook)(self.plugin_token, target.as_ptr()) } {
            OK => Ok(()),
            error => Err(error),
        }
    }

    /// Release a reservation only after the hook is disabled and callbacks quiesce.
    pub fn relinquish_hook(&self, target: std::ptr::NonNull<c_void>) -> Result<(), i32> {
        // SAFETY: The host uses target only as an opaque ownership-map key.
        match unsafe { (self.release_hook)(self.plugin_token, target.as_ptr()) } {
            OK => Ok(()),
            error => Err(error),
        }
    }
}

/// Plugin-owned entry point fills caller-owned output only when V1 is accepted.
/// start and stop must contain Rust panics. A failed start is followed by stop
/// to clean partial initialization. stop must disable hooks and quiesce workers.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct PluginV1 {
    /// Size of this C record in bytes; negotiation requires an exact match.
    pub struct_size: u32,
    /// Version of this table; negotiate before invoking its callbacks.
    pub abi_version: u32,
    /// NUL-terminated ASCII plugin identifier used for discovery and dependencies.
    pub id: [u8; 64],
    /// NUL-terminated ASCII plugin display name.
    pub name: [u8; 96],
    /// NUL-terminated ASCII plugin release version.
    pub version: [u8; 32],
    /// Startup callback called once with the negotiated stable host table.
    pub start: Option<Start>,
    /// Cleanup callback called after callback groups and workers are quiesced.
    pub stop: Option<Stop>,
}

impl PluginV1 {
    /// Zeroed output placeholder for a negotiation or query.
    pub const EMPTY: Self = Self {
        struct_size: 0,
        abi_version: 0,
        id: [0; 64],
        name: [0; 96],
        version: [0; 32],
        start: None,
        stop: None,
    };
    /// Create a plugin descriptor with bounded ASCII metadata and lifecycle callbacks.
    ///
    /// # Panics
    /// Panics if metadata is empty, non-ASCII, contains NUL, or exceeds its buffer.
    pub const fn new(id: &str, name: &str, version: &str, start: Start, stop: Stop) -> Self {
        Self {
            struct_size: size_of::<Self>() as u32,
            abi_version: ABI_VERSION,
            id: metadata(id),
            name: metadata(name),
            version: metadata(version),
            start: Some(start),
            stop: Some(stop),
        }
    }
}

/// Copy an immutable descriptor to a host-provided V1 output buffer.
/// # Safety
/// For an accepted ABI and size, output must point to aligned writable storage
/// for one PluginV1. Rejected ABI/size requests never dereference output.
pub unsafe fn export_descriptor(
    abi: u32,
    output: *mut PluginV1,
    size: usize,
    descriptor: PluginV1,
) -> i32 {
    if abi != ABI_VERSION || size != size_of::<PluginV1>() {
        return INCOMPATIBLE;
    }
    if output.is_null() {
        return ERROR;
    }
    // SAFETY: Accepted request; the caller guarantees writable aligned storage.
    unsafe { output.write(descriptor) };
    OK
}

const fn metadata<const N: usize>(text: &str) -> [u8; N] {
    assert!(!text.is_empty() && text.len() < N);
    let mut result = [0; N];
    let bytes = text.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        assert!(bytes[index] > 0 && bytes[index] < 128);
        result[index] = bytes[index];
        index += 1;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    unsafe extern "C" fn start(_: *const HostV1) -> i32 {
        OK
    }
    unsafe extern "C" fn stop() -> i32 {
        OK
    }
    #[test]
    fn v1_layout_and_output_contract() {
        if cfg!(target_pointer_width = "64") {
            assert_eq!(size_of::<HostV1>(), 56);
            assert_eq!(size_of::<PluginV1>(), 216);
        }
        #[repr(C)]
        struct Guard {
            before: u64,
            descriptor: PluginV1,
            after: u64,
        }
        let mut guard = Guard {
            before: 0xfeedface,
            descriptor: PluginV1::EMPTY,
            after: 0xdeadbeef,
        };
        let descriptor = PluginV1::new("test", "Test mod", "1.0", start, stop);
        // SAFETY: Output is valid aligned writable PluginV1 storage; wrong ABI
        // and size requests must leave it untouched, preserving adjacent guards.
        unsafe {
            assert_eq!(
                export_descriptor(99, &mut guard.descriptor, size_of::<PluginV1>(), descriptor),
                INCOMPATIBLE
            );
            assert_eq!(
                export_descriptor(ABI_VERSION, &mut guard.descriptor, 1, descriptor),
                INCOMPATIBLE
            );
            assert_eq!(guard.descriptor.abi_version, 0);
            assert_eq!(
                export_descriptor(
                    ABI_VERSION,
                    std::ptr::null_mut(),
                    size_of::<PluginV1>(),
                    descriptor
                ),
                ERROR
            );
            assert_eq!(
                export_descriptor(
                    ABI_VERSION,
                    &mut guard.descriptor,
                    size_of::<PluginV1>(),
                    descriptor
                ),
                OK
            );
        }
        assert_eq!(guard.before, 0xfeedface);
        assert_eq!(guard.after, 0xdeadbeef);
        assert_eq!(guard.descriptor.abi_version, ABI_VERSION);
        assert_eq!(&guard.descriptor.id[..5], b"test\0");
    }
}
