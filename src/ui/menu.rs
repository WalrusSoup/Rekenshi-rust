//! Hash-gated MyGUI adapter. Widget access stays on MyGUI's own UI thread;
//! no widget pointer or C++ object is retained between callbacks.
use super::{Api, NativeString, UI, symbol, theme};
use crate::{logging, plugin, settings as internal_plugins};
use detour::{Transaction, TypedDetour};
use rekenshi_native::symbols::mygui::{
    any, button, gui, i_cropped_rectangle, tab_control, text_box, widget, widget_input,
};
use rekenshi_plugin_runtime::{Control, Value};
use std::cell::{Cell, RefCell};
use std::ffi::{CStr, CString, c_void};
use std::sync::OnceLock;
use std::sync::atomic::Ordering;
use windows_sys::Win32::Foundation::HMODULE;

type Frame = unsafe extern "C" fn(*mut c_void, f32);
type Click = unsafe extern "C" fn(*mut c_void);
type Count = unsafe extern "C" fn(*mut c_void) -> usize;
type Item = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
type Visible = unsafe extern "C" fn(*mut c_void, bool);
type GetVisible = unsafe extern "C" fn(*const c_void) -> bool;
type Parent = unsafe extern "C" fn(*const c_void) -> *mut c_void;
type Find = unsafe extern "C" fn(*mut c_void, *const c_void, bool) -> *mut c_void;
type Enumerate = unsafe extern "C" fn(*mut c_void, *mut Enumerator) -> *mut Enumerator;
type CreateChild = unsafe extern "C" fn(
    *mut c_void,
    *const c_void,
    *const c_void,
    *const Coord,
    u32,
    *const c_void,
) -> *mut c_void;
type FindTab = unsafe extern "C" fn(*mut c_void, *const c_void) -> *mut c_void;
type AddTab = unsafe extern "C" fn(*mut c_void, *const c_void, *mut c_void) -> *mut c_void;
type Extent = unsafe extern "C" fn(*const c_void) -> i32;
type SetCoord = unsafe extern "C" fn(*mut c_void, i32, i32, i32, i32);
type ButtonWidth = unsafe extern "C" fn(*mut c_void, usize, i32);
type SetColour = unsafe extern "C" fn(*mut c_void, *const theme::Colour);
type Property = unsafe extern "C" fn(*mut c_void, *const c_void, *const c_void);
#[repr(C)]
struct Coord {
    left: i32,
    top: i32,
    width: i32,
    height: i32,
}
#[repr(C)]
#[derive(Default)]
struct Enumerator {
    first: u64,
    current: usize,
    end: usize,
}
struct Menu {
    frame: TypedDetour<Frame>,
    click: TypedDetour<Click>,
    enumerate: Enumerate,
    find: Find,
    children: Count,
    child: Item,
    tabs: Count,
    find_tab: FindTab,
    add_tab: AddTab,
    data: super::tab_data::Api,
    width: Extent,
    height: Extent,
    set_coord: SetCoord,
    button_width: ButtonWidth,
    create_child: CreateChild,
    visible: Visible,
    enabled: Visible,
    selected: Visible,
    get_visible: GetVisible,
    parent: Parent,
    colour: SetColour,
    text_colour: SetColour,
    property: Property,
    hover: GetVisible,
}
static MENU: OnceLock<Menu> = OnceLock::new();
thread_local! {
    static ELAPSED: Cell<f32> = const { Cell::new(1.) };
    static OPTIONS_NAME: RefCell<String> = const { RefCell::new(String::new()) };
    static PAGE_SIZE: Cell<usize> = const { Cell::new(5) };
    static TAB_ERROR: Cell<bool> = const { Cell::new(false) };
    static PAGE: Cell<usize> = const { Cell::new(0) };
    static MESSAGE: RefCell<String> = RefCell::new("Changes apply immediately and are saved automatically.".into());
}

