//! Owned public operation registrations and third-party provider publication.
//!
//! Callbacks run synchronously on the advertised native lane, possibly concurrently.
//! They consume copied bytes and may not access ordinary mutable plugin state.
//! Removal from a callback fails; Drop retains userdata if quiescence is impossible.
//! Provider generation revocation must precede destruction of native resources.
use crate::{
    Error,
    abi::{self, platform::*},
};
use std::{ffi::c_void, sync::Mutex};

/// Public compiler source-transform contract implemented by the shader-cache provider.
pub const SHADER_SOURCE: &str = "rekenshi.shader.source@1";
/// Stable shader-source metadata schema: entry bytes, NUL, then target profile bytes.
pub const SHADER_SCHEMA: &str = "rekenshi.shader.entry-profile@1";

/// Cached, copyable connection to the host's extensible provider registry.
#[derive(Clone, Copy, Debug)]
pub struct Operations(Platform);
fn status(code: i32) -> Result<(), Error> {
    if code == abi::OK {
        Ok(())
    } else if code == FAULTED {
        Err(Error::restart_required())
    } else {
        Err(Error::host("platform", code))
    }
}
fn field<const N: usize>(text: &str) -> Result<[u8; N], Error> {
    if text.is_empty() || text.len() >= N || text.contains('\0') {
        return Err("invalid or oversized contract field".into());
    }
    let mut bytes = [0; N];
    bytes[..text.len()].copy_from_slice(text.as_bytes());
    Ok(bytes)
}

/// Description of the provider's byte schema, native lane and exact evidence.
#[derive(Debug)]
pub struct Contract(ContractV1);
impl Contract {
    /// Declare a bounded composed operation; evidence and lane are mandatory.
    ///
    /// # Errors
    /// Rejects empty/oversized fields or an invalid payload limit.
    pub fn new(
        name: &str,
        schema: &str,
        lane: &str,
        evidence: &str,
        limit: usize,
    ) -> Result<Self, Error> {
        if limit == 0 || limit > MAX_PAYLOAD {
            return Err("invalid contract payload bound".into());
        }
        Ok(Self(ContractV1 {
            struct_size: size_of::<ContractV1>() as u32,
            mode: TRANSFORM,
            max_payload: limit as u32,
            reserved: 0,
            name: field(name)?,
            schema: field(schema)?,
            lane: field(lane)?,
            evidence: field(evidence)?,
        }))
    }
    /// Require one exclusive consumer instead of composing transforms.
    pub fn exclusive(mut self) -> Self {
        self.0.mode = EXCLUSIVE;
        self
    }
    /// Declare lifecycle readiness/revocation instead of an interceptable operation.
    pub fn lifecycle(mut self) -> Self {
        self.0.mode = LIFECYCLE;
        self
    }
    /// Declare bounded copied domain occurrences rather than synchronous transforms.
    pub fn events(mut self) -> Self {
        self.0.mode = EVENT;
        self
    }
}

