//! C transport for public, extensible operation providers and consumers.
use rekenshi_plugin_api::{ERROR, INCOMPATIBLE, OK, platform::*};
use rekenshi_plugin_runtime::platform::Registry;
use std::{ffi::c_void, sync::OnceLock};

fn registry() -> &'static Registry {
    static REGISTRY: OnceLock<Registry> = OnceLock::new();
    REGISTRY.get_or_init(Registry::default)
}
pub(crate) fn diagnostics_enabled(enabled: bool) {
    registry().diagnostics_enabled(enabled);
}
pub(crate) fn diagnostics() -> Vec<String> {
    registry().diagnostics(7)
}
pub(crate) fn release(owner: u64) -> Result<(), String> {
    registry()
        .release(owner)
        .map_err(|code| format!("platform cleanup failed ({code})"))
}
fn boundary(action: impl FnOnce() -> Result<(), i32>) -> i32 {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(action)) {
        Ok(Ok(())) => OK,
        Ok(Err(code)) => code,
        Err(payload) => {
            std::mem::forget(payload);
            ERROR
        }
    }
}
fn owner(token: u64) -> Result<(), i32> {
    if crate::extensions::allowed(token) {
        Ok(())
    } else {
        Err(ERROR)
    }
}
unsafe fn bytes<'a>(pointer: *const u8, length: usize, limit: usize) -> Result<&'a [u8], i32> {
    if length > limit || (pointer.is_null() && length != 0) {
        return Err(ERROR);
    }
    if length == 0 {
        return Ok(&[]);
    }
    // SAFETY: FFI callers guarantee readable bytes for this synchronous call.
    Ok(unsafe { std::slice::from_raw_parts(pointer, length) })
}
unsafe fn name<'a>(pointer: *const u8, length: usize) -> Result<&'a str, i32> {
    // SAFETY: Forward caller's borrowed input contract with a bounded length.
    std::str::from_utf8(unsafe { bytes(pointer, length, 127)? }).map_err(|_| ERROR)
}

