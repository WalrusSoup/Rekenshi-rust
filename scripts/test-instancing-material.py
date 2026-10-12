"""Exercise native material/shader cloning and restoration for Forest instancing.

Creates an isolated renderer, clones shipped object and shadow shader sources,
and verifies native compilation, preserved parameters, material properties,
caster binding, retained-reference restoration, and registration cleanup.
No game process, hooks, or rendered frame are used. Writes stay under target/.
"""
import sys,ctypes as C,pathlib,shutil,os,json,uuid,hashlib,subprocess,struct,math
from importlib.machinery import SourceFileLoader
project=pathlib.Path(__file__).resolve().parent.parent
if len(sys.argv) not in (2,3):
 raise SystemExit('Usage: test-instancing-material.py <Kenshi directory>')
if len(sys.argv)==2:
 subprocess.run([sys.executable,str(pathlib.Path(__file__).resolve()),sys.argv[1],'--child'],cwd=project,check=True,timeout=120)
 raise SystemExit(0)
assert sys.argv[2]=='--child'
os.chdir(project)
assert C.sizeof(C.c_void_p)==8
m=SourceFileLoader('fixture',str(project/'scripts/test-performance.py')).load_module()
exports={}
def function(dll,needle,result,args):
 if dll._name not in exports:
  exports[dll._name]=[e.name.decode() for e in m.pefile.PE(dll._name).DIRECTORY_ENTRY_EXPORT.symbols if e.name]
 names=[name for name in exports[dll._name] if needle in name]
 assert len(names)==1,(needle,names)
 return C.CFUNCTYPE(result,*args)(C.cast(getattr(dll,names[0]),C.c_void_p).value)
m.function=function
game=pathlib.Path(sys.argv[1]).resolve()
# These private test calls are permitted only for the inspected native binaries.
for name,expected in [
 ('kenshi_x64.exe','A596AB4E407C67B58599C54FFB32DC1BF2B64510CDEBD3FA9359EF05A576AEB1'),
 ('OgreMain_x64.dll','7D3888D21398647519F10A89B1F300D8606F761AD159E6021A797B1B9B3DF5F6')]:
 assert hashlib.sha256((game/name).read_bytes()).hexdigest().upper()==expected,name
stage=project/'target/instancing-material-tests'/uuid.uuid4().hex;stage.mkdir(parents=True)
for n in ['OgreMain_x64.dll','RenderSystem_Direct3D11_x64.dll','D3DCompiler_43.dll']:shutil.copy2(game/n,stage/n)


os.chdir(stage);search=os.add_dll_directory(str(game));
ogre=C.WinDLL(str(stage/'OgreMain_x64.dll'));renderer=C.WinDLL(str(stage/'RenderSystem_Direct3D11_x64.dll'))
root=C.create_string_buffer(4096);empty=m.String('');log=m.String('probe.log');config=m.String('ogre.cfg');pathlib.Path('ogre.cfg').write_text('Render System=Direct3D11 Rendering Subsystem\n[Direct3D11 Rendering Subsystem]\n')
m.function(ogre,'??0Root@Ogre@@',C.c_void_p,[C.c_void_p,C.POINTER(m.String),C.POINTER(m.String),C.POINTER(m.String)])(root,C.byref(empty),C.byref(config),C.byref(log))
renderer.dllStartPlugin();assert m.function(ogre,'?restoreConfig@Root@Ogre@@',C.c_bool,[C.c_void_p])(root)
m.function(ogre,'?initialise@Root@Ogre@@',C.c_void_p,[C.c_void_p,C.c_bool,C.POINTER(m.String),C.POINTER(m.String)])(root,False,C.byref(empty),C.byref(empty))
print('native Ogre GPU ready',flush=True)
# Borrowed VC100 const map: node value precedes flags, container head at +8.
# Only the fingerprinted renderer reads it; no ownership crosses the call.
class MapNode(C.Structure):
 _fields_=[('left',C.c_void_p),('parent',C.c_void_p),('right',C.c_void_p),('key',m.String),('value',m.String),('color',C.c_ubyte),('nil',C.c_ubyte),('pad',C.c_ubyte*6)]