impl Operations {
    /// Negotiate once during installation; works for bundled or independent DLLs.
    ///
    /// # Errors
    /// Returns a structured host status when the optional broker is unavailable.
    pub fn new(context: &crate::Context) -> Result<Self, Error> {
        Self::from_host(context.host.native())
    }
    /// Connect a native provider through its trusted host table.
    ///
    /// # Errors
    /// Returns the negotiation or layout failure.
    pub fn from_host(host: abi::HostV1) -> Result<Self, Error> {
        host.platform()
            .map(Self)
            .map_err(|code| Error::host("platform", code))
    }
    /// Stable policy/version digest for persistent cache identity.
    ///
    /// # Errors
    /// Returns an error for an unpublished contract or invalid ordering.
    pub fn manifest(&self, name: &str) -> Result<[u8; 32], Error> {
        let mut output = [0; 32];
        // SAFETY: Pinned callback, readable name, writable exact 32-byte digest.
        status(unsafe { (self.0.api().manifest)(name.as_ptr(), name.len(), output.as_mut_ptr()) })?;
        Ok(output)
    }
    /// Query copied evidence and current generation without granting native access.
    ///
    /// # Errors
    /// Returns UNAVAILABLE for a contract that has never been published.
    pub fn query(&self, name: &str) -> Result<(ContractV1, StatusV1), Error> {
        let mut contract = std::mem::MaybeUninit::uninit();
        let mut state = StatusV1::default();
        // SAFETY: Pinned table; name is readable, outputs are aligned and writable.
        status(unsafe {
            (self.0.api().query)(name.as_ptr(), name.len(), contract.as_mut_ptr(), &mut state)
        })?;
        // SAFETY: OK initializes the entire descriptor.
        Ok((unsafe { contract.assume_init() }, state))
    }
    /// Publish a verified native operation and own its current usable generation.
    ///
    /// Publication does not install native detours or verify evidence text.
    ///
    /// # Errors
    /// Rejects duplicate providers and malformed/incompatible contracts.
    pub fn publish(&self, contract: Contract) -> Result<Provider, Error> {
        let mut generation = 0;
        // SAFETY: Descriptor is owned/readable; generation output is writable.
        status(unsafe {
            (self.0.api().publish)(
                self.0.token(),
                &contract.0,
                size_of::<ContractV1>(),
                &mut generation,
            )
        })?;
        let end = contract
            .0
            .name
            .iter()
            .position(|b| *b == 0)
            .ok_or("invalid provider name")?;
        let name = String::from_utf8(contract.0.name[..end].to_vec())?;
        Ok(Provider {
            operations: *self,
            name,
            generation,
            limit: contract.0.max_payload as usize,
            active: true,
        })
    }
    /// Register a composable transform, including before its provider exists.
    ///
    /// Priority then namespaced identity determines order. Inspect `late()`;
    /// existing native resources are never retroactively transformed.
    ///
    /// # Errors
    /// Rejects duplicate identities, exclusive conflicts and callback reentry.
    pub fn transform(
        &self,
        name: &str,
        identity: &str,
        priority: i32,
        callback: impl FnMut(&[u8], &[u8]) -> Result<Option<Vec<u8>>, Error> + Send + 'static,
    ) -> Result<Registration, Error> {
        self.register(name, identity, priority, TRANSFORM, callback)
    }
    /// Register the sole replacement for an exclusive provider operation.
    ///
    /// # Errors
    /// Rejects an existing consumer or a nonexclusive provider.
    pub fn replace(
        &self,
        name: &str,
        identity: &str,
        callback: impl FnMut(&[u8], &[u8]) -> Result<Option<Vec<u8>>, Error> + Send + 'static,
    ) -> Result<Registration, Error> {
        self.register(name, identity, 0, EXCLUSIVE, callback)
    }
    fn register(
        &self,
        name: &str,
        identity: &str,
        priority: i32,
        mode: u32,
        callback: impl FnMut(&[u8], &[u8]) -> Result<Option<Vec<u8>>, Error> + Send + 'static,
    ) -> Result<Registration, Error> {
        let mut data = Box::new(Callback(Mutex::new(Box::new(callback))));
        let mut handle = 0;
        // SAFETY: Heap-stable callback state is retained until remove quiesces all
        // calls. The C thunk contains panics and copies output into host storage.
        let code = unsafe {
            (self.0.api().subscribe)(
                self.0.token(),
                name.as_ptr(),
                name.len(),
                identity.as_ptr(),
                identity.len(),
                priority,
                mode,
                invoke,
                (&mut *data as *mut Callback).cast(),
                &mut handle,
            )
        };
        if code != abi::OK && code != LATE {
            return Err(Error::host("platform", code));
        }
        Ok(Registration {
            operations: *self,
            handle,
            late: code == LATE,
            data: Some(data),
        })
    }
}

type Function = dyn FnMut(&[u8], &[u8]) -> Result<Option<Vec<u8>>, Error> + Send;
struct Callback(Mutex<Box<Function>>);
unsafe extern "C" fn invoke(
    data: *mut c_void,
    metadata: *const u8,
    metadata_len: usize,
    input: *const u8,
    input_len: usize,
    output: *mut u8,
    capacity: usize,
    written: *mut usize,
) -> i32 {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        // SAFETY: Negotiated broker supplies valid borrowed buffers and retains
        // Callback until remove finishes. Zero-length inputs use nonnull pointers.
        let (callback, metadata, input) = unsafe {
            (
                &*data.cast::<Callback>(),
                std::slice::from_raw_parts(metadata, metadata_len),
                std::slice::from_raw_parts(input, input_len),
            )
        };
        let Ok(mut callback) = callback.0.lock() else {
            return abi::ERROR;
        };
        match callback(metadata, input) {
            Ok(None) => PASS,
            Ok(Some(bytes)) if bytes.len() <= capacity => {
                // SAFETY: Broker provided writable bounded output and length.
                unsafe {
                    std::ptr::copy_nonoverlapping(bytes.as_ptr(), output, bytes.len());
                    written.write(bytes.len());
                }
                abi::OK
            }
            _ => abi::ERROR,
        }
    })) {
        Ok(code) => code,
        Err(payload) => {
            std::mem::forget(payload);
            FAULTED
        }
    }
}

/// Owned consumer registration; removal waits for all native calls using its userdata.
pub struct Registration {
    operations: Operations,
    handle: u64,
    late: bool,
    data: Option<Box<Callback>>,
}
impl Registration {
    /// Constrain order against optional peers before the provider's first use.
    ///
    /// # Errors
    /// Rejects cycles, foreign handles, malformed identities or changes after use.
    pub fn order(&self, before: &[&str], after: &[&str]) -> Result<(), Error> {
        let before = before.join("\n");
        let after = after.join("\n");
        // SAFETY: DLL-local strings remain readable throughout this pinned callback.
        status(unsafe {
            (self.operations.0.api().order)(
                self.operations.0.token(),
                self.handle,
                before.as_ptr(),
                before.len(),
                after.as_ptr(),
                after.len(),
            )
        })
    }

