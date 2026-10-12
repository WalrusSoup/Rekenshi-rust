//! Material and shader ownership for the fingerprinted Forest instancing path.
//!
//! Preparation and retirement run synchronously on the native batch owner lane.
//! Original materials are retained until clear/rebuild; registered clones are
//! removed explicitly so repeated region loading does not accumulate resources.
use rekenshi_native::ogre::{NativeString, Shared};
use std::{ffi::c_void, marker::PhantomData, rc::Rc};

type Ptr = *mut c_void;
type SharedCopy = unsafe extern "C-unwind" fn(*mut Shared, *const Shared) -> *mut Shared;
type Release = unsafe extern "C-unwind" fn(*mut Shared);
type CloneMaterial = unsafe extern "C-unwind" fn(
    Ptr,
    *mut Shared,
    *const c_void,
    bool,
    *const c_void,
) -> *mut Shared;
type GetMaterial =
    unsafe extern "C-unwind" fn(Ptr, *mut Shared, *const c_void, *const c_void) -> *mut Shared;
type SetCaster = unsafe extern "C-unwind" fn(Ptr, *mut Shared);
type CloneProgram = unsafe extern "C-unwind" fn(Ptr, *mut Shared, *const c_void) -> *mut Shared;
type Singleton = unsafe extern "C-unwind" fn() -> Ptr;
type Remove = unsafe extern "C-unwind" fn(Ptr, *const c_void);
type GetAt = unsafe extern "C-unwind" fn(Ptr, u16) -> Ptr;
type GetCount = unsafe extern "C-unwind" fn(Ptr) -> u16;
type GetShared = unsafe extern "C-unwind" fn(Ptr) -> *const Shared;
type GetOwned = unsafe extern "C-unwind" fn(Ptr, *mut Shared) -> *mut Shared;
type CopyParameters = unsafe extern "C-unwind" fn(Ptr, Ptr);
type GetBool = unsafe extern "C-unwind" fn(Ptr) -> bool;
type Load = unsafe extern "C-unwind" fn(Ptr, bool);
type SetVertex = unsafe extern "C-unwind" fn(Ptr, *const c_void, bool);
type SetShadow = unsafe extern "C-unwind" fn(Ptr, *const c_void);
type Name = unsafe extern "C-unwind" fn(Ptr) -> *const c_void;
type GetProperty =
    unsafe extern "C-unwind" fn(Ptr, *mut NativeString, *const c_void) -> *mut NativeString;
type SetProperty = unsafe extern "C-unwind" fn(Ptr, *const c_void, *const c_void) -> bool;
type SetSource = unsafe extern "C-unwind" fn(Ptr, *const c_void);
type Delete = unsafe extern "C-unwind" fn(Ptr);