pub(super) fn prepare(module: HMODULE) -> Result<(), String> {
    if MENU.get().is_some() {
        return Ok(());
    }
    // SAFETY: Parent prepare verified this exact MyGUI DLL's SHA-256 and pinned
    // it. Decorated signatures specify x64 member ABI and native object types.
    let menu = unsafe {
        let frame: Frame = std::mem::transmute(body(module, gui::FRAME_EVENT, 0xd9f60)?);
        let click: Click = std::mem::transmute(body(
            module,
            widget_input::RISE_MOUSE_BUTTON_CLICK,
            0x1c53c0,
        )?);
        for address in [frame as *const c_void, click as *const c_void] {
            crate::extensions::reserve_core(address)?;
        }
        Menu {
            frame: TypedDetour::new(frame, frame_hook as Frame).map_err(|e| e.to_string())?,
            click: TypedDetour::new(click, click_hook as Click).map_err(|e| e.to_string())?,
            enumerate: std::mem::transmute::<super::ExportAddress, Enumerate>(symbol(
                module,
                gui::GET_ENUMERATOR,
            )?),
            find: std::mem::transmute::<super::ExportAddress, Find>(symbol(
                module,
                gui::FIND_WIDGET_T,
            )?),
            children: std::mem::transmute::<super::ExportAddress, Count>(symbol(
                module,
                widget::GET_CHILD_COUNT,
            )?),
            child: std::mem::transmute::<super::ExportAddress, Item>(symbol(
                module,
                widget::GET_CHILD_AT,
            )?),
            tabs: std::mem::transmute::<super::ExportAddress, Count>(symbol(
                module,
                tab_control::GET_ITEM_COUNT,
            )?),
            find_tab: std::mem::transmute::<super::ExportAddress, FindTab>(symbol(
                module,
                tab_control::FIND_ITEM_WITH,
            )?),
            add_tab: std::mem::transmute::<*const c_void, AddTab>(body(
                module,
                tab_control::ADD_ITEM,
                0x1a8630,
            )?),
            data: super::tab_data::Api {
                copy: std::mem::transmute::<*const c_void, super::tab_data::Copy>(body(
                    module,
                    any::COPY_CONSTRUCT,
                    0x61f00,
                )?),
                type_of: std::mem::transmute::<*const c_void, super::tab_data::Type>(body(
                    module,
                    any::GET_TYPE,
                    0x61fc0,
                )?),
                destroy: std::mem::transmute::<*const c_void, super::tab_data::Destroy>(body(
                    module,
                    any::DESTROY,
                    0x61f40,
                )?),
            },
            width: std::mem::transmute::<super::ExportAddress, Extent>(symbol(
                module,
                i_cropped_rectangle::GET_WIDTH,
            )?),
            height: std::mem::transmute::<super::ExportAddress, Extent>(symbol(
                module,
                i_cropped_rectangle::GET_HEIGHT,
            )?),
            set_coord: std::mem::transmute::<super::ExportAddress, SetCoord>(symbol(
                module,
                widget::SET_COORD,
            )?),
            button_width: std::mem::transmute::<super::ExportAddress, ButtonWidth>(symbol(
                module,
                tab_control::SET_BUTTON_WIDTH_AT,
            )?),
            create_child: std::mem::transmute::<super::ExportAddress, CreateChild>(symbol(
                module,
                widget::CREATE_WIDGET_T,
            )?),
            visible: std::mem::transmute::<super::ExportAddress, Visible>(symbol(
                module,
                widget::SET_VISIBLE,
            )?),
            enabled: std::mem::transmute::<super::ExportAddress, Visible>(symbol(
                module,
                widget::SET_ENABLED,
            )?),
            selected: std::mem::transmute::<super::ExportAddress, Visible>(symbol(
                module,
                button::SET_STATE_SELECTED,
            )?),
            get_visible: std::mem::transmute::<super::ExportAddress, GetVisible>(symbol(
                module,
                widget::GET_VISIBLE,
            )?),
            parent: std::mem::transmute::<super::ExportAddress, Parent>(symbol(
                module,
                widget::GET_PARENT,
            )?),
            colour: std::mem::transmute::<super::ExportAddress, SetColour>(symbol(
                module,
                widget::SET_COLOUR,
            )?),
            text_colour: std::mem::transmute::<super::ExportAddress, SetColour>(symbol(
                module,
                text_box::SET_TEXT_COLOUR,
            )?),
            property: std::mem::transmute::<super::ExportAddress, Property>(symbol(
                module,
                widget::SET_PROPERTY,
            )?),
            hover: std::mem::transmute::<super::ExportAddress, GetVisible>(symbol(
                module,
                widget_input::GET_ROOT_MOUSE_FOCUS,
            )?),
        }
    };
    MENU.set(menu).map_err(|_| "menu initialized twice")?;
    logging::write("MyGUI settings adapter verified; frame/click hooks prepared");
    Ok(())
}
/// # Safety
/// module is the fingerprinted pinned MyGUI binary; RVA is its verified body.
unsafe fn body(module: HMODULE, name: &CStr, rva: usize) -> Result<*const c_void, String> {
    let address = symbol(module, name)? as *const u8;
    plugin::pin(address.cast())?;
    // SAFETY: Verified mapped PE export is an E9 linker thunk; the hash pins its
    // complete contents and the independently inspected implementation RVA.
    unsafe {
        if address.read() != 0xe9 {
            return Err("menu export is not expected linker thunk".into());
        }
        let body =
            address.wrapping_offset(5 + address.add(1).cast::<i32>().read_unaligned() as isize);
        if body as usize != module as usize + rva {
            return Err("menu implementation RVA differs".into());
        }
        Ok(body.cast())
    }
}
pub(crate) fn enable(tx: &mut Transaction<'_>) {
    if let Some(m) = MENU.get() {
        tx.enable(&m.frame);
        tx.enable(&m.click);
    }
}
pub(crate) fn prepared() -> bool {
    MENU.get().is_some()
}
pub(crate) fn disable(tx: &mut Transaction<'_>) {
    if let Some(m) = MENU.get() {
        tx.disable(&m.click);
        tx.disable(&m.frame);
    }
}