class Map(C.Structure): _fields_=[('allocator',C.c_size_t),('head',C.c_void_p),('size',C.c_size_t)]
head=MapNode();entry=MapNode();head.left=head.parent=head.right=C.addressof(entry);head.nil=1;head.color=1
entry.left=entry.right=entry.parent=C.addressof(head);entry.color=1;entry.key=m.String('hidden');entry.value=m.String('true');params=Map(0,C.addressof(head),1)
window=m.function(ogre,'?createRenderWindow@Root@Ogre@@',C.c_void_p,[C.c_void_p,C.POINTER(m.String),C.c_uint32,C.c_uint32,C.c_bool,C.POINTER(Map)])(root,C.byref(m.String('native-instancing-test')),64,64,False,C.byref(params));assert window
print('hidden native render window ready',flush=True)
# Instancing culling method 1 is required by this pinned Ogre HWBasic backend.
# Method 0 left its per-instance visible list empty in the native frame regression.
scene=m.function(ogre,'?createSceneManager@Root@Ogre@@QEAAPEAVSceneManager@2@G',C.c_void_p,[C.c_void_p,C.c_uint16,C.c_size_t,C.c_int,C.POINTER(m.String)])(root,1,1,1,C.byref(empty));assert scene
node=m.function(ogre,'?getRootSceneNode@SceneManager',C.c_void_p,[C.c_void_p,C.c_int])(scene,1);assert node
material_manager=m.function(ogre,'?getSingletonPtr@MaterialManager',C.c_void_p,[])()
create_material=m.function(ogre,'?create@MaterialManager@Ogre@@',C.c_void_p,[C.c_void_p,C.POINTER(m.Shared),C.POINTER(m.String),C.POINTER(m.String),C.c_bool,C.c_void_p,C.c_void_p])
materials=[]
for material_name in ('BaseWhite','BaseWhiteNoLighting'):
 material=m.Shared();create_material(material_manager,C.byref(material),C.byref(m.String(material_name)),C.byref(m.String('General')),False,None,None);materials.append(material)
print('native parent materials ready',flush=True)
entity=m.function(ogre,'?createEntity@SceneManager@Ogre@@UEAAPEAVEntity@2@W4PrefabType',C.c_void_p,[C.c_void_p,C.c_int,C.c_int])(scene,1,1);assert entity
setter=getattr(ogre,'?setMaterialName@Entity@Ogre@@QEAAXAEBV?$basic_string@DU?$char_traits@D@std@@V?$allocator@D@2@@std@@0@Z')
setter.argtypes=[C.c_void_p,C.POINTER(m.String),C.POINTER(m.String)]
setter(entity,C.byref(m.String('BaseWhite')),C.byref(m.String('General')))



for folder in ['common','deferred']:
 for path in (game/'data/materials'/folder).glob('*.hlsl'):shutil.copy2(path,stage/path.name)
