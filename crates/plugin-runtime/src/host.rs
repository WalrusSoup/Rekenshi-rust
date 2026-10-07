//! Managed callbacks for plugins running in the host or a plugin DLL.
#![forbid(unsafe_code)]

use crate::Error;
use rekenshi_plugin_api::HostV1;
use std::cell::Cell;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, RwLock, Weak};

/// Typed events supplied by the host's inspected native adapters.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hook {
    /// After Ogre's root getter returns; this is not a game update event.
    OgreCalls,
    /// After MyGUI's frame event returns, on its UI thread.
    UiFrame,
}

type Connector = dyn Fn(Hook) -> Result<(), Error> + Send + Sync;

/// The plugin-facing host. The runtime owns all registered callbacks.
///
/// Register once in `Plugin::start`; callbacks are removed and in-flight calls
/// finish before `Plugin::stop`. Captures must be owned (`'static`). Callbacks
/// may run concurrently, so shared mutable state should use atomics or locks.
///
/// ```
/// use rekenshi_plugin_runtime::{Context, Error};
/// use std::sync::{Arc, atomic::{AtomicU64, Ordering}};
///
/// fn register(context: &Context) -> Result<(), Error> {
///     let calls = Arc::new(AtomicU64::new(0));
///     context.host.hook_ogre_calls(move || {
///         calls.fetch_add(1, Ordering::Relaxed);
///     })?;
///     context.host.hook_ui_frame(move |delta_seconds| {
///         // Update your plugin using this frame's elapsed time.
///         let _ = delta_seconds;
///     })?;
///     Ok(())
/// }
/// ```
pub struct Host {
    native: HostV1,
    events: HookEvents,
    owned: Mutex<Owned>,
    connector: Option<Arc<Connector>>,
    group: Arc<Group>,
}

struct Group {
    faulted: AtomicBool,
    restart: Arc<AtomicBool>,
}

#[derive(Default)]
struct Owned {
    closed: bool,
    callbacks: Vec<Arc<dyn Managed>>,
}

impl Host {
    /// Constructed by the host, not by plugin implementations.
    pub fn new(native: HostV1, events: HookEvents) -> Self {
        let group = Arc::new(Group {
            faulted: AtomicBool::new(false),
            restart: events.restart.clone(),
        });
        Self {
            native,
            events,
            owned: Mutex::default(),
            connector: None,
            group,
        }
    }
    /// SDK transport connects events lazily; captures remain owned by this Host.
    pub fn with_connector(
        mut self,
        connector: impl Fn(Hook) -> Result<(), Error> + Send + Sync + 'static,
    ) -> Self {
        self.connector = Some(Arc::new(connector));
        self
    }
    /// Detach callbacks and wait for calls already in flight.
    pub fn close(&self) -> Result<(), Error> {
        self.shutdown()
    }

    /// Send a synchronous diagnostic through the trusted host logger.
    pub fn log(&self, message: &str) {
        self.native.write_log(message);
    }

    /// An owned logging handle that may be captured by a callback.
    pub fn logger(&self) -> Logger {
        Logger(self.native)
    }

    /// Report whether the host supports the requested event or command.
    pub fn supports(&self, hook: Hook) -> bool {
        if self.faulted() {
            return false;
        }
        match hook {
            Hook::OgreCalls => self.events.ogre.available.load(Ordering::Acquire),
            Hook::UiFrame => self.events.frame.available.load(Ordering::Acquire),
        }
    }

    /// Observe Ogre getter calls after the original runs. Its return is preserved.
    /// Multiple plugins may subscribe without competing for the native detour.
    pub fn hook_ogre_calls(
        &self,
        callback: impl Fn() + Send + Sync + 'static,
    ) -> Result<(), Error> {
        self.register(
            &self.events.ogre,
            move |()| callback(),
            "Ogre calls",
            Hook::OgreCalls,
        )
    }

    /// Observe every MyGUI frame after the original runs, receiving elapsed seconds.
    /// This is a UI frame event, not a simulation tick or a graphics draw callback.
    pub fn hook_ui_frame(
        &self,
        callback: impl Fn(f32) + Send + Sync + 'static,
    ) -> Result<(), Error> {
        self.register(&self.events.frame, callback, "MyGUI frame", Hook::UiFrame)
    }