/// Negotiate public operation registration without altering frozen host/plugin records.
/// # Safety
/// Accepted requests provide aligned writable PlatformV1 storage.
// SAFETY: Unique versioned host export implementing GetApi.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rekenshi_platform_api(
    version: u32,
    output: *mut PlatformV1,
    size: usize,
) -> i32 {
    if version != VERSION || size != size_of::<PlatformV1>() {
        return INCOMPATIBLE;
    }
    if output.is_null() {
        return ERROR;
    }
    // SAFETY: Caller provides aligned exact-sized writable storage.
    unsafe {
        output.write(PlatformV1 {
            struct_size: size as u32,
            abi_version: VERSION,
            publish,
            query,
            subscribe,
            remove,
            dispatch,
            revoke,
            watch,
            next,
            unwatch,
            stamp,
            changed,
            authorized,
            watch_events,
            next_event,
            order,
            manifest,
            quarantine,
        });
    }
    OK
}
extern "C" fn quarantine(token: u64) -> i32 {
    boundary(|| {
        owner(token)?;
        registry().quarantine(token)
    })
}
unsafe extern "C" fn publish(
    token: u64,
    descriptor: *const ContractV1,
    size: usize,
    generation: *mut u64,
) -> i32 {
    boundary(|| {
        owner(token)?;
        if size != size_of::<ContractV1>() || descriptor.is_null() || generation.is_null() {
            return Err(ERROR);
        }
        // SAFETY: Caller provides a readable descriptor and writable generation.
        unsafe {
            generation.write(registry().publish(token, descriptor.read())?);
        }
        Ok(())
    })
}
unsafe extern "C" fn query(
    pointer: *const u8,
    length: usize,
    descriptor: *mut ContractV1,
    status: *mut StatusV1,
) -> i32 {
    boundary(|| {
        if descriptor.is_null() || status.is_null() {
            return Err(ERROR);
        }
        // SAFETY: Caller supplies bounded readable name and both writable outputs.
        unsafe {
            let (contract, snapshot) = registry().query(name(pointer, length)?)?;
            descriptor.write(contract);
            status.write(snapshot);
        }
        Ok(())
    })
}
unsafe extern "C" fn subscribe(
    token: u64,
    pointer: *const u8,
    length: usize,
    identity: *const u8,
    identity_length: usize,
    priority: i32,
    mode: u32,
    callback: Intercept,
    userdata: *mut c_void,
    output: *mut u64,
) -> i32 {
    let mut late = false;
    let status = boundary(|| {
        owner(token)?;
        if output.is_null() {
            return Err(ERROR);
        }
        crate::plugin::pin(callback as *const ()).map_err(|_| ERROR)?;
        let address = userdata as usize;
        // SAFETY: Caller supplies borrowed bounded UTF-8 names.
        let (contract, id) = unsafe { (name(pointer, length)?, name(identity, identity_length)?) };
        let (handle, missed) = registry().subscribe(
            token,
            contract,
            id,
            priority,
            mode,
            move |metadata, input| {
                // Output never crosses allocator ownership. The callback writes into
                // a host allocation; plugin output is copied before it returns.
                let mut output = vec![0; MAX_PAYLOAD];
                let mut written = 0;
                // SAFETY: Pinned callback, borrowed readable inputs, bounded writable
                // output. remove/release waits for this call before userdata is freed.
                let status = unsafe {
                    callback(
                        address as *mut c_void,
                        metadata.as_ptr(),
                        metadata.len(),
                        input.as_ptr(),
                        input.len(),
                        output.as_mut_ptr(),
                        output.len(),
                        &mut written,
                    )
                };
                match status {
                    PASS => Ok(None),
                    OK if written <= output.len() => {
                        output.truncate(written);
                        Ok(Some(output))
                    }
                    FAULTED => Err(FAULTED),
                    _ => Err(ERROR),
                }
            },
        )?;
        late = missed;
        // SAFETY: Caller supplied writable handle output.
        unsafe {
            output.write(handle);
        }
        Ok(())
    });
    if status == OK && late { LATE } else { status }
}
extern "C" fn remove(token: u64, handle: u64) -> i32 {
    // Cleanup is allowed after token revocation, but handles remain owner-checked.
    boundary(|| registry().remove(token, handle))
}
unsafe extern "C" fn dispatch(
    token: u64,
    pointer: *const u8,
    length: usize,
    generation: u64,
    metadata: *const u8,
    metadata_length: usize,
    input: *const u8,
    input_length: usize,
    output: *mut u8,
    capacity: usize,
    written: *mut usize,
) -> i32 {
    boundary(|| {
        owner(token)?;
        if output.is_null() || written.is_null() {
            return Err(ERROR);
        }
        // SAFETY: Caller supplies readable inputs and writable bounded output.
        unsafe {
            let contract = name(pointer, length)?;
            let result = registry().dispatch(
                token,
                contract,
                generation,
                bytes(metadata, metadata_length, 65536)?,
                bytes(input, input_length, MAX_PAYLOAD)?,
            )?;
            if result.len() > capacity {
                return Err(ERROR);
            }
            std::ptr::copy_nonoverlapping(result.as_ptr(), output, result.len());
            written.write(result.len());
        }
        Ok(())
    })
}
unsafe extern "C" fn revoke(token: u64, pointer: *const u8, length: usize, generation: u64) -> i32 {
    boundary(|| {
        // SAFETY: Caller supplies a borrowed bounded UTF-8 name.
        registry().revoke(token, unsafe { name(pointer, length)? }, generation)
    })
}
unsafe extern "C" fn watch(token: u64, pointer: *const u8, length: usize, output: *mut u64) -> i32 {
    boundary(|| {
        owner(token)?;
        if output.is_null() {
            return Err(ERROR);
        }
        // SAFETY: Borrowed bounded name and aligned writable handle output.
        unsafe {
            output.write(registry().watch(token, name(pointer, length)?)?);
        }
        Ok(())
    })
}
unsafe extern "C" fn next(token: u64, handle: u64, output: *mut StatusV1) -> i32 {
    boundary(|| {
        owner(token)?;
        if output.is_null() {
            return Err(ERROR);
        }
        let snapshot = registry().next_transition(token, handle)?.ok_or(PASS)?;
        // SAFETY: Caller supplied aligned writable snapshot storage.
        unsafe {
            output.write(snapshot);
        }
        Ok(())
    })
}
extern "C" fn unwatch(token: u64, handle: u64) -> i32 {
    boundary(|| registry().unwatch(token, handle))
}
unsafe extern "C" fn stamp(
    token: u64,
    pointer: *const u8,
    length: usize,
    generation: u64,
    output: *mut u64,
    subscribers: *mut u32,
) -> i32 {
    boundary(|| {
        owner(token)?;
        if output.is_null() || subscribers.is_null() {
            return Err(ERROR);
        }
        // SAFETY: Caller supplies a borrowed name and aligned writable output.
        unsafe {
            let (revision, count) = registry().stamp(token, name(pointer, length)?, generation)?;
            output.write(revision);
            subscribers.write(count);
        }
        Ok(())
    })
}
extern "C" fn changed(token: u64, handle: u64) -> i32 {
    boundary(|| {
        owner(token)?;
        registry().changed(token, handle)
    })
}
extern "C" fn authorized(token: u64) -> u32 {
    crate::extensions::allowed(token) as u32
}
unsafe extern "C" fn watch_events(
    token: u64,
    pointer: *const u8,
    length: usize,
    output: *mut u64,
) -> i32 {
    boundary(|| {
        owner(token)?;
        if output.is_null() {
            return Err(ERROR);
        }
        // SAFETY: Borrowed bounded name and aligned writable handle output.
        unsafe {
            output.write(registry().watch_events(token, name(pointer, length)?)?);
        }
        Ok(())
    })
}
unsafe extern "C" fn next_event(token: u64, handle: u64, output: *mut EventV1) -> i32 {
    boundary(|| {
        owner(token)?;
        if output.is_null() {
            return Err(ERROR);
        }
        let event = registry().next_event(token, handle)?.ok_or(PASS)?;
        // SAFETY: Caller supplies exact aligned writable event storage.
        unsafe {
            output.write(event);
        }
        Ok(())
    })
}

unsafe extern "C" fn order(
    token: u64,
    handle: u64,
    before: *const u8,
    before_len: usize,
    after: *const u8,
    after_len: usize,
) -> i32 {
    boundary(|| {
        owner(token)?;
        // SAFETY: Callers provide readable bounded UTF-8 buffers for this synchronous call.
        let before =
            std::str::from_utf8(unsafe { bytes(before, before_len, 4096)? }).map_err(|_| ERROR)?;
        // SAFETY: Same readable buffer contract for the second list.
        let after =
            std::str::from_utf8(unsafe { bytes(after, after_len, 4096)? }).map_err(|_| ERROR)?;
        registry().order(
            token,
            handle,
            before.lines().map(str::to_owned).collect(),
            after.lines().map(str::to_owned).collect(),
        )
    })
}
unsafe extern "C" fn manifest(pointer: *const u8, length: usize, output: *mut u8) -> i32 {
    boundary(|| {
        if output.is_null() {
            return Err(ERROR);
        }
        // SAFETY: Caller supplies readable name and writable 32-byte result storage.
        unsafe {
            let value = registry().manifest(name(pointer, length)?)?;
            std::ptr::copy_nonoverlapping(value.as_ptr(), output, 32);
        }
        Ok(())
    })
}
