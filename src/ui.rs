//! Narrow bindings to the installed legacy MyGUI binary. No Rust-owned C++
//! strings cross the boundary: native constructors/destructors own their data.
use crate::{logging, plugin};
use detour::TypedDetour;
use rekenshi_native::symbols::mygui::{text_box, ustring, widget};
use sha2::{Digest, Sha256};
use std::ffi::{CStr, c_char, c_void};
use std::mem::MaybeUninit;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU32, Ordering};
use windows_sys::Win32::Foundation::HMODULE;
use windows_sys::Win32::System::LibraryLoader::{
    GetModuleFileNameW, GetModuleHandleW, GetProcAddress,
};
pub(crate) mod menu;
mod tab_data;
mod theme;
mod visibility;

const MYGUI_SHA256: &str = "fea6f1f935703812485bd6606ff4d3ceea172ece131ed4c6dcbf7a7e25b083a8";
const BADGE: &str = concat!(
    "  |  #77DD99RE_Kenshi Rust ",
    env!("CARGO_PKG_VERSION"),
    " loaded"
);
const MARKER: &[u8] = b"RE_Kenshi Rust ";

// The fingerprinted VC++ binary uses 64 bytes, alignment 8: string state and
// conversion cache, whose last pointer occupies offsets 0x38..0x40. Storage
// stays at a stable address and is never interpreted as a Rust/C++ string.
#[repr(C, align(8))]
struct UStringStorage([MaybeUninit<u8>; 64]);

pub(crate) type Caption = unsafe extern "C" fn(*mut c_void, *const c_void);
type Construct = unsafe extern "C" fn(*mut c_void, *const c_char) -> *mut c_void;
type CopyConstruct = unsafe extern "C" fn(*mut c_void, *const c_void) -> *mut c_void;
type Destroy = unsafe extern "C" fn(*mut c_void);
type Append = unsafe extern "C" fn(*mut c_void, *const c_char, usize) -> *mut c_void;
type Utf8 = unsafe extern "C" fn(*const c_void) -> *const c_char;
type Name = unsafe extern "C" fn(*const c_void) -> *const c_void;
type ExportAddress = unsafe extern "system" fn() -> isize;

struct Api {
    construct: Construct,
    copy: CopyConstruct,
    from_std_string: CopyConstruct,
    destroy: Destroy,
    append: Append,
    utf8: Utf8,
    name: Name,
    std_string: Name,
}

struct UiHook {
    api: Api,
    detour: TypedDetour<Caption>,
}
static UI: OnceLock<UiHook> = OnceLock::new();
static STATUS: AtomicU32 = AtomicU32::new(0);

pub(crate) fn status() -> u32 {
    STATUS.load(Ordering::Acquire)
}
pub(crate) fn unavailable() {
    STATUS.store(3, Ordering::Release);
}
pub(crate) fn armed() {
    STATUS.store(1, Ordering::Release);
    logging::write("MyGUI ABI probe passed; title caption hook armed");
}
pub(crate) fn stopped() {
    STATUS.store(0, Ordering::Release);
}
pub(crate) fn hook() -> Option<&'static TypedDetour<Caption>> {
    UI.get().map(|state| &state.detour)
}

