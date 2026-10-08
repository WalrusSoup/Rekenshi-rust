use crate::{CALLS, STATUS, logging};
use detour::{Threads, Transaction, TypedDetour};
use std::ffi::c_void;
use std::sync::atomic::Ordering;
use std::sync::{Mutex, OnceLock};
use windows_sys::Win32::System::LibraryLoader::{
    GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS, GET_MODULE_HANDLE_EX_FLAG_PIN, GetModuleHandleExW,
};

// Verified against the installed OgreMain_x64.dll export table. A static C++
// method on Windows x64 uses the C ABI here: no arguments, opaque pointer result.
type RootGetter = extern "C" fn() -> *mut c_void;
struct RootHook {
    detour: TypedDetour<RootGetter>,
    target: RootGetter,
}
static HOOK: OnceLock<RootHook> = OnceLock::new();
static LIFECYCLE: Mutex<()> = Mutex::new(());

pub(crate) fn start() -> Result<(), String> {
    rekenshi_plugin_runtime::HookEvents::check_shutdown()?;
    let _guard = LIFECYCLE.lock().map_err(|_| "lifecycle lock poisoned")?;
    if STATUS.load(Ordering::Acquire) == 1 {
        return Ok(());
    }
    logging::write(concat!(
        "dllStartPlugin entered; version ",
        env!("CARGO_PKG_VERSION")
    ));
    if let Ok(executable) = std::env::current_exe() {
        logging::write(&format!("host executable: {}", executable.display()));
    }

    if HOOK.get().is_none() {
        let symbol =
            match crate::catalog::target(rekenshi_plugin_api::hooks::HookId::OgreRootSingleton) {
                Ok(symbol) => symbol,
                Err(rekenshi_plugin_api::hooks::UNAVAILABLE) => {
                    STATUS.store(4, Ordering::Release);
                    logging::write("Ogre Root getter unavailable; no hooks installed");
                    return Ok(());
                }
                Err(error) => return Err(format!("Ogre target resolution: {error}")),
            };
        // Pin both modules before storing a target or trampoline. Hooks and code
        // may still be on another thread's stack even after they are disabled.
        pin(symbol as *const ())?;
        pin(crate::dllStartPlugin as *const ())?;
        // SAFETY: This exact decorated export identifies the static no-argument
        // getter returning Ogre::Root*. Both ABIs coincide on Windows x64. We
        // never dereference the returned pointer. Ogre is pinned for process life.
        let target = unsafe { std::mem::transmute::<*const c_void, RootGetter>(symbol) };
        crate::extensions::reserve_core(target as *const c_void)?;
        // SAFETY: target and root_getter_hook have identical ABIs and signatures;
        // both modules remain mapped for the process lifetime.
        let hook = unsafe { TypedDetour::<RootGetter>::new(target, root_getter_hook) }
            .map_err(|error| format!("create detour: {error}"))?;
        HOOK.set(RootHook {
            detour: hook,
            target,
        })
        .map_err(|_| "hook initialized twice")?;
        logging::write(&format!(
            "resolved Ogre Root::getSingletonPtr at {:p}",
            target as *const ()
        ));
    }
    let state = HOOK.get().ok_or("hook state unavailable")?;
    let hook = &state.detour;
    let before = hook.call();
    let ui_hook = match crate::ui::prepare() {
        Ok(hook) => hook,
        Err(error) => {
            crate::ui::unavailable();
            logging::write(&format!("title label unavailable: {error}"));
            None
        }
    };
    crate::gameplay::prepare();
    if let Err(error) = crate::internal_plugins::prepare() {
        logging::write(&format!("internal plugin preparation failed: {error}"));
    }
    crate::extensions::prepare();
    struct Prepared {
        installed: bool,
    }
    impl Drop for Prepared {
        fn drop(&mut self) {
            if !self.installed {
                crate::extensions::stop();
                let _ = crate::internal_plugins::stop();
            }
        }
    }
    let mut prepared = Prepared { installed: false };
    let mut transaction = Transaction::new();
    transaction.enable(hook);
    if let Some(ui_hook) = ui_hook {
        transaction.enable(ui_hook);
    }
    crate::ui::menu::enable(&mut transaction);
    rekenshi_dark_ui::enable(&mut transaction);
    crate::gameplay::enable(&mut transaction);
    // SAFETY: Startup is called by Ogre outside DllMain. Other existing threads
    // are suspended during patching. This getter's original body is a pointer
    // load and return, with no internal calls/return addresses into its prologue.
    unsafe { transaction.commit(Threads::All) }
        .map_err(|error| format!("enable detour: {error}"))?;
    let count = CALLS.load(Ordering::Relaxed);
    // Call the patched entrance, not hook.call(), which bypasses the hook.
    let after = (state.target)();
    if before != after || CALLS.load(Ordering::Relaxed) <= count {
        let mut rollback = Transaction::new();
        rollback.disable(hook);
        if let Some(ui_hook) = ui_hook {
            rollback.disable(ui_hook);
        }
        crate::ui::menu::disable(&mut rollback);
        // SAFETY: Startup rollback runs before mounting plugins; tracked UI values
        // remain on their owning thread and native calls are quiescent.
        unsafe { rekenshi_dark_ui::disable(&mut rollback) };
        crate::gameplay::disable(&mut rollback);
        // SAFETY: Same transaction invariants as the enable above; keep the
        // trampoline alive for callbacks already in flight.
        unsafe { rollback.commit(Threads::All) }
            .map_err(|error| format!("forwarding probe failed; rollback failed: {error}"))?;
        return Err("forwarding probe failed".into());
    }
    STATUS.store(1, Ordering::Release);
    crate::gameplay::start();
    crate::callbacks::ready(true, crate::ui::menu::prepared());
    if ui_hook.is_some() {
        crate::ui::armed();
    }
    logging::write("detour active; forwarding probe passed");
    crate::catalog::warm_up();
    if let Err(error) = crate::internal_plugins::start() {
        logging::write(&format!("internal plugins unavailable: {error}"));
    }
    crate::extensions::start();
    prepared.installed = true;
    Ok(())
}