extern "C" fn frame_hook(gui: *mut c_void, delta: f32) {
    let Some(m) = MENU.get() else { return };
    // SAFETY: MyGUI supplies its live Gui and elapsed time on its UI thread.
    unsafe { m.frame.call(gui, delta) };
    let _ = std::panic::catch_unwind(|| {
        // SAFETY: MyGUI invokes this frame hook on its UI thread; widget
        // destruction hooks invalidate all retained addresses before release.
        unsafe { rekenshi_dark_ui::frame() };
    });
    crate::callbacks::ui_frame(delta);
    let _ = std::panic::catch_unwind(|| {
        if crate::STATUS.load(Ordering::Acquire) != 1 {
            return;
        }
        let Some(ui) = UI.get() else { return };
        let elapsed = ELAPSED.with(|x| {
            let next = x.get() + delta.max(0.);
            x.set(next);
            next
        });
        if elapsed < 0.5 {
            return;
        }
        ELAPSED.with(|x| x.set(0.));
        // SAFETY: Same UI thread after the original frame; no native widget
        // pointers survive this call, and the adapter's binary is pinned.
        unsafe { refresh(m, &ui.api, gui) };
    });
}
extern "C" fn click_hook(input: *mut c_void) {
    let Some(m) = MENU.get() else { return };
    let name = std::panic::catch_unwind(|| {
        let Some(ui) = UI.get() else {
            return String::new();
        };
        // SAFETY: The hash-gated native click body constructs its Widget sender
        // with `lea rdx,[rbx-0x80]`. This is the verified WidgetInput base offset.
        // The native caller supplies that live subobject on its own UI thread.
        unsafe { widget_name(&ui.api, input.cast::<u8>().sub(0x80).cast()) }
    })
    .unwrap_or_default();
    // SAFETY: Forward the native WidgetInput pointer to the retained trampoline.
    // Only the owned name is used after delegates may destroy their sender.
    unsafe { m.click.call(input) };
    let _ = std::panic::catch_unwind(|| {
        if name.starts_with("RKPack.") && crate::STATUS.load(Ordering::Acquire) == 1 {
            action(&name);
            ELAPSED.with(|x| x.set(1.));
        }
    });
}
fn action(name: &str) {
    match name {
        "RKPack.next" => {
            PAGE.with(|p| {
                p.set((p.get() + 1).min(
                    internal_plugins::rows().len().saturating_sub(1) / PAGE_SIZE.with(Cell::get),
                ))
            });
            return;
        }
        "RKPack.previous" => {
            PAGE.with(|p| p.set(p.get().saturating_sub(1)));
            return;
        }
        _ => {}
    }
    let Some((operation, index)) = name.strip_prefix("RKPack.").and_then(|s| s.split_once('.'))
    else {
        return;
    };
    let Some(row) = index
        .parse::<usize>()
        .ok()
        .and_then(|i| internal_plugins::rows().into_iter().nth(i))
    else {
        return;
    };
    let value = match (operation, row.setting.control, row.value) {
        ("toggle", Control::Toggle, Value::Toggle(x)) => Value::Toggle(!x),
        ("plus" | "minus", Control::Number { min, max, step }, Value::Number(x)) => Value::Number(
            (((x + if operation == "plus" { step } else { -step }) * 10000.).round() / 10000.)
                .clamp(min, max),
        ),
        _ => return,
    };
    let message = match internal_plugins::set(row.setting.key, value) {
        Ok(()) => {
            logging::write(&format!("setting {} applied: {value:?}", row.setting.key));
            "Applied and saved. No restart needed.".to_owned()
        }
        Err(e) => {
            logging::write(&format!("settings change rejected: {e}"));
            format!("Change failed: {e}")
        }
    };
    MESSAGE.with(|m| *m.borrow_mut() = message);
}