    fn register<T: Copy + Send + Sync + 'static>(
        &self,
        event: &Arc<Event<T>>,
        callback: impl Fn(T) + Send + Sync + 'static,
        name: &str,
        hook: Hook,
    ) -> Result<(), Error> {
        let mut owned = self.owned.lock().unwrap_or_else(|e| e.into_inner());
        if owned.closed || self.faulted() {
            return Err("plugin host is stopped; callbacks cannot be registered".into());
        }
        if !event.available.load(Ordering::Acquire) {
            return Err(format!("{name} hook unavailable on this host").into());
        }
        if let Some(connect) = &self.connector {
            connect(hook)?;
        }
        owned
            .callbacks
            .push(event.subscribe(callback, self.group.clone()));
        Ok(())
    }

    /// Report whether a panic quarantined this host's entire callback group.
    ///
    /// Further dispatch, registration, and runtime updates are rejected. Calls
    /// already in flight must finish before resources can be released.
    pub fn faulted(&self) -> bool {
        self.group.faulted.load(Ordering::Acquire)
    }

    /// Low-level compatibility interface for inspected native adapters.
    /// Ordinary plugin callbacks do not need this table.
    pub fn native(&self) -> HostV1 {
        self.native
    }

    pub(crate) fn shutdown(&self) -> Result<(), Error> {
        Self::check_shutdown()?;
        let callbacks = self.detach();
        for callback in callbacks {
            callback.wait();
        }
        Ok(())
    }

    pub(crate) fn check_shutdown() -> Result<(), Error> {
        HookEvents::check_shutdown()
    }

    pub(crate) fn release_callbacks(&self) {
        let callbacks = self.detach();
        // A host dropped from a callback cannot wait for its own stack. Running
        // callbacks own their captures and their group is still fully detached.
        if Self::check_shutdown().is_ok() {
            for callback in callbacks {
                callback.wait();
            }
        }
    }

    fn detach(&self) -> Vec<Arc<dyn Managed>> {
        let callbacks = {
            let mut owned = self.owned.lock().unwrap_or_else(|e| e.into_inner());
            owned.closed = true;
            std::mem::take(&mut owned.callbacks)
        };
        // Disable the entire group before waiting for any one callback.
        for callback in &callbacks {
            callback.disable();
        }
        for callback in &callbacks {
            callback.detach();
        }
        callbacks
    }
}

impl HookEvents {
    /// Hosts must check before acquiring lifecycle locks to avoid a callback
    /// synchronously waiting for its own stack during shutdown.
    pub fn check_shutdown() -> Result<(), Error> {
        if DISPATCHING.with(|state| state.get() != 0) {
            Err("stop plugins outside a hook callback to allow in-flight calls to finish".into())
        } else {
            Ok(())
        }
    }
}

impl Drop for Host {
    fn drop(&mut self) {
        self.release_callbacks();
    }
}

/// Compatibility for hosts that supply only native adapters, without event dispatch.
impl From<HostV1> for Host {
    fn from(native: HostV1) -> Self {
        Self::new(native, HookEvents::default())
    }
}

/// Copyable logging handle; it owns no subscriptions or native game objects.
#[derive(Clone, Copy, Debug)]
pub struct Logger(HostV1);
impl Logger {
    /// Send a synchronous diagnostic through the trusted host logger.
    pub fn log(&self, message: &str) {
        self.0.write_log(message);
    }
}

/// Host-side dispatch shared by all internal plugins, supplied through `Host`.
/// Native adapters retain ownership of their detours and original trampolines.
#[derive(Clone)]
pub struct HookEvents {
    ogre: Arc<Event<()>>,
    frame: Arc<Event<f32>>,
    restart: Arc<AtomicBool>,
}

