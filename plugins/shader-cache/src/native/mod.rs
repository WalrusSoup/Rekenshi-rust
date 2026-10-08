//! Shader-cache native adapter, independent of game RVAs.
mod cache;
use detour::{Threads, Transaction, TypedDetour};
use rekenshi_plugin_api::{ERROR, HostV1, OK, hooks::HookId};
use rekenshi_plugin_sdk::operations::{
    Contract, Operations, Provider, SHADER_SCHEMA, SHADER_SOURCE,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::cell::Cell;
use std::ffi::{CStr, c_char, c_void};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock, RwLock};
use std::time::Instant;

const COMPILER_HASH: &str = "44C3A7E330B54A35A9EFA015831392593AA02E7DA1460BE429D17C3644850E8A";
const RENDERER_HASH: &str = "5F224882DED5B6D3F68F14D6BB4E1762267BBB602B7B9733BD153663D1541BB9";
type Compile = unsafe extern "system" fn(
    *const c_void,
    usize,
    *const c_char,
    *const Macro,
    *mut c_void,
    *const c_char,
    *const c_char,
    u32,
    u32,
    *mut *mut Blob,
    *mut *mut Blob,
) -> i32;
type CreateBlob = unsafe extern "system" fn(usize, *mut *mut Blob) -> i32;
#[repr(C)]
struct Macro {
    name: *const c_char,
    value: *const c_char,
}
#[repr(C)]
struct Blob {
    table: *const BlobTable,
}
#[repr(C)]
struct BlobTable {
    query: usize,
    add_ref: usize,
    release: unsafe extern "system" fn(*mut Blob) -> u32,
    data: unsafe extern "system" fn(*mut Blob) -> *mut c_void,
    size: unsafe extern "system" fn(*mut Blob) -> usize,
}
#[repr(C)]
struct IncludeTable {
    open: unsafe extern "system" fn(
        *mut c_void,
        u32,
        *const c_char,
        *const c_void,
        *mut *const c_void,
        *mut u32,
    ) -> i32,
    close: unsafe extern "system" fn(*mut c_void, *const c_void) -> i32,
}
#[repr(C)]
struct RecordingInclude {
    table: *const IncludeTable,
    original: *mut c_void,
    records: Vec<cache::Dependency>,
    valid: bool,
}
static INCLUDE_TABLE: IncludeTable = IncludeTable {
    open: record_open,
    close: record_close,
};
#[derive(Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(super) struct Settings {
    pub shader_cache: bool,
    pub max_cache_mb: u64,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            shader_cache: true,
            max_cache_mb: 512,
        }
    }
}
struct State {
    host: HostV1,
    hook: TypedDetour<Compile>,
    target: usize,
    create: CreateBlob,
    folder: PathBuf,
    budget: u64,
    used: AtomicU64,
    renderer: PathBuf,
    hits: AtomicU64,
    misses: AtomicU64,
    bypasses: AtomicU64,
    saved_us: AtomicU64,
    active: AtomicBool,
    enabled: AtomicBool,
    include_identity: OnceLock<Option<usize>>,
}
static STATE: OnceLock<State> = OnceLock::new();
static LIFECYCLE: Mutex<()> = Mutex::new(());
static CALLBACKS: RwLock<()> = RwLock::new(());
static SOURCE_PROVIDER: RwLock<Option<Provider>> = RwLock::new(None);
static BYTECODE: RwLock<Option<rekenshi_plugin_sdk::adapters::AdapterOperations>> =
    RwLock::new(None);
static POLICY: RwLock<Option<rekenshi_plugin_sdk::policies::PolicyProvider>> = RwLock::new(None);
thread_local! { static INSIDE: Cell<bool> = const { Cell::new(false) }; }