(stage/'probe.material').write_text("""
vertex_program Probe/Original hlsl { source objects.hlsl
 entry_point main_vs
 target vs_4_0
 preprocessor_defines COLOURING
 default_params { param_named tiling float2 2 3
 param_named_auto worldMatrix world_matrix
 param_named_auto worldViewProjMatrix worldviewproj_matrix
 param_named_auto cameraDir view_direction
 }
}
vertex_program Probe/Shadow hlsl { source shadowcaster.hlsl
 entry_point shadow_vs
 target vs_4_0
 default_params { param_named_auto worldViewProj worldviewproj_matrix }
}
vertex_program Probe/ShadowRTW hlsl { source shadowcaster.hlsl
 entry_point shadow_vs
 target vs_4_0
 preprocessor_defines RTW
 default_params { param_named_auto worldViewProj worldviewproj_matrix }
}
vertex_program Probe/ShadowTess hlsl { source rtwtessellator.hlsl
 entry_point tessellator_vs
 target vs_5_0
 default_params { param_named_auto worldViewProj worldviewproj_matrix }
}
material Probe/Caster {
 technique { pass { vertex_program_ref Probe/Shadow {} } }
 technique RTW { pass { vertex_program_ref Probe/ShadowRTW {} } }
 technique RTWTess { pass { vertex_program_ref Probe/ShadowTess {} } }
}
material Probe/Material {
 technique { shadow_caster_material Probe/Caster
 pass {
  depth_bias 2 3
  vertex_program_ref Probe/Original {}
  shadow_caster_vertex_program_ref Probe/Shadow {}
  texture_unit { texture_alias retained_alias }
 }}
}
""")
resource_manager=m.function(ogre,'?getSingletonPtr@ResourceGroupManager@',C.c_void_p,[])()
m.function(ogre,'?addResourceLocation@ResourceGroupManager@',None,[C.c_void_p,C.POINTER(m.String),C.POINTER(m.String),C.POINTER(m.String),C.c_bool,C.c_bool])(resource_manager,C.byref(m.String(str(stage))),C.byref(m.String('FileSystem')),C.byref(m.String('General')),False,True)
m.function(ogre,'?initialiseResourceGroup@ResourceGroupManager@',None,[C.c_void_p,C.POINTER(m.String)])(resource_manager,C.byref(m.String('General')))
release=m.function(ogre,'??1?$SharedPtr@VHardwareCounterBuffer@Ogre@@',None,[C.POINTER(m.Shared)])
get_material=m.function(ogre,'?getByName@MaterialManager@',C.c_void_p,[C.c_void_p,C.POINTER(m.Shared),C.POINTER(m.String),C.POINTER(m.String)])
original=m.Shared();get_material(material_manager,C.byref(original),C.byref(m.String('Probe/Material')),C.byref(m.String('General')));assert original.object
cloned=m.Shared();m.function(ogre,'?clone@Material@',C.c_void_p,[C.c_void_p,C.POINTER(m.Shared),C.POINTER(m.String),C.c_bool,C.POINTER(m.String)])(original.object,C.byref(cloned),C.byref(m.String('Probe/Cloned')),False,C.byref(m.String('General')))
get_tech=m.function(ogre,'?getTechnique@Material@Ogre@@QEAAPEAVTechnique@2@G@Z',C.c_void_p,[C.c_void_p,C.c_uint16]);get_pass=m.function(ogre,'?getPass@Technique@Ogre@@QEAAPEAVPass@2@G@Z',C.c_void_p,[C.c_void_p,C.c_uint16])
original_pass=get_pass(get_tech(original.object,0),0);cloned_pass=get_pass(get_tech(cloned.object,0),0)
get_vp=m.function(ogre,'?getVertexProgram@Pass@',C.c_void_p,[C.c_void_p]);get_shadow=m.function(ogre,'?getShadowCasterVertexProgram@Pass@',C.c_void_p,[C.c_void_p])
load=m.function(ogre,'?load@Resource@',None,[C.c_void_p,C.c_bool]);supported=m.function(ogre,'?isSupported@GpuProgram@',C.c_bool,[C.c_void_p])
clone_program=m.function(ogre,'?clone@HighLevelGpuProgram@',C.c_void_p,[C.c_void_p,C.POINTER(m.Shared),C.POINTER(m.String)])
def text(pointer):
 value=m.String.from_address(pointer)
 address=pointer if value.capacity<16 else C.c_void_p.from_address(pointer).value
 return C.string_at(address,value.length).decode()
def property(program,key):
 output=m.String('')
 m.virtual(program,3,C.c_void_p,[C.POINTER(m.String),C.POINTER(m.String)])(program,C.byref(output),C.byref(m.String(key)))
 result=text(C.addressof(output))
 if output.capacity>=16:
  delete=C.CFUNCTYPE(None,C.c_void_p)(C.c_void_p.from_address(ogre._handle+0x5b7b40).value)
  delete(C.c_void_p.from_address(C.addressof(output)).value)
 return result
def set_property(program,key,value):
 assert m.virtual(program,1,C.c_bool,[C.POINTER(m.String),C.POINTER(m.String)])(program,C.byref(m.String(key)),C.byref(m.String(value)))