impl Default for HookEvents {
    fn default() -> Self {
        Self {
            ogre: Arc::new(Event::new(1)),
            frame: Arc::new(Event::new(2)),
            restart: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl HookEvents {
    /// Report a persistent request to restart after any callback group panicked.
    ///
    /// Other owners may finish work, but the faulted owner cannot be resumed.
    pub fn restart_required(&self) -> bool {
        self.restart.load(Ordering::Acquire)
    }

    /// Enable a slot only after its native adapter successfully enables the hook.
    pub fn set_available(&self, hook: Hook, available: bool) {
        match hook {
            Hook::OgreCalls => self.ogre.available.store(available, Ordering::Release),
            Hook::UiFrame => self.frame.available.store(available, Ordering::Release),
        }
    }

    /// Dispatch after forwarding the original. Returns newly panicked callbacks.
    pub fn dispatch_ogre_calls(&self) -> usize {
        self.ogre.dispatch(())
    }

    /// Dispatch after forwarding the original MyGUI frame.
    pub fn dispatch_ui_frame(&self, delta_seconds: f32) -> usize {
        self.frame.dispatch(delta_seconds)
    }
}

thread_local! {
    // Suppress recursively emitted events of the same kind, while allowing
    // different event kinds to compose. Cleared even when a callback panics.
    static DISPATCHING: Cell<u8> = const { Cell::new(0) };
}

struct DispatchGuard(u8);
impl Drop for DispatchGuard {
    fn drop(&mut self) {
        DISPATCHING.with(|state| state.set(self.0));
    }
}

struct Event<T> {
    bit: u8,
    available: AtomicBool,
    populated: AtomicBool,
    callbacks: RwLock<Arc<[Arc<Callback<T>>]>>,
}

impl<T: Copy + Send + Sync + 'static> Event<T> {
    fn new(bit: u8) -> Self {
        Self {
            bit,
            available: AtomicBool::new(false),
            populated: AtomicBool::new(false),
            callbacks: RwLock::new(Arc::from([])),
        }
    }

    fn subscribe(
        self: &Arc<Self>,
        callback: impl Fn(T) + Send + Sync + 'static,
        group: Arc<Group>,
    ) -> Arc<Callback<T>> {
        let callback = Arc::new(Callback {
            state: Mutex::new(CallbackState {
                function: Some(Arc::new(callback)),
                running: 0,
            }),
            finished: Condvar::new(),
            source: Arc::downgrade(self),
            group,
        });
        let mut current = self.callbacks.write().unwrap_or_else(|e| e.into_inner());
        let mut next = current.to_vec();
        next.push(callback.clone());
        *current = next.into();
        self.populated.store(true, Ordering::Release);
        callback
    }

    fn dispatch(&self, argument: T) -> usize {
        // The common case with no plugin subscribers takes no locks/allocations.
        if !self.populated.load(Ordering::Acquire) || !self.available.load(Ordering::Acquire) {
            return 0;
        }
        let previous = DISPATCHING.with(|state| {
            let previous = state.get();
            if previous & self.bit == 0 {
                state.set(previous | self.bit);
            }
            previous
        });
        if previous & self.bit != 0 {
            return 0;
        }
        let _guard = DispatchGuard(previous);
        let callbacks = self
            .callbacks
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        // No registry lock is held while running user code. The snapshot requires
        // one Arc clone, with no per-call Vec allocation.
        callbacks
            .iter()
            .map(|callback| usize::from(callback.invoke(argument)))
            .sum()
    }
}

trait Managed: Send + Sync {
    fn disable(&self);
    fn detach(&self);
    fn wait(&self);
}

struct CallbackState<T> {
    function: Option<Arc<dyn Fn(T) + Send + Sync>>,
    running: usize,
}

struct Callback<T> {
    state: Mutex<CallbackState<T>>,
    finished: Condvar,
    source: Weak<Event<T>>,
    group: Arc<Group>,
}

fn panicked(action: impl FnOnce()) -> bool {
    match catch_unwind(AssertUnwindSafe(action)) {
        Ok(()) => false,
        Err(payload) => {
            // User-defined panic payloads can also panic on Drop.
            std::mem::forget(payload);
            true
        }
    }
}

impl<T: Copy + Send + Sync + 'static> Callback<T> {
    fn invoke(&self, argument: T) -> bool {
        let callback = {
            let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
            if self.group.faulted.load(Ordering::Acquire) {
                return false;
            }
            let Some(callback) = state.function.clone() else {
                return false;
            };
            state.running += 1;
            callback
        };
        let failed = panicked(|| callback(argument));
        if failed {
            self.group.faulted.store(true, Ordering::Release);
            self.group.restart.store(true, Ordering::Release);
            self.disable();
        }
        // Cleanup may have released the stored Arc concurrently. Contain a
        // captured value's destructor too, before marking this call finished.
        let drop_failed = panicked(|| drop(callback));
        if drop_failed {
            self.group.faulted.store(true, Ordering::Release);
            self.group.restart.store(true, Ordering::Release);
            self.disable();
        }
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        state.running -= 1;
        if state.running == 0 {
            self.finished.notify_all();
        }
        failed || drop_failed
    }
}

impl<T: Copy + Send + Sync + 'static> Managed for Callback<T> {
    fn disable(&self) {
        let callback = self
            .state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .function
            .take();
        if panicked(|| drop(callback)) {
            self.group.faulted.store(true, Ordering::Release);
            self.group.restart.store(true, Ordering::Release);
        }
    }

