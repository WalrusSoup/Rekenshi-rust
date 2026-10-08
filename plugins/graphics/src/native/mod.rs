//! Native Direct3D11 post-tonemap effect service. Does not use Kenshi private RVAs.
mod pipeline;
mod shader;
mod ssao;
use detour::{Threads, Transaction, TypedDetour};
use rekenshi_plugin_api::hooks::HookId;
use rekenshi_plugin_api::{ERROR, HostV1, INCOMPATIBLE, OK, graphics::*};
use std::cell::Cell;
use std::ffi::c_void;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Direct3D::*;
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::Dxgi::Common::*;
use windows::Win32::Graphics::Dxgi::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::{Interface, w};
type Draw = unsafe extern "system" fn(*mut c_void, u32, u32);
type CreatePixel = unsafe extern "system" fn(
    *mut c_void,
    *const c_void,
    usize,
    *mut c_void,
    *mut *mut c_void,
) -> i32;
struct Hooks {
    draw: TypedDetour<Draw>,
    target: usize,
    create: TypedDetour<CreatePixel>,
    create_target: usize,
}
struct Effect {
    token: u64,
    handle: u64,
    priority: i32,
    code: Vec<u8>,
    parameters: [f32; 16],
}
struct Backend {
    host: HostV1,
    game: PathBuf,
    effects: Vec<Effect>,
    serial: u64,
    gpu: Option<pipeline::Gpu>,
    context: usize,
    saw_lighting: bool,
    failed: bool,
    ssao: [f32; 4],
    ssao_gpu: Option<ssao::Gpu>,
}
static HOOKS: OnceLock<Hooks> = OnceLock::new();
static BACKEND: Mutex<Option<Backend>> = Mutex::new(None);
static ACTIVE: AtomicBool = AtomicBool::new(false);
static DRAWS: AtomicU64 = AtomicU64::new(0);
static PASSES: AtomicU64 = AtomicU64::new(0);
static AO_BINDINGS: AtomicU64 = AtomicU64::new(0);
static AO_BINDING_ERRORS: AtomicU64 = AtomicU64::new(0);
static EFFECTS: std::sync::RwLock<Option<rekenshi_plugin_sdk::operations::Provider>> =
    std::sync::RwLock::new(None);
static EFFECT_REVISION: AtomicU64 = AtomicU64::new(0);
thread_local! { static INSIDE: Cell<bool> = const { Cell::new(false) }; }

