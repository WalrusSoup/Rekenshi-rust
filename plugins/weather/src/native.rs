//! Fingerprinted ParticleUniverse -> Ogre billboard render seam. No particle
//! ownership, simulation changes, persistent object pointers or asset rewriting.
use super::policy::{RainStyle, WeatherKind, classify};
use detour::{RawDetour, Threads, Transaction};
use rekenshi_native::ogre;
use rekenshi_native::symbols::ogre::{billboard_set, camera, movable_object, node};
use rekenshi_native::symbols::particle_universe::{
    billboard_renderer, particle_renderer, particle_system, particle_technique,
};
use rekenshi_plugin_api::HostV1;
use rekenshi_plugin_sdk::adapters::{self, AdapterOperations};
use std::{
    cell::Cell,
    collections::BTreeMap,
    ffi::c_void,
    mem::MaybeUninit,
    path::Path,
    sync::{
        OnceLock, RwLock,
        atomic::{AtomicBool, Ordering},
    },
};

type Queue = unsafe extern "C-unwind" fn(
    *mut c_void,
    *mut c_void,
    *const c_void,
    *const c_void,
    *mut c_void,
);
type Inject = unsafe extern "C-unwind" fn(*mut c_void, *const c_void, *const c_void);
type Pointer = unsafe extern "C-unwind" fn(*const c_void) -> *const c_void;
type Scalar = unsafe extern "C" fn(*const c_void) -> f32;
type WorldSpace = unsafe extern "C" fn(*const c_void) -> bool;
type Dimensions = unsafe extern "C" fn(*mut c_void, f32, f32);
#[repr(C, align(8))]
struct BillboardStorage([u8; 0x58]);
struct State {
    platform: Option<rekenshi_plugin_api::platform::Platform>,
    queue: RawDetour,
    inject: RawDetour,
    targets: [usize; 2],
    parent_technique: Pointer,
    material: Pointer,
    parent_system: Pointer,
    template: Pointer,
    width: Scalar,
    height: Scalar,
    dimensions: Dimensions,
    world_space: WorldSpace,
    parent_node: Pointer,
    transform: Pointer,
    camera_position: Pointer,
}
static STATE: OnceLock<State> = OnceLock::new();
#[derive(Default)]
struct Config {
    toned_down: bool,
    rain: BTreeMap<u64, RainStyle>,
}
static CONFIG: RwLock<Config> = RwLock::new(Config {
    toned_down: false,
    rain: BTreeMap::new(),
});
static ACTIVE: AtomicBool = AtomicBool::new(false);
static OPERATIONS: std::sync::RwLock<Option<AdapterOperations>> = std::sync::RwLock::new(None);
static RAIN: RwLock<Option<rekenshi_plugin_sdk::policies::PolicyProvider>> = RwLock::new(None);
static POLICY: RwLock<Option<rekenshi_plugin_sdk::policies::PolicyProvider>> = RwLock::new(None);
pub(super) fn available() -> bool {
    ACTIVE.load(Ordering::Acquire)
}

pub(super) fn set_rain(token: u64, style: Option<RainStyle>) -> bool {
    let mut config = CONFIG.write().unwrap_or_else(|e| e.into_inner());
    if let Some(style) = style {
        // Check while holding the style lock: revocation followed by release
        // cannot race a worker into reinstating a stopped plugin's style.
        if !available() || !allowed(token) {
            return false;
        }
        config.rain.insert(token, style);
    } else {
        config.rain.remove(&token);
    }
    true
}
#[derive(Clone, Copy)]
struct DrawScope {
    set: usize,
    style: Option<RainStyle>,
    kind: WeatherKind,
    appearance: [f32; 2],
    public_particles: bool,
    camera: Option<[f32; 3]>,
    transform: Option<[f32; 16]>,
}
thread_local! { static DRAW: Cell<Option<DrawScope>> = const { Cell::new(None) }; }

