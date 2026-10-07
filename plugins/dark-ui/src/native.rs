//! Hash-gated MyGUI colour adapter. Lifecycle hooks pair every retained native
//! address with its destruction; all maps and colour updates stay on the UI thread.
use crate::palette::{self, Colour};
use detour::{Transaction, TypedDetour};
use rekenshi_native::symbols::mygui::{edit_text, resource_skin, skin_item, widget};
use rekenshi_plugin_runtime::Error;
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::ffi::{CStr, c_void};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};
use windows_sys::Win32::Foundation::HMODULE;

type Initialize = unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void) -> *const c_void;
type Destroy = unsafe extern "C" fn(*mut c_void);
type SetColour = unsafe extern "C" fn(*mut c_void, *const Colour);
type Name = unsafe extern "C" fn(*const c_void) -> *const c_void;
type CreateSkin = unsafe extern "C" fn(*mut c_void, *mut c_void);
/// Host operations needed by the native MyGUI adapter.
///
/// This record stays inside the linked host DLL; no Rust values cross DLLs.
#[derive(Clone, Copy, Debug)]
pub struct Adapter {
    /// Send a diagnostic through the host logger.
    pub log: fn(&str),
    /// Reserve an opaque native hook address in the host ownership registry.
    pub reserve: fn(*const c_void) -> Result<(), Error>,
    /// Copy a live MyGUI std::string using the verified native string implementation.
    pub text: unsafe fn(*const c_void) -> Option<String>,
    /// Report whether the host's UI frame adapter is ready.
    pub ready: fn() -> bool,
}
struct Hooks {
    host: Adapter,
    initialize: TypedDetour<Initialize>,
    shutdown: TypedDetour<Destroy>,
    colour: TypedDetour<SetColour>,
    skin_colour: TypedDetour<SetColour>,
    create_skin: TypedDetour<CreateSkin>,
    text: TypedDetour<SetColour>,
    destroy_text: TypedDetour<Destroy>,
    skin_name: Name,
}
static HOOKS: OnceLock<Hooks> = OnceLock::new();
// A single atomic snapshot makes enabling and brightness updates coherent.
static CONFIG: AtomicU64 = AtomicU64::new(0.6_f32.to_bits() as u64); // disabled
static EPOCH: AtomicU64 = AtomicU64::new(1);
#[derive(Default)]
struct Tracks {
    epoch: u64,
    applied: u64,
    panels: BTreeMap<usize, Panel>,
    texts: BTreeMap<usize, Colour>,
}
#[derive(Clone, Copy)]
struct Panel {
    original: Colour,
    skin: usize,
    minimum_brightness: f32,
}
thread_local! {
    static TRACKS: RefCell<Tracks> = RefCell::new(Tracks::default());
    static SURFACE: Cell<(usize, bool)> = const { Cell::new((0, false)) };
}
fn with_surface<T>(widget: usize, surface: bool, f: impl FnOnce() -> T) -> T {
    struct Reset((usize, bool));
    impl Drop for Reset {
        fn drop(&mut self) {
            SURFACE.with(|s| s.set(self.0));
        }
    }
    let _reset = Reset(SURFACE.with(|s| s.replace((widget, surface))));
    f()
}
fn tracked<T>(f: impl FnOnce(&mut Tracks) -> T) -> T {
    TRACKS.with(|tracks| {
        let mut tracks = tracks.borrow_mut();
        let epoch = EPOCH.load(Ordering::Acquire);
        if tracks.epoch != epoch {
            *tracks = Tracks {
                epoch,
                ..Tracks::default()
            };
        }
        f(&mut tracks)
    })
}
pub(super) fn prepared() -> bool {
    HOOKS.get().is_some_and(|hooks| (hooks.host.ready)())
}
pub(super) fn configure(enabled: bool, brightness: f64) {
    let value = (brightness.clamp(0.16, 0.60) as f32).to_bits() as u64 | ((enabled as u64) << 32);
    CONFIG.store(value, Ordering::Release);
}
fn configuration(value: u64) -> (bool, f32) {
    (value >> 32 != 0, f32::from_bits(value as u32))
}

