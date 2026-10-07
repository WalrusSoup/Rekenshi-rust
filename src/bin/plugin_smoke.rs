//! Loads the actual DLL through its C exports, optionally against Kenshi's Ogre.
#![cfg(all(windows, target_arch = "x86_64"))]

use rekenshi_native::symbols::ogre::root;
use rekenshi_plugin_api::{
    INCOMPATIBLE, OK,
    hooks::{GetApi, HookId, HookInfoV1, HooksV1, RESERVED, UNAVAILABLE},
};
use std::ffi::c_void;
use std::path::Path;
use windows_sys::Win32::Foundation::{FreeLibrary, HMODULE};
use windows_sys::Win32::System::LibraryLoader::{
    GetProcAddress, LOAD_LIBRARY_SEARCH_DEFAULT_DIRS, LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR,
    LoadLibraryExW,
};

type Entry = unsafe extern "C" fn();
type Status = unsafe extern "C" fn() -> u32;
type Calls = unsafe extern "C" fn() -> u64;
type Getter = unsafe extern "C" fn() -> *mut c_void;
type ExportAddress = unsafe extern "system" fn() -> isize;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.is_empty() || args.len() > 4 {
        return Err("usage: plugin-smoke <RE_Kenshi_Rust.dll> [Kenshi directory] [active plugins] [failed plugins]".into());
    }
    let expected_active: u32 = args
        .get(2)
        .map(|value| value.to_string_lossy().parse())
        .transpose()?
        .unwrap_or(0);
    let expected_failed: u32 = args
        .get(3)
        .map(|value| value.to_string_lossy().parse())
        .transpose()?
        .unwrap_or(0);
    let ogre = if let Some(directory) = args.get(1) {
        Some(load(&Path::new(directory).join("OgreMain_x64.dll"))?)
    } else {
        None
    };
    let plugin = load(Path::new(&args[0]))?;
    // SAFETY: Versioned export is built with the SDK's exact C signature.
    let get_hooks: GetApi =
        unsafe { std::mem::transmute(symbol(plugin.0, b"rekenshi_hooks_api\0")?) };
    let mut hook_api = std::mem::MaybeUninit::<HooksV1>::uninit();
    // SAFETY: Unknown ABI requests must reject without dereferencing null output;
    // accepted request supplies aligned writable storage of the exact table size.
    let hook_api = unsafe {
        ensure(
            get_hooks(99, std::ptr::null_mut(), size_of::<HooksV1>()) == INCOMPATIBLE,
            "catalog ABI rejection",
        )?;
        ensure(
            get_hooks(1, hook_api.as_mut_ptr(), size_of::<HooksV1>()) == OK,
            "catalog negotiation",
        )?;
        hook_api.assume_init()
    };
    // SAFETY: These exports have the same no-argument u32 C signature as Status.
    let (active_plugins, failed_plugins): (Status, Status) = unsafe {
        (
            std::mem::transmute::<ExportAddress, Status>(symbol(
                plugin.0,
                b"rekenshi_plugins_loaded\0",
            )?),
            std::mem::transmute::<ExportAddress, Status>(symbol(
                plugin.0,
                b"rekenshi_plugins_failed\0",
            )?),
        )
    };
    let _mygui = if let Some(directory) = args.get(1) {
        Some(load(&Path::new(directory).join("MyGUIEngine_x64.dll"))?)
    } else {
        None
    };
    // SAFETY: We built these exports with precisely these C signatures, and
    // plugin keeps the loaded module alive until after the final call below.
    let (start, stop, status, calls, ui_status): (Entry, Entry, Status, Calls, Status) = unsafe {
        (
            std::mem::transmute::<ExportAddress, Entry>(symbol(plugin.0, b"dllStartPlugin\0")?),
            std::mem::transmute::<ExportAddress, Entry>(symbol(plugin.0, b"dllStopPlugin\0")?),
            std::mem::transmute::<ExportAddress, Status>(symbol(plugin.0, b"rekenshi_status\0")?),
            std::mem::transmute::<ExportAddress, Calls>(symbol(
                plugin.0,
                b"rekenshi_hook_calls\0",
            )?),
            std::mem::transmute::<ExportAddress, Status>(symbol(
                plugin.0,
                b"rekenshi_ui_status\0",
            )?),
        )
    };
    let getter: Option<Getter> = if let Some(ogre) = &ogre {
        // SAFETY: This exact Ogre static getter uses C ABI on Windows x64 and
        // returns an opaque pointer without needing a constructed Root object.
        Some(unsafe {
            std::mem::transmute::<ExportAddress, Getter>(symbol(
                ogre.0,
                root::GET_SINGLETON_PTR.to_bytes_with_nul(),
            )?)
        })
    } else {
        None
    };
    // SAFETY: Valid export signatures and all modules are still alive.
    unsafe {
        ensure(status() == 0, "initial status")?;
        let original = getter.map(|get| get());
        start();
        verify_catalog(&hook_api, getter, expected_active)?;
        if let Some(get) = getter {
            ensure(status() == 1, "hook installation (see RE_Kenshi_Rust.log)")?;
            ensure(
                active_plugins() == expected_active,
                "configured plugin activation count",
            )?;
            ensure(failed_plugins() == expected_failed, "plugin failure count")?;
            ensure(
                ui_status() == 1,
                "native MyGUI ABI probe and UI hook installation",
            )?;
            let count = calls();
            for _ in 0..100 {
                ensure(Some(get()) == original, "original getter result preserved")?;
            }
            ensure(calls() == count + 100, "100 calls traversed detour")?;
            start();
            ensure(calls() == count + 100, "startup is idempotent")?;
            ensure(
                active_plugins() == expected_active,
                "duplicate startup preserves plugins",
            )?;
            stop();
            ensure(status() == 3, "shutdown status")?;
            ensure(ui_status() == 0, "UI hook disabled at shutdown")?;
            ensure(active_plugins() == 0, "plugins stopped at shutdown")?;
            if expected_active == 4 {
                let mut info = HookInfoV1::EMPTY;
                for id in [HookId::D3DCompile, HookId::D3D11Draw] {
                    ensure(
                        (hook_api.resolve)(id as u32, &mut info, size_of::<HookInfoV1>()) == OK
                            && info.status == rekenshi_plugin_api::hooks::AVAILABLE,
                        "named targets survive stop with ownership released",
                    )?;
                }
            }
            let stopped = calls();
            ensure(Some(get()) == original, "original restored after shutdown")?;
            ensure(calls() == stopped, "disabled hook no longer intercepts")?;
            start();
            ensure(status() == 1, "restart enables retained hook")?;
            ensure(
                active_plugins() == expected_active,
                "restart enables retained plugin instances",
            )?;
            ensure(
                failed_plugins() == expected_failed,
                "restart preserves failure counts",
            )?;
            ensure(ui_status() == 1, "restart enables retained UI hook")?;
            verify_catalog(&hook_api, getter, expected_active)?;
            let restarted = calls();
            ensure(Some(get()) == original, "restart preserves result")?;
            ensure(calls() == restarted + 1, "restart intercepts")?;
            stop();
            stop();
            ensure(status() == 3, "shutdown is idempotent")?;
            println!(
                "PASS: Ogre forwarded 100 calls; native MyGUI strings and title hook passed; shutdown, restart and duplicate lifecycle calls passed"
            );
        } else {
            ensure(status() == 4, "missing Ogre is handled without patching")?;
            ensure(calls() == 0, "no calls without Ogre")?;
            stop();
            ensure(status() == 3, "shutdown without Ogre")?;
            println!("PASS: DLL exports loaded; missing Ogre handled without patching");
        }
    }
    Ok(())
}

