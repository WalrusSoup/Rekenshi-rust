//! Opt-in DLL mods. Discovery, ABI validation and lifecycle stay behind start/stop.
use crate::{logging, plugin};
use rekenshi_plugin_api::services::{GetUi, PluginUiV1, SettingV1};
use rekenshi_plugin_api::{
    ABI_VERSION, ENTRY_EXPORT, ERROR, Entry, HOOK_BUSY, HostV1, OK, PluginV1,
};
use rekenshi_plugin_runtime::{Control, Row, Setting, Value};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::ffi::{CStr, c_char, c_void};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Mutex, OnceLock};
use windows_sys::Win32::Foundation::{FreeLibrary, HMODULE};
use windows_sys::Win32::System::LibraryLoader::{
    GET_MODULE_HANDLE_EX_FLAG_PIN, GetModuleHandleExW, GetProcAddress,
    LOAD_LIBRARY_SEARCH_DEFAULT_DIRS, LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR, LoadLibraryExW,
};

pub(crate) fn rows() -> Vec<Row> {
    let Some(manager) = LOADED_PLUGINS.get() else {
        return vec![];
    };
    let Ok(manager) = manager.lock() else {
        return vec![];
    };
    manager
        .plugins
        .iter()
        .flat_map(|plugin| {
            plugin
                .rows
                .iter()
                .enumerate()
                .map(|(index, definition)| {
                    let mut row = definition.clone();
                    if plugin.state != State::Active {
                        row.error = Some("Plugin unavailable; see RE_Kenshi_Rust.log".into());
                        return row;
                    }
                    let mut wire = SettingV1::default();
                    if let Some(api) = plugin.ui {
                        // SAFETY: Pinned validated SDK callback; output is aligned and sized.
                        let result =
                            unsafe { (api.row)(index as u32, &mut wire, size_of::<SettingV1>()) };
                        match wire_value(wire.kind, wire.value) {
                            Ok(value) if result == OK && row.setting.accepts(value) => {
                                row.value = value;
                                row.error =
                                    wire_text(&wire.error, true).ok().filter(|s| !s.is_empty());
                            }
                            _ => row.error = Some("Plugin returned invalid settings".into()),
                        }
                    }
                    row
                })
                .collect::<Vec<_>>()
        })
        .collect()
}
pub(crate) fn set(key: &str, value: Value) -> Result<(), String> {
    rekenshi_plugin_runtime::HookEvents::check_shutdown()?;
    let manager = LOADED_PLUGINS
        .get()
        .ok_or("DLL plugins unavailable")?
        .lock()
        .map_err(|_| "plugin manager poisoned")?;
    let plugin = manager
        .plugins
        .iter()
        .find(|p| p.rows.iter().any(|r| r.setting.key == key))
        .ok_or("unknown setting")?;
    if plugin.state != State::Active {
        return Err("plugin unavailable".into());
    }
    let row = plugin
        .rows
        .iter()
        .find(|r| r.setting.key == key)
        .ok_or("unknown setting")?;
    if !row.setting.accepts(value) {
        return Err("invalid setting value".into());
    }
    let api = plugin.ui.ok_or("plugin has no settings API")?;
    let (kind, number) = match value {
        Value::Toggle(v) => (1, f64::from(v)),
        Value::Number(v) => (2, v),
    };
    // SAFETY: Pinned callback; readable UTF-8 key remains live for this call.
    if unsafe { (api.set)(key.as_ptr(), key.len(), kind, number) } == OK {
        Ok(())
    } else {
        Err("plugin update rejected; see host log".into())
    }
}

static LOADED_PLUGINS: OnceLock<Mutex<LoadedPlugins>> = OnceLock::new();
static REGISTRY: OnceLock<Mutex<Registry>> = OnceLock::new();
static ACTIVE: AtomicU32 = AtomicU32::new(0);
static FAILED: AtomicU32 = AtomicU32::new(0);
pub(crate) fn allowed(token: u64) -> bool {
    registry().lock().is_ok_and(|r| r.allowed.contains(&token))
}
pub(crate) fn host_table(token: u64) -> HostV1 {
    // SAFETY: Core callbacks validate tokens, use synchronous buffers, and pin
    // resolved modules before exposing exports with their negotiated ABI.
    unsafe {
        HostV1::new(
            token,
            host_log,
            host_resolve,
            host_claim,
            host_release,
            crate::rekenshi_hook_calls,
        )
    }
}