unsafe fn widget_name(api: &Api, widget: *mut c_void) -> String {
    // SAFETY: Widget is live on the UI thread; native string is copied before
    // any native operation can invalidate it, using the owning DLL's CRT.
    unsafe {
        String::from_utf8_lossy(
            NativeString::from_native(api, (api.name)(widget), api.from_std_string).bytes(),
        )
        .into_owned()
    }
}
unsafe fn find(m: &Menu, api: &Api, gui: *mut c_void, name: &str) -> *mut c_void {
    let text = NativeString::new(api, &CString::new(name).expect("widget names have no NUL"));
    // SAFETY: Constructed native UString supplies a native std::string reference,
    // valid through this synchronous lookup. throw=false allows missing widgets.
    unsafe { (m.find)(gui, (api.std_string)(text.pointer()), false) }
}
unsafe fn caption(api: &Api, widget: *mut c_void, text: &str) {
    if widget.is_null() {
        return;
    }
    let text = NativeString::new(
        api,
        &CString::new(text.replace('\0', "")).expect("NUL removed"),
    );
    // SAFETY: Only adapter-created TextBox/Button/Window caption widgets enter
    // here. Their native implementation synchronously copies this UString.
    if let Some(ui) = UI.get() {
        // SAFETY: Live adapter-created caption widget and native-owned string.
        unsafe { ui.detour.call(widget, text.pointer()) };
    }
}
unsafe fn create(
    m: &Menu,
    api: &Api,
    parent: *mut c_void,
    kind: &str,
    skin: &str,
    name: &str,
) -> *mut c_void {
    if parent.is_null() {
        return std::ptr::null_mut();
    }
    let strings: Vec<_> = [kind, skin, name]
        .into_iter()
        .map(|s| NativeString::new(api, &CString::new(s).expect("literal native names")))
        .collect();
    let coord = Coord {
        left: 0,
        top: 0,
        width: 1,
        height: 1,
    };
    // SAFETY: Native strings/coordinates remain live through the synchronous
    // call. Align::Default is Left|Top = 10 in the fingerprinted MyGUI ABI.
    unsafe {
        let ptr: Vec<_> = strings
            .iter()
            .map(|s| (api.std_string)(s.pointer()))
            .collect();
        (m.create_child)(parent, ptr[0], ptr[1], &coord, 10, ptr[2])
    }
}
unsafe fn child_named(m: &Menu, api: &Api, parent: *mut c_void, name: &str) -> *mut c_void {
    // SAFETY: Traversal and string copies happen on the owning UI thread.
    unsafe {
        for i in 0..(m.children)(parent).min(4096) {
            let child = (m.child)(parent, i);
            if !child.is_null() && widget_name(api, child) == name {
                return child;
            }
        }
    }
    std::ptr::null_mut()
}
unsafe fn hierarchy_visible(m: &Menu, widget: *mut c_void) -> bool {
    // SAFETY: Live hierarchy on the UI thread; no pointer escapes the callback.
    unsafe {
        super::visibility::visible_ancestors((!widget.is_null()).then_some(widget), |widget| {
            let parent = (m.parent)(widget);
            (
                (m.get_visible)(widget),
                (!parent.is_null()).then_some(parent),
            )
        })
    }
}
unsafe fn options(m: &Menu, api: &Api, gui: *mut c_void) -> *mut c_void {
    // SAFETY: UI thread owns Gui's root vector and all descendants. The inspected
    // getEnumerator body writes bool at 0, begin at 8, end at 16 into its sret
    // buffer. Traversal completes before any adapter widget creation.
    unsafe {
        let cached = OPTIONS_NAME.with(|s| s.borrow().clone());
        if !cached.is_empty() {
            let widget = find(m, api, gui, &cached);
            if hierarchy_visible(m, widget) {
                return widget;
            }
        }
        let mut enumerator = Enumerator::default();
        (m.enumerate)(gui, &mut enumerator);
        let Some(bytes) = enumerator.end.checked_sub(enumerator.current) else {
            return std::ptr::null_mut();
        };
        if bytes > 4096 * 8 || bytes % 8 != 0 || enumerator.current == 0 {
            return std::ptr::null_mut();
        }
        let mut pending =
            std::slice::from_raw_parts(enumerator.current as *const *mut c_void, bytes / 8)
                .to_vec();
        let mut visited = 0;
        while let Some(widget) = pending.pop() {
            visited += 1;
            if widget.is_null() || visited > 8192 {
                continue;
            }
            let name = widget_name(api, widget);
            if (name == "OptionsTab" || name.ends_with("_OptionsTab"))
                && hierarchy_visible(m, widget)
            {
                OPTIONS_NAME.with(|s| *s.borrow_mut() = name);
                return widget;
            }
            let count = (m.children)(widget).min(4096);
            for i in 0..count {
                pending.push((m.child)(widget, i));
            }
        }
        std::ptr::null_mut()
    }
}