#[derive(Clone, Copy, Debug)]
pub(super) struct Bindings {
    copy_shared: SharedCopy,
    release: Release,
    clone_material: CloneMaterial,
    clone_program: CloneProgram,
    get_material: GetMaterial,
    set_caster: SetCaster,
    material_manager: Singleton,
    program_manager: Singleton,
    remove: Remove,
    get_technique: GetAt,
    get_pass: GetAt,
    num_techniques: GetCount,
    num_passes: GetCount,
    shadow_material: GetOwned,
    vertex: GetShared,
    shadow_vertex: GetShared,
    vertex_parameters: GetOwned,
    shadow_parameters: GetOwned,
    copy_parameters: CopyParameters,
    delegate: GetShared,
    transparent: GetBool,
    has_vertex: GetBool,
    has_shadow: GetBool,
    load: Load,
    supported: GetBool,
    set_vertex: SetVertex,
    set_shadow: SetShadow,
    delete_string: Delete,
}
impl Bindings {
    /// # Safety
    /// `base` must be the pinned, initialized Ogre module; its lifetime must
    /// include every prepared material and retirement recipe created here.
    pub(super) unsafe fn new(base: usize) -> Result<Self, String> {
        // SAFETY: Caller guarantees the exact fingerprint. Export signatures and
        // private setters were executed by test-instancing-material.py.
        unsafe {
            Ok(Self {
                copy_shared: std::mem::transmute::<*const c_void, SharedCopy>(rekenshi_native::export(base, c"??0?$SharedPtr@VHardwareCounterBuffer@Ogre@@@Ogre@@QEAA@AEBV01@@Z")?),
                release: std::mem::transmute::<*const c_void, Release>(rekenshi_native::export(base, c"??1?$SharedPtr@VHardwareCounterBuffer@Ogre@@@Ogre@@QEAA@XZ")?),
                clone_material: std::mem::transmute::<*const c_void, CloneMaterial>(rekenshi_native::export(base, c"?clone@Material@Ogre@@QEBA?AV?$SharedPtr@VMaterial@Ogre@@@2@AEBV?$basic_string@DU?$char_traits@D@std@@V?$allocator@D@2@@std@@_N0@Z")?),
                clone_program: std::mem::transmute::<*const c_void, CloneProgram>(rekenshi_native::export(base, c"?clone@HighLevelGpuProgram@Ogre@@QEAA?AV?$SharedPtr@VHighLevelGpuProgram@Ogre@@@2@AEBV?$basic_string@DU?$char_traits@D@std@@V?$allocator@D@2@@std@@@Z")?),
                material_manager: std::mem::transmute::<*const c_void, Singleton>(rekenshi_native::export(base, c"?getSingletonPtr@MaterialManager@Ogre@@SAPEAV12@XZ")?),
                program_manager: std::mem::transmute::<*const c_void, Singleton>(rekenshi_native::export(base, c"?getSingletonPtr@HighLevelGpuProgramManager@Ogre@@SAPEAV12@XZ")?),
                remove: std::mem::transmute::<*const c_void, Remove>(rekenshi_native::export(base, c"?remove@ResourceManager@Ogre@@UEAAXAEBV?$basic_string@DU?$char_traits@D@std@@V?$allocator@D@2@@std@@@Z")?),
                get_technique: std::mem::transmute::<*const c_void, GetAt>(rekenshi_native::export(base, c"?getTechnique@Material@Ogre@@QEAAPEAVTechnique@2@G@Z")?),
                get_pass: std::mem::transmute::<*const c_void, GetAt>(rekenshi_native::export(base, c"?getPass@Technique@Ogre@@QEAAPEAVPass@2@G@Z")?),
                num_techniques: std::mem::transmute::<*const c_void, GetCount>(rekenshi_native::export(base, c"?getNumTechniques@Material@Ogre@@QEBAGXZ")?),
                num_passes: std::mem::transmute::<*const c_void, GetCount>(rekenshi_native::export(base, c"?getNumPasses@Technique@Ogre@@QEBAGXZ")?),
                shadow_material: std::mem::transmute::<*const c_void, GetOwned>(rekenshi_native::export(base, c"?getShadowCasterMaterial@Technique@Ogre@@QEBA?AV?$SharedPtr@VMaterial@Ogre@@@2@XZ")?),
                vertex: std::mem::transmute::<*const c_void, GetShared>(rekenshi_native::export(base, c"?getVertexProgram@Pass@Ogre@@QEBAAEBV?$SharedPtr@VGpuProgram@Ogre@@@2@XZ")?),
                shadow_vertex: std::mem::transmute::<*const c_void, GetShared>(rekenshi_native::export(base, c"?getShadowCasterVertexProgram@Pass@Ogre@@QEBAAEBV?$SharedPtr@VGpuProgram@Ogre@@@2@XZ")?),
                vertex_parameters: std::mem::transmute::<*const c_void, GetOwned>(rekenshi_native::export(base, c"?getVertexProgramParameters@Pass@Ogre@@QEBA?AV?$SharedPtr@VGpuProgramParameters@Ogre@@@2@XZ")?),
                shadow_parameters: std::mem::transmute::<*const c_void, GetOwned>(rekenshi_native::export(base, c"?getShadowCasterVertexProgramParameters@Pass@Ogre@@QEBA?AV?$SharedPtr@VGpuProgramParameters@Ogre@@@2@XZ")?),
                copy_parameters: std::mem::transmute::<*const c_void, CopyParameters>(rekenshi_native::export(base, c"?copyMatchingNamedConstantsFrom@GpuProgramParameters@Ogre@@QEAAXAEBV12@@Z")?),
                delegate: std::mem::transmute::<*const c_void, GetShared>(rekenshi_native::export(base, c"?_getDelegate@UnifiedHighLevelGpuProgram@Ogre@@QEBAAEBV?$SharedPtr@VHighLevelGpuProgram@Ogre@@@2@XZ")?),
                transparent: std::mem::transmute::<*const c_void, GetBool>(rekenshi_native::export(base, c"?isTransparent@Pass@Ogre@@QEBA_NXZ")?),
                has_vertex: std::mem::transmute::<*const c_void, GetBool>(rekenshi_native::export(base, c"?hasVertexProgram@Pass@Ogre@@QEBA_NXZ")?),
                has_shadow: std::mem::transmute::<*const c_void, GetBool>(rekenshi_native::export(base, c"?hasShadowCasterVertexProgram@Pass@Ogre@@QEBA_NXZ")?),
                load: std::mem::transmute::<*const c_void, Load>(rekenshi_native::export(base, c"?load@Resource@Ogre@@UEAAX_N@Z")?),
                supported: std::mem::transmute::<*const c_void, GetBool>(rekenshi_native::export(base, c"?isSupported@GpuProgram@Ogre@@UEBA_NXZ")?),
                get_material: std::mem::transmute::<*const c_void, GetMaterial>(rekenshi_native::export(base, c"?getByName@MaterialManager@Ogre@@QEAA?AV?$SharedPtr@VMaterial@Ogre@@@2@AEBV?$basic_string@DU?$char_traits@D@std@@V?$allocator@D@2@@std@@0@Z")?),
                set_caster: std::mem::transmute::<usize, SetCaster>(base + 0x374960),
                // Private Pass setters preserve native GpuProgramUsage ownership.
                set_vertex: std::mem::transmute::<usize, SetVertex>(base + 0x214c30),
                set_shadow: std::mem::transmute::<usize, SetShadow>(base + 0x218090),
                // MSVCR100 operator delete, as called by native string destructors.
                delete_string: std::mem::transmute::<usize, Delete>(*((base + 0x5b7b40) as *const usize)),
            })
        }
    }