#[derive(Default)]
struct Registry {
    names: HashMap<u64, String>,
    hooks: HashMap<usize, u64>,
    allowed: HashSet<u64>,
    active: HashSet<String>,
}
fn registry() -> &'static Mutex<Registry> {
    REGISTRY.get_or_init(|| Mutex::new(Registry::default()))
}
pub(crate) fn available(id: &str) -> bool {
    registry().lock().is_ok_and(|r| r.active.contains(id))
}

pub(crate) fn active_count() -> u32 {
    ACTIVE.load(Ordering::Acquire)
}
pub(crate) fn failed_count() -> u32 {
    FAILED.load(Ordering::Acquire)
}

pub(crate) fn internal_host(id: &str, token: u64) -> Result<rekenshi_plugin_runtime::Host, String> {
    let mut registry = registry().lock().map_err(|_| "hook registry poisoned")?;
    registry.names.insert(token, format!("internal/{id}"));
    registry.allowed.insert(token);
    Ok(rekenshi_plugin_runtime::Host::new(
        host_table(token),
        crate::callbacks::events(),
    ))
}

pub(crate) fn release_internal() -> Result<(), String> {
    // Called only after every internal plugin successfully stopped and quiesced.
    let mut registry = registry().lock().map_err(|_| "hook registry poisoned")?;
    let tokens: Vec<_> = registry
        .allowed
        .iter()
        .copied()
        .filter(|token| *token >= 1001)
        .collect();
    registry.allowed.retain(|token| *token < 1001);
    registry.hooks.retain(|_, token| *token < 1001);
    drop(registry);
    for token in tokens {
        crate::gameplay::release(token);
    }
    Ok(())
}
pub(crate) fn hook_owner(target: *const c_void) -> Result<Option<u64>, i32> {
    Ok(registry()
        .lock()
        .map_err(|_| ERROR)?
        .hooks
        .get(&(target as usize))
        .copied())
}
pub(crate) fn owns_active_hook(token: u64, target: *const c_void) -> bool {
    registry().lock().is_ok_and(|r| {
        token != 0 && r.allowed.contains(&token) && r.hooks.get(&(target as usize)) == Some(&token)
    })
}

pub(crate) fn reserve_core(target: *const c_void) -> Result<(), String> {
    let mut registry = registry().lock().map_err(|_| "hook registry poisoned")?;
    match registry.hooks.get(&(target as usize)) {
        Some(0) | None => {
            registry.hooks.insert(target as usize, 0);
            Ok(())
        }
        Some(_) => Err("core hook target already reserved by a mod".into()),
    }
}

pub(crate) fn start() {
    let manager = LOADED_PLUGINS.get_or_init(|| Mutex::new(LoadedPlugins::discover()));
    let Ok(mut manager) = manager.lock() else {
        FAILED.store(1, Ordering::Release);
        logging::write("plugin manager lock poisoned");
        return;
    };
    for loaded in &mut manager.plugins {
        if loaded.state != State::Ready {
            continue;
        }
        if let Ok(mut registry) = registry().lock() {
            registry.allowed.insert(loaded.host.plugin_token);
        } else {
            loaded.state = State::Rejected;
            continue;
        }
        // SAFETY: Descriptor and ABI were validated; module is pinned and host
        // table has stable Box storage. V1 callbacks must contain their own panics.
        let result =
            unsafe { loaded.descriptor.start.expect("validated start")(loaded.host.as_ref()) };
        if result == OK {
            loaded.state = State::Active;
            if let Ok(mut registry) = registry().lock() {
                registry.active.insert(loaded.id.clone());
            }
            logging::write(&format!(
                "plugin {} started ({})",
                loaded.id,
                loaded.file.display()
            ));
        } else {
            logging::write(&format!(
                "plugin {} start rejected with code {result}; cleaning up",
                loaded.id
            ));
            loaded.stop(State::Rejected);
        }
    }
    manager.publish_counts();
    logging::write(&format!(
        "plugins: {} active, {} failed",
        active_count(),
        failed_count()
    ));
}

