//! Print the named catalog using real DLLs, without launching a game world.
use rekenshi_plugin_api::{
    OK,
    hooks::{AVAILABLE, GetApi, HookId, HookInfoV1, HooksV1, UNAVAILABLE},
};
use std::path::Path;
use windows_sys::Win32::System::LibraryLoader::{
    GetProcAddress, LOAD_LIBRARY_SEARCH_DEFAULT_DIRS, LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR,
    LoadLibraryExW,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 2 {
        return Err("usage: hook-list <RE_Kenshi_Rust.dll> <Kenshi directory>".into());
    }
    let game = Path::new(&args[1]);
    let _ogre = load(&game.join("OgreMain_x64.dll"))?;
    let _gui = load(&game.join("MyGUIEngine_x64.dll"))?;
    let _compiler = load(&game.join("D3DCompiler_43.dll"))?;
    let host = load(Path::new(&args[0]))?;
    // SAFETY: Our built host exports these exact signatures; Module guards stay
    // alive until after the final shutdown call, and the host pins invoked code.
    let (start, stop, get): (extern "C" fn(), extern "C" fn(), GetApi) =
        unsafe {
            (
                std::mem::transmute::<unsafe extern "system" fn() -> isize, extern "C" fn()>(
                    symbol(&host, c"dllStartPlugin")?,
                ),
                std::mem::transmute::<unsafe extern "system" fn() -> isize, extern "C" fn()>(
                    symbol(&host, c"dllStopPlugin")?,
                ),
                std::mem::transmute::<unsafe extern "system" fn() -> isize, GetApi>(symbol(
                    &host,
                    c"rekenshi_hooks_api",
                )?),
            )
        };
    start();
    struct Shutdown(extern "C" fn());
    impl Drop for Shutdown {
        fn drop(&mut self) {
            (self.0)();
        }
    }
    let _shutdown = Shutdown(stop);
    let mut api = std::mem::MaybeUninit::<HooksV1>::uninit();
    // SAFETY: Aligned exact-size writable negotiated output.
    if unsafe { get(1, api.as_mut_ptr(), size_of::<HooksV1>()) } != OK {
        return Err("catalog negotiation failed".into());
    }
    // SAFETY: Successful negotiation initialized the complete table.
    let api = unsafe { api.assume_init() };
    for id in HookId::ALL {
        let mut info = HookInfoV1::EMPTY;
        // SAFETY: Negotiated callback and known ID; writable exact-size output.
        match unsafe { (api.resolve)(id as u32, &mut info, size_of::<HookInfoV1>()) } {
            OK => {
                let owner = if info.status == AVAILABLE {
                    "unreserved".to_owned()
                } else if info.owner_token == 0 {
                    "reserved by core".to_owned()
                } else {
                    format!("reserved by plugin {}", info.owner_token)
                };
                println!("{id:?}: {:p} ({owner})", info.address);
            }
            UNAVAILABLE => println!("{id:?}: unavailable (module or provider missing/unsupported)"),
            error => return Err(format!("{}: catalog error {error}", id.name()).into()),
        }
    }
    Ok(())
}
struct Module(windows_sys::Win32::Foundation::HMODULE);
impl Drop for Module {
    fn drop(&mut self) {
        // SAFETY: Balance our one loader reference; pinned modules remain mapped.
        unsafe { windows_sys::Win32::Foundation::FreeLibrary(self.0) };
    }
}
fn load(path: &Path) -> Result<Module, Box<dyn std::error::Error>> {
    use std::os::windows::ffi::OsStrExt;
    let path = path.canonicalize()?;
    let wide: Vec<_> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    // SAFETY: Absolute terminated path with restricted dependency search.
    let module = unsafe {
        LoadLibraryExW(
            wide.as_ptr(),
            std::ptr::null_mut(),
            LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_DEFAULT_DIRS,
        )
    };
    if module.is_null() {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(Module(module))
}
fn symbol(
    module: &Module,
    name: &std::ffi::CStr,
) -> Result<unsafe extern "system" fn() -> isize, Box<dyn std::error::Error>> {
    // SAFETY: Loaded module and terminated export name.
    unsafe { GetProcAddress(module.0, name.as_ptr().cast()) }
        .ok_or_else(|| format!("missing {name:?}").into())
}