    /// Clone a material and prepare opaque object/shadow shader variants.
    /// # Safety
    /// `source` points to a live native Material SharedPtr on its owner lane.
    /// `name` is globally unique in Ogre's material and program registries.
    /// Ogre must remain initialized until the result/recipe has been retired.
    pub(super) unsafe fn prepare(
        &self,
        source: *const Shared,
        name: &str,
    ) -> Result<Prepared, String> {
        // SAFETY: Caller retains source and serializes native engine access.
        unsafe {
            if source.is_null() || (*source).object == 0 {
                return Err("missing effective material".into());
            }
            let original = Owned::copied(*self, source);
            let mut material = Owned::empty(*self);
            let native_name = NativeString::borrowed(name.as_bytes());
            let empty = NativeString::short(b"");
            (self.clone_material)(
                original.object(),
                &mut material.value,
                native_name.as_ptr(),
                false,
                std::ptr::from_ref(&empty).cast(),
            );
            if material.value.object == 0 {
                return Err("native material clone returned null".into());
            }
            let mut prepared = Prepared {
                api: *self,
                original,
                material,
                registrations: vec![Registration {
                    program: false,
                    name: name.into(),
                }],
                committed: false,
                _lane: PhantomData,
            };
            let techniques = (self.num_techniques)(prepared.material.object());
            if techniques == 0 || techniques > 16 {
                return Err("unsupported material technique count".into());
            }
            for technique_index in 0..techniques {
                let technique = (self.get_technique)(prepared.material.object(), technique_index);
                let mut caster = Owned::empty(*self);
                (self.shadow_material)(technique, &mut caster.value);
                if caster.value.object == 0 {
                    // Technique may retain only a caster name before its first
                    // load. Resolve that retained name without mutating source.
                    let caster_name = native_text((technique as *const u8).add(0x90).cast())?;
                    if !caster_name.is_empty() {
                        let group: Name =
                            std::mem::transmute(vtable(prepared.material.object(), 0x110));
                        let native_caster_name = NativeString::borrowed(caster_name.as_bytes());
                        (self.get_material)(
                            (self.material_manager)(),
                            &mut caster.value,
                            native_caster_name.as_ptr(),
                            group(prepared.material.object()),
                        );
                        if caster.value.object == 0 {
                            return Err("missing named shadow caster material".into());
                        }
                    }
                }
                let has_caster = caster.value.object != 0;
                if has_caster {
                    let caster_name = format!("{name}/t{technique_index}/caster");
                    let cloned_caster = prepared.caster(caster.object(), &caster_name)?;
                    let mut temporary = Owned::copied(*self, &cloned_caster.value);
                    // Native by-value setter consumes and zeros this temporary.
                    (self.set_caster)(technique, &mut temporary.value);
                }
                let passes = (self.num_passes)(technique);
                if passes == 0 || passes > 16 {
                    return Err("unsupported material pass count".into());
                }
                for pass_index in 0..passes {
                    let pass = (self.get_pass)(technique, pass_index);
                    if (self.transparent)(pass)
                        || !(self.has_vertex)(pass)
                        || (!has_caster && !(self.has_shadow)(pass))
                    {
                        return Err(
                            "requires opaque object and supported shadow vertex programs".into(),
                        );
                    }
                    for shadow in [false, true] {
                        if shadow && !(self.has_shadow)(pass) {
                            continue;
                        }
                        let suffix = if shadow { "shadow" } else { "vertex" };
                        let program_name =
                            format!("{name}/t{technique_index}/p{pass_index}/{suffix}");
                        prepared.program(pass, &program_name, shadow, !shadow)?;
                    }
                }
            }
            Ok(prepared)
        }
    }
}