pub(crate) fn stop() {
    let Some(manager) = LOADED_PLUGINS.get() else {
        return;
    };
    let Ok(mut manager) = manager.lock() else {
        logging::write("plugin manager shutdown lock poisoned");
        return;
    };
    for loaded in manager.plugins.iter_mut().rev() {
        if loaded.state == State::Active {
            loaded.stop(State::Ready);
        }
    }
    manager.publish_counts();
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    schema_version: u32,
    #[serde(default)]
    discover: bool,
    #[serde(default)]
    plugins: Vec<ConfiguredPlugin>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ConfiguredPlugin {
    file: String,
    enabled: bool,
}

impl Config {
    fn parse(text: &str) -> Result<Self, String> {
        let config: Self =
            toml::from_str(text).map_err(|error| format!("plugins.toml: {error}"))?;
        if config.schema_version != 1 {
            return Err("unsupported plugins.toml schema_version (expected 1)".into());
        }
        if config.plugins.len() > 64 {
            return Err("at most 64 plugins may be configured".into());
        }
        let mut names = HashSet::new();
        for entry in &config.plugins {
            if !valid_filename(&entry.file) {
                return Err(format!("invalid plugin DLL filename: {}", entry.file));
            }
            if !names.insert(entry.file.to_ascii_lowercase()) {
                return Err(format!("duplicate plugin DLL: {}", entry.file));
            }
        }
        Ok(config)
    }
}
fn valid_filename(name: &str) -> bool {
    name.len() > 4
        && name.len() <= 128
        && name.to_ascii_lowercase().ends_with(".dll")
        && !name.starts_with('.')
        && !name.contains("..")
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
}

#[derive(Default)]
struct LoadedPlugins {
    plugins: Vec<LoadedPlugin>,
    discovery_failures: u32,
}
impl LoadedPlugins {
    fn discover() -> Self {
        let mut manager = Self::default();
        if let Err(error) = manager.discover_inner() {
            manager.discovery_failures += 1;
            logging::write(&format!("plugins disabled: {error}"));
        }
        manager
    }
    fn discover_inner(&mut self) -> Result<(), String> {
        let root = logging::directory()
            .ok_or("cannot locate host DLL directory")?
            .join("RE_Kenshi_Rust");
        let config_path = root.join("plugins.toml");
        let text = match std::fs::metadata(&config_path) {
            Ok(metadata) if metadata.len() <= 65536 => {
                std::fs::read_to_string(&config_path).map_err(|e| e.to_string())?
            }
            Ok(_) => return Err("plugins.toml exceeds 64 KiB".into()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                "schema_version=1\ndiscover=true\nplugins=[]".into()
            }
            Err(error) => return Err(error.to_string()),
        };
        let mut config = Config::parse(&text)?;
        if config.discover {
            let folder = root.join("plugins");
            if folder.is_dir() {
                let mut discovered = std::fs::read_dir(&folder)
                    .map_err(|e| e.to_string())?
                    .filter_map(Result::ok)
                    .filter_map(|e| e.file_name().into_string().ok())
                    .filter(|n| valid_filename(n))
                    .collect::<Vec<_>>();
                discovered.sort_by_key(|n| n.to_ascii_lowercase());
                for file in discovered {
                    if !config
                        .plugins
                        .iter()
                        .any(|p| p.file.eq_ignore_ascii_case(&file))
                    {
                        config.plugins.push(ConfiguredPlugin {
                            file,
                            enabled: true,
                        });
                    }
                }
                if config.plugins.len() > 64 {
                    return Err("at most 64 DLL mods may be discovered".into());
                }
            }
        }
        // A missing plugins directory is harmless when every entry is disabled.
        if !config.plugins.iter().any(|entry| entry.enabled) {
            return Ok(());
        }
        let directory = root
            .join("plugins")
            .canonicalize()
            .map_err(|error| format!("plugin directory: {error}"))?;
        let mut ids: HashSet<_> = crate::internal_plugins::ids()
            .into_iter()
            .map(str::to_owned)
            .collect();
        let mut keys: HashSet<_> = crate::internal_plugins::rows()
            .into_iter()
            .map(|r| r.setting.key.to_owned())
            .collect();
        for (index, entry) in config.plugins.iter().enumerate() {
            if !entry.enabled {
                continue;
            }
            match load_plugin(&directory, &entry.file, index as u64 + 1, &mut ids) {
                Ok(plugin) => {
                    if plugin.rows.iter().any(|r| keys.contains(r.setting.key)) {
                        self.discovery_failures += 1;
                        logging::write(&format!(
                            "plugin {} skipped: duplicate setting key",
                            plugin.id
                        ));
                        continue;
                    }
                    keys.extend(plugin.rows.iter().map(|r| r.setting.key.to_owned()));
                    self.plugins.push(plugin);
                }
                Err(error) => {
                    self.discovery_failures += 1;
                    logging::write(&format!("plugin {} skipped: {error}", entry.file));
                }
            }
        }
        Ok(())
    }
    fn publish_counts(&self) {
        ACTIVE.store(
            self.plugins
                .iter()
                .filter(|plugin| plugin.state == State::Active)
                .count() as u32,
            Ordering::Release,
        );
        FAILED.store(
            self.discovery_failures
                + self
                    .plugins
                    .iter()
                    .filter(|plugin| matches!(plugin.state, State::Rejected | State::Quarantined))
                    .count() as u32,
            Ordering::Release,
        );
    }
}

#[derive(Clone, Copy, PartialEq)]
enum State {
    Ready,
    Active,
    Rejected,
    Quarantined,
}
struct LoadedPlugin {
    file: PathBuf,
    id: String,
    descriptor: PluginV1,
    host: Box<HostV1>,
    state: State,
    ui: Option<PluginUiV1>,
    rows: Vec<Row>,
}
impl LoadedPlugin {
    fn stop(&mut self, next: State) {
        // Revoke gameplay even if managed callback cleanup quarantines this DLL.
        // Cleanup does not need authorization; workers cannot enqueue after this.
        if let Ok(mut registry) = registry().lock() {
            registry.allowed.remove(&self.host.plugin_token);
            registry.active.remove(&self.id);
        }
        crate::gameplay::release(self.host.plugin_token);
        rekenshi_weather::release(self.host.plugin_token);
        if let Err(error) = crate::services::release(self.host.plugin_token) {
            self.state = State::Quarantined;
            logging::write(&error);
            return;
        }
        // SAFETY: Validated stop callback, pinned code. Called once per start
        // attempt, before core hooks stop. Plugin must quiesce its workers/hooks.
        let result = unsafe { self.descriptor.stop.expect("validated stop")() };
        if result == OK {
            self.state = next;
            if let Ok(mut registry) = registry().lock() {
                registry
                    .hooks
                    .retain(|_, owner| *owner != self.host.plugin_token);
            }
            logging::write(&format!("plugin {} stopped", self.id));
        } else {
            self.state = State::Quarantined;
            logging::write(&format!(
                "plugin {} stop failed ({result}); code and hook reservations retained; restart required",
                self.id
            ));
        }
    }
}

fn load_plugin(
    directory: &Path,
    filename: &str,
    token: u64,
    ids: &mut HashSet<String>,
) -> Result<LoadedPlugin, String> {
    let file = directory
        .join(filename)
        .canonicalize()
        .map_err(|error| format!("locate DLL: {error}"))?;
    if file.parent() != Some(directory) {
        return Err("DLL resolves outside the configured plugins directory".into());
    }
    use std::os::windows::ffi::OsStrExt;
    let wide: Vec<u16> = file.as_os_str().encode_wide().chain(Some(0)).collect();
    // SAFETY: Absolute terminated path; search uses DLL directory plus standard
    // Windows locations. Called from Ogre startup, never from DllMain.
    let module = unsafe {
        LoadLibraryExW(
            wide.as_ptr(),
            std::ptr::null_mut(),
            LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_DEFAULT_DIRS,
        )
    };
    if module.is_null() {
        return Err(format!("load DLL: {}", std::io::Error::last_os_error()));
    }
    let module = Module(module);
    // Retain even rejected DLLs: DllMain has already executed during loading.
    // Never unmap code that might still have an in-flight callback or worker.
    plugin::pin(module.0.cast())?;
    // SAFETY: module remains loaded throughout lookup; name is terminated.
    let symbol = unsafe { GetProcAddress(module.0, ENTRY_EXPORT.as_ptr()) }
        .ok_or("missing rekenshi_plugin_entry export")?;
    // Pin before invoking plugin code: its entry may retain pointers or initialize
    // state even if later validation fails. We never unload invoked plugin code.
    plugin::pin(symbol as *const ())?;
    // SAFETY: Native mods implementing this named export must use the V1 entry
    // signature. ABI negotiation precedes all lifecycle callbacks.
    let entry =
        unsafe { std::mem::transmute::<unsafe extern "system" fn() -> isize, Entry>(symbol) };
    let mut descriptor = PluginV1::EMPTY;
    // SAFETY: Live, aligned, correctly sized caller-owned descriptor storage.
    let result = unsafe { entry(ABI_VERSION, &mut descriptor, size_of::<PluginV1>()) };
    if result != OK {
        return Err(format!(
            "entry rejected ABI v{ABI_VERSION} with code {result}"
        ));
    }
    if descriptor.abi_version != ABI_VERSION
        || descriptor.struct_size as usize != size_of::<PluginV1>()
    {
        return Err("descriptor ABI or size mismatch".into());
    }
    if descriptor.start.is_none() || descriptor.stop.is_none() {
        return Err("descriptor missing start/stop callback".into());
    }
    let id = metadata_text(&descriptor.id)?;
    if !id
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    {
        return Err("invalid plugin id".into());
    }
    let name = metadata_text(&descriptor.name)?;
    let version = metadata_text(&descriptor.version)?;
    if ids.contains(&id) {
        return Err(format!("duplicate plugin id: {id}"));
    }
    // Callback pointers can live in another DLL; pin those modules too. Addresses
    // still must have the declared signatures, as required by the mod contract.
    plugin::pin(descriptor.start.expect("validated start") as *const ())?;
    plugin::pin(descriptor.stop.expect("validated stop") as *const ())?;
    registry()
        .lock()
        .map_err(|_| "hook registry poisoned")?
        .names
        .insert(token, id.clone());
    ids.insert(id.clone());
    logging::write(&format!(
        "plugin discovered: {id} ({name} {version}), ABI v{ABI_VERSION}"
    ));
    let (ui, rows) = read_ui(module.0)?;
    Ok(LoadedPlugin {
        file,
        id,
        descriptor,
        state: State::Ready,
        ui,
        rows,
        host: Box::new(host_table(token)),
    })
}
fn wire_text(bytes: &[u8], empty: bool) -> Result<String, String> {
    let length = bytes
        .iter()
        .position(|b| *b == 0)
        .ok_or("unterminated settings metadata")?;
    if length == 0 && !empty {
        return Err("empty settings metadata".into());
    }
    std::str::from_utf8(&bytes[..length])
        .map(str::to_owned)
        .map_err(|e| e.to_string())
}
fn wire_value(kind: u32, value: f64) -> Result<Value, String> {
    match (kind, value) {
        (1, 0.) => Ok(Value::Toggle(false)),
        (1, 1.) => Ok(Value::Toggle(true)),
        (2, v) if v.is_finite() => Ok(Value::Number(v)),
        _ => Err("invalid setting type/value".into()),
    }
}
fn read_ui(module: HMODULE) -> Result<(Option<PluginUiV1>, Vec<Row>), String> {
    // SAFETY: Loaded module is pinned; optional export name is terminated.
    let Some(symbol) = (unsafe { GetProcAddress(module, c"rekenshi_plugin_ui".as_ptr().cast()) })
    else {
        return Ok((None, vec![]));
    };
    // SAFETY: This optional SDK export implements GetUi's negotiated signature.
    let get: GetUi = unsafe { std::mem::transmute(symbol) };
    let mut output = std::mem::MaybeUninit::uninit();
    // SAFETY: Caller-owned aligned output with exact version/size.
    let result = unsafe { get(1, output.as_mut_ptr(), size_of::<PluginUiV1>()) };
    if result != OK {
        return Err("plugin settings ABI rejected".into());
    }
    // SAFETY: Successful negotiation initializes the complete table.
    let api: PluginUiV1 = unsafe { output.assume_init() };
    if api.struct_size as usize != size_of::<PluginUiV1>() || api.abi_version != 1 {
        return Err("plugin settings ABI mismatch".into());
    }
    for address in [
        api.count as *const (),
        api.row as *const (),
        api.set as *const (),
    ] {
        plugin::pin(address)?;
    }
    let count = (api.count)();
    if count > 64 {
        return Err("at most 64 settings per DLL mod".into());
    }
    let mut rows = vec![];
    let mut keys = HashSet::new();
    for index in 0..count {
        let mut wire = SettingV1::default();
        // SAFETY: Pinned row callback writes this correctly sized output.
        if unsafe { (api.row)(index, &mut wire, size_of::<SettingV1>()) } != OK
            || wire.struct_size as usize != size_of::<SettingV1>()
        {
            return Err("plugin settings row rejected".into());
        }
        let key = wire_text(&wire.key, false)?;
        if !key
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.'))
            || !keys.insert(key.clone())
        {
            return Err("invalid or duplicate setting key".into());
        }
        let control = match wire.kind {
            1 => Control::Toggle,
            2 if wire.min.is_finite()
                && wire.max.is_finite()
                && wire.step.is_finite()
                && wire.min <= wire.max
                && wire.step > 0. =>
            {
                Control::Number {
                    min: wire.min,
                    max: wire.max,
                    step: wire.step,
                }
            }
            _ => return Err("invalid setting control".into()),
        };
        let default = wire_value(wire.kind, wire.default_value)?;
        // Metadata is bounded (64 mods x 64 rows) and discovered once per process.
        // Host owns these immutable strings for its pinned registry's lifetime.
        let setting = Setting {
            key: Box::leak(key.into_boxed_str()),
            label: Box::leak(wire_text(&wire.label, false)?.into_boxed_str()),
            description: Box::leak(wire_text(&wire.description, true)?.into_boxed_str()),
            default,
            control,
        };
        if !setting.accepts(default) {
            return Err("invalid setting default".into());
        }
        rows.push(Row {
            plugin: Box::leak(wire_text(&wire.title, false)?.into_boxed_str()),
            setting,
            value: default,
            error: None,
        });
    }
    Ok((Some(api), rows))
}
fn metadata_text(bytes: &[u8]) -> Result<String, String> {
    let length = bytes
        .iter()
        .position(|byte| *byte == 0)
        .ok_or("unterminated plugin metadata")?;
    if length == 0 || !bytes[..length].is_ascii() {
        return Err("plugin metadata must be non-empty ASCII".into());
    }
    Ok(String::from_utf8_lossy(&bytes[..length]).into_owned())
}
struct Module(HMODULE);
impl Drop for Module {
    fn drop(&mut self) {
        // SAFETY: Release exactly the LoadLibrary reference. PIN, when used,
        // keeps the module and its pointers alive for the remainder of the process.
        unsafe { FreeLibrary(self.0) };
    }
}

unsafe extern "C" fn host_log(token: u64, pointer: *const u8, length: usize) {
    let _ = std::panic::catch_unwind(|| {
        if pointer.is_null() || length > 65536 {
            return;
        }
        let Some(id) = registry()
            .lock()
            .ok()
            .and_then(|registry| registry.names.get(&token).cloned())
        else {
            return;
        };
        // SAFETY: V1 caller supplies a readable message slice for this call. It
        // is copied/formatted synchronously and the pointer is not retained.
        let message =
            String::from_utf8_lossy(unsafe { std::slice::from_raw_parts(pointer, length) });
        logging::write(&format!("[{id}] {message}"));
    });
}
unsafe extern "C" fn host_resolve(module: *const c_char, name: *const c_char) -> *const c_void {
    std::panic::catch_unwind(|| {
        if module.is_null() || name.is_null() {
            return std::ptr::null();
        }
        // SAFETY: V1 supplies live terminated strings for this synchronous call.
        let module = unsafe { CStr::from_ptr(module) };
        let Ok(module) = module.to_str() else {
            return std::ptr::null();
        };
        let wide: Vec<u16> = module.encode_utf16().chain(Some(0)).collect();
        let mut handle = std::ptr::null_mut();
        // SAFETY: Valid terminated module name and writable handle output. PIN
        // retains the module before export lookup, avoiding unload races.
        if unsafe { GetModuleHandleExW(GET_MODULE_HANDLE_EX_FLAG_PIN, wide.as_ptr(), &mut handle) }
            == 0
        {
            return std::ptr::null();
        }
        // SAFETY: Live loaded module; the caller's export string is terminated.
        let Some(symbol) = (unsafe { GetProcAddress(handle, name.cast()) }) else {
            return std::ptr::null();
        };
        if plugin::pin(symbol as *const ()).is_err() {
            return std::ptr::null();
        }
        symbol as *const c_void
    })
    .unwrap_or(std::ptr::null())
}
unsafe extern "C" fn host_claim(token: u64, target: *const c_void) -> i32 {
    std::panic::catch_unwind(|| {
        if target.is_null() {
            return ERROR;
        }
        let Ok(mut registry) = registry().lock() else {
            return ERROR;
        };
        if !registry.allowed.contains(&token) {
            return ERROR;
        }
        if registry.hooks.contains_key(&(target as usize)) {
            return HOOK_BUSY;
        }
        if plugin::pin(target.cast()).is_err() {
            return ERROR;
        }
        registry.hooks.insert(target as usize, token);
        OK
    })
    .unwrap_or(ERROR)
}
unsafe extern "C" fn host_release(token: u64, target: *const c_void) -> i32 {
    std::panic::catch_unwind(|| {
        let Ok(mut registry) = registry().lock() else {
            return ERROR;
        };
        if token == 0 || registry.hooks.get(&(target as usize)) != Some(&token) {
            return ERROR;
        }
        registry.hooks.remove(&(target as usize));
        OK
    })
    .unwrap_or(ERROR)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn opt_in_and_path_validation() {
        let config =
            Config::parse("schema_version=1\n[[plugins]]\nfile='disabled.dll'\nenabled=false")
                .unwrap();
        assert!(!config.plugins[0].enabled);
        for filename in [
            "../evil.dll",
            "C:\\evil.dll",
            "sub/evil.dll",
            "a.dll:stream",
            "a..dll",
        ] {
            assert!(!valid_filename(filename));
        }
        assert!(valid_filename("mod-v1.dll"));
        assert!(Config::parse("schema_version=2").is_err());
        assert!(Config::parse("schema_version=1\n[[plugins]]\nfile='a.dll'").is_err());
        assert!(Config::parse("schema_version=1\n[[plugins]]\nfile='a.dll'\nenabled=true\n[[plugins]]\nfile='A.dll'\nenabled=false").is_err());
    }
    #[test]
    fn metadata_is_bounded() {
        assert!(metadata_text(&[b'a'; 64]).is_err());
        assert!(metadata_text(&[0; 64]).is_err());
        assert!(metadata_text(&[0xff, 0]).is_err());
        assert_eq!(metadata_text(b"diagnostics\0").unwrap(), "diagnostics");
    }
}