pub(crate) fn stop() -> Result<(), String> {
    rekenshi_plugin_runtime::HookEvents::check_shutdown()?;
    let _guard = LIFECYCLE.lock().map_err(|_| "lifecycle lock poisoned")?;
    crate::gameplay::stop();
    crate::callbacks::ready(false, false);
    crate::extensions::stop();
    crate::internal_plugins::stop()?;
    crate::ui::menu::stop_overlay();
    if let Some(state) = HOOK.get() {
        let mut transaction = Transaction::new();
        transaction.disable(&state.detour);
        if let Some(ui_hook) = crate::ui::hook() {
            transaction.disable(ui_hook);
        }
        crate::ui::menu::disable(&mut transaction);
        // SAFETY: Host shutdown quiesces observers and runs on the UI thread,
        // or a thread with an empty tracking map, before detaching native hooks.
        unsafe { rekenshi_dark_ui::disable(&mut transaction) };
        crate::gameplay::disable(&mut transaction);
        // SAFETY: Same patching conditions as start. The trampoline is retained
        // in HOOK and both DLLs are pinned, including for callbacks already running.
        unsafe { transaction.commit(Threads::All) }
            .map_err(|error| format!("disable detour: {error}"))?;
    }
    STATUS.store(3, Ordering::Release);
    crate::ui::stopped();
    logging::write(&format!(
        "dllStopPlugin; hook disabled; total forwarded calls: {}",
        CALLS.load(Ordering::Relaxed)
    ));
    Ok(())
}

extern "C" fn root_getter_hook() -> *mut c_void {
    CALLS.fetch_add(1, Ordering::Relaxed);
    // HOOK is published before enabling the patch and is never dropped. Forward
    // first; observers never receive or alter the native object pointer.
    let result = match HOOK.get() {
        Some(state) => state.detour.call(),
        None => std::ptr::null_mut(), // Defensive; unreachable for an enabled hook.
    };
    crate::callbacks::ogre_call();
    result
}