pub(super) fn initialize(host: HostV1, root: &std::path::Path) -> Result<(), String> {
    let mut backend = BACKEND.lock().map_err(|_| "graphics lock poisoned")?;
    let game = root.parent().ok_or("game directory")?.to_path_buf();
    if HOOKS.get().is_none() {
        // SAFETY: Create a hidden, disposable Windows STATIC window only to ask
        // D3D11 for its native context vtable; never subclass a game window.
        let window = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("STATIC"),
                w!("RE Kenshi graphics bootstrap"),
                WS_OVERLAPPEDWINDOW,
                0,
                0,
                64,
                64,
                None,
                None,
                None,
                None,
            )
        }
        .map_err(|e| e.to_string())?;
        let desc = DXGI_SWAP_CHAIN_DESC {
            BufferDesc: DXGI_MODE_DESC {
                Width: 64,
                Height: 64,
                Format: DXGI_FORMAT_R8G8B8A8_UNORM,
                ..Default::default()
            },
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
            BufferCount: 1,
            OutputWindow: window,
            Windowed: true.into(),
            SwapEffect: DXGI_SWAP_EFFECT_DISCARD,
            ..Default::default()
        };
        let mut device = None;
        let mut context = None;
        let mut swap = None;
        // SAFETY: Valid hidden window and writable optional COM outputs. No
        // game rendering occurs on this temporary device; resources release normally.
        let result = unsafe {
            D3D11CreateDeviceAndSwapChain(
                None,
                D3D_DRIVER_TYPE_HARDWARE,
                HMODULE::default(),
                D3D11_CREATE_DEVICE_FLAG(0),
                Some(&[D3D_FEATURE_LEVEL_11_0]),
                D3D11_SDK_VERSION,
                Some(&desc),
                Some(&mut swap),
                Some(&mut device),
                None,
                Some(&mut context),
            )
        };
        // SAFETY: This window was created above on this thread and is no longer needed.
        let _ = unsafe { DestroyWindow(window) };
        result.map_err(|e| e.to_string())?;
        let context = context.ok_or("no D3D context")?;
        let device = device.ok_or("no D3D device")?;
        let address = Interface::vtable(&context).Draw as *const c_void;
        // SAFETY: The typed COM vtable identifies exactly ID3D11DeviceContext::Draw.
        let target: Draw = unsafe { std::mem::transmute(address) };
        // SAFETY: Identical Windows x64 signature. Runtime pinned by host reservation.
        let draw =
            unsafe { TypedDetour::new(target, intercepted as Draw) }.map_err(|e| e.to_string())?;
        let create_address = Interface::vtable(&device).CreatePixelShader as *const c_void;
        // SAFETY: Documented device vtable method and identical Windows x64 ABI.
        let create = unsafe {
            TypedDetour::new(
                std::mem::transmute::<*const c_void, CreatePixel>(create_address),
                create_pixel as CreatePixel,
            )
        }
        .map_err(|e| e.to_string())?;
        HOOKS
            .set(Hooks {
                draw,
                target: address as usize,
                create,
                create_target: create_address as usize,
            })
            .map_err(|_| "hooks already initialized")?;
    }
    let hooks = HOOKS.get().ok_or("hooks unavailable")?;
    host.reserve_hook(std::ptr::NonNull::new(hooks.target as *mut c_void).ok_or("draw target")?)
        .map_err(|e| format!("Draw hook conflict: {e}"))?;
    host.reserve_hook(
        std::ptr::NonNull::new(hooks.create_target as *mut c_void).ok_or("shader target")?,
    )
    .map_err(|e| format!("CreatePixelShader hook conflict: {e}"))?;
    let catalog = host.hooks().map_err(|e| format!("hook catalog: {e}"))?;
    // SAFETY: This is the actual typed D3D11 vtable method discovered above,
    // reserved/pinned through the host, not our trampoline or callback.
    unsafe {
        catalog.publish(
            HookId::D3D11Draw,
            std::ptr::NonNull::new(hooks.target as *mut c_void).ok_or("draw target")?,
        )
    }
    .map_err(|e| format!("Draw publication: {e}"))?;
    *backend = Some(Backend {
        host,
        game,
        effects: vec![],
        serial: 0,
        gpu: None,
        context: 0,
        saw_lighting: false,
        failed: false,
        ssao: [0., 1.5, 0.04, 0.],
        ssao_gpu: None,
    });
    let mut tx = Transaction::new();
    tx.enable(&hooks.draw);
    tx.enable(&hooks.create);
    // SAFETY: Installed outside DllMain; patch transaction suspends existing
    // threads and relocates live instruction pointers. Code/trampoline retained.
    unsafe { tx.commit(Threads::All) }.map_err(|e| e.to_string())?;
    ACTIVE.store(true, Ordering::Release);
    ssao::publish(host)?;
    *EFFECTS.write().map_err(|_| "effect provider lock")? = Some(rekenshi_plugin_sdk::operations::Operations::from_host(host)
        .and_then(|operations| operations.publish(rekenshi_plugin_sdk::operations::Contract::new(
            rekenshi_plugin_sdk::render::EFFECTS, "rekenshi.effect-list-le@1",
            "Synchronous renderer lane; copied HLSL and constants; native COM resources remain provider-owned",
            "D3D11 Draw vtable, inspected HDR lighting then LDR tonemap target; resources rebuilt for device/size/format identity", 9*1024*1024)?))
        .map_err(|e| e.to_string())?);
    EFFECT_REVISION.store(0, Ordering::Release);
    host.write_log("D3D11 Draw hook active; awaiting world HDR resolve; GUI excluded");
    Ok(())
}
pub(super) fn stop() -> i32 {
    std::panic::catch_unwind(|| {
        let Ok(mut effects) = EFFECTS.write() else {
            return ERROR;
        };
        if let Some(provider) = effects.as_mut() {
            if provider.revoke().is_err() {
                return ERROR;
            }
        }
        *effects = None;
        drop(effects);
        if ssao::revoke().is_err() {
            return ERROR;
        }
        ACTIVE.store(false, Ordering::Release);
        if let Some(h) = HOOKS.get() {
            let mut tx = Transaction::new();
            tx.disable(&h.draw);
            tx.disable(&h.create);
            // SAFETY: Same patching invariants as start; retain the trampoline.
            if unsafe { tx.commit(Threads::All) }.is_err() {
                return ERROR;
            }
        }
        let Ok(mut backend) = BACKEND.lock() else {
            return ERROR;
        };
        if let Some(b) = backend.take() {
            b.host.write_log(&format!(
                "graphics stopped; inspected draws={} effect passes={}",
                DRAWS.load(Ordering::Relaxed),
                PASSES.load(Ordering::Relaxed)
            ));
        }
        OK
    })
    .unwrap_or(ERROR)
}

