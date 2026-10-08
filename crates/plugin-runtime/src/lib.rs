//! Plugin composition, validated settings, and managed callback ownership.
//!
//! Build a composition with [`Plugins`], register implementations of [`Plugin`],
//! then start it with a persistence directory and trusted host factory. [`Runtime`]
//! applies settings transactionally and shuts plugins down in reverse startup order.
//! Filesystem and TOML failures preserve their original cause in [`Error`].
//!
//! Each [`Host`] owns a callback group. [`HookEvents`] dispatches after native
//! forwarding and waits for admitted callbacks during cleanup. Callback captures
//! must be owned, thread-safe, and initialized before registration. A panic
//! quarantines that group's callbacks and records a persistent restart request.
//! Already-running calls may finish; restart the host process before reusing
//! faulted plugin state. See [`Plugin`] and [`Host`] for complete examples.
#![forbid(unsafe_code)]
#![deny(missing_docs)]
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

mod error;
#[doc(inline)]
pub use error::Error;
mod host;
pub mod platform;
#[doc(inline)]
pub use host::{Hook, HookEvents, Host, Logger};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
/// Validated boolean or numeric plugin setting value.
pub enum Value {
    /// Boolean switch value or control.
    Toggle(bool),
    /// Finite numeric value or bounded numeric control.
    Number(f64),
}

#[derive(Clone, Copy, Debug)]
/// Setting control kind with numeric bounds when applicable.
pub enum Control {
    /// Boolean switch value or control.
    Toggle,
    /// Numeric control with finite inclusive bounds and a positive increment.
    Number {
        /// Inclusive minimum accepted value.
        min: f64,
        /// Inclusive maximum accepted value.
        max: f64,
        /// Positive increment used by the menu control.
        step: f64,
    },
}

#[derive(Clone, Copy, Debug)]
/// Declarative setting schema shared by persistence and the menu.
pub struct Setting {
    /// Stable unique setting key used for persistence and updates.
    pub key: &'static str,
    /// Control label displayed in the settings menu.
    pub label: &'static str,
    /// Explanation of the setting shown to the user.
    pub description: &'static str,
    /// Value used when no persisted setting exists.
    pub default: Value,
    /// Control kind and numeric bounds used to validate this setting.
    pub control: Control,
}
impl Setting {
    /// Check the setting value type, finiteness, and inclusive numeric bounds.
    pub fn accepts(&self, value: Value) -> bool {
        match (self.control, value) {
            (Control::Toggle, Value::Toggle(_)) => true,
            (Control::Number { min, max, .. }, Value::Number(x)) => {
                x.is_finite() && (min..=max).contains(&x)
            }
            _ => false,
        }
    }
}

#[derive(Clone, Default, Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
/// Complete validated snapshot of a plugin composition settings file.
pub struct Settings {
    values: BTreeMap<String, Value>,
}
impl Settings {
    /// Return a copied setting value, or None when its key is absent.
    pub fn get(&self, key: &str) -> Option<Value> {
        self.values.get(key).copied()
    }
    /// Read a toggle; absent keys and numeric values produce false.
    pub fn enabled(&self, key: &str) -> bool {
        self.get(key) == Some(Value::Toggle(true))
    }
    /// Read a number; absent keys and toggles produce zero.
    pub fn number(&self, key: &str) -> f64 {
        match self.get(key) {
            Some(Value::Number(x)) => x,
            _ => 0.,
        }
    }
}

/// Plugin startup context with a trusted host and persistence directory.
#[derive(Debug)]
pub struct Context {
    /// Trusted host connection owning this plugin callback group.
    pub host: Host,
    /// Directory containing plugin configuration and persistent data.
    pub root: PathBuf,
}
impl Context {
    /// Send a synchronous diagnostic through the trusted host logger.
    pub fn log(&self, text: &str) {
        self.host.log(text);
    }
}