pub(super) fn initialize(
    host: HostV1,
    mod_root: &std::path::Path,
    settings: Settings,
) -> Result<(), String> {
    let _guard = LIFECYCLE.lock().map_err(|_| "lifecycle poisoned")?;
    if STATE.get().is_none() {
        let game = mod_root.parent().ok_or("game parent")?;
        if !(16..=4096).contains(&settings.max_cache_mb) {
            return Err("max_cache_mb must be 16..4096".into());
        }
        if !settings.shader_cache {
            host.write_log("shader cache disabled by settings");
            return Ok(());
        }
        let path = game.join("D3DCompiler_43.dll");
        if hash(&std::fs::read(&path).map_err(|e| e.to_string())?) != COMPILER_HASH {
            return Err("unsupported D3DCompiler_43 fingerprint; cache not installed".into());
        }
        rekenshi_native::load_pinned(&path)?;
        let catalog = host.hooks().map_err(|e| format!("hook catalog: {e}"))?;
        let target = catalog
            .target(HookId::D3DCompile)
            .map_err(|e| format!("D3DCompile unavailable: {e}"))?
            .as_ptr()
            .cast_const();

        // SAFETY: Verified compiler exports implement the documented Windows x64 signatures.
        let (compile, create) = unsafe {
            (
                std::mem::transmute::<*const c_void, Compile>(target),
                std::mem::transmute::<*const c_void, CreateBlob>(
                    catalog
                        .target(HookId::D3DCreateBlob)
                        .map_err(|e| format!("D3DCreateBlob unavailable: {e}"))?
                        .as_ptr()
                        .cast_const(),
                ),
            )
        };
        // SAFETY: Both functions have the same native signature and pinned lifetime.
        let hook = unsafe { TypedDetour::new(compile, intercepted as Compile) }
            .map_err(|e| e.to_string())?;
        let folder = mod_root.join("cache/shaders");
        std::fs::create_dir_all(&folder).map_err(|e| format!("create shader cache: {e}"))?;
        cache::trim(&folder, settings.max_cache_mb * 1024 * 1024);
        let used = AtomicU64::new(cache::usage(&folder));
        STATE
            .set(State {
                host,
                hook,
                target: target as usize,
                create,
                folder,
                budget: settings.max_cache_mb * 1024 * 1024,
                used,
                renderer: game.join("RenderSystem_Direct3D11_x64.dll"),
                hits: AtomicU64::new(0),
                misses: AtomicU64::new(0),
                bypasses: AtomicU64::new(0),
                saved_us: AtomicU64::new(0),
                active: AtomicBool::new(false),
                enabled: AtomicBool::new(true),
                include_identity: OnceLock::new(),
            })
            .map_err(|_| "state already initialized")?;
    }
    if let Some(state) = STATE.get() {
        let contract = Contract::new(
            SHADER_SOURCE, SHADER_SCHEMA,
            "D3DCompile caller; synchronous/concurrent; copied bytes only; no native access; no broker reentry",
            "D3DCompiler_43 SHA256 44C3A7E330B54A35A9EFA015831392593AA02E7DA1460BE429D17C3644850E8A; documented Windows x64 compiler ABI; transform before cache hash; compilation failure retries original; existing shaders require restart; native detour first-use ordering requires game validation",
            16 * 1024 * 1024,
        ).map_err(|e| e.to_string())?;
        let operations = Operations::from_host(host).map_err(|e| e.to_string())?;
        host.reserve_hook(std::ptr::NonNull::new(state.target as *mut c_void).ok_or("target")?)
            .map_err(|e| e.to_string())?;
        let mut tx = Transaction::new();
        tx.enable(&state.hook);
        // SAFETY: Verified prologue contains register saves, no internal calls;
        // all threads suspend during patching. Trampoline lives for process life.
        unsafe { tx.commit(Threads::All) }.map_err(|e| e.to_string())?;
        state.active.store(true, Ordering::Release);
        // Advertise usable availability only after native detour installation
        // succeeds. Earlier compiler calls cannot be intercepted retroactively;
        // bootstrap/activation first-use ordering still requires game evidence.
        *SOURCE_PROVIDER
            .write()
            .map_err(|_| "source provider poisoned")? =
            Some(operations.publish(contract).map_err(|e| e.to_string())?);
        *BYTECODE.write().map_err(|_| "bytecode provider lock")? = Some(rekenshi_plugin_sdk::adapters::AdapterOperations::publish(host,
            "Fingerprint-gated D3DCompiler_43; D3DDisassemble validates replacement DXBC; D3DCreateBlob allocates replacement, Release retires original only after success",
            &[(rekenshi_plugin_sdk::adapters::SHADER_BYTECODE,SHADER_SCHEMA,16*1024*1024)]).map_err(|e| e.to_string())?);
        *POLICY.write().map_err(|_| "shader policy lock")? = rekenshi_plugin_sdk::policies::PolicyProvider::optional(host, rekenshi_plugin_sdk::policies::SHADER_CACHE,
            "D3DCompiler_43 SHA256 44C3A7E330B54A35A9EFA015831392593AA02E7DA1460BE429D17C3644850E8A; native blob allocation/Release; dependency validation; cache policy applies before lookup").map_err(|e| e.to_string())?;
        host.write_log(
            "shader cache active; compiler fingerprint verified; unknown include handlers bypassed",
        );
    }
    Ok(())
}
pub(super) fn stop() -> i32 {
    std::panic::catch_unwind(|| {
        let Ok(_guard) = LIFECYCLE.lock() else {
            return ERROR;
        };
        if let Some(s) = STATE.get() {
            s.active.store(false, Ordering::Release);
            let mut tx = Transaction::new();
            tx.disable(&s.hook);
            // SAFETY: Same patching conditions as startup; retain all state for late callbacks.
            if unsafe { tx.commit(Threads::All) }.is_err() {
                return ERROR;
            }
            let Ok(_callbacks) = CALLBACKS.write() else {
                return ERROR;
            };
            let Ok(mut provider) = SOURCE_PROVIDER.write() else {
                return ERROR;
            };
            if let Some(provider) = provider.as_mut() {
                if provider.revoke().is_err() {
                    return ERROR;
                }
            }
            *provider = None;
            let Ok(mut bytecode) = BYTECODE.write() else {
                return ERROR;
            };
            if let Some(provider) = bytecode.as_mut() {
                if provider.revoke().is_err() {
                    return ERROR;
                }
            }
            *bytecode = None;
            let Ok(mut policy) = POLICY.write() else {
                return ERROR;
            };
            if let Some(policy) = policy.as_mut() {
                if policy.revoke().is_err() {
                    return ERROR;
                }
            }
            *policy = None;
            s.host.write_log(&format!(
                "shader cache stopped: hits={} misses={} bypasses={} avoided_compile_ms={:.2}",
                s.hits.load(Ordering::Relaxed),
                s.misses.load(Ordering::Relaxed),
                s.bypasses.load(Ordering::Relaxed),
                s.saved_us.load(Ordering::Relaxed) as f64 / 1000.0
            ));
            cache::trim(&s.folder, s.budget);
        }
        OK
    })
    .unwrap_or(ERROR)
}
fn hash(bytes: &[u8]) -> String {
    format!("{:X}", Sha256::digest(bytes))
}

