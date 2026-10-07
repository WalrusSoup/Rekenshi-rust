use crate::{
    Control, Plugin, Value,
    abi::{self, ERROR, HostV1, INCOMPATIBLE, OK, PluginV1, services::*},
    transport::Connection,
};
use rekenshi_plugin_runtime::{HookEvents, Plugins, Runtime};
use std::{
    marker::PhantomData,
    panic::{AssertUnwindSafe, catch_unwind},
    sync::{Arc, Mutex},
};
/// Plugin DLL lifecycle and settings transport used by the export macro.
pub struct Dll<P> {
    session: Mutex<Option<Session>>,
    plugin: PhantomData<fn() -> P>,
}
struct Session {
    runtime: Runtime,
    connection: Arc<Connection>,
}
pub(crate) fn boundary(action: impl FnOnce() -> i32) -> i32 {
    match catch_unwind(AssertUnwindSafe(action)) {
        Ok(result) => result,
        Err(payload) => {
            std::mem::forget(payload);
            ERROR
        }
    }
}
fn copy<const N: usize>(output: &mut [u8; N], text: &str) -> Result<(), crate::Error> {
    if text.len() >= N || text.contains('\0') {
        return Err("plugin metadata too long or contains NUL".into());
    }
    output[..text.len()].copy_from_slice(text.as_bytes());
    Ok(())
}
fn number(value: Value) -> f64 {
    match value {
        Value::Toggle(v) => f64::from(v),
        Value::Number(v) => v,
    }
}
impl<P: Plugin + Default + 'static> Default for Dll<P> {
    fn default() -> Self {
        Self::new()
    }
}
impl<P: Plugin + Default + 'static> Dll<P> {
    /// Create an inactive DLL lifecycle transport.
    pub const fn new() -> Self {
        Self {
            session: Mutex::new(None),
            plugin: PhantomData,
        }
    }
    /// Export bounded plugin metadata into negotiated caller-owned storage.
    ///
    /// # Safety
    /// Caller obeys export_descriptor's negotiated writable-output contract.
    pub unsafe fn entry(
        &self,
        version: u32,
        output: *mut PluginV1,
        size: usize,
        start: abi::Start,
        stop: abi::Stop,
        release: &str,
    ) -> i32 {
        boundary(|| {
            if version != abi::ABI_VERSION || size != size_of::<PluginV1>() {
                return INCOMPATIBLE;
            }
            if output.is_null() {
                return ERROR;
            }
            let plugin = P::default();
            // SAFETY: Forward the caller's negotiated output buffer contract.
            unsafe {
                abi::export_descriptor(
                    version,
                    output,
                    size,
                    PluginV1::new(plugin.id(), plugin.title(), release, start, stop),
                )
            }
        })
    }
    /// Start the plugin session with the loader's trusted host table.
    ///
    /// # Safety
    /// Pointer refers to the loader's negotiated stable HostV1 table.
    pub unsafe fn start(&self, pointer: *const HostV1) -> i32 {
        boundary(|| {
            if HookEvents::check_shutdown().is_err() {
                return ERROR;
            }
            // SAFETY: Loader provides the valid negotiated host table.
            let host = match unsafe { HostV1::read(pointer) } {
                Ok(h) => h,
                Err(e) => return e,
            };
            let result: Result<(), crate::Error> = (|| {
                let mut state = self.session.lock().map_err(|_| "plugin session poisoned")?;
                if state.is_some() {
                    return Err("plugin already started".into());
                }
                let connection = Connection::new(host)?;
                let root = connection.root()?;
                let plugin = P::default();
                let id = plugin.id();
                if id.is_empty()
                    || !id
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
                {
                    return Err("invalid plugin id".into());
                }
                let folder = root.join("settings");
                std::fs::create_dir_all(&folder)?;
                let mut plugins = Plugins::new().settings_path(folder.join(format!("{id}.toml")));
                for dependency in plugin.requires() {
                    connection.requires(dependency)?;
                    plugins = plugins.provide(dependency);
                }
                let runtime = plugins
                    .register(plugin)
                    .start(root, |_, _| Ok::<_, crate::Error>(connection.host()))?;
                let active = runtime.active_count() == 1;
                *state = Some(Session {
                    runtime,
                    connection,
                });
                if active {
                    Ok(())
                } else {
                    Err("plugin failed to start; see host log".into())
                }
            })();
            match result {
                Ok(()) => OK,
                Err(error) => {
                    host.write_log(&error.to_string());
                    ERROR
                }
            }
        })
    }
    /// Quiesce callback groups and clean plugin resources in reverse startup order.
    pub fn stop(&self) -> i32 {
        boundary(|| {
            if HookEvents::check_shutdown().is_err() {
                return ERROR;
            }
            let Ok(mut state) = self.session.lock() else {
                return ERROR;
            };
            let Some(session) = state.as_mut() else {
                return OK;
            };
            if session.runtime.stop().is_err() || session.connection.close().is_err() {
                return ERROR;
            }
            *state = None;
            OK
        })
    }
    /// Return the number of declared settings; return zero if metadata construction panics.
    pub fn count(&self) -> u32 {
        let mut count = 0;
        boundary(|| {
            count = P::default().settings().len() as u32;
            OK
        });
        count
    }
    /// Copy a setting declaration and its current value into caller storage.
    ///
    /// # Safety
    /// Exact-size requests provide writable aligned SettingV1 storage.
    pub unsafe fn row(&self, index: u32, output: *mut SettingV1, size: usize) -> i32 {
        boundary(|| {
            if output.is_null() || size != size_of::<SettingV1>() {
                return ERROR;
            }
            let plugin = P::default();
            let Some(setting) = plugin.settings().get(index as usize) else {
                return ERROR;
            };
            let mut row = SettingV1::default();
            if copy(&mut row.title, plugin.title())
                .and_then(|_| copy(&mut row.key, setting.key))
                .and_then(|_| copy(&mut row.label, setting.label))
                .and_then(|_| copy(&mut row.description, setting.description))
                .is_err()
            {
                return ERROR;
            }
            row.default_value = number(setting.default);
            row.value = row.default_value;
            match setting.control {
                Control::Toggle => row.kind = 1,
                Control::Number { min, max, step } => {
                    row.kind = 2;
                    row.min = min;
                    row.max = max;
                    row.step = step;
                }
            }
            let Ok(state) = self.session.lock() else {
                return ERROR;
            };
            if let Some(session) = state.as_ref() {
                if let Some(current) = session.runtime.rows().get(index as usize) {
                    row.value = number(current.value);
                    if let Some(error) = &current.error {
                        let _ = copy(&mut row.error, error);
                    }
                }
            }
            // SAFETY: Caller supplies exact-sized aligned writable row output.
            unsafe {
                output.write(row);
            }
            OK
        })
    }
    /// Validate, apply, and persist a setting supplied by the host menu.
    ///
    /// # Safety
    /// Key points to len readable UTF-8 bytes for this call.
    pub unsafe fn set(&self, key: *const u8, len: usize, kind: u32, value: f64) -> i32 {
        boundary(|| {
            if HookEvents::check_shutdown().is_err() || key.is_null() || len == 0 || len > 128 {
                return ERROR;
            }
            // SAFETY: Host guarantees this bounded key is readable synchronously.
            let Ok(key) = std::str::from_utf8(unsafe { std::slice::from_raw_parts(key, len) })
            else {
                return ERROR;
            };
            let value = match (kind, value) {
                (1, 0.) => Value::Toggle(false),
                (1, 1.) => Value::Toggle(true),
                (2, v) if v.is_finite() => Value::Number(v),
                _ => return ERROR,
            };
            let Ok(mut state) = self.session.lock() else {
                return ERROR;
            };
            let Some(session) = state.as_mut() else {
                return ERROR;
            };
            match session.runtime.set(key, value) {
                Ok(()) => OK,
                Err(error) => {
                    session.connection.host().log(&error.to_string());
                    ERROR
                }
            }
        })
    }
    /// Export the negotiated settings callbacks into caller-owned storage.
    ///
    /// # Safety
    /// Accepted negotiation supplies aligned writable PluginUiV1 output.
    pub unsafe fn ui(
        version: u32,
        output: *mut PluginUiV1,
        size: usize,
        count: extern "C" fn() -> u32,
        row: unsafe extern "C" fn(u32, *mut SettingV1, usize) -> i32,
        set: unsafe extern "C" fn(*const u8, usize, u32, f64) -> i32,
    ) -> i32 {
        if version != VERSION || size != size_of::<PluginUiV1>() {
            return INCOMPATIBLE;
        }
        if output.is_null() {
            return ERROR;
        }
        // SAFETY: Caller guarantees negotiated output storage; only C pointers copied.
        unsafe {
            output.write(PluginUiV1 {
                struct_size: size as u32,
                abi_version: VERSION,
                count,
                row,
                set,
            });
        }
        OK
    }
}