struct Owned {
    value: Shared,
    api: Bindings,
    _lane: PhantomData<Rc<()>>,
}
impl Owned {
    fn empty(api: Bindings) -> Self {
        Self {
            value: Shared::default(),
            api,
            _lane: PhantomData,
        }
    }
    unsafe fn copied(api: Bindings, source: *const Shared) -> Self {
        let mut result = Self::empty(api);
        // SAFETY: Caller supplies a native SharedPtr; copy increments its native
        // control block without transferring either side's ownership.
        unsafe { (api.copy_shared)(&mut result.value, source) };
        result
    }
    fn object(&self) -> Ptr {
        self.value.object as Ptr
    }
}
impl Drop for Owned {
    fn drop(&mut self) {
        // SAFETY: Owned guards are !Send and created/destroyed on the native lane;
        // each contains exactly one native-owned reference, possibly null.
        unsafe { (self.api.release)(&mut self.value) };
    }
}

#[derive(Debug)]
struct Registration {
    program: bool,
    name: String,
}

/// Temporary native ownership; this value cannot cross thread boundaries.
pub(super) struct Prepared {
    api: Bindings,
    original: Owned,
    material: Owned,
    registrations: Vec<Registration>,
    committed: bool,
    _lane: PhantomData<Rc<()>>,
}
impl Prepared {
    unsafe fn caster(&mut self, source: Ptr, name: &str) -> Result<Owned, String> {
        // SAFETY: Source is retained by the caller's Owned caster reference;
        // all clones and program bindings remain confined to this owner lane.
        unsafe {
            let native_name = NativeString::borrowed(name.as_bytes());
            let empty = NativeString::short(b"");
            let mut material = Owned::empty(self.api);
            (self.api.clone_material)(
                source,
                &mut material.value,
                native_name.as_ptr(),
                false,
                std::ptr::from_ref(&empty).cast(),
            );
            if material.value.object == 0 {
                return Err("shadow caster clone returned null".into());
            }
            self.registrations.push(Registration {
                program: false,
                name: name.into(),
            });
            let techniques = (self.api.num_techniques)(material.object());
            if techniques == 0 || techniques > 16 {
                return Err("unsupported caster technique count".into());
            }
            for technique_index in 0..techniques {
                let technique = (self.api.get_technique)(material.object(), technique_index);
                let passes = (self.api.num_passes)(technique);
                if passes == 0 || passes > 16 {
                    return Err("unsupported caster pass count".into());
                }
                for pass_index in 0..passes {
                    let pass = (self.api.get_pass)(technique, pass_index);
                    if !(self.api.has_vertex)(pass) {
                        return Err("caster has no vertex program".into());
                    }
                    let program_name = format!("{name}/t{technique_index}/p{pass_index}/vertex");
                    self.program(pass, &program_name, true, true)?;
                }
            }
            Ok(material)
        }
    }