/// An internal plugin compiled into the host DLL. It declares its settings and
/// owns its lifecycle; registration generates the in-game controls.
/// Only `id` and `start` are required for a callback-only plugin:
///
/// ```
/// use rekenshi_plugin_runtime::{Context, Error, Plugin, Settings};
/// use std::sync::{Arc, atomic::{AtomicU64, Ordering}};
///
/// #[derive(Default)]
/// struct OgreCounter { calls: Arc<AtomicU64> }
///
/// impl Plugin for OgreCounter {
///     fn id(&self) -> &'static str { "ogre-counter" }
///     fn start(&mut self, context: &Context, _: &Settings) -> Result<(), Error> {
///         let calls = self.calls.clone();
///         context.host.hook_ogre_calls(move || {
///             calls.fetch_add(1, Ordering::Relaxed);
///         })?;
///         Ok(())
///     }
/// }
/// ```
pub trait Plugin: Send {
    /// Return the stable unique plugin identifier used for dependencies and persistence.
    fn id(&self) -> &'static str;
    /// Return the plugin group title; defaults to its stable identifier.
    fn title(&self) -> &'static str {
        self.id()
    }
    /// Return plugin identifiers that must already be active before startup.
    fn requires(&self) -> &'static [&'static str] {
        &[]
    }
    /// Return the static setting schema used for validation and menu generation.
    fn settings(&self) -> &'static [Setting] {
        &[]
    }
    /// Register desired behavior before any native provider in this composition starts.
    ///
    /// Do not assume native resources exist. Failure is cleaned by stop, and
    /// prevents this plugin from starting. Independent DLLs use this same phase.
    fn register(&mut self, _context: &Context, _settings: &Settings) -> Result<(), Error> {
        Ok(())
    }
    /// Prepare native resources once. On failure, stop cleans partial preparation.
    fn start(&mut self, _context: &Context, _settings: &Settings) -> Result<(), Error> {
        Ok(())
    }
    /// Activate providers/resources after the desired-registration barrier commits.
    ///
    /// Existing implementations of start remain supported through this default.
    fn install(&mut self, context: &Context, settings: &Settings) -> Result<(), Error> {
        self.start(context, settings)
    }
    /// Name capability/lifecycle contracts to observe on the serial plugin lane.
    ///
    /// Missing providers remain pending. Names alone never establish native support.
    fn lifecycle_contracts(&self) -> &'static [&'static str] {
        &[]
    }
    /// Domain occurrences to deliver later on ordinary serialized plugin state.
    fn event_contracts(&self) -> &'static [&'static str] {
        &[]
    }
    /// React to a copied occurrence, never to elapsed time or a generic update.
    /// World/resource identifiers in the payload may already have been invalidated.
    fn event(
        &mut self,
        _context: &Context,
        _name: &str,
        _event: &rekenshi_plugin_api::platform::EventV1,
    ) -> Result<(), Error> {
        Ok(())
    }
    /// Receive a copied provider generation transition at an eligible game dispatch point.
    ///
    /// READY does not grant native pointers or guarantee before-first-use ordering.
    /// Delayed REVOKED delivery never prolongs a native resource's lifetime.
    fn lifecycle_changed(
        &mut self,
        _context: &Context,
        _name: &str,
        _status: rekenshi_plugin_api::platform::StatusV1,
    ) -> Result<(), Error> {
        Ok(())
    }
    /// Apply a complete validated snapshot. An error must preserve the old state.
    fn update(&mut self, _context: &Context, _settings: &Settings) -> Result<(), Error> {
        Ok(())
    }
    /// Apply validated settings, independently of native frames or simulation updates.
    fn settings_changed(&mut self, context: &Context, settings: &Settings) -> Result<(), Error> {
        self.update(context, settings)
    }
    /// Quiesce resources; called in reverse composition order, including failed starts.
    fn stop(&mut self) -> Result<(), Error> {
        Ok(())
    }
}