// Called under the plugin lifecycle lock, before the hook is enabled.
pub(crate) fn prepare() -> Result<Option<&'static TypedDetour<Caption>>, String> {
    if let Some(hook) = hook() {
        return Ok(Some(hook));
    }
    let name: Vec<u16> = "MyGUIEngine_x64.dll"
        .encode_utf16()
        .chain(Some(0))
        .collect();
    // SAFETY: name is terminated and remains alive during the Win32 call.
    let module = unsafe { GetModuleHandleW(name.as_ptr()) };
    if module.is_null() {
        unavailable();
        logging::write("MyGUI unavailable; title label hook skipped");
        return Ok(None);
    }
    verify_binary(module)?;
    let caption = symbol(module, text_box::SET_CAPTION)?;
    plugin::pin(caption as *const ())?;
    // SAFETY: The hash-gated binary has these exact decorated C++ signatures.
    // Windows x64 member calls use RCX=this, RDX=argument, matching these pointer
    // C signatures. Constructors return this; all owning work uses native CRT.
    let api = unsafe {
        Api {
            construct: std::mem::transmute::<ExportAddress, Construct>(symbol(
                module,
                ustring::CONSTRUCT_FROM_C_STR,
            )?),
            copy: std::mem::transmute::<ExportAddress, CopyConstruct>(symbol(
                module,
                ustring::COPY_CONSTRUCT,
            )?),
            from_std_string: std::mem::transmute::<ExportAddress, CopyConstruct>(symbol(
                module,
                ustring::CONSTRUCT_FROM_STD_STRING,
            )?),
            destroy: std::mem::transmute::<ExportAddress, Destroy>(symbol(
                module,
                ustring::DESTROY,
            )?),
            append: std::mem::transmute::<ExportAddress, Append>(symbol(module, ustring::APPEND)?),
            utf8: std::mem::transmute::<ExportAddress, Utf8>(symbol(
                module,
                ustring::AS_UTF8_C_STR,
            )?),
            name: std::mem::transmute::<ExportAddress, Name>(symbol(module, widget::GET_NAME)?),
            std_string: std::mem::transmute::<ExportAddress, Name>(symbol(
                module,
                ustring::AS_UTF8,
            )?),
        }
    };
    api.probe()?;
    // Exports are linker E9 thunks; vtables use the implementation address.
    // Hook the verified body so both virtual calls and export calls reach us.
    // SAFETY: caption belongs to the pinned, fingerprinted binary. This export
    // has RVA 0x3bd4 and its relative jump points to body RVA 0x1a94b0.
    let target = unsafe {
        let entry = caption as *const u8;
        let displacement = entry.add(1).cast::<i32>().read_unaligned();
        let body = entry.wrapping_offset(5 + displacement as isize);
        if entry as usize != module as usize + 0x3bd4
            || entry.read() != 0xe9
            || body as usize != module as usize + 0x1a94b0
            || std::slice::from_raw_parts(body, 10)
                != [0x48, 0x89, 0x5c, 0x24, 0x08, 0x57, 0x48, 0x83, 0xec, 0x20]
        {
            return Err("MyGUI caption entry differs from verified implementation".into());
        }
        std::mem::transmute::<*const u8, Caption>(body)
    };
    crate::extensions::reserve_core(target as *const c_void)?;
    crate::catalog::publish_core(
        rekenshi_plugin_api::hooks::HookId::MyGuiSetCaption,
        target as *const c_void,
    )?;
    // SAFETY: Matching Windows x64 ABI; both hook and native target remain mapped
    // for process life. First patch instructions are register saves, no calls.
    let detour = unsafe {
        TypedDetour::<Caption>::new(
            target,
            caption_hook as extern "C" fn(*mut c_void, *const c_void),
        )
    }
    .map_err(|error| format!("create title hook: {error}"))?;
    UI.set(UiHook { api, detour })
        .map_err(|_| "UI initialized twice")?;
    if let Err(error) = menu::prepare(module) {
        logging::write(&format!("settings menu unavailable: {error}"));
    }
    let dark_ui = rekenshi_dark_ui::Adapter {
        log: logging::write,
        reserve: reserve_dark_target,
        text: copy_my_gui_text,
        ready: menu::prepared,
    };
    // SAFETY: verify_binary hash-gates MyGUI, plugin::pin keeps it mapped,
    // and api.probe verified native string copying above. The host callbacks
    // remain in the pinned host DLL and use its ownership registry/UI lifecycle.
    if let Err(error) = unsafe { rekenshi_dark_ui::prepare(module, dark_ui) } {
        logging::write(&format!("dark game UI unavailable: {error}"));
    }
    Ok(hook())
}

fn reserve_dark_target(target: *const c_void) -> Result<(), rekenshi_plugin_runtime::Error> {
    crate::extensions::reserve_core(target).map_err(Into::into)
}

/// # Safety
/// source must borrow a live std::string from the verified MyGUI module during this call.
unsafe fn copy_my_gui_text(source: *const c_void) -> Option<String> {
    if source.is_null() {
        return None;
    }
    let ui = UI.get()?;
    // SAFETY: The adapter supplies a live native string; this verified constructor
    // copies it using MyGUI's own CRT and the wrapper releases the copy locally.
    let text = unsafe { NativeString::from_native(&ui.api, source, ui.api.from_std_string) };
    Some(String::from_utf8_lossy(text.bytes()).into_owned())
}

extern "C" fn caption_hook(widget: *mut c_void, caption: *const c_void) {
    let Some(state) = UI.get() else { return };
    // SAFETY: The native caller supplies a live TextBox (Widget base at offset
    // zero) and UString, valid for this synchronous call on its own UI thread.
    // No widget pointer is retained. Native strings outlive the original call.
    let modified = std::panic::catch_unwind(|| unsafe {
        let name = NativeString::from_native(
            &state.api,
            (state.api.name)(widget),
            state.api.from_std_string,
        );
        if !version_widget(name.bytes()) {
            return None;
        }
        let mut text = NativeString::from_native(&state.api, caption, state.api.copy);
        if !text
            .bytes()
            .windows(MARKER.len())
            .any(|part| part == MARKER)
        {
            let count = crate::extensions::active_count() as usize
                + crate::internal_plugins::active_count();
            let noun = if count == 1 { "plugin" } else { "plugins" };
            text.append(&format!("{BADGE} | {count} {noun}"));
        }
        Some(text)
    });
    match modified {
        Ok(Some(text)) => {
            // SAFETY: Live native widget and constructed native caption; the
            // original synchronously copies the value before text is destroyed.
            unsafe { state.detour.call(widget, text.pointer()) };
            if STATUS.swap(2, Ordering::AcqRel) != 2 {
                logging::write("title screen label applied: RE_Kenshi Rust loaded");
            }
        }
        _ => {
            // SAFETY: Forward original arguments unchanged, including on panic.
            unsafe { state.detour.call(widget, caption) };
        }
    }
}

