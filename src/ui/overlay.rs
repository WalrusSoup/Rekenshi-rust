//! Diagnostic root widget on its own topmost layer, recreated by name each frame.
//! All native access stays on the existing verified MyGUI frame lane.
use super::{Coord, Menu, NativeString, caption, child_named, find, property, text, theme};
use crate::ui::{Api as Strings, ExportAddress, symbol};
use rekenshi_native::symbols::discovered::mygui as names;
use std::cell::{Cell, RefCell};
use std::ffi::{CString, c_void};
use windows_sys::Win32::Foundation::HMODULE;
use windows_sys::Win32::System::Threading::GetCurrentProcessId;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_F11};
use windows_sys::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

type Singleton = unsafe extern "C" fn() -> *mut c_void;
type Create = unsafe extern "C" fn(
    *mut c_void,
    *const c_void,
    *const c_void,
    *const Coord,
    u32,
    *const c_void,
    *const c_void,
) -> *mut c_void;
type Destroy = unsafe extern "C" fn(*mut c_void, *mut c_void);
type FindLayer = unsafe extern "C" fn(*mut c_void, *const c_void, bool) -> *mut c_void;
type Font = unsafe extern "C" fn(*mut c_void, *const c_void) -> *mut c_void;
type Count = unsafe extern "C" fn(*const c_void) -> usize;
type CreateLayer =
    unsafe extern "C" fn(*mut c_void, *const c_void, *const c_void, usize) -> *mut c_void;
pub(super) struct Api {
    gui: Singleton,
    layers: Singleton,
    create: Create,
    destroy: Destroy,
    find_layer: FindLayer,
    count: Count,
    create_layer: CreateLayer,
    resources: Singleton,
    resource: FindLayer,
    fonts: Singleton,
    font: Font,
}
const ROOT: &str = "RKDebug.overlay";
const BODY: &str = "RKDebug.text";
const LAYER: &str = "RKDebug.layer";
thread_local! {
    static UI_THREAD: Cell<bool> = const { Cell::new(false) };
    static SNAPSHOT: RefCell<String> = const { RefCell::new(String::new()) };
    static HOTKEY: Cell<Hotkey> = const { Cell::new(Hotkey { held: false }) };
}
#[derive(Clone, Copy)]
struct Hotkey {
    held: bool,
}
impl Hotkey {
    fn update(&mut self, down: bool, focused: bool) -> bool {
        let pressed = down && !self.held && focused;
        self.held = down;
        pressed
    }
}
/// Poll every UI frame so short presses are not lost to caption throttling.
pub(super) fn hotkey() -> bool {
    // SAFETY: Win32 accepts the constant virtual key and a writable process-ID
    // output. The foreground HWND is borrowed only for this immediate query.
    let (down, focused) = unsafe {
        let window = GetForegroundWindow();
        let mut process = 0;
        if !window.is_null() {
            GetWindowThreadProcessId(window, &mut process);
        }
        (
            GetAsyncKeyState(i32::from(VK_F11)) < 0,
            process == GetCurrentProcessId(),
        )
    };
    HOTKEY.with(|state| {
        let mut key = state.get();
        let pressed = key.update(down, focused);
        state.set(key);
        if pressed {
            crate::diagnostics::toggle();
        }
        pressed
    })
}
impl Api {
    pub(super) fn prepare(module: HMODULE) -> Result<Self, String> {
        // SAFETY: The parent verifies the entire DLL hash and pins it before
        // resolving these exports. Signatures match the shipped x64 decorated
        // exports/native headers, exercised against that DLL in test-menu.py.
        unsafe {
            Ok(Self {
                gui: std::mem::transmute::<ExportAddress, Singleton>(symbol(
                    module,
                    names::MY_GUI_VGUI_SINGLETON_GET_INSTANCE_PTR_157245913BF0,
                )?),
                layers: std::mem::transmute::<ExportAddress, Singleton>(symbol(
                    module,
                    names::MY_GUI_VLAYER_MANAGER_SINGLETON_GET_INSTANCE_PTR_6B64E242AD6F,
                )?),
                create: std::mem::transmute::<ExportAddress, Create>(symbol(
                    module,
                    names::MY_GUI_GUI_CREATE_WIDGET_T_8B14849E7CE0,
                )?),
                destroy: std::mem::transmute::<ExportAddress, Destroy>(symbol(
                    module,
                    names::MY_GUI_GUI_DESTROY_WIDGET_F1C6CAF70CAB,
                )?),
                find_layer: std::mem::transmute::<ExportAddress, FindLayer>(symbol(
                    module,
                    names::MY_GUI_LAYER_MANAGER_GET_BY_NAME_1D79252369C2,
                )?),
                count: std::mem::transmute::<ExportAddress, Count>(symbol(
                    module,
                    names::MY_GUI_LAYER_MANAGER_GET_LAYER_COUNT_AAA4696930EF,
                )?),
                create_layer: std::mem::transmute::<ExportAddress, CreateLayer>(symbol(
                    module,
                    names::MY_GUI_LAYER_MANAGER_CREATE_LAYER_AT_CB331F9B57C3,
                )?),
                resources: std::mem::transmute::<ExportAddress, Singleton>(symbol(
                    module,
                    names::MY_GUI_VRESOURCE_MANAGER_SINGLETON_GET_INSTANCE_PTR_543104401348,
                )?),
                resource: std::mem::transmute::<ExportAddress, FindLayer>(symbol(
                    module,
                    names::MY_GUI_RESOURCE_MANAGER_GET_BY_NAME_1B8487A375D3,
                )?),
                fonts: std::mem::transmute::<ExportAddress, Singleton>(symbol(
                    module,
                    names::MY_GUI_VFONT_MANAGER_SINGLETON_GET_INSTANCE_PTR_EB9991E7F818,
                )?),
                font: std::mem::transmute::<ExportAddress, Font>(symbol(
                    module,
                    names::MY_GUI_FONT_MANAGER_GET_BY_NAME_BA54FA16A612,
                )?),
            })
        }
    }