    unsafe fn program(
        &mut self,
        pass: Ptr,
        name: &str,
        shadow: bool,
        as_vertex: bool,
    ) -> Result<(), String> {
        // SAFETY: All pointers belong to our live clone, entered on the native
        // owner lane. Shared getters are borrowed; owned getters use RAII slots.
        unsafe {
            let getter = if shadow && !as_vertex {
                self.api.shadow_vertex
            } else {
                self.api.vertex
            };
            let source_ref = getter(pass);
            if source_ref.is_null() || (*source_ref).object == 0 {
                return Err("missing source vertex program".into());
            }
            let mut source = (*source_ref).object as Ptr;
            let language: Name = std::mem::transmute(vtable(source, 0x250));
            let mut language_name = native_text(language(source))?;
            if language_name == "unified" {
                let delegate = (self.api.delegate)(source);
                if delegate.is_null() || (*delegate).object == 0 {
                    return Err("unified vertex program has no supported delegate".into());
                }
                source = (*delegate).object as Ptr;
                let language: Name = std::mem::transmute(vtable(source, 0x250));
                language_name = native_text(language(source))?;
            }
            if language_name != "hlsl" {
                return Err("requires HLSL vertex program".into());
            }
            (self.api.load)(source, false);
            let file_name: Name = std::mem::transmute(vtable(source, 0x188));
            let path = native_text(file_name(source))?;
            let leaf = path.rsplit(['/', '\\']).next().unwrap_or("");
            let supported_source = if shadow {
                matches!(leaf, "shadowcaster.hlsl" | "rtwtessellator.hlsl")
            } else {
                leaf == "objects.hlsl"
            };
            if !supported_source {
                return Err(format!("unsupported vertex source {path}"));
            }
            let old_defines = property(self.api, source, "preprocessor_defines")?;
            let defines = instancing_defines(&old_defines, shadow)?;
            let mut parameters = Owned::empty(self.api);
            let get_parameters = if shadow && !as_vertex {
                self.api.shadow_parameters
            } else {
                self.api.vertex_parameters
            };
            get_parameters(pass, &mut parameters.value);
            let mut program = Owned::empty(self.api);
            let native_name = NativeString::borrowed(name.as_bytes());
            (self.api.clone_program)(source, &mut program.value, native_name.as_ptr());
            if program.value.object == 0 {
                return Err("native vertex program clone returned null".into());
            }
            self.registrations.push(Registration {
                program: true,
                name: name.into(),
            });
            let set_property: SetProperty = std::mem::transmute(vtable(program.object(), 8));
            let key = NativeString::borrowed(b"preprocessor_defines");
            let value = NativeString::borrowed(defines.as_bytes());
            if !set_property(program.object(), key.as_ptr(), value.as_ptr()) {
                return Err("native shader rejects preprocessor definitions".into());
            }
            if !shadow {
                let get_source: Name = std::mem::transmute(vtable(source, 0x190));
                let source_text = native_text(get_source(source))?;
                let patched = forest_source(&source_text)?;
                let native_source = NativeString::borrowed(patched.as_bytes());
                let set_source: SetSource = std::mem::transmute(vtable(program.object(), 0x170));
                set_source(program.object(), native_source.as_ptr());
            }
            (self.api.load)(program.object(), false);
            if !(self.api.supported)(program.object()) {
                return Err("instanced vertex program failed native compilation".into());
            }
            if shadow && !as_vertex {
                (self.api.set_shadow)(pass, native_name.as_ptr());
            } else {
                (self.api.set_vertex)(pass, native_name.as_ptr(), true);
            }
            let mut replacement_parameters = Owned::empty(self.api);
            get_parameters(pass, &mut replacement_parameters.value);
            if parameters.value.object != 0 && replacement_parameters.value.object != 0 {
                // Match by uniform name, since compiler register offsets may
                // change when the new instancing inputs/source are introduced.
                (self.api.copy_parameters)(replacement_parameters.object(), parameters.object());
            }
            Ok(())
        }
    }