pub(super) fn allowed(token: u64) -> bool {
    STATE
        .get()
        .and_then(|state| state.platform)
        .is_some_and(|platform| platform.authorized(token))
}

pub(super) fn start(host: HostV1, game: &Path) -> Result<(), String> {
    let ogre = ogre::initialize(game)?;
    if STATE.get().is_none() {
        let path = game.join("Plugin_ParticleUniverse_x64.dll");
        ogre::verify(
            &path,
            "24705F105A13C596179964082B89CC5997FE70091CECC030A4CC380E52AD61A0",
        )?;
        let particles = rekenshi_native::load_pinned(&path)?;
        let queue = rekenshi_native::export(particles, billboard_renderer::UPDATE_RENDER_QUEUE)?;
        let inject = rekenshi_native::export(ogre.base, billboard_set::INJECT_BILLBOARD)?;
        // SAFETY: Both binaries are hash-gated. Export signatures and the renderer
        // set pointer at 0xd8 were corroborated by their native callers/bodies.
        let state = unsafe {
            State {
                platform: host.platform().ok(),
                queue: RawDetour::new(queue.cast(), render_queue as *const ())
                    .map_err(|e| e.to_string())?,
                inject: RawDetour::new(inject.cast(), inject_billboard as *const ())
                    .map_err(|e| e.to_string())?,
                targets: [queue as usize, inject as usize],
                parent_technique: std::mem::transmute::<*const c_void, Pointer>(
                    rekenshi_native::export(particles, particle_renderer::GET_PARENT_TECHNIQUE)?,
                ),
                material: std::mem::transmute::<*const c_void, Pointer>(rekenshi_native::export(
                    particles,
                    particle_technique::GET_MATERIAL_NAME,
                )?),
                parent_system: std::mem::transmute::<*const c_void, Pointer>(
                    rekenshi_native::export(particles, particle_technique::GET_PARENT_SYSTEM)?,
                ),
                template: std::mem::transmute::<*const c_void, Pointer>(rekenshi_native::export(
                    particles,
                    particle_system::GET_TEMPLATE_NAME,
                )?),
                width: std::mem::transmute::<*const c_void, Scalar>(rekenshi_native::export(
                    ogre.base,
                    billboard_set::GET_DEFAULT_WIDTH,
                )?),
                height: std::mem::transmute::<*const c_void, Scalar>(rekenshi_native::export(
                    ogre.base,
                    billboard_set::GET_DEFAULT_HEIGHT,
                )?),
                dimensions: std::mem::transmute::<*const c_void, Dimensions>(
                    rekenshi_native::export(ogre.base, billboard_set::SET_DEFAULT_DIMENSIONS)?,
                ),
                world_space: std::mem::transmute::<*const c_void, WorldSpace>(
                    rekenshi_native::export(
                        ogre.base,
                        billboard_set::GET_BILLBOARDS_IN_WORLD_SPACE,
                    )?,
                ),
                parent_node: std::mem::transmute::<*const c_void, Pointer>(
                    rekenshi_native::export(ogre.base, movable_object::GET_PARENT_SCENE_NODE)?,
                ),
                transform: std::mem::transmute::<*const c_void, Pointer>(rekenshi_native::export(
                    ogre.base,
                    node::GET_FULL_TRANSFORM,
                )?),
                camera_position: std::mem::transmute::<*const c_void, Pointer>(
                    rekenshi_native::export(ogre.base, camera::GET_DERIVED_POSITION)?,
                ),
            }
        };
        STATE
            .set(state)
            .map_err(|_| "weather adapter initialized twice")?;
    }
    let state = STATE.get().ok_or("weather adapter")?;
    for target in state.targets {
        host.reserve_hook(std::ptr::NonNull::new(target as *mut c_void).ok_or("weather target")?)
            .map_err(|e| format!("weather hook conflict: {e}"))?;
    }
    let mut tx = Transaction::new();
    tx.enable(&state.queue);
    tx.enable(&state.inject);
    // SAFETY: Startup outside DllMain; suspend native callers while patching.
    unsafe { tx.commit(Threads::All) }.map_err(|e| e.to_string())?;
    ACTIVE.store(true, Ordering::Release);
    *OPERATIONS.write().map_err(|_| "operation registry lock")? = Some(AdapterOperations::publish(host, "Existing adapter fingerprint and native signatures validated before hook enable; see implementation inventory and adapter source for evidence limits", &[(adapters::PARTICLE,"rekenshi.billboard-scalars@1",40)]).map_err(|e| e.to_string())?);
    *RAIN.write().map_err(|_| "rain policy lock")? = Some(rekenshi_plugin_sdk::policies::PolicyProvider::composed(host,rekenshi_plugin_sdk::policies::RAIN,
        "Fingerprint-gated ParticleUniverse render queue and Ogre billboard injection; scoped geometry and camera fade; native dimensions restored on unwind").map_err(|e| e.to_string())?);
    *POLICY.write().map_err(|_| "particle policy lock")? = rekenshi_plugin_sdk::policies::PolicyProvider::optional(host, rekenshi_plugin_sdk::policies::WEATHER,
        "ParticleUniverse SHA256 24705F105A13C596179964082B89CC5997FE70091CECC030A4CC380E52AD61A0 and hash-gated Ogre; renderer queue and scoped billboard copies; no emitter lifetime/readiness claim").map_err(|e| e.to_string())?;
    host.write_log("weather: shared native particle renderer mounted");
    Ok(())
}
pub(super) fn stop() -> Result<(), String> {
    let mut rain = RAIN.write().map_err(|_| "rain policy lock")?;
    if let Some(provider) = rain.as_mut() {
        provider.revoke().map_err(|e| e.to_string())?;
    }
    *rain = None;
    drop(rain);
    if let Some(mut operations) = OPERATIONS
        .write()
        .map_err(|_| "operation registry lock")?
        .take()
    {
        operations.revoke().map_err(|e| e.to_string())?;
    }
    if let Some(mut policy) = POLICY.write().map_err(|_| "particle policy lock")?.take() {
        policy.revoke().map_err(|e| e.to_string())?;
    }
    ACTIVE.store(false, Ordering::Release);
    *CONFIG.write().unwrap_or_else(|e| e.into_inner()) = Config::default();
    if let Some(state) = STATE.get() {
        let mut tx = Transaction::new();
        tx.disable(&state.queue);
        tx.disable(&state.inject);
        // SAFETY: Host shutdown quiesces rendering; retain trampolines for process life.
        unsafe { tx.commit(Threads::All) }.map_err(|e| e.to_string())?;
    }
    Ok(())
}
struct Restore {
    set: usize,
    width: f32,
    height: f32,
    previous: Option<DrawScope>,
}
impl Drop for Restore {
    fn drop(&mut self) {
        DRAW.set(self.previous);
        if let Some(state) = STATE.get() {
            // SAFETY: The set is borrowed from this synchronous renderer call and
            // remains live even if a native exception unwinds the original call.
            unsafe {
                (state.dimensions)(self.set as *mut c_void, self.width, self.height);
            }
        }
    }
}
unsafe extern "C-unwind" fn render_queue(
    this: *mut c_void,
    queue: *mut c_void,
    camera: *const c_void,
    lod: *const c_void,
    pool: *mut c_void,
) {
    let Some(state) = STATE.get() else {
        return;
    };
    let (toned_down, mut style) = {
        let config = CONFIG.read().unwrap_or_else(|e| e.into_inner());
        // Multiple refinement plugins combine by choosing the least intrusive size
        // and lowest visibility, rather than repeatedly shrinking the same drop.
        let style = config.rain.values().copied().reduce(|a, b| RainStyle {
            visibility: a.visibility.min(b.visibility),
            size: a.size.min(b.size),
        });
        (config.toned_down, style)
    };
    if let Some(values) = RAIN.read().ok().and_then(|provider| {
        provider
            .as_ref()
            .map(|provider| provider.values([0., 1., 2., 0.]))
    }) {
        if values[0] == 1. && (0. ..=1.).contains(&values[1]) && (0.1..=2.).contains(&values[2]) {
            let public = RainStyle {
                visibility: values[1],
                size: values[2],
            };
            style = Some(style.map_or(public, |existing| RainStyle {
                visibility: existing.visibility.min(public.visibility),
                size: existing.size.min(public.size),
            }));
        }
    }
    let appearance = POLICY
        .read()
        .ok()
        .and_then(|p| p.as_ref().map(|p| p.values([1., 1., 0., 0.])))
        .map_or([if toned_down { 0.35 } else { 1. }; 2], |p| {
            [p[0].clamp(0., 1.), p[1].clamp(0., 1.)]
        });
    let toned_down = appearance != [1.; 2];
    let public_particles = OPERATIONS
        .read()
        .ok()
        .and_then(|o| o.as_ref().map(|o| o.active(adapters::PARTICLE)))
        .unwrap_or(false);
    // SAFETY: Exact native member ABI. Only synchronous live objects from the
    // engine are accessed; no pointer escapes this renderer call.
    unsafe {
        let original: Queue = std::mem::transmute(state.queue.trampoline());
        let technique = (state.parent_technique)(this);
        let kind = if (!toned_down && style.is_none() && !public_particles) || technique.is_null() {
            None
        } else {
            let system = (state.parent_system)(technique);
            let template = if system.is_null() {
                None
            } else {
                ogre::string((state.template)(system))
            };
            ogre::string((state.material)(technique))
                .and_then(|name| classify(&name, template.as_deref().unwrap_or_default()))
        };
        let style = style.filter(|_| kind == Some(WeatherKind::Rain));
        let set = this.cast::<u8>().add(0xd8).cast::<*mut c_void>().read();
        if let Some(kind) =
            kind.filter(|_| (toned_down || style.is_some() || public_particles) && !set.is_null())
        {
            let width = (state.width)(set);
            let height = (state.height)(set);
            let transform = if style.is_none() || (state.world_space)(set) {
                None
            } else {
                let node = (state.parent_node)(set);
                if node.is_null() {
                    None
                } else {
                    Some((state.transform)(node).cast::<[f32; 16]>().read())
                }
            };
            let camera_position = if style.is_none() || camera.is_null() {
                None
            } else {
                Some((state.camera_position)(camera).cast::<[f32; 3]>().read())
            };
            let previous = DRAW.replace(Some(DrawScope {
                set: set as usize,
                style,
                kind,
                appearance,
                public_particles,
                camera: camera_position,
                transform,
            }));
            let _restore = Restore {
                set: set as usize,
                width,
                height,
                previous,
            };
            if let Some(style) = style {
                let (width, height) = style.dimensions(width, height);
                (state.dimensions)(set, width, height);
            }
            original(this, queue, camera, lod, pool);
        } else {
            // Nested unrelated rendering must not inherit a weather scope.
            let previous = DRAW.replace(None);
            struct Reset(Option<DrawScope>);
            impl Drop for Reset {
                fn drop(&mut self) {
                    DRAW.set(self.0);
                }
            }
            let _reset = Reset(previous);
            original(this, queue, camera, lod, pool);
        }
    }
}
unsafe extern "C-unwind" fn inject_billboard(
    this: *mut c_void,
    billboard: *const c_void,
    camera: *const c_void,
) {
    let Some(state) = STATE.get() else {
        return;
    };
    // SAFETY: Fingerprinted Ogre Billboard is a non-owning 0x58-byte value;
    // padding/unused dimensions may be uninitialized, so copy opaque storage
    // without interpreting it. Native caller retains ownership of the original.
    unsafe {
        let original: Inject = std::mem::transmute(state.inject.trampoline());
        if let Some(scope) = DRAW.get().filter(|scope| scope.set == this as usize) {
            let mut copy = MaybeUninit::<BillboardStorage>::uninit();
            std::ptr::copy_nonoverlapping(
                billboard.cast::<u8>(),
                copy.as_mut_ptr().cast::<u8>(),
                0x58,
            );
            let pointer = copy.as_mut_ptr().cast::<u8>();
            if let Some(style) = scope.style.filter(|_| pointer.read() != 0) {
                let width = pointer.add(0x14).cast::<f32>().read();
                let height = pointer.add(0x18).cast::<f32>().read();
                let (width, height) = style.dimensions(width, height);
                pointer.add(0x14).cast::<f32>().write(width);
                pointer.add(0x18).cast::<f32>().write(height);
            }
            let position = pointer.add(0x1c).cast::<[f32; 3]>().read();
            let brightness = scope.style.map_or(1., |style| {
                style.brightness_at(position, scope.camera, scope.transform)
            }) * if scope.kind != WeatherKind::Alpha {
                scope.appearance[0]
            } else {
                1.
            };
            if scope.kind == WeatherKind::Alpha {
                let alpha = pointer.add(0x4c).cast::<f32>();
                alpha.write(alpha.read() * scope.appearance[1]);
            }
            for channel in 0..3 {
                let colour = pointer.add(0x40 + channel * 4).cast::<f32>();
                colour.write(colour.read() * brightness);
            }
            if scope.public_particles {
                let own = pointer.read() != 0;
                let mut values = Vec::with_capacity(40);
                for offset in [0x1c, 0x20, 0x24, 0x40, 0x44, 0x48, 0x4c] {
                    values
                        .extend_from_slice(&pointer.add(offset).cast::<f32>().read().to_le_bytes());
                }
                for offset in [0x14, 0x18] {
                    values.extend_from_slice(
                        &if own {
                            pointer.add(offset).cast::<f32>().read()
                        } else {
                            0.
                        }
                        .to_le_bytes(),
                    );
                }
                values.extend_from_slice(&(own as u32).to_le_bytes());
                let kind = match scope.kind {
                    WeatherKind::Rain => 0u32,
                    WeatherKind::Additive => 1,
                    WeatherKind::Alpha => 2,
                };
                if let Some(result) = OPERATIONS
                    .read()
                    .ok()
                    .and_then(|o| {
                        o.as_ref().and_then(|o| {
                            o.transform(adapters::PARTICLE, &kind.to_le_bytes(), &values)
                        })
                    })
                    .filter(|b| b.len() == 40)
                {
                    let scalars: Vec<_> = result[..36]
                        .chunks_exact(4)
                        .map(|b| f32::from_le_bytes(b.try_into().expect("exact four-byte chunk")))
                        .collect();
                    let flag =
                        u32::from_le_bytes(result[36..].try_into().expect("validated flag size"));
                    if scalars.iter().all(|v| v.is_finite())
                        && flag <= 1
                        && (flag == 0 || scalars[7..].iter().all(|v| (0.001..=10000.).contains(v)))
                    {
                        for (index, offset) in [0x1c, 0x20, 0x24, 0x40, 0x44, 0x48, 0x4c]
                            .into_iter()
                            .enumerate()
                        {
                            pointer.add(offset).cast::<f32>().write(scalars[index]);
                        }
                        pointer.write(flag as u8);
                        if flag == 1 {
                            pointer.add(0x14).cast::<f32>().write(scalars[7]);
                            pointer.add(0x18).cast::<f32>().write(scalars[8]);
                        }
                    }
                }
            }
            original(this, pointer.cast(), camera);
        } else {
            original(this, billboard, camera);
        }
    }
}