fn verify_catalog(
    api: &HooksV1,
    getter: Option<Getter>,
    expected_active: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut info = HookInfoV1::EMPTY;
    // SAFETY: Pinned negotiated table; every accepted output points to valid
    // caller-owned storage. Invalid IDs/sizes must leave it untouched.
    unsafe {
        ensure(
            (api.resolve)(u32::MAX, &mut info, size_of::<HookInfoV1>()) == INCOMPATIBLE
                && info.struct_size == 0,
            "unknown hook ID leaves output untouched",
        )?;
        ensure(
            (api.resolve)(HookId::OgreRootSingleton as u32, &mut info, 0) == INCOMPATIBLE
                && info.struct_size == 0,
            "wrong hook output size rejected",
        )?;
        ensure(
            (api.publish)(
                999,
                HookId::D3D11Draw as u32,
                std::ptr::dangling::<c_void>(),
            ) != OK,
            "unowned dynamic target publication rejected",
        )?;
        for id in HookId::ALL {
            let result = (api.resolve)(id as u32, &mut info, size_of::<HookInfoV1>());
            if result == UNAVAILABLE {
                continue;
            }
            ensure(
                result == OK && info.id == id as u32 && !info.address.is_null(),
                "named target query",
            )?;
            let first = info.address;
            ensure(
                (api.resolve)(id as u32, &mut info, size_of::<HookInfoV1>()) == OK
                    && info.address == first,
                "catalog returns stable cached addresses",
            )?;
            if id == HookId::OgreRootSingleton {
                ensure(
                    getter.is_some_and(|g| g as *const c_void == first)
                        && info.status == RESERVED
                        && info.owner_token == 0,
                    "enum resolves actual core-owned Ogre target",
                )?;
            }
        }
        if getter.is_some() {
            ensure(
                (api.resolve)(
                    HookId::MyGuiSetCaption as u32,
                    &mut info,
                    size_of::<HookInfoV1>(),
                ) == OK
                    && info.status == RESERVED
                    && info.owner_token == 0,
                "catalog resolves verified caption implementation",
            )?;
        }
        if expected_active == 4 {
            for id in [HookId::D3DCompile, HookId::D3DCreateBlob, HookId::D3D11Draw] {
                ensure(
                    (api.resolve)(id as u32, &mut info, size_of::<HookInfoV1>()) == OK,
                    "enhancement targets auto-resolve during startup",
                )?;
                if id != HookId::D3DCreateBlob {
                    ensure(
                        info.status == RESERVED && info.owner_token != 0,
                        "enhancement hook ownership visible by name",
                    )?;
                }
            }
        }
    }
    Ok(())
}