programs=[]
for is_shadow,getter in [(False,get_vp),(True,get_shadow)]:
 original_program=C.c_void_p.from_address(getter(original_pass)).value
 load(original_program,False);assert supported(original_program)
 before=property(original_program,'preprocessor_defines')
 source=text(m.virtual(original_program,0x190//8,C.c_void_p,[])(original_program)).replace('\r\n','\n')
 result=m.Shared();name='Probe/ClonedShadow' if is_shadow else 'Probe/ClonedVertex'
 clone_program(original_program,C.byref(result),C.byref(m.String(name)));assert result.object
 defines=before+',INSTANCED'+(',TEXCOORDS' if is_shadow else ',FOREST_INSTANCED')
 set_property(result.object,'preprocessor_defines',defines)
 if not is_shadow:
  anchor='worldMatrix = float4x4(instanceMatrix0, instanceMatrix1, instanceMatrix2, float4(0,0,0,1));\n\tposition = mul(worldMatrix, position);\n\toWorldPos = position;'
  replacement='#ifdef FOREST_INSTANCED\n\tfloat4x4 forestParentWorld = worldMatrix;\n\tfloat4x4 instanceMatrix = float4x4(instanceMatrix0, instanceMatrix1, instanceMatrix2, float4(0,0,0,1));\n\tposition = mul(instanceMatrix, position);\n\toWorldPos = mul(forestParentWorld, position).xyz;\n\tworldMatrix = mul(forestParentWorld, instanceMatrix);\n\t#else\n\t'+anchor+'\n\t#endif'
  assert source.count(anchor)==1
  patched=source.replace(anchor,replacement)
  camera='oCameraDir= mul((float3x3)worldMatrix, cameraDir.xyz);'
  assert source.count(camera)==1
  patched=patched.replace(camera,'#ifdef FOREST_INSTANCED\n\toCameraDir= mul((float3x3)forestParentWorld, cameraDir.xyz);\n\t#else\n\t'+camera+'\n\t#endif')
  m.virtual(result.object,0x170//8,None,[C.POINTER(m.String)])(result.object,C.byref(m.String(patched)))
 load(result.object,False);assert supported(result.object),name
 assert property(original_program,'preprocessor_defines')==before
 assert text(m.virtual(original_program,0x190//8,C.c_void_p,[])(original_program)).replace('\r\n','\n')==source
 get_parameters=m.function(ogre,'?getShadowCasterVertexProgramParameters@Pass@' if is_shadow else '?getVertexProgramParameters@Pass@',C.c_void_p,[C.c_void_p,C.POINTER(m.Shared)])
 old_parameters=m.Shared();get_parameters(original_pass,C.byref(old_parameters))
 setter=C.CFUNCTYPE(None,C.c_void_p,C.POINTER(m.String),C.c_bool)(ogre._handle+0x218090 if is_shadow else ogre._handle+0x214c30)
 setter(cloned_pass,C.byref(m.String(name)),True)
 new_parameters=m.Shared();get_parameters(cloned_pass,C.byref(new_parameters))
 m.function(ogre,'?copyMatchingNamedConstantsFrom@GpuProgramParameters@',None,[C.c_void_p,C.c_void_p])(new_parameters.object,old_parameters.object)
 auto_count=m.function(ogre,'?getAutoConstantCount@GpuProgramParameters@',C.c_size_t,[C.c_void_p])
 assert auto_count(new_parameters.object)==auto_count(old_parameters.object)>0
 if not is_shadow:
  definition=m.function(ogre,'?_findNamedConstantDefinition@GpuProgramParameters@',C.c_void_p,[C.c_void_p,C.POINTER(m.String),C.c_bool])(new_parameters.object,C.byref(m.String('tiling')),True)
  index=C.c_size_t.from_address(definition+8).value
  output=(C.c_float*2)()
  m.function(ogre,'?_readRawConstants@GpuProgramParameters@Ogre@@QEAAX_K0PEAM@Z',None,[C.c_void_p,C.c_size_t,C.c_size_t,C.c_void_p])(new_parameters.object,index,2,output)
  assert list(output)==[2,3],list(output)
 release(C.byref(new_parameters));release(C.byref(old_parameters))
 assert C.c_void_p.from_address(getter(cloned_pass)).value==result.object
 assert C.c_void_p.from_address(getter(original_pass)).value==original_program
 print('PASS cloned native shader',name,'defines',property(result.object,'preprocessor_defines'),flush=True)
 programs.append(result)
# Clone the technique caster including basic, RTW, and tessellated variants.
caster_source=m.Shared();get_material(material_manager,C.byref(caster_source),C.byref(m.String('Probe/Caster')),C.byref(m.String('General')))
caster_clone=m.Shared();m.function(ogre,'?clone@Material@',C.c_void_p,[C.c_void_p,C.POINTER(m.Shared),C.POINTER(m.String),C.c_bool,C.POINTER(m.String)])(caster_source.object,C.byref(caster_clone),C.byref(m.String('Probe/CasterClone')),False,C.byref(m.String('General')))
extra_program_names=[]
for technique_index in range(3):
 caster_pass=get_pass(get_tech(caster_clone.object,technique_index),0)
 original_program=C.c_void_p.from_address(get_vp(caster_pass)).value
 load(original_program,False)
 result=m.Shared();name=f'Probe/CasterClone/Vertex{technique_index}'
 clone_program(original_program,C.byref(result),C.byref(m.String(name)))
 defines=property(original_program,'preprocessor_defines')
 set_property(result.object,'preprocessor_defines',defines+',INSTANCED,FOREST_INSTANCED,TEXCOORDS')
 load(result.object,False);assert supported(result.object),name
 C.CFUNCTYPE(None,C.c_void_p,C.POINTER(m.String),C.c_bool)(ogre._handle+0x214c30)(caster_pass,C.byref(m.String(name)),True)
 programs.append(result);extra_program_names.append(name)
copy_shared=m.function(ogre,'??0?$SharedPtr@VHardwareCounterBuffer@Ogre@@@Ogre@@QEAA@AEBV01@@Z',C.c_void_p,[C.POINTER(m.Shared),C.POINTER(m.Shared)])
temporary=m.Shared();copy_shared(C.byref(temporary),C.byref(caster_clone))
C.CFUNCTYPE(None,C.c_void_p,C.POINTER(m.Shared))(ogre._handle+0x374960)(get_tech(cloned.object,0),C.byref(temporary))
assert not temporary.object and not temporary.info
bound_caster=m.Shared();m.function(ogre,'?getShadowCasterMaterial@Technique@',C.c_void_p,[C.c_void_p,C.POINTER(m.Shared)])(get_tech(cloned.object,0),C.byref(bound_caster))
assert bound_caster.object==caster_clone.object
release(C.byref(bound_caster))
print('PASS cloned technique caster preserves and compiles basic, RTW and tessellated instanced variants',flush=True)
assert m.function(ogre,'?getNumTextureUnitStates@Pass@',C.c_uint16,[C.c_void_p])(cloned_pass)==1
bias=m.function(ogre,'?getDepthBiasConstant@Pass@',C.c_float,[C.c_void_p]);assert bias(cloned_pass)==bias(original_pass)==2
print('PASS native material preserves original passes, texture unit and depth bias; original shader bindings/source unchanged',flush=True)
copy_shared=m.function(ogre,'??0?$SharedPtr@VHardwareCounterBuffer@Ogre@@@Ogre@@QEAA@AEBV01@@Z',C.c_void_p,[C.POINTER(m.Shared),C.POINTER(m.Shared)])
slot=m.Shared();copy_shared(C.byref(slot),C.byref(cloned))
retained_original=m.Shared();copy_shared(C.byref(retained_original),C.byref(original))
remove=m.function(ogre,'?remove@ResourceManager@Ogre@@UEAAXAEBV',None,[C.c_void_p,C.POINTER(m.String)])
# Prove restoration uses an actual retained reference, even if the original is
# removed from the manager before the batch clears.
remove(material_manager,C.byref(m.String('Probe/Material')))
release(C.byref(slot));slot=retained_original;retained_original=m.Shared()
assert slot.object==original.object
release(C.byref(cloned))
remove(material_manager,C.byref(m.String('Probe/Cloned')))
program_manager=m.function(ogre,'?getSingletonPtr@HighLevelGpuProgramManager@',C.c_void_p,[])()
for name in ['Probe/ClonedShadow','Probe/ClonedVertex']+extra_program_names:remove(program_manager,C.byref(m.String(name)))
lookup=m.Shared();get_material(material_manager,C.byref(lookup),C.byref(m.String('Probe/Cloned')),C.byref(m.String('General')));assert not lookup.object
assert C.c_void_p.from_address(get_vp(get_pass(get_tech(slot.object,0),0))).value==C.c_void_p.from_address(get_vp(original_pass)).value
release(C.byref(slot));release(C.byref(original))
release(C.byref(caster_clone));release(C.byref(caster_source))
remove(material_manager,C.byref(m.String('Probe/CasterClone')))
print('PASS retained original restores after manager removal; cloned resource registrations removed',flush=True)
for program in programs:release(C.byref(program))
for material in materials:release(C.byref(material))
m.function(ogre,'??1Root@Ogre@@QEAA@XZ',None,[C.c_void_p])(root)
print('PASS native cloned material/program shutdown',stage,flush=True)