    fn detach(&self) {
        let Some(source) = self.source.upgrade() else {
            return;
        };
        let mut current = source.callbacks.write().unwrap_or_else(|e| e.into_inner());
        let next: Vec<_> = current
            .iter()
            .filter(|entry| !std::ptr::eq(entry.as_ref(), self))
            .cloned()
            .collect();
        source.populated.store(!next.is_empty(), Ordering::Release);
        *current = next.into();
    }

    fn wait(&self) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        while state.running != 0 {
            state = self.finished.wait(state).unwrap_or_else(|e| e.into_inner());
        }
    }
}

impl std::fmt::Debug for Host {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Host").finish_non_exhaustive()
    }
}

impl std::fmt::Debug for HookEvents {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HookEvents")
            .field("restart_required", &self.restart_required())
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicU64;
    use std::sync::mpsc;
    use std::time::Duration;

    pub(super) fn native() -> HostV1 {
        HostV1::stub(1)
    }

    fn available() -> HookEvents {
        let events = HookEvents::default();
        events.set_available(Hook::OgreCalls, true);
        events.set_available(Hook::UiFrame, true);
        events
    }

    #[test]
    fn multiple_plugins_share_callbacks_and_stop_independently() {
        let events = available();
        let first = Host::new(native(), events.clone());
        let second = Host::new(native(), events.clone());
        let order = Arc::new(Mutex::new(Vec::new()));
        let copy = order.clone();
        first
            .hook_ogre_calls(move || copy.lock().unwrap().push(1))
            .unwrap();
        let copy = order.clone();
        second
            .hook_ogre_calls(move || copy.lock().unwrap().push(2))
            .unwrap();
        let copy = order.clone();
        second
            .hook_ui_frame(move |delta| copy.lock().unwrap().push((delta * 100.) as i32))
            .unwrap();
        assert_eq!(events.dispatch_ogre_calls(), 0);
        events.dispatch_ui_frame(0.25);
        assert_eq!(*order.lock().unwrap(), [1, 2, 25]);
        first.shutdown().unwrap();
        events.dispatch_ogre_calls();
        assert_eq!(*order.lock().unwrap(), [1, 2, 25, 2]);
        assert!(first.hook_ogre_calls(|| {}).is_err());
        drop(second);
        events.dispatch_ogre_calls();
        assert_eq!(*order.lock().unwrap(), [1, 2, 25, 2]);
        assert!(!events.ogre.populated.load(Ordering::Relaxed));
        assert!(!events.frame.populated.load(Ordering::Relaxed));
    }

    #[test]
    fn unavailable_hooks_fail_and_do_not_retain_captures() {
        let events = HookEvents::default();
        let host = Host::new(native(), events.clone());
        let value = Arc::new(AtomicU64::new(0));
        let weak = Arc::downgrade(&value);
        assert!(!host.supports(Hook::OgreCalls));
        assert!(
            host.hook_ogre_calls(move || {
                value.fetch_add(1, Ordering::Relaxed);
            })
            .is_err()
        );
        assert!(weak.upgrade().is_none());
        events.set_available(Hook::OgreCalls, true);
        assert!(host.supports(Hook::OgreCalls));
        host.hook_ogre_calls(|| {}).unwrap();
    }