// All helpers consume live widgets within this callback. No native object or
// C++ string allocation escapes MyGUI's own thread or owning runtime.
unsafe fn property(m: &Menu, api: &Api, widget: *mut c_void, key: &str, value: &str) {
    if widget.is_null() {
        return;
    }
    let key = NativeString::new(api, &CString::new(key).expect("literal property"));
    let value = NativeString::new(api, &CString::new(value).expect("literal property value"));
    // SAFETY: Native-owned std::string references are live for this synchronous call.
    unsafe {
        (m.property)(
            widget,
            (api.std_string)(key.pointer()),
            (api.std_string)(value.pointer()),
        )
    };
}
unsafe fn surface(
    m: &Menu,
    api: &Api,
    parent: *mut c_void,
    name: &str,
    colour: theme::Colour,
) -> *mut c_void {
    // SAFETY: Parent is live on MyGUI's thread; WhiteSkin uses the shipped white
    // atlas tile. These are native rectangles, not modified image assets.
    unsafe {
        let widget = create(m, api, parent, "Widget", theme::SURFACE_SKIN, name);
        if !widget.is_null() {
            (m.colour)(widget, &colour);
        }
        property(m, api, widget, "NeedMouse", "false");
        property(m, api, widget, "NeedKey", "false");
        property(m, api, widget, "InheritsPick", "true");
        widget
    }
}
unsafe fn text(
    m: &Menu,
    api: &Api,
    parent: *mut c_void,
    name: &str,
    size: &str,
    colour: theme::Colour,
) -> *mut c_void {
    // SAFETY: The verified text-only skin has transparent background texels.
    // Colour is four floats as declared in MyGUI_Colour.h. Text never takes focus.
    unsafe {
        let widget = create(m, api, parent, "TextBox", theme::TEXT_SKIN, name);
        property(m, api, widget, "FontName", theme::FONT);
        property(m, api, widget, "FontHeight", size);
        property(m, api, widget, "TextAlign", "Left VCenter");
        property(m, api, widget, "TextShadow", "false");
        property(m, api, widget, "NeedMouse", "false");
        property(m, api, widget, "NeedKey", "false");
        if !widget.is_null() {
            (m.text_colour)(widget, &colour);
        }
        widget
    }
}
unsafe fn flat_button(
    m: &Menu,
    api: &Api,
    parent: *mut c_void,
    name: &str,
    label: &str,
) -> *mut c_void {
    // SAFETY: Button owns its decorative label. NeedMouse=false prevents the
    // label from intercepting input intended for its parent's native click.
    unsafe {
        let button = create(m, api, parent, "Button", theme::SURFACE_SKIN, name);
        property(m, api, button, "NeedKey", "false");
        let caption_widget = text(
            m,
            api,
            button,
            &format!("{name}.caption"),
            "16",
            theme::TEXT,
        );
        property(m, api, caption_widget, "TextAlign", "Center");
        caption(api, caption_widget, label);
        button
    }
}
unsafe fn refresh(m: &Menu, api: &Api, gui: *mut c_void) {
    let rows = internal_plugins::rows();
    if rows.is_empty() {
        return;
    }
    // SAFETY: All widget lookup, creation and updates happen on MyGUI's thread.
    // Each Options instance owns its tab and descendants; no widget survives in
    // Rust state. Closing, tab selection and destruction use native ownership.
    unsafe {
        let tabs = options(m, api, gui);
        if tabs.is_null() {
            return;
        }
        let count = (m.tabs)(tabs);
        if count == 0 || count > 64 {
            return;
        }
        let name = NativeString::new(api, c"REKENSHI-RUST");
        let mut tab = (m.find_tab)(tabs, name.pointer());
        if tab.is_null() {
            let mut data = match super::tab_data::PanelData::null_for_tabs(&m.data, tabs, count) {
                Ok(data) => data,
                Err(error) => {
                    if !TAB_ERROR.with(|failed| failed.replace(true)) {
                        logging::write(&format!("Options tab unavailable: {error}"));
                    }
                    return;
                }
            };
            TAB_ERROR.with(|failed| failed.set(false));
            tab = (m.add_tab)(tabs, name.pointer(), data.transfer());
            if tab.is_null() {
                return;
            }
            (m.button_width)(tabs, count, 190);
            logging::write("REKENSHI-RUST tab added to Options");
        }
        // ICroppedRectangle is Widget's second base, at +8, independently
        // verified by Widget::setCoord's native virtual dispatch adjustment.
        let rectangle = tab.cast::<u8>().add(8).cast();
        let width = (m.width)(rectangle) - 24;
        let height = (m.height)(rectangle) - 24;
        let mut panel = child_named(m, api, tab, "RKPack.panel");
        if width < 320 || height < theme::ROW_START + theme::ROW_HEIGHT + theme::FOOTER {
            if !panel.is_null() {
                (m.visible)(panel, false);
            }
            return;
        }
        let page_size =
            ((height - theme::ROW_START - theme::FOOTER) / theme::ROW_HEIGHT).clamp(1, 5) as usize;
        let last_page = rows.len().saturating_sub(1) / page_size;
        let page = PAGE.with(|page| {
            let old_size = PAGE_SIZE.with(|size| size.replace(page_size));
            let next = (page.get() * old_size / page_size).min(last_page);
            page.set(next);
            next
        });
        if panel.is_null() {
            panel = surface(m, api, tab, "RKPack.panel", theme::BACKGROUND);
            if panel.is_null() {
                return;
            }
            let title = text(m, api, panel, "RKPack.title", "24", theme::TEXT);
            caption(api, title, "Mod settings");
            let subtitle = text(m, api, panel, "RKPack.subtitle", "13", theme::MUTED);
            caption(api, subtitle, "Built-in optimizations and installed mods");
            surface(m, api, panel, "RKPack.divider", theme::CONTROL);
            text(m, api, panel, "RKPack.message", "13", theme::MUTED);
            text(m, api, panel, "RKPack.page", "13", theme::MUTED);
            for (name, label) in [("RKPack.previous", "Previous"), ("RKPack.next", "Next")] {
                flat_button(m, api, panel, name, label);
            }
            // Backgrounds are siblings created before labels, so every label
            // and input draws above the card. Decorations cannot take focus.
            for i in 0..rows.len() {
                surface(m, api, panel, &format!("RKPack.row.{i}"), theme::SURFACE);
            }
            for (i, row) in rows.iter().enumerate() {
                let label = text(
                    m,
                    api,
                    panel,
                    &format!("RKPack.label.{i}"),
                    "16",
                    theme::TEXT,
                );
                caption(
                    api,
                    label,
                    &if row.setting.label == "Enabled" {
                        row.plugin.to_owned()
                    } else {
                        format!("{} / {}", row.plugin, row.setting.label)
                    },
                );
                let description = text(
                    m,
                    api,
                    panel,
                    &format!("RKPack.description.{i}"),
                    "13",
                    theme::MUTED,
                );
                property(m, api, description, "TextAlign", "Left Top");
                match row.setting.control {
                    Control::Toggle => {
                        let checkbox = create(
                            m,
                            api,
                            panel,
                            "Button",
                            theme::SURFACE_SKIN,
                            &format!("RKPack.toggle.{i}"),
                        );
                        property(m, api, checkbox, "NeedKey", "false");
                        let inset = surface(
                            m,
                            api,
                            checkbox,
                            &format!("RKPack.inset.{i}"),
                            theme::SURFACE,
                        );
                        if !inset.is_null() {
                            (m.set_coord)(inset, 1, 1, 20, 20);
                        }
                        let mark = surface(
                            m,
                            api,
                            checkbox,
                            &format!("RKPack.check.{i}"),
                            theme::ACCENT,
                        );
                        if !mark.is_null() {
                            (m.set_coord)(mark, 5, 5, 12, 12);
                        }
                    }
                    Control::Number { .. } => {
                        flat_button(m, api, panel, &format!("RKPack.minus.{i}"), "-");
                        let value = text(
                            m,
                            api,
                            panel,
                            &format!("RKPack.value.{i}"),
                            "16",
                            theme::TEXT,
                        );
                        property(m, api, value, "TextAlign", "Center");
                        flat_button(m, api, panel, &format!("RKPack.plus.{i}"), "+");
                    }
                }
            }
        }
        (m.visible)(panel, true);
        (m.set_coord)(panel, 12, 12, width, height);
        let place = |name: &str, left, top, width, height| {
            let widget = child_named(m, api, panel, name);
            if !widget.is_null() {
                (m.set_coord)(widget, left, top, width, height);
            }
            widget
        };
        let style_button = |button: *mut c_void, width: i32, height: i32, enabled: bool| {
            if button.is_null() {
                return;
            }
            (m.enabled)(button, enabled);
            let hovered = (m.hover)(button.cast::<u8>().add(0x80).cast());
            (m.colour)(
                button,
                &if !enabled {
                    theme::SURFACE
                } else if hovered {
                    theme::HOVER
                } else {
                    theme::CONTROL
                },
            );
            let label = child_named(
                m,
                api,
                button,
                &format!("{}.caption", widget_name(api, button)),
            );
            if !label.is_null() {
                (m.set_coord)(label, 0, 0, width, height);
                (m.text_colour)(
                    label,
                    &if enabled {
                        theme::TEXT
                    } else {
                        theme::DISABLED
                    },
                );
            }
        };
        place("RKPack.title", 18, 12, width - 36, 30);
        place("RKPack.subtitle", 18, 43, width - 36, 18);
        place("RKPack.divider", 18, 65, width - 36, 1);
        place("RKPack.message", 18, height - 72, width - 36, 27);
        let previous = place("RKPack.previous", 18, height - 38, 96, 28);
        let next = place("RKPack.next", 122, height - 38, 76, 28);
        for (button, enabled, item_width) in
            [(previous, page > 0, 96), (next, page < last_page, 76)]
        {
            if !button.is_null() {
                (m.visible)(button, last_page > 0);
                style_button(button, item_width, 28, enabled);
            }
        }
        let counter = place("RKPack.page", width - 118, height - 38, 100, 28);
        property(m, api, counter, "TextAlign", "Right VCenter");
        caption(api, counter, &format!("{} / {}", page + 1, last_page + 1));
        for (i, row) in rows.iter().enumerate() {
            let shown = page == i / page_size;
            let top = theme::ROW_START + (i % page_size) as i32 * theme::ROW_HEIGHT;
            let toggle = matches!(row.setting.control, Control::Toggle);
            place(&format!("RKPack.row.{i}"), 10, top, width - 20, 53);
            let left = if toggle { 50 } else { 22 };
            let label = place(
                &format!("RKPack.label.{i}"),
                left,
                top + 3,
                if toggle {
                    width - left - 22
                } else {
                    (width - 184).max(1)
                },
                21,
            );
            if !label.is_null() {
                (m.text_colour)(
                    label,
                    &if row.error.is_some() {
                        theme::DISABLED
                    } else {
                        theme::TEXT
                    },
                );
            }
            let description = place(
                &format!("RKPack.description.{i}"),
                left,
                top + 26,
                width - left - 22,
                26,
            );
            let checkbox = place(&format!("RKPack.toggle.{i}"), 22, top + 3, 22, 22);
            for (operation, left, item_width) in [
                ("minus", width - 154, 28),
                ("value", width - 122, 68),
                ("plus", width - 50, 28),
            ] {
                let button = place(
                    &format!("RKPack.{operation}.{i}"),
                    left,
                    top + 2,
                    item_width,
                    24,
                );
                if operation != "value" {
                    style_button(button, item_width, 24, row.error.is_none());
                }
            }
            for operation in [
                "row",
                "label",
                "description",
                "toggle",
                "value",
                "minus",
                "plus",
            ] {
                let widget = child_named(m, api, panel, &format!("RKPack.{operation}.{i}"));
                if !widget.is_null() {
                    (m.visible)(widget, shown);
                }
            }
            let desc = match &row.error {
                Some(error) => format!("Unavailable: {error}"),
                None => row.setting.description.to_owned(),
            };
            caption(
                api,
                description,
                &theme::description(&desc, width - left - 22),
            );
            if !description.is_null() {
                (m.text_colour)(
                    description,
                    &if row.error.is_some() {
                        theme::WARNING
                    } else {
                        theme::MUTED
                    },
                );
            }
            match row.value {
                Value::Toggle(checked) if !checkbox.is_null() => {
                    (m.selected)(checkbox, checked && row.error.is_none());
                    (m.enabled)(checkbox, row.error.is_none());
                    (m.colour)(
                        checkbox,
                        &if row.error.is_some() {
                            theme::CONTROL
                        } else if checked {
                            theme::ACCENT
                        } else {
                            theme::BORDER
                        },
                    );
                    let mark = child_named(m, api, checkbox, &format!("RKPack.check.{i}"));
                    if !mark.is_null() {
                        (m.visible)(mark, checked && row.error.is_none());
                    }
                }
                Value::Number(value) => {
                    let field = child_named(m, api, panel, &format!("RKPack.value.{i}"));
                    caption(
                        api,
                        field,
                        &if row.error.is_some() {
                            "--".to_owned()
                        } else {
                            format!("{value:.2}")
                        },
                    );
                    if !field.is_null() {
                        (m.text_colour)(
                            field,
                            &if row.error.is_some() {
                                theme::DISABLED
                            } else {
                                theme::TEXT
                            },
                        );
                    }
                }
                _ => {}
            }
        }
        MESSAGE.with(|message| {
            let widget = child_named(m, api, panel, "RKPack.message");
            let message = message.borrow();
            caption(api, widget, &theme::description(&message, width - 36));
            if !widget.is_null() {
                (m.text_colour)(
                    widget,
                    &if message.starts_with("Change failed") {
                        theme::WARNING
                    } else {
                        theme::MUTED
                    },
                );
            }
        });
    }
}