/// Prepare dark UI hooks using the host's verified MyGUI module and operations.
///
/// # Errors
/// Returns an error if an export, hook reservation, or detour construction fails.
///
/// # Safety
/// The module must be the fingerprinted MyGUI binary and remain pinned for process life.
/// Its native signatures and string ABI must match the inspected bindings.
/// The text callback must synchronously copy the supplied native std::string;
/// callbacks must remain valid for process life and uphold the host ownership registry.
pub unsafe fn prepare(module: HMODULE, host: Adapter) -> Result<(), Error> {
    if HOOKS.get().is_some() {
        return Ok(());
    }
    // SAFETY: ui::prepare has verified and pinned this exact MyGUI DLL. The
    // headers/export signatures specify x64 this pointers and const Colour&.
    let hooks = unsafe {
        let initialize: Initialize =
            std::mem::transmute(target(module, widget::INITIALISE_SKIN, host)?);
        let shutdown: Destroy = std::mem::transmute(target(module, widget::SHUTDOWN_SKIN, host)?);
        let colour: SetColour = std::mem::transmute(target(module, widget::SET_COLOUR, host)?);
        let skin_colour: SetColour =
            std::mem::transmute(target(module, skin_item::SET_COLOUR, host)?);
        let create_skin: CreateSkin =
            std::mem::transmute(target(module, skin_item::CREATE_SKIN, host)?);
        let text: SetColour =
            std::mem::transmute(target(module, edit_text::APPLY_TEXT_COLOUR, host)?);
        let destroy_text: Destroy = std::mem::transmute(target(module, edit_text::DESTROY, host)?);
        Hooks {
            host,
            initialize: TypedDetour::new(initialize, initialize_hook as Initialize)
                .map_err(|e| e.to_string())?,
            shutdown: TypedDetour::new(shutdown, shutdown_hook as Destroy)
                .map_err(|e| e.to_string())?,
            colour: TypedDetour::new(colour, colour_hook as SetColour)
                .map_err(|e| e.to_string())?,
            skin_colour: TypedDetour::new(skin_colour, skin_colour_hook as SetColour)
                .map_err(|e| e.to_string())?,
            create_skin: TypedDetour::new(create_skin, create_skin_hook as CreateSkin)
                .map_err(|e| e.to_string())?,
            text: TypedDetour::new(text, text_hook as SetColour).map_err(|e| e.to_string())?,
            destroy_text: TypedDetour::new(destroy_text, destroy_text_hook as Destroy)
                .map_err(|e| e.to_string())?,
            skin_name: std::mem::transmute::<*const c_void, Name>(rekenshi_native::export(
                module as usize,
                resource_skin::GET_RESOURCE_NAME,
            )?),
        }
    };
    HOOKS.set(hooks).map_err(|_| "dark UI initialized twice")?;
    (host.log)(
        "dark UI: native skin creation, destruction, panel and text colour adapters prepared",
    );
    Ok(())
}
unsafe fn target(module: HMODULE, name: &CStr, host: Adapter) -> Result<*const c_void, Error> {
    let entry = rekenshi_native::export(module as usize, name)? as *const u8;
    // SAFETY: Fingerprinted mapped PE exports are E9 linker thunks. Hook their
    // actual body so native virtual/internal calls and export calls both reach us.
    let body = unsafe {
        if entry.read() != 0xe9 {
            return Err("dark UI export is not a linker thunk".into());
        }
        entry.wrapping_offset(5 + entry.add(1).cast::<i32>().read_unaligned() as isize)
    };
    (host.reserve)(body.cast())?;
    Ok(body.cast())
}
/// Add prepared hooks to the host's coordinated startup transaction.
pub fn enable(tx: &mut Transaction<'_>) {
    EPOCH.fetch_add(1, Ordering::AcqRel);
    if let Some(h) = HOOKS.get() {
        tx.enable(&h.initialize);
        tx.enable(&h.shutdown);
        tx.enable(&h.colour);
        tx.enable(&h.skin_colour);
        tx.enable(&h.create_skin);
        tx.enable(&h.text);
        tx.enable(&h.destroy_text);
    }
}
/// Restore tracked colours and add hooks to the host's shutdown transaction.
///
/// # Safety
/// The caller must quiesce native UI work and call on the tracking UI thread,
/// or a thread without tracked widgets. The transaction must obey detour's contract.
pub unsafe fn disable(tx: &mut Transaction<'_>) {
    // If shutdown is on MyGUI's thread, restore all live colours before detaching.
    // A different thread has an empty TLS map and never touches native widgets.
    configure(false, 0.6);
    // SAFETY: Forward the host's quiescent UI-thread contract.
    unsafe { frame() };
    tracked(|tracks| {
        tracks.panels.clear();
        tracks.texts.clear();
    });
    if let Some(h) = HOOKS.get() {
        tx.disable(&h.destroy_text);
        tx.disable(&h.text);
        tx.disable(&h.colour);
        tx.disable(&h.skin_colour);
        tx.disable(&h.create_skin);
        tx.disable(&h.shutdown);
        tx.disable(&h.initialize);
    }
}