fn version_widget(name: &[u8]) -> bool {
    name == b"VersionText" || name.ends_with(b"_VersionText")
}

struct NativeString<'a> {
    storage: Box<MaybeUninit<UStringStorage>>,
    api: &'a Api,
}
impl<'a> NativeString<'a> {
    fn new(api: &'a Api, text: &CStr) -> Self {
        let mut storage = Box::<UStringStorage>::new_uninit();
        // SAFETY: The verified native constructor initializes sufficiently sized,
        // aligned storage at a stable address from valid terminated UTF-8 input.
        unsafe { (api.construct)(storage.as_mut_ptr().cast(), text.as_ptr()) };
        Self { storage, api }
    }

    /// # Safety
    /// source must be a live native object of the type accepted by constructor:
    /// either UString or the binary's std::string. Constructor must initialize
    /// UStringStorage and belong to api's pinned MyGUI module.
    unsafe fn from_native(api: &'a Api, source: *const c_void, constructor: CopyConstruct) -> Self {
        let mut storage = Box::<UStringStorage>::new_uninit();
        // SAFETY: Source/constructor contract is upheld by the caller; destination
        // is stable, aligned native object storage, owned until native destruction.
        unsafe { constructor(storage.as_mut_ptr().cast(), source) };
        Self { storage, api }
    }
    fn pointer(&self) -> *const c_void {
        self.storage.as_ptr().cast()
    }
    fn bytes(&self) -> &[u8] {
        // SAFETY: This constructed UString's native UTF-8 cache is terminated and
        // remains alive until modification/destruction. Callers use it only on
        // the same thread; borrow prevents append during the returned slice's use.
        unsafe { CStr::from_ptr((self.api.utf8)(self.pointer())).to_bytes() }
    }
    fn append(&mut self, text: &str) {
        // SAFETY: Native object is initialized; text is valid UTF-8, and explicit
        // byte length limits reads. Allocation and eventual free use native CRT.
        unsafe {
            (self.api.append)(
                self.storage.as_mut_ptr().cast(),
                text.as_ptr().cast(),
                text.len(),
            )
        };
    }
}
impl Drop for NativeString<'_> {
    fn drop(&mut self) {
        // SAFETY: Exactly one native destruction for the constructed object;
        // native CRT frees its contents; Rust then frees only opaque storage.
        unsafe { (self.api.destroy)(self.storage.as_mut_ptr().cast()) };
    }
}

impl Api {
    fn probe(&self) -> Result<(), String> {
        // Exercises short/long strings, Unicode conversion, native copy, native
        // allocation/free, and caption augmentation without constructing a GUI.
        for input in [
            c"1.0.68",
            c"Kenshi \u{2014} \u{65e5}\u{672c}\u{8a9e} caption longer than inline storage",
        ] {
            let original = NativeString::new(self, input);
            // SAFETY: original owns a live UString in this same native module.
            let mut copy =
                unsafe { NativeString::from_native(self, original.pointer(), self.copy) };
            copy.append(BADGE);
            let expected = [input.to_bytes(), BADGE.as_bytes()].concat();
            if original.bytes() != input.to_bytes() || copy.bytes() != expected {
                return Err("native MyGUI string/caption probe failed".into());
            }
        }
        Ok(())
    }
}

fn symbol(module: HMODULE, name: &CStr) -> Result<ExportAddress, String> {
    // SAFETY: Live module handle; CStr guarantees the export name is terminated.
    unsafe { GetProcAddress(module, name.as_ptr().cast()) }
        .ok_or_else(|| format!("missing MyGUI export: {}", name.to_string_lossy()))
}
fn verify_binary(module: HMODULE) -> Result<(), String> {
    use std::os::windows::ffi::OsStringExt;
    let mut path = vec![0u16; 32768];
    // SAFETY: Live module handle and writable, correctly bounded output buffer.
    let length =
        unsafe { GetModuleFileNameW(module, path.as_mut_ptr(), path.len() as u32) } as usize;
    if length == 0 || length >= path.len() {
        return Err("cannot locate loaded MyGUI binary".into());
    }
    let path = std::ffi::OsString::from_wide(&path[..length]);
    let contents = std::fs::read(path).map_err(|error| format!("read MyGUI: {error}"))?;
    if format!("{:x}", Sha256::digest(contents)) != MYGUI_SHA256 {
        return Err("unsupported MyGUI binary; legacy string ABI not verified".into());
    }
    Ok(())
}
