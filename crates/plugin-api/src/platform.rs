//! Extensible copied-byte operation contracts, independent of native target catalogs.
//!
//! Providers own native bindings and advertise evidence, payload schema and lane.
//! The broker owns registration order and quiescence, never native game objects.
//! Names include a protocol version (for example `org.example.operation@1`).
//! Frozen HostV1 and PluginV1 layouts are unchanged.
use crate::{HostV1, INCOMPATIBLE, OK, hooks::UNAVAILABLE};
use std::ffi::c_void;

/// Optional platform table version.
pub const VERSION: u32 = 1;
/// A composed sequence of source/result transforms.
pub const TRANSFORM: u32 = 1;
/// Exactly one consumer may replace this operation.
pub const EXCLUSIVE: u32 = 2;
/// A lifecycle/capability generation; never accepts operation interception.
pub const LIFECYCLE: u32 = 3;
/// Copied occurrence events delivered later on ordinary serialized plugin state.
pub const EVENT: u32 = 4;
/// Provider has not published a usable generation.
pub const WAITING: u32 = 0;
/// Provider generation is usable.
pub const READY: u32 = 1;
/// Generation has been revoked; native access is no longer permitted.
pub const REVOKED: u32 = 2;
/// Registration missed a broker-observed operation; earlier native bootstrap is not observable here.
pub const LATE: i32 = 5;
/// Payload or registration conflicts with its declared contract.
pub const CONFLICT: i32 = 6;
/// Reentrant mutation or dispatch is rejected to prevent callback deadlocks.
pub const REENTRANT: i32 = 7;
/// Callback explicitly leaves the current payload unchanged.
pub const PASS: i32 = 8;
/// Callback panicked; its owner is quarantined and requires restart.
pub const FAULTED: i32 = 9;
/// Maximum copied operation payload (16 MiB).
pub const MAX_PAYLOAD: usize = 16 * 1024 * 1024;

/// Copied, bounded description of a provider's synchronous operation.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ContractV1 {
    /// Exact record size.
    pub struct_size: u32,
    /// TRANSFORM, EXCLUSIVE, LIFECYCLE or EVENT composition mode.
    pub mode: u32,
    /// Maximum input/output payload size in bytes.
    pub max_payload: u32,
    /// Reserved; must be zero.
    pub reserved: u32,
    /// NUL-terminated namespaced operation and protocol version.
    pub name: [u8; 128],
    /// NUL-terminated schema identity; never a Rust TypeId.
    pub schema: [u8; 128],
    /// NUL-terminated execution lane, reentrancy and native-access contract.
    pub lane: [u8; 256],
    /// NUL-terminated build fingerprints, binding evidence and verification limits.
    pub evidence: [u8; 1024],
}

/// Copied generation and activation status; contains no native pointer.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StatusV1 {
    /// Token owning the provider, or zero when absent.
    pub provider: u64,
    /// Monotonically increasing generation, including across provider replacement.
    pub generation: u64,
    /// Monotonically increasing status revision.
    pub revision: u64,
    /// WAITING, READY or REVOKED.
    pub phase: u32,
    /// Nonzero after the first attempted dispatch; readiness alone is not first use.
    pub used: u32,
}
/// Bounded copied occurrence, never a grant of access to native resources.
#[repr(C)]
#[derive(Clone, Debug)]
pub struct EventV1 {
    /// Provider generation when observed, independent of any world ID in the payload.
    pub generation: u64,
    /// Initialized metadata byte count, at most 512.
    pub metadata_len: u32,
    /// Initialized payload byte count, at most 4096.
    pub payload_len: u32,
    /// Provider-defined copied metadata.
    pub metadata: [u8; 512],
    /// Provider-defined copied occurrence data.
    pub payload: [u8; 4096],
}
impl EventV1 {
    /// Read bounded copied payload bytes.
    pub fn data(&self) -> &[u8] {
        &self.payload[..(self.payload_len as usize).min(self.payload.len())]
    }
    /// Read bounded copied metadata bytes.
    pub fn metadata(&self) -> &[u8] {
        &self.metadata[..(self.metadata_len as usize).min(self.metadata.len())]
    }
}