    /// A UI frame alone does not establish that native skins/fonts are loaded.
    /// The caller must be on the verified MyGUI lane with live string bindings.
    unsafe fn resources_ready(&self, strings: &Strings) -> bool {
        // SAFETY: Singleton pointers are borrowed on the UI lane, never cached.
        // Nonthrowing ResourceManager lookups accept live native std::strings.
        // Query FontManager only after the named font resource is registered.
        unsafe {
            let resources = (self.resources)();
            let fonts = (self.fonts)();
            if resources.is_null() || fonts.is_null() {
                return false;
            }
            for name in [theme::SURFACE_SKIN, theme::TEXT_SKIN, theme::FONT] {
                let native_name =
                    NativeString::new(strings, &CString::new(name).expect("literal resource name"));
                let name = (strings.std_string)(native_name.pointer());
                if (self.resource)(resources, name, false).is_null() {
                    return false;
                }
            }
            let font = NativeString::new(
                strings,
                &CString::new(theme::FONT).expect("literal font name"),
            );
            !(self.font)(fonts, (strings.std_string)(font.pointer())).is_null()
        }
    }
}
pub(super) fn stop(m: &Menu, strings: &Strings) {
    // Lifecycle shutdown on another thread must not dereference UI objects.
    // The supported native shutdown lane is the UI thread; no address is cached.
    if !UI_THREAD.replace(false) {
        return;
    }
    // SAFETY: This thread has run the verified UI callback; singleton lookup
    // returns null after Gui shutdown. Destroy only our freshly found root.
    unsafe {
        let gui = (m.overlay.gui)();
        if !gui.is_null() {
            let root = find(m, strings, gui, ROOT);
            if !root.is_null() {
                (m.overlay.destroy)(gui, root);
            }
        }
    }
    SNAPSHOT.with(|s| s.borrow_mut().clear());
    HOTKEY.with(|key| key.set(Hotkey { held: false }));
}
/// Gui and all widgets are borrowed only for this native frame callback.
pub(super) unsafe fn refresh(m: &Menu, strings: &Strings, gui: *mut c_void) {
    UI_THREAD.set(true);
    // SAFETY: The caller supplies the live native Gui on its UI thread. Native
    // strings are constructed/destructed by that same DLL; no pointer escapes.
    unsafe {
        let mut root = find(m, strings, gui, ROOT);
        if !crate::diagnostics::enabled() {
            if !root.is_null() {
                (m.overlay.destroy)(gui, root);
            }
            SNAPSHOT.with(|s| s.borrow_mut().clear());
            return;
        }
        if root.is_null() {
            // Kenshi refreshes existing text fonts after its first UI frames.
            // EditText::getFontName returns mFont+8 without a null check in the
            // shipped DLL: captioned text created before font registration
            // crashes that refresh. Wait for the actual resources, not time.
            if !m.overlay.resources_ready(strings) {
                return;
            }
            let layers = (m.overlay.layers)();
            if layers.is_null() {
                return;
            }
            let native: Vec<_> = [
                "Widget",
                theme::SURFACE_SKIN,
                LAYER,
                ROOT,
                "OverlappedLayer",
            ]
            .iter()
            .map(|s| NativeString::new(strings, &CString::new(*s).expect("literal names")))
            .collect();
            let ptr: Vec<_> = native
                .iter()
                .map(|s| (strings.std_string)(s.pointer()))
                .collect();
            if (m.overlay.find_layer)(layers, ptr[2], false).is_null() {
                (m.overlay.create_layer)(layers, ptr[2], ptr[4], (m.overlay.count)(layers));
            }
            root = (m.overlay.create)(
                gui,
                ptr[0],
                ptr[1],
                &Coord {
                    left: 12,
                    top: 12,
                    width: 680,
                    height: 450,
                },
                10,
                ptr[2],
                ptr[3],
            );
            if root.is_null() {
                return;
            }
            (m.colour)(root, &theme::Colour(0.025, 0.035, 0.05, 0.92));
            property(m, strings, root, "NeedMouse", "false");
            property(m, strings, root, "NeedKey", "false");
            property(m, strings, root, "InheritsPick", "false");
            let body = text(m, strings, root, BODY, "14", theme::TEXT);
            if !body.is_null() {
                (m.set_coord)(body, 12, 10, 656, 430);
                property(m, strings, body, "TextAlign", "Left Top");
                property(m, strings, body, "InheritsPick", "false");
            }
            crate::logging::write(
                "debug overlay: native font and skin resources ready; view created",
            );
        }
        let body = child_named(m, strings, root, BODY);
        SNAPSHOT.with(|snapshot| {
            let mut snapshot = snapshot.borrow_mut();
            if snapshot.is_empty() || !crate::diagnostics::frozen() {
                *snapshot = crate::diagnostics::snapshot();
            }
            caption(strings, body, &snapshot);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::Hotkey;

    #[test]
    fn f11_toggles_once_per_press_only_while_focused() {
        let mut key = Hotkey { held: false };
        assert!(!key.update(false, true));
        assert!(key.update(true, true));
        assert!(!key.update(true, true));
        assert!(!key.update(false, true));
        assert!(key.update(true, true));
        assert!(!key.update(false, false));
        assert!(!key.update(true, false));
        assert!(
            !key.update(true, true),
            "focus return with a held key is not a press"
        );
        assert!(!key.update(false, true));
        assert!(key.update(true, true));
    }
}