fn ensure(condition: bool, description: &str) -> Result<(), Box<dyn std::error::Error>> {
    if condition {
        Ok(())
    } else {
        Err(format!("FAIL: {description}").into())
    }
}

struct Module(HMODULE);

impl Drop for Module {
    fn drop(&mut self) {
        // SAFETY: This reference was acquired by LoadLibraryExW and is released
        // once. The plugin pins both modules when hooking; OS exit frees them.
        unsafe { FreeLibrary(self.0) };
    }
}

fn load(path: &Path) -> Result<Module, Box<dyn std::error::Error>> {
    use std::os::windows::ffi::OsStrExt;
    let absolute = path.canonicalize()?;
    let wide: Vec<u16> = absolute.as_os_str().encode_wide().chain(Some(0)).collect();
    // SAFETY: Absolute, terminated path; dependencies are searched in this DLL's
    // directory and standard Windows locations. No loader callback work here.
    let module = unsafe {
        LoadLibraryExW(
            wide.as_ptr(),
            std::ptr::null_mut(),
            LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_DEFAULT_DIRS,
        )
    };
    if module.is_null() {
        Err(format!(
            "load {}: {}",
            absolute.display(),
            std::io::Error::last_os_error()
        )
        .into())
    } else {
        Ok(Module(module))
    }
}

fn symbol(module: HMODULE, name: &[u8]) -> Result<ExportAddress, Box<dyn std::error::Error>> {
    // SAFETY: module belongs to a live Module; all callers supply terminated
    // literal export names. The returned address is used while Module is alive.
    unsafe { GetProcAddress(module, name.as_ptr()) }
        .ok_or_else(|| format!("missing export: {}", String::from_utf8_lossy(name)).into())
}