/// Synchronous callback; inputs and output are borrowed only until return.
///
/// Return OK with initialized output/length, PASS to preserve input, or ERROR.
/// Never retain pointers or unwind. Userdata lives until remove returns OK.
pub type Intercept = unsafe extern "C" fn(
    *mut c_void,
    *const u8,
    usize,
    *const u8,
    usize,
    *mut u8,
    usize,
    *mut usize,
) -> i32;
/// Negotiate into exact-sized, aligned caller-owned output.
pub type GetApi = unsafe extern "C" fn(u32, *mut PlatformV1, usize) -> i32;

/// Versioned broker transport; all allocations stay in their originating DLL.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct PlatformV1 {
    /// Exact table size.
    pub struct_size: u32,
    /// VERSION of this independently negotiated table.
    pub abi_version: u32,
    /// Publish a contract; only the resulting provider may dispatch/revoke it.
    pub publish: unsafe extern "C" fn(u64, *const ContractV1, usize, *mut u64) -> i32,
    /// Copy the full descriptor and current status, including revoked generations.
    pub query: unsafe extern "C" fn(*const u8, usize, *mut ContractV1, *mut StatusV1) -> i32,
    /// Register while waiting or ready; OK/LATE both return an owned handle.
    pub subscribe: unsafe extern "C" fn(
        u64,
        *const u8,
        usize,
        *const u8,
        usize,
        i32,
        u32,
        Intercept,
        *mut c_void,
        *mut u64,
    ) -> i32,
    /// Remove an owned subscription and quiesce admitted callbacks before OK.
    pub remove: extern "C" fn(u64, u64) -> i32,
    /// Transform copied bytes synchronously; only the live provider may call.
    pub dispatch: unsafe extern "C" fn(
        u64,
        *const u8,
        usize,
        u64,
        *const u8,
        usize,
        *const u8,
        usize,
        *mut u8,
        usize,
        *mut usize,
    ) -> i32,
    /// Revoke this provider generation before retiring its native resources.
    pub revoke: unsafe extern "C" fn(u64, *const u8, usize, u64) -> i32,
    /// Watch capability generations, replaying current status once if already published.
    pub watch: unsafe extern "C" fn(u64, *const u8, usize, *mut u64) -> i32,
    /// Drain one copied transition; PASS means no pending transition.
    pub next: unsafe extern "C" fn(u64, u64, *mut StatusV1) -> i32,
    /// Remove an owned lifecycle watch; no callbacks or borrowed userdata are retained.
    pub unwatch: extern "C" fn(u64, u64) -> i32,
    /// Copy the current composition revision for a live provider generation.
    pub stamp: unsafe extern "C" fn(u64, *const u8, usize, u64, *mut u64, *mut u32) -> i32,
    /// Invalidate cached policy after changing a registration's DLL-local captures.
    pub changed: extern "C" fn(u64, u64) -> i32,
    /// Whether a host-issued owner remains authorized; zero denies access.
    pub authorized: extern "C" fn(u64) -> u32,
    /// Subscribe to future occurrences; does not replay historical events.
    pub watch_events: unsafe extern "C" fn(u64, *const u8, usize, *mut u64) -> i32,
    /// Drain one copied event; PASS when empty, ERROR on queue overflow.
    pub next_event: unsafe extern "C" fn(u64, u64, *mut EventV1) -> i32,
    /// Set newline-separated semantic ordering identities before first use.
    pub order: unsafe extern "C" fn(u64, u64, *const u8, usize, *const u8, usize) -> i32,
    /// Copy a stable 32-byte SHA-256 policy manifest for a named contract.
    pub manifest: unsafe extern "C" fn(*const u8, usize, *mut u8) -> i32,
    /// Quiesce and permanently quarantine the calling owner's operation providers/consumers.
    pub quarantine: extern "C" fn(u64) -> i32,
}

/// Cached connection used identically by bundled and independently compiled providers.
#[derive(Clone, Copy, Debug)]
pub struct Platform {
    api: PlatformV1,
    token: u64,
}