unsafe extern "system" fn intercepted(
    src: *const c_void,
    len: usize,
    name: *const c_char,
    defines: *const Macro,
    include: *mut c_void,
    entry: *const c_char,
    target: *const c_char,
    flags1: u32,
    flags2: u32,
    code: *mut *mut Blob,
    errors: *mut *mut Blob,
) -> i32 {
    let Some(s) = STATE.get() else {
        return -2147467259;
    };
    // Native arguments are valid for the compiler; bounded cache logic either
    // uses that contract synchronously or forwards unchanged to the trampoline.
    // SAFETY: Intercepted function's signature equals the original D3DCompile.
    unsafe {
        if !s.active.load(Ordering::Acquire) || INSIDE.with(|v| v.replace(true)) {
            return s.hook.call(
                src, len, name, defines, include, entry, target, flags1, flags2, code, errors,
            );
        }
        struct Reset;
        impl Drop for Reset {
            fn drop(&mut self) {
                INSIDE.with(|v| v.set(false));
            }
        }
        let _reset = Reset;
        let Ok(_callbacks) = CALLBACKS.read() else {
            return -2147467259;
        };
        std::panic::catch_unwind(|| {
            // Transform first so cached bytecode always reflects the exact shader
            // actually compiled. Cache-off still permits the host lighting feature.
            let transformed = if !src.is_null()
                && !code.is_null()
                && len <= 16 * 1024 * 1024
                && !entry.is_null()
                && !target.is_null()
            {
                let source = std::slice::from_raw_parts(src.cast::<u8>(), len);
                let metadata = [
                    CStr::from_ptr(entry).to_bytes(),
                    &[0],
                    CStr::from_ptr(target).to_bytes(),
                ]
                .concat();
                SOURCE_PROVIDER.read().ok().and_then(|provider| {
                    provider.as_ref().and_then(|provider| {
                        match provider.dispatch(&metadata, source) {
                            Ok(output) if output != source => Some(output),
                            Ok(_) => None,
                            Err(error) => {
                                s.host.write_log(&format!(
                                    "shader source transform failed; original preserved: {error}"
                                ));
                                None
                            }
                        }
                    })
                })
            } else {
                None
            };
            if let Some(transformed) = transformed {
                let mut prepared_code = std::ptr::null_mut();
                let mut prepared_errors = std::ptr::null_mut();
                let result = compile_cached(
                    s,
                    transformed.as_ptr().cast(),
                    transformed.len(),
                    name,
                    defines,
                    include,
                    entry,
                    target,
                    flags1,
                    flags2,
                    &mut prepared_code,
                    if errors.is_null() {
                        std::ptr::null_mut()
                    } else {
                        &mut prepared_errors
                    },
                );
                if result >= 0 {
                    transform_bytecode(s, entry, target, &mut prepared_code);
                    code.write(prepared_code);
                    if !errors.is_null() {
                        errors.write(prepared_errors);
                    }
                    s.host.write_log("shader source transforms prepared");
                    return result;
                }
                // Release only compiler-owned outputs from the failed attempt,
                // then retry the untouched shader with the original arguments.
                for output in [prepared_code, prepared_errors] {
                    if !output.is_null() {
                        ((*(*output).table).release)(output);
                    }
                }
                s.host
                    .write_log("shader source compilation failed; using original source");
            }
            let result = compile_cached(
                s, src, len, name, defines, include, entry, target, flags1, flags2, code, errors,
            );
            if result >= 0 {
                transform_bytecode(s, entry, target, code);
            }
            result
        })
        .unwrap_or(-2147467259)
    }
}
// SAFETY contracts for internal native operations come from D3DCompile callers.
#[expect(
    clippy::too_many_arguments,
    reason = "fingerprinted native callback signature fixes argument order and count"
)]
unsafe fn compile_cached(
    s: &State,
    src: *const c_void,
    len: usize,
    name: *const c_char,
    defines: *const Macro,
    include: *mut c_void,
    entry: *const c_char,
    target: *const c_char,
    flags1: u32,
    flags2: u32,
    code: *mut *mut Blob,
    errors: *mut *mut Blob,
) -> i32 {
    // SAFETY: All arguments and COM outputs obey the original compiler contract.
    unsafe {
        let enabled = POLICY
            .read()
            .ok()
            .and_then(|p| p.as_ref().map(|p| p.values([0.; 4])[0] == 1.))
            .unwrap_or_else(|| s.enabled.load(Ordering::Acquire));
        let supported = enabled && (include.is_null() || known_include(s, include));
        let key = if supported && !code.is_null() && !src.is_null() && len <= 16 * 1024 * 1024 {
            key(
                src,
                len,
                name,
                defines,
                entry,
                target,
                flags1,
                flags2,
                !include.is_null(),
            )
        } else {
            None
        };
        let Some(key) = key else {
            s.bypasses.fetch_add(1, Ordering::Relaxed);
            return s.hook.call(
                src, len, name, defines, include, entry, target, flags1, flags2, code, errors,
            );
        };
        let path = s.folder.join(format!("{key}.shader"));
        if let Some(record) =
            cache::read(&path, &key).filter(|r| validate_dependencies(include, &r.dependencies))
        {
            if let Some(blob) = make_blob(s, &record.code) {
                let warning = if !errors.is_null() {
                    record.warnings.as_deref().map(|v| make_blob(s, v))
                } else {
                    None
                };
                if !matches!(warning, Some(None)) {
                    code.write(blob);
                    if !errors.is_null() {
                        errors.write(warning.flatten().unwrap_or(std::ptr::null_mut()));
                    }
                    let hits = s.hits.fetch_add(1, Ordering::Relaxed) + 1;
                    s.saved_us.fetch_add(record.compile_us, Ordering::Relaxed);
                    if hits == 1 || hits % 100 == 0 {
                        s.host.write_log(&format!("shader cache hit #{hits}"));
                    }
                    return record.result;
                }
                ((*(*blob).table).release)(blob);
            }
        }
        s.misses.fetch_add(1, Ordering::Relaxed);
        let mut recorder = RecordingInclude {
            table: &INCLUDE_TABLE,
            original: include,
            records: vec![],
            valid: true,
        };
        let proxy = if include.is_null() {
            include
        } else {
            (&mut recorder as *mut RecordingInclude).cast()
        };
        let before = Instant::now();
        let result = s.hook.call(
            src, len, name, defines, proxy, entry, target, flags1, flags2, code, errors,
        );
        let elapsed = before.elapsed().as_micros().min(u64::MAX as u128) as u64;
        if result >= 0 && !(*code).is_null() && recorder.valid {
            if let Some(bytecode) = blob_bytes(*code) {
                let warnings = if errors.is_null() || (*errors).is_null() {
                    None
                } else {
                    blob_bytes(*errors)
                };
                let record = cache::Record {
                    key,
                    result,
                    compile_us: elapsed,
                    dependencies: recorder.records,
                    code: bytecode,
                    warnings,
                };
                if !errors.is_null() && !(*errors).is_null() && record.warnings.is_none() {
                    return result;
                }
                if let Err(e) = cache::write(&path, &record, &s.used, s.budget) {
                    s.host
                        .write_log(&format!("shader cache write skipped: {e}"));
                }
            }
        }
        result
    }
}
// These inputs intentionally match the compiler API's cache-relevant arguments.
#[expect(
    clippy::too_many_arguments,
    reason = "fingerprinted native callback signature fixes argument order and count"
)]
unsafe fn key(
    src: *const c_void,
    len: usize,
    name: *const c_char,
    defines: *const Macro,
    entry: *const c_char,
    target: *const c_char,
    flags1: u32,
    flags2: u32,
    includes: bool,
) -> Option<String> {
    let mut h = Sha256::new();
    h.update(b"REKR_SHADER_CACHE_V2");
    if let Some(state) = STATE.get() {
        if let Ok(operations) = Operations::from_host(state.host) {
            if let Ok(manifest) = operations.manifest(SHADER_SOURCE) {
                h.update(manifest);
            }
        }
    }
    h.update(COMPILER_HASH);
    h.update(RENDERER_HASH);
    fn part(h: &mut Sha256, b: Option<&[u8]>) {
        h.update([u8::from(b.is_some())]);
        if let Some(b) = b {
            h.update((b.len() as u64).to_le_bytes());
            h.update(b);
        }
    }
    // SAFETY: Original compile's source and terminated input strings remain readable for this call.
    unsafe {
        part(
            &mut h,
            Some(std::slice::from_raw_parts(src.cast::<u8>(), len)),
        );
        for text in [name, entry, target] {
            part(
                &mut h,
                if text.is_null() {
                    None
                } else {
                    Some(CStr::from_ptr(text).to_bytes())
                },
            );
        }
        h.update(flags1.to_le_bytes());
        h.update(flags2.to_le_bytes());
        h.update([u8::from(includes)]);
        h.update([u8::from(!defines.is_null())]);
        if !defines.is_null() {
            for i in 0..1024 {
                let m = &*defines.add(i);
                if m.name.is_null() {
                    return Some(format!("{:x}", h.finalize()));
                }
                part(&mut h, Some(CStr::from_ptr(m.name).to_bytes()));
                part(
                    &mut h,
                    if m.value.is_null() {
                        None
                    } else {
                        Some(CStr::from_ptr(m.value).to_bytes())
                    },
                );
            }
            return None;
        }
    }
    Some(format!("{:x}", h.finalize()))
}
unsafe fn known_include(s: &State, include: *mut c_void) -> bool {
    if include as usize <= 1 {
        return false;
    } // D3D_COMPILE_STANDARD_FILE_INCLUDE sentinel is never dereferenced.
    if s.host
        .find_export(c"RenderSystem_Direct3D11_x64.dll", c"dllStartPlugin")
        .is_none()
    {
        return false;
    }
    let identity = s.include_identity.get_or_init(|| {
        let symbol = s
            .host
            .find_export(c"RenderSystem_Direct3D11_x64.dll", c"dllStartPlugin")?;
        let dir = rekenshi_native::directory(symbol.as_ptr().cast()).ok()?;
        if hash(&std::fs::read(dir.join("RenderSystem_Direct3D11_x64.dll")).ok()?) != RENDERER_HASH
        {
            return None;
        }
        // Export RVA is intentionally not used; obtain module base from verified
        // PE mapping through the native loader helper.
        rekenshi_native::load_pinned(&s.renderer).ok()
    });
    let Some(base) = identity else { return false };
    // SAFETY: include is a native ID3DInclude supplied to D3DCompile. Only this
    // exact verified Ogre vtable is eligible; custom handlers are untouched.
    unsafe {
        let table = *(include.cast::<usize>());
        if table != base + 0x78808 {
            return false;
        }
        let slots = std::slice::from_raw_parts(table as *const usize, 2);
        slots == [base + 0x9e80, base + 0x9ff0]
            && hash(std::slice::from_raw_parts(
                (base + 0x9e80) as *const u8,
                355,
            )) == "2D8E79019EA292069853A52037948FAE83DCD73F118DC703FBD64D00AE3B0039"
            && hash(std::slice::from_raw_parts((base + 0x9ff0) as *const u8, 20))
                == "9725407485E00A61D2F86433B7CB12DDF90C4F009325EE6D38EE6980174BA203"
    }
}
unsafe extern "system" fn record_open(
    this: *mut c_void,
    kind: u32,
    name: *const c_char,
    parent: *const c_void,
    data: *mut *const c_void,
    len: *mut u32,
) -> i32 {
    // SAFETY: Compiler calls our stack-scoped proxy using ID3DInclude's ABI.
    unsafe {
        let r = &mut *this.cast::<RecordingInclude>();
        let table = *r.original.cast::<*const IncludeTable>();
        let result = ((*table).open)(r.original, kind, name, parent, data, len);
        if result >= 0 {
            if name.is_null()
                || data.is_null()
                || len.is_null()
                || (*data).is_null()
                || *len > 16 * 1024 * 1024
                || r.records.len() >= 512
            {
                r.valid = false;
            } else {
                r.records.push(cache::Dependency {
                    kind,
                    name: CStr::from_ptr(name).to_bytes().to_vec(),
                    digest: hash(std::slice::from_raw_parts(
                        (*data).cast::<u8>(),
                        *len as usize,
                    )),
                });
            }
        } else {
            r.valid = false;
        }
        result
    }
}
unsafe extern "system" fn record_close(this: *mut c_void, data: *const c_void) -> i32 {
    // SAFETY: Forward exactly the native allocation pointer; original handler owns it.
    unsafe {
        let r = &mut *this.cast::<RecordingInclude>();
        let table = *r.original.cast::<*const IncludeTable>();
        let result = ((*table).close)(r.original, data);
        if result < 0 {
            r.valid = false;
        }
        result
    }
}
unsafe fn validate_dependencies(include: *mut c_void, records: &[cache::Dependency]) -> bool {
    if include.is_null() {
        return records.is_empty();
    }
    // SAFETY: Eligibility already verifies Ogre's exact read-only handler. Its
    // Open ignores ParentData/type; each native allocation gets one native Close.
    unsafe {
        let table = *include.cast::<*const IncludeTable>();
        for r in records {
            let Ok(name) = std::ffi::CString::new(r.name.as_slice()) else {
                return false;
            };
            let mut data = std::ptr::null();
            let mut len = 0;
            if ((*table).open)(
                include,
                r.kind,
                name.as_ptr(),
                std::ptr::null(),
                &mut data,
                &mut len,
            ) < 0
            {
                return false;
            }
            let valid = !data.is_null()
                && len <= 16 * 1024 * 1024
                && hash(std::slice::from_raw_parts(data.cast::<u8>(), len as usize)) == r.digest;
            let closed = ((*table).close)(include, data) >= 0;
            if !valid || !closed {
                return false;
            }
        }
        true
    }
}
unsafe fn blob_bytes(blob: *mut Blob) -> Option<Vec<u8>> {
    // SAFETY: A successful compiler/CreateBlob output implements ID3DBlob.
    unsafe {
        let table = &*(*blob).table;
        let len = (table.size)(blob);
        let data = (table.data)(blob);
        if len > cache::MAX_RECORD || data.is_null() {
            None
        } else {
            Some(std::slice::from_raw_parts(data.cast::<u8>(), len).to_vec())
        }
    }
}
unsafe fn make_blob(s: &State, bytes: &[u8]) -> Option<*mut Blob> {
    let mut blob = std::ptr::null_mut();
    // SAFETY: Native allocation stays owned by native COM; caller releases it.
    unsafe {
        if (s.create)(bytes.len(), &mut blob) < 0 || blob.is_null() {
            return None;
        }
        let data = ((*(*blob).table).data)(blob);
        if data.is_null() {
            ((*(*blob).table).release)(blob);
            return None;
        }
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), data.cast::<u8>(), bytes.len());
    }
    Some(blob)
}