    /// Install the clone and return inert ownership for native clear/rebuild.
    /// # Safety
    /// `slot` is the same live SubBatch Material SharedPtr supplied to prepare.
    /// Caller must invoke the returned recipe's retirement on its native owner
    /// lane before Ogre shutdown, including clear, rebuild and destruction paths.
    pub(super) unsafe fn commit_into(mut self, slot: *mut Shared) -> RetiredResources {
        // SAFETY: Caller provides exclusive native slot access. Copy an owned
        // clone ref into it and release its old ref; retain original for restore.
        unsafe {
            let mut replacement = Shared::default();
            (self.api.copy_shared)(&mut replacement, &self.material.value);
            let mut old = slot.replace(replacement);
            (self.api.release)(&mut old);
            self.committed = true;
            RetiredResources {
                api: self.api,
                original: std::mem::take(&mut self.original.value),
                installed_object: self.material.value.object,
                registrations: std::mem::take(&mut self.registrations),
            }
        }
    }
}
impl Drop for Prepared {
    fn drop(&mut self) {
        if !self.committed {
            // SAFETY: Uncommitted preparation never leaves the !Send owner lane.
            unsafe { remove_registered(self.api, &self.registrations) };
        }
    }
}

/// Inert native reference ownership; never dereferenced or released by Drop.
///
/// Opaque words may be stored in a synchronized registry, but only the unsafe
/// native-lane retirement method may access them. Deliberately no Clone or Drop:
/// arbitrary-thread registry teardown must not release native engine objects.
#[must_use = "retire on the native clear/rebuild lane before engine shutdown"]
#[derive(Debug)]
pub(super) struct RetiredResources {
    api: Bindings,
    original: Shared,
    installed_object: usize,
    registrations: Vec<Registration>,
}
impl RetiredResources {
    /// Restore the original material and remove all registered clone resources.
    /// # Safety
    /// `slot` is the still-live native Material SharedPtr originally installed.
    /// This runs on its serialized native owner lane before engine shutdown.
    pub(super) unsafe fn restore_and_retire(mut self, slot: *mut Shared) {
        // SAFETY: Recipe owns its retained original reference. Preserve a slot
        // another renderer changed; otherwise transfer the retained ref back.
        unsafe {
            if (*slot).object == self.installed_object {
                let mut old = slot.replace(std::mem::take(&mut self.original));
                (self.api.release)(&mut old);
            } else {
                (self.api.release)(&mut self.original);
            }
            remove_registered(self.api, &self.registrations);
        }
    }
}

unsafe fn remove_registered(api: Bindings, resources: &[Registration]) {
    // SAFETY: Callers guarantee initialized native managers and owner lane.
    unsafe {
        for resource in resources.iter().rev() {
            let manager = if resource.program {
                (api.program_manager)()
            } else {
                (api.material_manager)()
            };
            let name = NativeString::borrowed(resource.name.as_bytes());
            (api.remove)(manager, name.as_ptr());
        }
    }
}
unsafe fn vtable(object: Ptr, offset: usize) -> usize {
    // SAFETY: Caller supplies a live pinned object and verified virtual offset.
    unsafe { *((*(object as *const usize) + offset) as *const usize) }
}
unsafe fn native_text(value: *const c_void) -> Result<String, String> {
    // SAFETY: Borrowed native getter output stays live for this synchronous copy.
    unsafe { rekenshi_native::ogre::string(value) }
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .ok_or_else(|| "invalid native shader string".into())
}
unsafe fn property(api: Bindings, object: Ptr, key: &str) -> Result<String, String> {
    let key = NativeString::borrowed(key.as_bytes());
    let mut result = NativeString::short(b"");
    // SAFETY: The pinned native getter writes an owned VC100 string. Its long
    // storage uses the exact imported MSVCR100 operator delete identified above.
    unsafe {
        let get: GetProperty = std::mem::transmute(vtable(object, 0x18));
        get(object, &mut result, key.as_ptr());
        let text = native_text(std::ptr::from_ref(&result).cast());
        let words = std::ptr::from_ref(&result).cast::<usize>();
        if words.add(3).read() >= 16 {
            (api.delete_string)(words.read() as Ptr);
        }
        text
    }
}

fn instancing_defines(original: &str, shadow: bool) -> Result<String, String> {
    let tokens: Vec<_> = original
        .split([',', ';'])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    for token in &tokens {
        let name = token.split('=').next().unwrap_or("").trim();
        if matches!(
            name,
            "INSTANCED" | "FOREST_INSTANCED" | "CONSTRUCTION" | "WIND" | "SKELETAL"
        ) {
            return Err(format!("unsupported shader definition {name}"));
        }
    }
    let mut result = tokens.join(",");
    if !result.is_empty() {
        result.push(',');
    }
    result.push_str("INSTANCED,FOREST_INSTANCED");
    if shadow
        && !tokens
            .iter()
            .any(|token| token.split('=').next() == Some("TEXCOORDS"))
    {
        result.push_str(",TEXCOORDS");
    }
    Ok(result)
}