unsafe extern "C" fn initialize_hook(
    widget: *mut c_void,
    skin: *mut c_void,
    template: *mut c_void,
) -> *const c_void {
    let Some(h) = HOOKS.get() else {
        return std::ptr::null();
    };
    let surface = std::panic::catch_unwind(|| {
        if skin.is_null() {
            return false;
        }
        // SAFETY: MyGUI owns this ResourceSkin and its native std::string for
        // the synchronous initialization; copy it through the owning old CRT.
        unsafe {
            let Some(name) = (h.host.text)((h.skin_name)(skin)) else {
                return false;
            };
            let surface = palette::is_surface(&name);
            if surface {
                tracked(|t| {
                    t.panels.insert(
                        widget as usize,
                        Panel {
                            original: Colour(1., 1., 1., 1.),
                            skin: 0,
                            minimum_brightness: palette::minimum_brightness(&name),
                        },
                    );
                });
            }
            surface
        }
    })
    .unwrap_or(false);
    // SAFETY: Forward exactly once with the original native arguments.
    let result = with_surface(widget as usize, surface, || unsafe {
        h.initialize.call(widget, skin, template)
    });
    if surface {
        let _ = std::panic::catch_unwind(|| {
            if let Some(original) = tracked(|t| t.panels.get(&(widget as usize)).copied()) {
                let (enabled, brightness) = configuration(CONFIG.load(Ordering::Acquire));
                if original.skin != 0 {
                    let output = if enabled {
                        palette::surface(
                            original.original,
                            brightness.max(original.minimum_brightness),
                        )
                    } else {
                        original.original
                    };
                    // SAFETY: MyGUI supplied this live SkinItem base pointer
                    // during creation. Direct colour updates leave child tints intact.
                    unsafe { h.skin_colour.call(original.skin as *mut c_void, &output) };
                }
            }
        });
    }
    result
}
unsafe extern "C" fn create_skin_hook(skin: *mut c_void, resource: *mut c_void) {
    let Some(h) = HOOKS.get() else { return };
    let _ = std::panic::catch_unwind(|| {
        let (widget, surface) = SURFACE.with(Cell::get);
        if surface {
            tracked(|t| {
                if let Some(panel) = t.panels.get_mut(&widget) {
                    panel.skin = skin as usize;
                }
            });
        }
    });
    // SAFETY: MyGUI's native initializer supplies this live base and ResourceSkin.
    unsafe { h.create_skin.call(skin, resource) };
}
unsafe extern "C" fn shutdown_hook(widget: *mut c_void) {
    let Some(h) = HOOKS.get() else { return };
    let _ = std::panic::catch_unwind(|| {
        tracked(|t| {
            t.panels.remove(&(widget as usize));
        })
    });
    // SAFETY: Remove the lifecycle borrow before native skin teardown/reuse.
    unsafe { h.shutdown.call(widget) };
}
unsafe extern "C" fn destroy_text_hook(text: *mut c_void) {
    let Some(h) = HOOKS.get() else { return };
    let _ = std::panic::catch_unwind(|| {
        tracked(|t| {
            t.texts.remove(&(text as usize));
        })
    });
    // SAFETY: Original destructor receives the live EditText this pointer.
    unsafe { h.destroy_text.call(text) };
}
unsafe extern "C" fn colour_hook(widget: *mut c_void, input: *const Colour) {
    let Some(h) = HOOKS.get() else { return };
    if input.is_null() {
        return;
    }
    // SAFETY: The native signature supplies a readable Colour reference.
    let original = unsafe { *input };
    let surface = std::panic::catch_unwind(|| {
        tracked(|t| {
            if let Some(value) = t.panels.get_mut(&(widget as usize)) {
                value.original = original;
                true
            } else {
                false
            }
        })
    })
    .unwrap_or(false);
    // SAFETY: Forward the original colour so MyGUI's recursive skin-child
    // propagation cannot multiply the tint or darken a child's artwork. The
    // final SkinItem colour hook applies each classified widget's own tint.
    with_surface(widget as usize, surface, || unsafe {
        h.colour.call(widget, &original)
    });
}
unsafe extern "C" fn skin_colour_hook(skin: *mut c_void, input: *const Colour) {
    let Some(h) = HOOKS.get() else { return };
    if input.is_null() {
        return;
    }
    // SAFETY: Native SkinItem supplies a valid colour reference and this pointer.
    let original = unsafe { *input };
    let (enabled, brightness) = configuration(CONFIG.load(Ordering::Acquire));
    let output = std::panic::catch_unwind(|| {
        let (widget, surface) = SURFACE.with(Cell::get);
        let mut minimum_brightness = 0.;
        if surface {
            // Capture the native base pointer supplied by MyGUI itself; no C++
            // base offsets or Rust guesses about object layout are required.
            tracked(|t| {
                if let Some(panel) = t.panels.get_mut(&widget) {
                    panel.skin = skin as usize;
                    minimum_brightness = panel.minimum_brightness;
                }
            });
        }
        if enabled && surface {
            palette::surface(original, brightness.max(minimum_brightness))
        } else {
            original
        }
    })
    .unwrap_or(original);
    // SAFETY: This only colours this SkinItem's subskins, not its child widgets.
    unsafe { h.skin_colour.call(skin, &output) };
}
unsafe extern "C" fn text_hook(text: *mut c_void, input: *const Colour) {
    let Some(h) = HOOKS.get() else { return };
    if input.is_null() {
        return;
    }
    // SAFETY: The native signature supplies a readable Colour reference.
    let original = unsafe { *input };
    let output = std::panic::catch_unwind(|| {
        tracked(|t| {
            t.texts.insert(text as usize, original);
        });
        if configuration(CONFIG.load(Ordering::Acquire)).0 {
            palette::text(original)
        } else {
            original
        }
    })
    .unwrap_or(original);
    // SAFETY: Final EditText colour path covers manual text and native skin states;
    // shadow colour/alpha, captions, font metrics and cursor behavior are untouched.
    unsafe { h.text.call(text, &output) };
}
/// Apply pending settings to tracked widgets on MyGUI's thread.
///
/// # Safety
/// Call only on the native UI thread while its tracked widgets remain live,
/// or on a thread with no tracked widgets during quiescent shutdown.
pub unsafe fn frame() {
    let Some(h) = HOOKS.get() else { return };
    let config = CONFIG.load(Ordering::Acquire);
    let changes = tracked(|t| {
        if t.applied == config {
            return None;
        }
        t.applied = config;
        Some((
            t.panels.iter().map(|(&p, &c)| (p, c)).collect::<Vec<_>>(),
            t.texts.iter().map(|(&p, &c)| (p, c)).collect::<Vec<_>>(),
        ))
    });
    let Some((panels, texts)) = changes else {
        return;
    };
    let (enabled, brightness) = configuration(config);
    // SAFETY: This callback runs on the native UI thread. Lifecycle hooks remove
    // addresses before destruction. These native colour setters do not raise
    // events or delete widgets; TLS borrows are released before entering MyGUI.
    unsafe {
        for (_, panel) in panels {
            if panel.skin != 0 {
                // Direct SkinItem updates avoid reapplying a parent's colour to
                // its skin children (portraits/status controls can have own tints).
                h.skin_colour.call(
                    panel.skin as *mut c_void,
                    &if enabled {
                        palette::surface(panel.original, brightness.max(panel.minimum_brightness))
                    } else {
                        panel.original
                    },
                );
            }
        }
        for (p, original) in texts {
            h.text.call(
                p as *mut c_void,
                &if enabled {
                    palette::text(original)
                } else {
                    original
                },
            );
        }
    }
}