// SAFETY: Scalar diagnostic export for smoke tests; no pointers cross this call.
pub fn hits() -> u64 {
    STATE.get().map_or(0, |s| s.hits.load(Ordering::Relaxed))
}

unsafe fn transform_bytecode(
    state: &State,
    entry: *const c_char,
    target: *const c_char,
    output: *mut *mut Blob,
) {
    if output.is_null() || entry.is_null() || target.is_null() {
        return;
    }
    if !BYTECODE.read().ok().is_some_and(|operations| {
        operations.as_ref().is_some_and(|operations| {
            operations.active(rekenshi_plugin_sdk::adapters::SHADER_BYTECODE)
        })
    }) {
        return;
    }
    // SAFETY: Successful D3DCompile initialized the output blob; native strings remain live.
    unsafe {
        let Some(bytes) = blob_bytes(*output) else {
            return;
        };
        let metadata = [
            CStr::from_ptr(entry).to_bytes(),
            &[0],
            CStr::from_ptr(target).to_bytes(),
        ]
        .concat();
        let Some(replacement) = BYTECODE.read().ok().and_then(|operations| {
            operations.as_ref().and_then(|operations| {
                operations.transform(
                    rekenshi_plugin_sdk::adapters::SHADER_BYTECODE,
                    &metadata,
                    &bytes,
                )
            })
        }) else {
            return;
        };
        type Disassemble = unsafe extern "system" fn(
            *const c_void,
            usize,
            u32,
            *const c_char,
            *mut *mut Blob,
        ) -> i32;
        let Some(address) = state
            .host
            .find_export(c"D3DCompiler_43.dll", c"D3DDisassemble")
        else {
            return;
        };
        let disassemble: Disassemble = std::mem::transmute(address.as_ptr());
        let mut validation = std::ptr::null_mut();
        let valid = disassemble(
            replacement.as_ptr().cast(),
            replacement.len(),
            0,
            std::ptr::null(),
            &mut validation,
        ) >= 0;
        if !validation.is_null() {
            ((*(*validation).table).release)(validation);
        }
        if !valid {
            return;
        }
        let Some(blob) = make_blob(state, &replacement) else {
            return;
        };
        let original = *output;
        *output = blob;
        if !original.is_null() {
            ((*(*original).table).release)(original);
        }
    }
}