#[derive(Default)]
/// Composition builder validating plugins and settings before startup.
pub struct Plugins {
    plugins: Vec<Box<dyn Plugin>>,
    provided: Vec<&'static str>,
    settings_path: Option<PathBuf>,
}
impl Plugins {
    /// Create an empty composition builder.
    pub fn new() -> Self {
        Self::default()
    }
    /// Append a plugin to the composition startup order.
    pub fn register(mut self, plugin: impl Plugin + 'static) -> Self {
        self.plugins.push(Box::new(plugin));
        self
    }
    /// The embedding host verifies these dependencies before calling start.
    pub fn provide(mut self, id: &'static str) -> Self {
        self.provided.push(id);
        self
    }
    /// Use a separate file for an independently distributed plugin.
    pub fn settings_path(mut self, path: PathBuf) -> Self {
        self.settings_path = Some(path);
        self
    }
    /// Validate and mount the composition, cleaning partial starts and blocking dependents.
    pub fn start<H: Into<Host>>(
        self,
        root: PathBuf,
        host: impl FnMut(&str, u64) -> Result<H, Error>,
    ) -> Result<Runtime, Error> {
        let mut runtime = self.prepare(root, host)?;
        runtime.install()?;
        Ok(runtime)
    }
    /// Commit desired registrations without enabling native providers.
    ///
    /// # Errors
    /// Rejects invalid composition/settings or host connection failures.
    pub fn prepare<H: Into<Host>>(
        mut self,
        root: PathBuf,
        mut host: impl FnMut(&str, u64) -> Result<H, Error>,
    ) -> Result<Runtime, Error> {
        Host::check_shutdown()?;
        let schema = self.validate()?;
        let path = self
            .settings_path
            .clone()
            .unwrap_or_else(|| root.join("modpack.toml"));
        let settings = read_settings(&path, &schema)?;
        let mut mounted: Vec<Mounted> = Vec::new();
        let mut prepared = Vec::with_capacity(self.plugins.len());
        for (i, plugin) in self.plugins.iter_mut().enumerate() {
            let context = match host(plugin.id(), 1001 + i as u64) {
                Ok(host) => Context {
                    host: host.into(),
                    root: root.clone(),
                },
                Err(e) => {
                    for (context, _) in &prepared {
                        let context: &Context = context;
                        context.host.shutdown()?;
                    }
                    for plugin in self.plugins[..i].iter_mut().rev() {
                        let _ = plugin.stop();
                    }
                    return Err(e);
                }
            };
            let result = (|| {
                plugin.register(&context, &settings)?;
                let mut watches = Vec::new();
                if !plugin.lifecycle_contracts().is_empty() {
                    let platform = context
                        .host
                        .native()
                        .platform()
                        .map_err(|code| Error::host("platform", code))?;
                    for name in plugin.lifecycle_contracts() {
                        watches.push((
                            *name,
                            platform
                                .watch(name)
                                .map_err(|code| Error::host("platform", code))?,
                        ));
                    }
                }
                let mut events = Vec::new();
                if !plugin.event_contracts().is_empty() {
                    let platform = context
                        .host
                        .native()
                        .platform()
                        .map_err(|code| Error::host("platform events", code))?;
                    for name in plugin.event_contracts() {
                        events.push((
                            *name,
                            platform
                                .watch_events(name)
                                .map_err(|code| Error::host("platform events", code))?,
                        ));
                    }
                }
                Ok::<_, Error>((watches, events))
            })();
            if result.is_err() {
                // Failed desired registrations must not become active when an
                // earlier provider starts at the installation barrier.
                if let Err(error) = context.host.shutdown() {
                    context.log(&format!("registration callback cleanup failed: {error}"));
                }
                if let Err(error) = plugin.stop() {
                    context.log(&format!(
                        "registration cleanup failed; resources retained: {error}"
                    ));
                }
            }
            prepared.push((context, result));
        }
        for (plugin, (context, prepared)) in self.plugins.into_iter().zip(prepared) {
            let (watches, events, error) = match prepared {
                Ok((watches, events)) => (watches, events, None),
                Err(error) => (Vec::new(), Vec::new(), Some(error.to_string())),
            };
            mounted.push(Mounted {
                plugin,
                context,
                error,
                watches,
                events,
            });
        }
        Ok(Runtime {
            mounted,
            installed: false,
            provided: self.provided,
            schema,
            settings,
            path,
        })
    }
    fn validate(&self) -> Result<Vec<Setting>, Error> {
        let mut ids: HashSet<_> = self.provided.iter().copied().collect();
        let mut keys = HashSet::new();
        let mut schema = Vec::new();
        for plugin in &self.plugins {
            if !ids.insert(plugin.id()) {
                return Err(format!("duplicate plugin {}", plugin.id()).into());
            }
            for required in plugin.requires() {
                if *required == plugin.id() || !ids.contains(required) {
                    return Err(
                        format!("{} must be registered before {}", required, plugin.id()).into(),
                    );
                }
            }
            for setting in plugin.settings() {
                let valid_control = match setting.control {
                    Control::Toggle => true,
                    Control::Number { min, max, step } => {
                        min.is_finite()
                            && max.is_finite()
                            && step.is_finite()
                            && min <= max
                            && step > 0.
                    }
                };
                if !keys.insert(setting.key) || !valid_control || !setting.accepts(setting.default)
                {
                    return Err(format!("invalid or duplicate setting {}", setting.key).into());
                }
                schema.push(*setting);
            }
        }
        Ok(schema)
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    schema_version: u32,
    settings: Settings,
}
fn read_settings(path: &Path, schema: &[Setting]) -> Result<Settings, Error> {
    let mut settings = Settings {
        values: schema
            .iter()
            .map(|s| (s.key.to_owned(), s.default))
            .collect(),
    };
    let text = match std::fs::read_to_string(path) {
        Ok(text) if text.len() <= 65536 => text,
        Ok(_) => return Err("modpack settings exceed 64 KiB".into()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(settings),
        Err(e) => return Err(e.into()),
    };
    let file: File = toml::from_str(&text)?;
    if file.schema_version != 1 {
        return Err("unsupported modpack settings version".into());
    }
    for (key, value) in file.settings.values {
        if !schema.iter().any(|s| s.key == key && s.accepts(value)) {
            return Err(format!("unknown or invalid setting {key}").into());
        }
        settings.values.insert(key, value);
    }
    Ok(settings)
}
struct Mounted {
    plugin: Box<dyn Plugin>,
    context: Context,
    error: Option<String>,
    watches: Vec<(&'static str, rekenshi_plugin_api::platform::Watch)>,
    events: Vec<(&'static str, rekenshi_plugin_api::platform::EventWatch)>,
}
#[derive(Clone, Debug)]
/// Owned menu row containing a setting value and plugin availability.
pub struct Row {
    /// Display title of the plugin owning this settings row.
    pub plugin: &'static str,
    /// Declarative schema defining this row control and validation rules.
    pub setting: Setting,
    /// Current validated setting value.
    pub value: Value,
    /// Current failure description, or empty storage when the plugin is available.
    pub error: Option<String>,
}
/// Mounted plugin composition with validated settings and reverse-order cleanup.
pub struct Runtime {
    mounted: Vec<Mounted>,
    installed: bool,
    provided: Vec<&'static str>,
    schema: Vec<Setting>,
    settings: Settings,
    path: PathBuf,
}
impl Drop for Runtime {
    fn drop(&mut self) {
        // Release every group before dropping any plugin-owned Rust resources.
        for item in &self.mounted {
            item.context.host.release_callbacks();
        }
    }
}
impl Runtime {
    /// Enable providers after the host has prepared every participating composition.
    ///
    /// # Errors
    /// Rejects duplicate installation and forbidden shutdown lanes. Individual
    /// plugin failures are logged and reflected in availability, with cleanup.
    pub fn install(&mut self) -> Result<(), Error> {
        Host::check_shutdown()?;
        if self.installed {
            return Err("composition already installed".into());
        }
        let mut active = self.provided.clone();
        for item in &mut self.mounted {
            if item.error.is_some() {
                continue;
            }
            let blocked = item
                .plugin
                .requires()
                .iter()
                .find(|id| !active.contains(id));
            let result = match blocked {
                Some(id) => Err(format!("requires unavailable plugin {id}").into()),
                None => item.plugin.install(&item.context, &self.settings),
            };
            if let Err(error) = result {
                item.context
                    .log(&format!("plugin {} unavailable: {error}", item.plugin.id()));
                item.context.host.shutdown()?;
                if let Err(cleanup) = item.plugin.stop() {
                    item.context.log(&format!(
                        "plugin cleanup failed; resources retained: {cleanup}"
                    ));
                }
                item.error = Some(error.to_string());
            } else {
                active.push(item.plugin.id());
            }
        }
        self.installed = true;
        Ok(())
    }
    /// Copy the mounted plugin identifiers in composition order.
    pub fn ids(&self) -> Vec<&'static str> {
        self.mounted.iter().map(|m| m.plugin.id()).collect()
    }
    /// Report whether a mounted plugin started successfully and remains unquarantined.
    pub fn available(&self, id: &str) -> bool {
        self.mounted.iter().any(|m| {
            self.installed && m.plugin.id() == id && m.error.is_none() && !m.context.host.faulted()
        })
    }
    /// Count successfully started plugins whose callback groups remain healthy.
    pub fn active_count(&self) -> usize {
        self.mounted
            .iter()
            .filter(|m| self.installed && m.error.is_none() && !m.context.host.faulted())
            .count()
    }
    /// Copy current menu values and availability diagnostics for all declared settings.
    pub fn rows(&self) -> Vec<Row> {
        self.mounted
            .iter()
            .flat_map(|m| {
                m.plugin.settings().iter().map(|s| Row {
                    plugin: m.plugin.title(),
                    setting: *s,
                    value: self.settings.get(s.key).unwrap_or(s.default),
                    error: m.error.clone().or_else(|| {
                        m.context
                            .host
                            .faulted()
                            .then(|| "plugin panicked; restart Kenshi required".into())
                    }),
                })
            })
            .collect()
    }
    /// Apply a validated setting, persist it, and roll back on save failure.
    ///
    /// # Errors
    /// Returns a failure when service negotiation, validation, or the host operation fails.
    pub fn set(&mut self, key: &str, value: Value) -> Result<(), Error> {
        Host::check_shutdown()?;
        let setting = self
            .schema
            .iter()
            .find(|s| s.key == key)
            .ok_or("unknown setting")?;
        if !setting.accepts(value) {
            return Err("setting value outside supported range/type".into());
        }
        let item = self
            .mounted
            .iter_mut()
            .find(|m| m.plugin.settings().iter().any(|s| s.key == key))
            .ok_or("setting plugin unavailable")?;
        if item.context.host.faulted() {
            return Err(Error::restart_required());
        }
        if let Some(e) = &item.error {
            return Err(format!("plugin unavailable: {e}").into());
        }
        if self.settings.get(key) == Some(value) {
            return Ok(());
        }
        let mut next = self.settings.clone();
        next.values.insert(key.to_owned(), value);
        item.plugin.settings_changed(&item.context, &next)?;
        let result = (|| {
            let text = toml::to_string_pretty(&File {
                schema_version: 1,
                settings: next.clone(),
            })?;
            let pending = self.path.with_extension("toml.pending");
            std::fs::write(&pending, text)?;
            std::fs::rename(&pending, &self.path).map_err(Error::from)
        })();
        if let Err(error) = result {
            if let Err(rollback) = item.plugin.settings_changed(&item.context, &self.settings) {
                item.error = Some(format!("settings rollback failed: {rollback}"));
                return Err(format!("save failed: {error}; rollback failed: {rollback}").into());
            }
            return Err(format!("settings were not saved; change rolled back: {error}").into());
        }
        self.settings = next;
        Ok(())
    }
    /// Quiesce callback groups and clean plugin resources in reverse startup order.
    ///
    /// # Errors
    /// Returns a failure when service negotiation, validation, or the host operation fails.
    pub fn stop(&mut self) -> Result<(), Error> {
        Host::check_shutdown()?;
        // Detach all plugin callbacks before stopping any plugin, so an observer
        // cannot access another plugin after its native resources are destroyed.
        for item in &self.mounted {
            item.context.host.shutdown()?;
        }
        let mut errors = Vec::new();
        for item in self.mounted.iter_mut().rev() {
            item.watches.clear();
            item.events.clear();
            if let Err(e) = item.plugin.stop() {
                errors.push(format!("{}: {e}", item.plugin.id()));
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join("; ").into())
        }
    }

    /// Deliver only pending lifecycle transitions at the host's verified game-lane point.
    ///
    /// This is host infrastructure, not a plugin tick. Each owner receives at
    /// most 64 transitions per dispatch; mutable plugin state stays serialized.
    ///
    /// # Errors
    /// Quarantines failed observers and requests restart after a panic or overflow.
    pub fn dispatch_lifecycle(&mut self) -> Result<(), Error> {
        if !self.installed {
            return Ok(());
        }
        let mut failure = None;
        for item in &mut self.mounted {
            if item.error.is_some() {
                continue;
            }
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                if item.context.host.faulted() {
                    return Err(Error::restart_required());
                }
                let mut budget = 64;
                for (name, watch) in &item.watches {
                    while budget != 0 {
                        let Some(status) = watch
                            .next_transition()
                            .map_err(|code| Error::host("platform lifecycle", code))?
                        else {
                            break;
                        };
                        budget -= 1;
                        item.plugin.lifecycle_changed(&item.context, name, status)?;
                    }
                }
                for (name, watch) in &item.events {
                    while budget != 0 {
                        let Some(event) = watch
                            .next_event()
                            .map_err(|code| Error::host("platform event", code))?
                        else {
                            break;
                        };
                        budget -= 1;
                        item.plugin.event(&item.context, name, &event)?;
                    }
                }
                Ok::<_, Error>(())
            }));
            let error = match result {
                Ok(Ok(())) => continue,
                Ok(Err(error)) => error,
                Err(payload) => {
                    std::mem::forget(payload);
                    Error::restart_required()
                }
            };
            item.context.log(&format!(
                "lifecycle observer disabled; restart required: {error}"
            ));
            item.error = Some(error.to_string());
            if let Ok(platform) = item.context.host.native().platform() {
                (platform.api().quarantine)(platform.token());
            }
            item.context.host.shutdown()?;
            if let Err(cleanup) = item.plugin.stop() {
                item.context.log(&format!(
                    "failed observer cleanup retained resources: {cleanup}"
                ));
            }
            item.watches.clear();
            item.events.clear();
            failure = Some(error);
        }
        failure.map_or(Ok(()), Err)
    }
}