fn forest_source(source: &str) -> Result<String, String> {
    let source = source.replace("\r\n", "\n");
    let anchor = "worldMatrix = float4x4(instanceMatrix0, instanceMatrix1, instanceMatrix2, float4(0,0,0,1));\n\tposition = mul(worldMatrix, position);\n\toWorldPos = position;";
    if source.matches(anchor).count() != 1 {
        return Err("object shader instancing anchor differs from inspected source".into());
    }
    let replacement = format!(
        "#ifdef FOREST_INSTANCED\n\tfloat4x4 forestParentWorld = worldMatrix;\n\tfloat4x4 instanceMatrix = float4x4(instanceMatrix0, instanceMatrix1, instanceMatrix2, float4(0,0,0,1));\n\tposition = mul(instanceMatrix, position);\n\toWorldPos = mul(forestParentWorld, position).xyz;\n\tworldMatrix = mul(forestParentWorld, instanceMatrix);\n\t#else\n\t{anchor}\n\t#endif"
    );
    let camera = "oCameraDir= mul((float3x3)worldMatrix, cameraDir.xyz);";
    if source.matches(camera).count() != 1 {
        return Err("object shader camera-direction anchor differs from inspected source".into());
    }
    let camera_replacement = format!(
        "#ifdef FOREST_INSTANCED\n\toCameraDir= mul((float3x3)forestParentWorld, cameraDir.xyz);\n\t#else\n\t{camera}\n\t#endif"
    );
    Ok(source
        .replacen(anchor, &replacement, 1)
        .replacen(camera, &camera_replacement, 1))
}

#[cfg(test)]
mod tests {
    use super::{forest_source, instancing_defines};

    #[test]
    fn incompatible_shader_features_fail_closed_even_with_spaced_values() {
        for definitions in [
            "COLOURING,CONSTRUCTION = 1",
            "WIND=0;DX11",
            "INSTANCED=0",
            "FOREST_INSTANCED",
            "SKELETAL = 1",
        ] {
            assert!(
                instancing_defines(definitions, false).is_err(),
                "unsupported shader feature passed eligibility: {definitions}"
            );
        }
        assert!(forest_source("").is_err());
        assert!(forest_source("custom vertex shader without known transformation seams").is_err());
    }

    #[test]
    #[ignore = "requires installed game shaders and REKENSHI_TEST_GAME compiler DLL"]
    fn shipped_vertex_shader_compiles_with_parent_world_and_camera_preserved() {
        let game = std::path::PathBuf::from(
            std::env::var_os("REKENSHI_TEST_GAME").expect("set REKENSHI_TEST_GAME"),
        );
        let original = std::fs::read_to_string(game.join("data/materials/deferred/objects.hlsl"))
            .expect("read shipped object shader");
        let patched = forest_source(&original).expect("prepare shipped Forest shader source");
        // Compile the shipped vertex entry through the same Rust patch function
        // used by production. This entry needs no functions from its PS includes.
        let start = patched.find("void main_vs(").expect("shipped vertex entry");
        let end = patched[start..]
            .find("// =================================================================")
            .map(|offset| start + offset)
            .expect("vertex/pixel boundary");
        for definitions in [
            "#define INSTANCED\n#define FOREST_INSTANCED\n",
            "#define INSTANCED\n#define FOREST_INSTANCED\n#define COLOURING\n",
            "#define INSTANCED\n#define FOREST_INSTANCED\n#define DUAL_TEXTURE\n",
        ] {
            let source = format!(
                "{definitions}{}",
                patched[start..end].replacen("void main_vs(", "void main(", 1)
            );
            super::super::shader::compile(&game, source.as_bytes(), true)
                .expect("compile actual patched vertex shader variant");
        }
        // Unknown edits around either seam must reject the shader completely.
        let camera_edited = original.replace("oCameraDir=", "customCameraDirection=");
        assert!(forest_source(&camera_edited).is_err());
        assert!(forest_source(&original.repeat(2)).is_err());
    }
}