impl<P> std::fmt::Debug for Dll<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Dll").finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Context, Setting, Settings};
    #[derive(Default)]
    struct Example;
    impl Plugin for Example {
        fn id(&self) -> &'static str {
            "example"
        }
        fn settings(&self) -> &'static [Setting] {
            &[Setting {
                key: "example.amount",
                label: "Amount",
                description: "Example",
                default: Value::Number(1.),
                control: Control::Number {
                    min: 0.,
                    max: 2.,
                    step: 0.1,
                },
            }]
        }
        fn start(&mut self, _: &Context, _: &Settings) -> Result<(), crate::Error> {
            Ok(())
        }
    }
    unsafe extern "C" fn start(_: *const HostV1) -> i32 {
        OK
    }
    unsafe extern "C" fn stop() -> i32 {
        OK
    }
    #[test]
    fn negotiation_preserves_rejected_output_and_copies_complete_settings() {
        let plugin = Dll::<Example>::new();
        let mut output = PluginV1::EMPTY;
        // SAFETY: Aligned writable descriptor; rejected calls must not write.
        unsafe {
            assert_eq!(
                plugin.entry(99, &mut output, size_of::<PluginV1>(), start, stop, "1"),
                INCOMPATIBLE
            );
            assert_eq!(output.struct_size, 0);
            assert_eq!(
                plugin.entry(1, &mut output, size_of::<PluginV1>(), start, stop, "1"),
                OK
            );
        }
        assert_eq!(output.abi_version, 1);
        assert_eq!(plugin.count(), 1);
        let mut row = SettingV1::default();
        // SAFETY: Correct writable row buffer; invalid size/index must reject.
        unsafe {
            assert_eq!(plugin.row(0, &mut row, 1), ERROR);
            assert_eq!(plugin.row(2, &mut row, size_of::<SettingV1>()), ERROR);
            assert_eq!(plugin.row(0, &mut row, size_of::<SettingV1>()), OK);
            assert_eq!(plugin.set(b"example.amount".as_ptr(), 14, 2, 1.), ERROR); // not started
        }
        assert_eq!(row.kind, 2);
        assert_eq!(row.value, 1.);
        assert_eq!(row.max, 2.);
        assert_eq!(&row.key[..14], b"example.amount");
    }
    #[test]
    fn panic_and_panicking_payload_drop_cannot_cross_c_callbacks() {
        struct Payload;
        impl Drop for Payload {
            fn drop(&mut self) {
                panic!("payload destructor")
            }
        }
        assert_eq!(boundary(|| std::panic::panic_any(Payload)), ERROR);
    }
}