impl std::fmt::Debug for Plugins {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Plugins").finish_non_exhaustive()
    }
}

impl std::fmt::Debug for Runtime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Runtime").finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rekenshi_plugin_api::HostV1;
    use std::sync::{Arc, Mutex};
    const SWITCH: &[Setting] = &[Setting {
        key: "test.enabled",
        label: "Test",
        description: "Test switch",
        default: Value::Toggle(true),
        control: Control::Toggle,
    }];
    const NUMBER: &[Setting] = &[Setting {
        key: "test.amount",
        label: "Amount",
        description: "Test value",
        default: Value::Number(1.),
        control: Control::Number {
            min: 0.,
            max: 4.,
            step: 0.1,
        },
    }];
    struct TestPlugin {
        id: &'static str,
        requires: &'static [&'static str],
        fields: &'static [Setting],
        events: Arc<Mutex<Vec<String>>>,
        fail_start: bool,
        fail_update: bool,
    }
    impl Plugin for TestPlugin {
        fn id(&self) -> &'static str {
            self.id
        }
        fn title(&self) -> &'static str {
            self.id
        }
        fn requires(&self) -> &'static [&'static str] {
            self.requires
        }
        fn settings(&self) -> &'static [Setting] {
            self.fields
        }
        fn start(&mut self, _: &Context, _: &Settings) -> Result<(), Error> {
            self.events
                .lock()
                .unwrap()
                .push(format!("start {}", self.id));
            if self.fail_start {
                Err("start failed".into())
            } else {
                Ok(())
            }
        }
        fn update(&mut self, _: &Context, settings: &Settings) -> Result<(), Error> {
            self.events.lock().unwrap().push(format!(
                "update {} {:?}",
                self.id,
                settings.get("test.enabled")
            ));
            if self.fail_update {
                Err("update rejected".into())
            } else {
                Ok(())
            }
        }
        fn stop(&mut self) -> Result<(), Error> {
            self.events
                .lock()
                .unwrap()
                .push(format!("stop {}", self.id));
            Ok(())
        }
    }
    fn plugin(id: &'static str, events: &Arc<Mutex<Vec<String>>>) -> TestPlugin {
        TestPlugin {
            id,
            requires: &[],
            fields: &[],
            events: events.clone(),
            fail_start: false,
            fail_update: false,
        }
    }
    fn host(_: &str, token: u64) -> Result<HostV1, Error> {
        Ok(HostV1::stub(token))
    }
    fn folder() -> PathBuf {
        static SERIAL: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "rekenshi-pack-test-{}-{}",
            std::process::id(),
            SERIAL.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).unwrap();
        path
    }
    #[test]
    fn composition_mounts_forward_and_stops_reverse() {
        let events = Arc::default();
        let mut second = plugin("second", &events);
        second.requires = &["first"];
        let mut runtime = Plugins::new()
            .register(plugin("first", &events))
            .register(second)
            .start(folder(), host)
            .unwrap();
        assert_eq!(runtime.active_count(), 2);
        runtime.stop().unwrap();
        assert_eq!(
            *events.lock().unwrap(),
            ["start first", "start second", "stop second", "stop first"]
        );
    }
    #[test]
    fn rejects_bad_order_duplicate_plugins_and_duplicate_settings_before_start() {
        let events = Arc::default();
        let mut second = plugin("second", &events);
        second.requires = &["first"];
        assert!(
            Plugins::new()
                .register(second)
                .start(folder(), host)
                .is_err()
        );
        assert!(
            Plugins::new()
                .register(plugin("first", &events))
                .register(plugin("first", &events))
                .start(folder(), host)
                .is_err()
        );
        let mut a = plugin("first", &events);
        a.fields = SWITCH;
        let mut b = plugin("second", &events);
        b.fields = SWITCH;
        assert!(
            Plugins::new()
                .register(a)
                .register(b)
                .start(folder(), host)
                .is_err()
        );
        assert!(events.lock().unwrap().is_empty());
    }
    #[test]
    fn live_changes_persist_and_survive_restart_with_validation() {
        let events = Arc::default();
        let path = folder();
        let mut a = plugin("first", &events);
        a.fields = NUMBER;
        let mut runtime = Plugins::new()
            .register(a)
            .start(path.clone(), host)
            .unwrap();
        assert!(runtime.set("test.amount", Value::Toggle(false)).is_err());
        assert!(runtime.set("test.amount", Value::Number(f64::NAN)).is_err());
        assert!(runtime.set("test.amount", Value::Number(5.)).is_err());
        runtime.set("test.amount", Value::Number(1.5)).unwrap();
        runtime.set("test.amount", Value::Number(2.)).unwrap(); // Replaces an existing file on Windows.
        let mut b = plugin("first", &events);
        b.fields = NUMBER;
        let loaded = Plugins::new().register(b).start(path, host).unwrap();
        assert_eq!(loaded.rows()[0].value, Value::Number(2.));
    }
    #[test]
    fn failed_save_rolls_back_the_effect_and_model() {
        let events = Arc::default();
        let mut a = plugin("first", &events);
        a.fields = SWITCH;
        let mut runtime = Plugins::new().register(a).start(folder(), host).unwrap();
        runtime.path = runtime.path.parent().unwrap().join("missing/modpack.toml");
        assert!(runtime.set("test.enabled", Value::Toggle(false)).is_err());
        assert_eq!(runtime.rows()[0].value, Value::Toggle(true));
        assert!(events.lock().unwrap().ends_with(&[
            "update first Some(Toggle(false))".into(),
            "update first Some(Toggle(true))".into()
        ]));
    }
    #[test]
    fn failed_update_does_not_save_or_change_model() {
        let events = Arc::default();
        let mut a = plugin("first", &events);
        a.fields = SWITCH;
        a.fail_update = true;
        let mut runtime = Plugins::new().register(a).start(folder(), host).unwrap();
        assert!(runtime.set("test.enabled", Value::Toggle(false)).is_err());
        assert!(!runtime.path.exists());
        assert_eq!(runtime.rows()[0].value, Value::Toggle(true));
    }
    #[test]
    fn failed_start_cleans_partial_resources_and_blocks_dependents() {
        let events = Arc::default();
        let mut a = plugin("first", &events);
        a.fail_start = true;
        let mut b = plugin("second", &events);
        b.requires = &["first"];
        b.fields = SWITCH;
        let mut runtime = Plugins::new()
            .register(a)
            .register(b)
            .start(folder(), host)
            .unwrap();
        assert_eq!(runtime.active_count(), 0);
        assert!(runtime.rows()[0].error.is_some());
        assert!(runtime.set("test.enabled", Value::Toggle(false)).is_err());
        assert_eq!(
            *events.lock().unwrap(),
            ["start first", "stop first", "stop second"]
        );
    }
    #[test]
    fn malformed_and_unknown_saved_settings_fail_before_mount() {
        let events = Arc::default();
        let path = folder();
        for text in [
            "bad toml =",
            "schema_version=2\n[settings.values]",
            "schema_version=1\n[settings.values]\nunknown=true",
        ] {
            std::fs::write(path.join("modpack.toml"), text).unwrap();
            let mut a = plugin("first", &events);
            a.fields = SWITCH;
            assert!(
                Plugins::new()
                    .register(a)
                    .start(path.clone(), host)
                    .is_err()
            );
        }
        assert!(events.lock().unwrap().is_empty());
    }

    struct CallbackPlugin {
        id: &'static str,
        events: HookEvents,
        calls: Arc<std::sync::atomic::AtomicU64>,
        stopped: Arc<std::sync::atomic::AtomicU64>,
        fail: bool,
    }
    impl Plugin for CallbackPlugin {
        fn id(&self) -> &'static str {
            self.id
        }
        fn title(&self) -> &'static str {
            self.id
        }
        fn start(&mut self, context: &Context, _: &Settings) -> Result<(), Error> {
            let calls = self.calls.clone();
            context.host.hook_ogre_calls(move || {
                calls.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            })?;
            if self.fail {
                Err("failed after subscribing".into())
            } else {
                Ok(())
            }
        }
        fn update(&mut self, _: &Context, _: &Settings) -> Result<(), Error> {
            Ok(())
        }
        fn stop(&mut self) -> Result<(), Error> {
            let before = self.calls.load(std::sync::atomic::Ordering::Relaxed);
            self.events.dispatch_ogre_calls();
            assert_eq!(
                self.calls.load(std::sync::atomic::Ordering::Relaxed),
                before,
                "runtime must detach callbacks before stop destroys plugin resources"
            );
            self.stopped
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            Ok(())
        }
    }

    #[test]
    fn callback_fault_retires_owner_at_serial_dispatch_without_pending_events() {
        let events = HookEvents::default();
        events.set_available(Hook::OgreCalls, true);
        let history = Arc::default();
        let mut runtime = Plugins::new()
            .register(plugin("faulted", &history))
            .start(folder(), |id, token| {
                host(id, token).map(|raw| Host::new(raw, events.clone()))
            })
            .unwrap();
        runtime.mounted[0]
            .context
            .host
            .hook_ogre_calls(|| panic!("callback failed"))
            .unwrap();
        assert_eq!(events.dispatch_ogre_calls(), 1);
        assert!(runtime.dispatch_lifecycle().is_err());
        assert!(runtime.mounted[0].error.is_some());
        assert_eq!(*history.lock().unwrap(), ["start faulted", "stop faulted"]);
        assert!(runtime.dispatch_lifecycle().is_ok());
        assert_eq!(*history.lock().unwrap(), ["start faulted", "stop faulted"]);
    }

    #[test]
    fn runtime_owns_callback_cleanup_for_stop_failed_start_and_failed_host_creation() {
        use std::sync::atomic::{AtomicU64, Ordering};
        for failure in [0, 1, 2, 3] {
            let events = HookEvents::default();
            events.set_available(Hook::OgreCalls, true);
            let calls = Arc::new(AtomicU64::new(0));
            let stopped = Arc::new(AtomicU64::new(0));
            let make = |id| CallbackPlugin {
                id,
                events: events.clone(),
                calls: calls.clone(),
                stopped: stopped.clone(),
                fail: failure == 1,
            };
            let result = Plugins::new()
                .register(make("one"))
                .register(make("two"))
                .start(folder(), |id, token| {
                    if id == "two" && failure == 2 {
                        Err("host creation failed".into())
                    } else {
                        host(id, token).map(|raw| Host::new(raw, events.clone()))
                    }
                });
            if failure == 2 {
                assert!(result.is_err());
                assert_eq!(stopped.load(Ordering::Relaxed), 1);
            } else {
                let mut runtime = result.unwrap();
                events.dispatch_ogre_calls();
                assert_eq!(
                    calls.load(Ordering::Relaxed),
                    if failure == 1 { 0 } else { 2 }
                );
                if failure != 3 {
                    runtime.stop().unwrap();
                }
                // Failed starts already received stop for partial initialization.
                assert_eq!(
                    stopped.load(Ordering::Relaxed),
                    match failure {
                        1 => 4,
                        3 => 0,
                        _ => 2,
                    }
                );
            }
            let before = calls.load(Ordering::Relaxed);
            events.dispatch_ogre_calls();
            assert_eq!(calls.load(Ordering::Relaxed), before);
        }
    }
}