/// Owned copied lifecycle subscription, drained on the serial plugin lane.
#[derive(Debug)]
pub struct Watch {
    platform: Platform,
    handle: u64,
}
/// Owned copied event stream, drained on the serial plugin lane.
#[derive(Debug)]
pub struct EventWatch {
    platform: Platform,
    handle: u64,
}
impl EventWatch {
    /// Drain one occurrence. Overflow fails explicitly and requires restart/resubscription.
    ///
    /// # Errors
    /// Rejects revoked owners, missing streams and overflow.
    pub fn next_event(&self) -> Result<Option<EventV1>, i32> {
        let mut event = std::mem::MaybeUninit::uninit();
        // SAFETY: Pinned table and exact aligned writable event storage.
        match unsafe {
            (self.platform.api.next_event)(self.platform.token, self.handle, event.as_mut_ptr())
        } {
            // SAFETY: OK initializes the entire fixed record.
            OK => Ok(Some(unsafe { event.assume_init() })),
            PASS => Ok(None),
            code => Err(code),
        }
    }
}
impl Drop for EventWatch {
    fn drop(&mut self) {
        (self.platform.api.unwatch)(self.platform.token, self.handle);
    }
}
impl Platform {
    /// Watch future copied domain occurrences, including before publication.
    ///
    /// # Errors
    /// Rejects invalid names, incompatible modes, excessive watches and revoked owners.
    pub fn watch_events(&self, name: &str) -> Result<EventWatch, i32> {
        let mut handle = 0;
        // SAFETY: Borrowed name and aligned writable handle output.
        let result =
            unsafe { (self.api.watch_events)(self.token, name.as_ptr(), name.len(), &mut handle) };
        if result != OK {
            return Err(result);
        }
        Ok(EventWatch {
            platform: *self,
            handle,
        })
    }
    /// Validate an owner for provider-local scalar services through portable C ABI.
    pub fn authorized(&self, owner: u64) -> bool {
        (self.api.authorized)(owner) == 1
    }
    /// Copy the negotiated C table; invoking raw callbacks requires their documented safety contracts.
    pub fn api(&self) -> PlatformV1 {
        self.api
    }
    /// Return the host-issued owner token for this connection.
    pub fn token(&self) -> u64 {
        self.token
    }
    /// Watch a namespaced provider generation, including while its provider is absent.
    ///
    /// # Errors
    /// Rejects malformed names, excessive watches, revoked owners and callback reentry.
    pub fn watch(&self, name: &str) -> Result<Watch, i32> {
        let mut handle = 0;
        // SAFETY: Borrowed readable name and aligned writable output.
        let result =
            unsafe { (self.api.watch)(self.token, name.as_ptr(), name.len(), &mut handle) };
        if result != OK {
            return Err(result);
        }
        Ok(Watch {
            platform: *self,
            handle,
        })
    }
}
impl Watch {
    /// Drain one transition; snapshots never grant access to native objects.
    ///
    /// Superseded ready notifications are removed before revocation is enqueued.
    /// Queues are bounded; overflow returns ERROR and requires resubscription.
    ///
    /// # Errors
    /// Returns owner, queue overflow or host-state failures.
    pub fn next_transition(&self) -> Result<Option<StatusV1>, i32> {
        let mut output = StatusV1::default();
        // SAFETY: Aligned writable output; the broker retains no pointer.
        match unsafe { (self.platform.api.next)(self.platform.token, self.handle, &mut output) } {
            OK => Ok(Some(output)),
            PASS => Ok(None),
            code => Err(code),
        }
    }
}
impl Drop for Watch {
    fn drop(&mut self) {
        (self.platform.api.unwatch)(self.platform.token, self.handle);
    }
}

impl HostV1 {
    /// Negotiate the extensible operation broker without changing HostV1.
    ///
    /// # Errors
    /// Returns UNAVAILABLE or the negotiation/layout failure status.
    pub fn platform(&self) -> Result<Platform, i32> {
        let address = self
            .find_export(c"RE_Kenshi_Rust.dll", c"rekenshi_platform_api")
            .ok_or(UNAVAILABLE)?;
        // SAFETY: Host resolver pins the documented versioned GetApi export.
        let get: GetApi = unsafe { std::mem::transmute(address.as_ptr()) };
        let mut output = std::mem::MaybeUninit::uninit();
        // SAFETY: Exact version and writable aligned output storage.
        let result = unsafe { get(VERSION, output.as_mut_ptr(), size_of::<PlatformV1>()) };
        if result != OK {
            return Err(result);
        }
        // SAFETY: OK guarantees the complete negotiated record was initialized.
        let api: PlatformV1 = unsafe { output.assume_init() };
        if api.struct_size as usize != size_of::<PlatformV1>() || api.abi_version != VERSION {
            return Err(INCOMPATIBLE);
        }
        Ok(Platform {
            api,
            token: self.plugin_token,
        })
    }
}