    /// Notify providers that this registration's captured policy has changed.
    ///
    /// # Errors
    /// Rejects removed/foreign handles and callback reentry.
    pub fn changed(&self) -> Result<(), Error> {
        status((self.operations.0.api().changed)(
            self.operations.0.token(),
            self.handle,
        ))
    }
    /// Report earlier broker-observed operations; false does not prove native bootstrap ordering.
    pub fn late(&self) -> bool {
        self.late
    }
    /// Quiesce this registration before freeing callback captures.
    ///
    /// # Errors
    /// Fails from an interceptor or when host quiescence cannot be established.
    pub fn remove(&mut self) -> Result<(), Error> {
        if self.data.is_some() {
            let code = (self.operations.0.api().remove)(self.operations.0.token(), self.handle);
            // Owner cleanup can remove a registration first. An ERROR does not
            // prove quiescence, so retain captures unless explicit removal succeeds.
            status(code)?;
            self.data = None;
        }
        Ok(())
    }
}
impl Drop for Registration {
    fn drop(&mut self) {
        if self.remove().is_err() {
            if let Some(data) = self.data.take() {
                Box::leak(data);
            }
        }
    }
}
impl std::fmt::Debug for Registration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Registration")
            .field("handle", &self.handle)
            .field("late", &self.late)
            .finish_non_exhaustive()
    }
}

/// Provider-owned generation; dropping it revokes access through this contract.
#[derive(Debug)]
pub struct Provider {
    operations: Operations,
    name: String,
    generation: u64,
    limit: usize,
    active: bool,
}
impl Provider {
    /// Read the current policy revision; unchanged revisions permit cached policy.
    ///
    /// # Errors
    /// Rejects revoked generations and callback reentry.
    pub fn stamp(&self) -> Result<u64, Error> {
        self.composition().map(|c| c.0)
    }
    /// Return the active callback count; adapters can bypass copying when zero.
    ///
    /// # Errors
    /// Rejects revoked generations and callback reentry.
    pub fn subscribers(&self) -> Result<u32, Error> {
        self.composition().map(|c| c.1)
    }
    /// Namespaced contract identity owned by this provider.
    pub fn name(&self) -> &str {
        &self.name
    }
    fn composition(&self) -> Result<(u64, u32), Error> {
        let mut revision = 0;
        let mut count = 0;
        // SAFETY: Borrowed name, pinned callback and writable aligned output.
        status(unsafe {
            (self.operations.0.api().stamp)(
                self.operations.0.token(),
                self.name.as_ptr(),
                self.name.len(),
                self.generation,
                &mut revision,
                &mut count,
            )
        })?;
        Ok((revision, count))
    }
    /// Return the generation to tag associated copied identities and resources.
    pub fn generation(&self) -> u64 {
        self.generation
    }
    /// Compose desired behavior before invoking the original native operation.
    ///
    /// # Errors
    /// Fails on revocation, consumer failure, oversized output or recursive dispatch.
    /// Providers must preserve the original native fallback on every failure.
    pub fn dispatch(&self, metadata: &[u8], input: &[u8]) -> Result<Vec<u8>, Error> {
        self.dispatch_bounded(metadata, input, self.limit)
    }
    /// Declared maximum copied payload size.
    pub fn limit(&self) -> usize {
        self.limit
    }
    /// Dispatch with a tighter output bound for fixed-size/chunked native records.
    ///
    /// # Errors
    /// Rejects output exceeding the chosen capacity or the declared contract limit.
    pub fn dispatch_bounded(
        &self,
        metadata: &[u8],
        input: &[u8],
        capacity: usize,
    ) -> Result<Vec<u8>, Error> {
        if capacity > self.limit {
            return Err("operation output bound exceeds contract".into());
        }
        let mut output = vec![0; capacity];
        let mut written = 0;
        // SAFETY: Pinned table and caller-owned bounded synchronous buffers.
        status(unsafe {
            (self.operations.0.api().dispatch)(
                self.operations.0.token(),
                self.name.as_ptr(),
                self.name.len(),
                self.generation,
                metadata.as_ptr(),
                metadata.len(),
                input.as_ptr(),
                input.len(),
                output.as_mut_ptr(),
                output.len(),
                &mut written,
            )
        })?;
        if written > output.len() {
            return Err("provider returned an invalid output size".into());
        }
        output.truncate(written);
        Ok(output)
    }
    /// Revoke and quiesce this generation before retiring its native resources.
    ///
    /// # Errors
    /// Fails for stale generations and callback reentry; native resources must remain alive.
    pub fn revoke(&mut self) -> Result<(), Error> {
        if self.active {
            // SAFETY: Name is borrowed only during this pinned callback.
            status(unsafe {
                (self.operations.0.api().revoke)(
                    self.operations.0.token(),
                    self.name.as_ptr(),
                    self.name.len(),
                    self.generation,
                )
            })?;
            self.active = false;
        }
        Ok(())
    }
}
impl Drop for Provider {
    fn drop(&mut self) {
        let _ = self.revoke();
    }
}

/// Decode entry/profile metadata supplied by the compiler source provider.
pub fn shader_metadata(metadata: &[u8]) -> Option<(&[u8], &[u8])> {
    let split = metadata.iter().position(|b| *b == 0)?;
    Some((&metadata[..split], &metadata[split + 1..]))
}