    #[test]
    fn panic_quarantines_owner_across_events_and_requests_restart() {
        let events = available();
        let host = Host::new(native(), events.clone());
        let independent = Host::new(native(), events.clone());
        let count = Arc::new(AtomicU64::new(0));
        host.hook_ogre_calls(|| panic!("bad plugin callback"))
            .unwrap();
        let copy = count.clone();
        host.hook_ogre_calls(move || {
            copy.fetch_add(1, Ordering::Relaxed);
        })
        .unwrap();
        let copy = count.clone();
        host.hook_ui_frame(move |_| {
            copy.fetch_add(100, Ordering::Relaxed);
        })
        .unwrap();
        let copy = count.clone();
        independent
            .hook_ogre_calls(move || {
                copy.fetch_add(10, Ordering::Relaxed);
            })
            .unwrap();
        assert_eq!(events.dispatch_ogre_calls(), 1);
        assert_eq!(events.dispatch_ogre_calls(), 0);
        events.dispatch_ui_frame(1.);
        assert_eq!(count.load(Ordering::Relaxed), 20);
        assert!(host.faulted());
        assert!(!host.supports(Hook::UiFrame));
        assert!(host.hook_ogre_calls(|| {}).is_err());
        assert!(events.restart_required());
        assert!(!independent.faulted());
        host.shutdown().unwrap();
    }

    #[test]
    fn recursive_emissions_are_suppressed_and_lifecycle_changes_fail_promptly() {
        let events = available();
        let host = Arc::new(Host::new(native(), events.clone()));
        let weak = Arc::downgrade(&host);
        let nested = events.clone();
        let count = Arc::new(AtomicU64::new(0));
        let copy = count.clone();
        host.hook_ogre_calls(move || {
            copy.fetch_add(1, Ordering::Relaxed);
            assert!(weak.upgrade().unwrap().shutdown().is_err());
            assert_eq!(nested.dispatch_ogre_calls(), 0);
            nested.dispatch_ui_frame(0.5);
        })
        .unwrap();
        let copy = count.clone();
        host.hook_ui_frame(move |_| {
            copy.fetch_add(10, Ordering::Relaxed);
        })
        .unwrap();
        events.dispatch_ogre_calls();
        assert_eq!(count.load(Ordering::Relaxed), 11);
        host.shutdown().unwrap();
        assert_eq!(Arc::strong_count(&host), 1);
    }

    #[test]
    fn shutdown_waits_for_in_flight_callbacks_and_releases_their_captures() {
        let events = available();
        let host = Arc::new(Host::new(native(), events.clone()));
        let (started, begun) = mpsc::channel();
        let (resume, paused) = mpsc::channel();
        let paused = Mutex::new(paused);
        let capture = Arc::new(());
        let weak = Arc::downgrade(&capture);
        host.hook_ogre_calls(move || {
            let _keep = &capture;
            started.send(()).unwrap();
            paused
                .lock()
                .unwrap()
                .recv_timeout(Duration::from_secs(5))
                .unwrap();
        })
        .unwrap();
        let dispatch = std::thread::spawn(move || events.dispatch_ogre_calls());
        begun.recv_timeout(Duration::from_secs(5)).unwrap();
        let (stopped, finished) = mpsc::channel();
        let closing = host.clone();
        let shutdown = std::thread::spawn(move || {
            closing.shutdown().unwrap();
            stopped.send(()).unwrap();
        });
        assert!(finished.recv_timeout(Duration::from_millis(30)).is_err());
        assert!(weak.upgrade().is_some());
        resume.send(()).unwrap();
        finished.recv_timeout(Duration::from_secs(5)).unwrap();
        dispatch.join().unwrap();
        shutdown.join().unwrap();
        assert!(weak.upgrade().is_none());
    }

    #[test]
    fn panic_in_capture_destructor_cannot_escape_dispatch_or_cleanup() {
        struct BadDrop;
        impl Drop for BadDrop {
            fn drop(&mut self) {
                panic!("drop failed");
            }
        }
        let events = available();
        let host = Host::new(native(), events.clone());
        let capture = BadDrop;
        host.hook_ogre_calls(move || {
            let _keep = &capture;
            panic!("call failed");
        })
        .unwrap();
        assert_eq!(events.dispatch_ogre_calls(), 1);
        assert_eq!(events.dispatch_ogre_calls(), 0);
        host.shutdown().unwrap();
        let host = Host::new(native(), events);
        let capture = BadDrop;
        host.hook_ui_frame(move |_| {
            let _keep = &capture;
        })
        .unwrap();
        host.shutdown().unwrap();
    }
}