/// Negotiate the independent graphics service. No effect is implicitly enabled.
/// # Safety
/// Accepted requests supply aligned writable GraphicsV1 output storage.
// SAFETY: Unique versioned C export, independent of the plugin V1 layout.
pub unsafe extern "C" fn rekenshi_graphics_api(
    version: u32,
    output: *mut GraphicsV1,
    size: usize,
) -> i32 {
    if version != VERSION || size != size_of::<GraphicsV1>() {
        return INCOMPATIBLE;
    }
    if output.is_null() || !ACTIVE.load(Ordering::Acquire) {
        return ERROR;
    }
    // SAFETY: Negotiated writable storage guaranteed by the caller.
    unsafe {
        output.write(GraphicsV1 {
            struct_size: size_of::<GraphicsV1>() as u32,
            abi_version: VERSION,
            register,
            remove,
        })
    };
    OK
}
/// # Safety
/// Accepted requests supply aligned writable GraphicsV2 storage.
pub unsafe extern "C" fn api_v2(version: u32, output: *mut GraphicsV2, size: usize) -> i32 {
    if version != VERSION_V2 || size != size_of::<GraphicsV2>() {
        return INCOMPATIBLE;
    }
    if output.is_null() || !ACTIVE.load(Ordering::Acquire) {
        return ERROR;
    }
    // SAFETY: Caller guarantees negotiated writable storage.
    unsafe {
        output.write(GraphicsV2 {
            struct_size: size as u32,
            abi_version: VERSION_V2,
            register,
            remove,
            update,
        });
    }
    OK
}
unsafe extern "C" fn update(token: u64, handle: u64, parameters: *const f32, count: usize) -> i32 {
    std::panic::catch_unwind(|| {
        if parameters.is_null() || count != 16 {
            return ERROR;
        }
        // SAFETY: Caller guarantees count readable floats for this synchronous call.
        let parameters = unsafe { std::slice::from_raw_parts(parameters, count) };
        let Ok(parameters) = parameters.try_into() else {
            return ERROR;
        };
        if update_effect(token, handle, parameters).is_ok() {
            OK
        } else {
            ERROR
        }
    })
    .unwrap_or(ERROR)
}
unsafe extern "C" fn register(token: u64, pointer: *const EffectV1, output: *mut u64) -> i32 {
    std::panic::catch_unwind(|| {
        if token == 0 || pointer.is_null() || output.is_null() || !ACTIVE.load(Ordering::Acquire) {
            return ERROR;
        }
        // SAFETY: Service caller supplies a readable descriptor for this call.
        let desc = unsafe { pointer.read() };
        if desc.struct_size as usize != size_of::<EffectV1>()
            || desc.stage != POST_TONEMAP
            || desc.reserved != 0
            || desc.hlsl.is_null()
            || desc.hlsl_len == 0
            || desc.hlsl_len > 65536
            || desc.parameters.iter().any(|x| !x.is_finite())
        {
            return INCOMPATIBLE;
        }
        let Ok(mut backend) = BACKEND.lock() else {
            return ERROR;
        };
        let Some(b) = backend.as_mut() else {
            return ERROR;
        };
        if b.effects.len() >= 32 {
            return ERROR;
        }
        // SAFETY: Effect descriptor's source is readable until registration returns.
        let source = unsafe { std::slice::from_raw_parts(desc.hlsl, desc.hlsl_len) };
        let code = match shader::compile(&b.game, source, false) {
            Ok(code) => code,
            Err(e) => {
                b.host.write_log(&format!("effect compile failed: {e}"));
                return ERROR;
            }
        };
        b.serial += 1;
        let handle = b.serial;
        b.effects.push(Effect {
            token,
            handle,
            priority: desc.priority,
            code,
            parameters: desc.parameters,
        });
        b.effects.sort_by_key(|e| (e.priority, e.handle));
        b.gpu = None;
        b.failed = false;
        // SAFETY: Caller supplied writable u64 handle output; backend owns all copies.
        unsafe { output.write(handle) };
        b.host.write_log(&format!(
            "post-tonemap effect registered: handle={handle} owner={token}"
        ));
        OK
    })
    .unwrap_or(ERROR)
}
unsafe extern "C" fn remove(token: u64, handle: u64) -> i32 {
    std::panic::catch_unwind(|| {
        let Ok(mut backend) = BACKEND.lock() else {
            return ERROR;
        };
        let Some(b) = backend.as_mut() else {
            return ERROR;
        };
        let Some(i) = b
            .effects
            .iter()
            .position(|e| e.token == token && e.handle == handle)
        else {
            return ERROR;
        };
        b.effects.remove(i);
        b.gpu = None;
        OK
    })
    .unwrap_or(ERROR)
}
unsafe extern "system" fn create_pixel(
    this: *mut c_void,
    code: *const c_void,
    length: usize,
    linkage: *mut c_void,
    output: *mut *mut c_void,
) -> i32 {
    let Some(h) = HOOKS.get() else {
        return -2147467259;
    };
    // SAFETY: Forward the original COM method unchanged, tag only successful outputs.
    unsafe {
        let result = h.create.call(this, code, length, linkage, output);
        if result >= 0 && !output.is_null() && ACTIVE.load(Ordering::Acquire) {
            ssao::tag(code, length, output.read());
        }
        result
    }
}
unsafe extern "system" fn intercepted(this: *mut c_void, count: u32, first: u32) {
    let Some(h) = HOOKS.get() else { return };
    if !ACTIVE.load(Ordering::Acquire)
        || !matches!(count, 3 | 4)
        || INSIDE.with(|c| c.replace(true))
    {
        // SAFETY: Original native arguments; inactive/reentrant draws forward once.
        unsafe { h.draw.call(this, count, first) };
        return;
    }
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            INSIDE.with(|c| c.set(false));
        }
    }
    let _reset = Reset;
    let binding = std::panic::catch_unwind(|| {
        let mut backend = BACKEND.lock().ok()?;
        let b = backend.as_mut()?;
        // SAFETY: Borrowed native Draw context lives through the synchronous call.
        let context = unsafe { ID3D11DeviceContext::from_raw_borrowed(&this) }?;
        // The private shader tag is our positive identification; avoid another
        // full render-target/resource scan on every draw before the original.
        match ssao::bind(context, &mut b.ssao_gpu, b.ssao) {
            Ok(binding) => {
                if binding.is_some() && AO_BINDINGS.fetch_add(1, Ordering::Relaxed) == 0 {
                    b.host.write_log(
                        "SSAO: tagged deferred lighting draw bound; ambient-only shading available",
                    );
                }
                binding
            }
            Err(error) => {
                if AO_BINDING_ERRORS.fetch_add(1, Ordering::Relaxed) == 0 {
                    b.host.write_log(&format!("SSAO binding skipped: {error}"));
                }
                None
            }
        }
    })
    .ok()
    .flatten();
    // SAFETY: The exact original draw runs with scoped host constants, when tagged.
    unsafe { h.draw.call(this, count, first) };
    drop(binding);
    let _ = std::panic::catch_unwind(|| {
        let Ok(mut backend) = BACKEND.lock() else {
            return;
        };
        let Some(b) = backend.as_mut() else { return };
        if !ACTIVE.load(Ordering::Acquire) {
            return;
        }
        refresh_effects(b);
        DRAWS.fetch_add(1, Ordering::Relaxed);
        // SAFETY: Draw's COM this pointer is borrowed for this synchronous callback.
        let Some(context) = (unsafe { ID3D11DeviceContext::from_raw_borrowed(&this) }) else {
            return;
        };
        match pipeline::detect(context) {
            Some(pipeline::Pass::Lighting) => {
                b.context = this as usize;
                b.saw_lighting = true;
            }
            Some(pipeline::Pass::Tonemap(target, desc))
                if b.saw_lighting
                    && b.context == this as usize
                    && !b.effects.is_empty()
                    && !b.failed =>
            {
                match pipeline::render(b, context, &target, &desc) {
                    Ok(true) => {}
                    Ok(false) => return,
                    Err(e) => {
                        b.host.write_log(&format!("graphics pass disabled: {e}"));
                        b.failed = true;
                        b.gpu = None;
                        return;
                    }
                }
                let passes = PASSES.fetch_add(1, Ordering::Relaxed) + 1;
                if passes == 1 {
                    b.host.write_log(&format!(
                        "first world effect pass: {}x{} format={} before GUI",
                        desc.Width, desc.Height, desc.Format.0
                    ));
                }
            }
            _ => {}
        }
    });
}
fn refresh_effects(backend: &mut Backend) {
    let Ok(provider) = EFFECTS.read() else {
        return;
    };
    let Some(provider) = provider.as_ref() else {
        return;
    };
    let Ok(revision) = provider.stamp() else {
        return;
    };
    if EFFECT_REVISION.load(Ordering::Acquire) == revision {
        return;
    }
    let specifications = provider
        .dispatch(&[], &[])
        .and_then(|bytes| rekenshi_plugin_sdk::render::decode_effects(&bytes));
    let result: Result<Vec<Effect>, String> =
        specifications
            .map_err(|e| e.to_string())
            .and_then(|specifications| {
                specifications
                    .into_iter()
                    .enumerate()
                    .map(|(index, effect)| {
                        Ok(Effect {
                            token: 0,
                            handle: index as u64,
                            priority: effect.priority,
                            code: shader::compile(&backend.game, &effect.source, false)?,
                            parameters: effect.parameters,
                        })
                    })
                    .collect()
            });
    // Remove retired policy resources even when a replacement fails to compile.
    backend.effects.retain(|effect| effect.token != 0);
    match result {
        Ok(effects) => backend.effects.extend(effects),
        Err(error) => backend
            .host
            .write_log(&format!("desired postprocess rejected: {error}")),
    }
    backend
        .effects
        .sort_by_key(|effect| (effect.priority, effect.token, effect.handle));
    backend.gpu = None;
    backend.failed = false;
    EFFECT_REVISION.store(revision, Ordering::Release);
}
// SAFETY: Scalar status export with no pointers.
pub fn passes() -> u64 {
    PASSES.load(Ordering::Relaxed)
}

pub(super) fn update_effect(token: u64, handle: u64, parameters: [f32; 16]) -> Result<(), String> {
    if parameters.iter().any(|x| !x.is_finite()) {
        return Err("effect parameters must be finite".into());
    }
    let mut backend = BACKEND.lock().map_err(|_| "graphics lock poisoned")?;
    let b = backend.as_mut().ok_or("graphics unavailable")?;
    let effect = b
        .effects
        .iter_mut()
        .find(|e| e.token == token && e.handle == handle)
        .ok_or("effect unavailable")?;
    effect.parameters = parameters;
    b.gpu = None;
    b.failed = false;
    Ok(())
}