pub(crate) fn pin(address: *const ()) -> Result<(), String> {
    let mut module = std::ptr::null_mut();
    // SAFETY: address belongs to a mapped DLL; FROM_ADDRESS treats it as an
    // address, and module points to valid output storage. PIN is intentional.
    let success = unsafe {
        GetModuleHandleExW(
            GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_PIN,
            address.cast(),
            &mut module,
        )
    };
    if success == 0 {
        Err(format!("pin module: {}", std::io::Error::last_os_error()))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::AtomicU64;

    #[test]
    #[ignore = "requires REKENSHI_TEST_GAME with the supported Ogre/MyGUI binaries"]
    fn managed_callbacks_forward_native_ogre_and_mygui_and_stop_cleanly() {
        let game = std::path::PathBuf::from(
            std::env::var_os("REKENSHI_TEST_GAME").expect("game directory"),
        );
        rekenshi_native::load_pinned(&game.join("OgreMain_x64.dll")).unwrap();
        rekenshi_native::load_pinned(&game.join("MyGUIEngine_x64.dll")).unwrap();
        start().unwrap();
        assert_eq!(STATUS.load(Ordering::Acquire), 1);
        let address =
            crate::catalog::target(rekenshi_plugin_api::hooks::HookId::OgreRootSingleton).unwrap();
        // SAFETY: The pinned Ogre export has the inspected no-argument getter ABI;
        // the opaque returned object is compared only, never dereferenced.
        let getter = unsafe { std::mem::transmute::<*const c_void, RootGetter>(address) };
        let original = getter();
        let host = crate::extensions::internal_host("callback-test", 10001).unwrap();
        let fixture =
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/native-menu.dll");
        let fixture = rekenshi_native::load_pinned(&fixture).expect("build-menu-fixture.cmd");
        type Setup = unsafe extern "C" fn(*const std::ffi::c_char);
        type Frame = unsafe extern "C" fn();
        // SAFETY: These functions are built from our native-menu.cpp fixture with
        // these exact C signatures. Its DLL and the imported MyGUI remain pinned.
        let (setup, frame, shutdown): (Setup, Frame, Frame) = unsafe {
            (
                std::mem::transmute::<*const c_void, Setup>(
                    rekenshi_native::export(fixture, c"menu_setup").unwrap(),
                ),
                std::mem::transmute::<*const c_void, Frame>(
                    rekenshi_native::export(fixture, c"menu_frame").unwrap(),
                ),
                std::mem::transmute::<*const c_void, Frame>(
                    rekenshi_native::export(fixture, c"menu_shutdown").unwrap(),
                ),
            )
        };
        let resources =
            std::ffi::CString::new(include_str!("../tests/fixtures/menu-resources.xml")).unwrap();
        // SAFETY: Fixture copies/parses the terminated XML during this call and
        // builds a real headless MyGUI hierarchy on the current UI thread.
        unsafe { setup(resources.as_ptr()) };
        let deltas = Arc::new(Mutex::new(Vec::new()));
        let copy = deltas.clone();
        host.hook_ui_frame(move |delta| copy.lock().unwrap().push(delta))
            .unwrap();
        // SAFETY: The fixture's live Gui is initialized on this same thread.
        unsafe { frame() };
        assert_eq!(*deltas.lock().unwrap(), [0.6]);
        let count = Arc::new(AtomicU64::new(0));
        let copy = count.clone();
        let nested_getter = getter;
        host.hook_ogre_calls(move || {
            copy.fetch_add(1, Ordering::Relaxed);
            // Native recursion still forwards normally, but must not recursively
            // invoke this observer and overflow the stack.
            nested_getter();
        })
        .unwrap();
        for _ in 0..100 {
            assert_eq!(getter(), original);
        }
        assert_eq!(count.load(Ordering::Relaxed), 100);
        drop(host);
        getter();
        assert_eq!(count.load(Ordering::Relaxed), 100);
        // SAFETY: Same initialized fixture and UI thread. Dropping the host must
        // remove the observer while the native frame continues to forward.
        unsafe { frame() };
        assert_eq!(*deltas.lock().unwrap(), [0.6]);
        stop().unwrap();
        assert_eq!(getter(), original);
        start().unwrap();
        let host = crate::extensions::internal_host("callback-test", 10001).unwrap();
        let copy = count.clone();
        host.hook_ogre_calls(move || {
            copy.fetch_add(1, Ordering::Relaxed);
        })
        .unwrap();
        getter();
        assert_eq!(count.load(Ordering::Relaxed), 101);
        drop(host);
        stop().unwrap();
        // SAFETY: The fixture shuts down its initialized Gui after detours stop.
        unsafe { shutdown() };
    }
}
