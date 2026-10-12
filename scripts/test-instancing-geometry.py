"""Exercise actual Rust forest instancing, geometry reuse, and native retirement.

The executable is mapped with imports bound; its entry point is never run.
All writes stay under target/. The renderer uses a hidden 64x64 window.
"""
import sys,ctypes as C,pathlib,shutil,os,json,uuid,hashlib,subprocess,struct,math
from importlib.machinery import SourceFileLoader
project=pathlib.Path(__file__).resolve().parent.parent
if len(sys.argv) not in (2,3):
 raise SystemExit('Usage: test-instancing-geometry.py <Kenshi directory>')
if len(sys.argv)==2:
 subprocess.run([sys.executable,str(pathlib.Path(__file__).resolve()),sys.argv[1],'--child'],cwd=project,check=True,timeout=180)
 raise SystemExit(0)
assert sys.argv[2]=='--child'
os.chdir(project)
assert C.sizeof(C.c_void_p)==8
m=SourceFileLoader('fixture',str(project/'scripts/test-performance.py')).load_module()
exports={}
def function(dll,needle,result,args):
 if dll._name not in exports:exports[dll._name]=[e.name.decode() for e in m.pefile.PE(dll._name).DIRECTORY_ENTRY_EXPORT.symbols if e.name]
 names=[name for name in exports[dll._name] if needle in name]
 assert len(names)==1,(needle,names)
 return C.CFUNCTYPE(result,*args)(C.cast(getattr(dll,names[0]),C.c_void_p).value)
m.function=function
cache_export=SourceFileLoader('instance_cache',str(project/'scripts/export-instance-cache.py')).load_module()
game=pathlib.Path(sys.argv[1]).resolve()
# These private test calls are permitted only for the inspected native binaries.
for name,expected in [
 ('kenshi_x64.exe','A596AB4E407C67B58599C54FFB32DC1BF2B64510CDEBD3FA9359EF05A576AEB1'),
 ('OgreMain_x64.dll','7D3888D21398647519F10A89B1F300D8606F761AD159E6021A797B1B9B3DF5F6'),
 ('RenderSystem_Direct3D11_x64.dll','5F224882DED5B6D3F68F14D6BB4E1762267BBB602B7B9733BD153663D1541BB9')]:
 assert hashlib.sha256((game/name).read_bytes()).hexdigest().upper()==expected,name
stage=project/'target/instancing-geometry-tests'/uuid.uuid4().hex;stage.mkdir(parents=True)
for n in ['kenshi_x64.exe','OgreMain_x64.dll','MyGUIEngine_x64.dll','RenderSystem_Direct3D11_x64.dll','D3DCompiler_43.dll']:shutil.copy2(game/n,stage/n)
shutil.copy2(project/'target/release/RE_Kenshi_Rust.dll',stage/'RE_Kenshi_Rust.dll')
r=stage/'RE_Kenshi_Rust';r.mkdir();(r/'plugins.toml').write_text('schema_version=1\ndiscover=false\nplugins=[]\n');(r/'modpack.toml').write_text('schema_version=1\n[settings.values]\n"graphics.resource_capture"=true\n')
os.chdir(stage);search=os.add_dll_directory(str(game));base,imports=m.mapped_exe(stage,game)
# Mapping without the EXE entry point skips its CRT startup. Forest's first
# getMovableType constructs a static std::string and registers its destructor.
# The pinned EXE _onexit at ed6ca0 routes the encoded -1 sentinel through the
# already initialized MSVCR100 atexit instead of an uninitialized DLL table.
# This initializes only the isolated fixture's CRT state, before Forest calls.
kernel=C.WinDLL('kernel32');kernel.EncodePointer.argtypes=[C.c_void_p];kernel.EncodePointer.restype=C.c_void_p
C.c_void_p.from_address(base+0x214bff8).value=kernel.EncodePointer(C.c_void_p(-1))
ogre=C.WinDLL(str(stage/'OgreMain_x64.dll'));renderer=C.WinDLL(str(stage/'RenderSystem_Direct3D11_x64.dll'));host=C.CDLL(str(stage/'RE_Kenshi_Rust.dll'))
root=C.create_string_buffer(4096);empty=m.String('');log=m.String('probe.log');config=m.String('ogre.cfg');pathlib.Path('ogre.cfg').write_text('Render System=Direct3D11 Rendering Subsystem\n[Direct3D11 Rendering Subsystem]\n')
m.function(ogre,'??0Root@Ogre@@',C.c_void_p,[C.c_void_p,C.POINTER(m.String),C.POINTER(m.String),C.POINTER(m.String)])(root,C.byref(empty),C.byref(config),C.byref(log))
renderer.dllStartPlugin();assert m.function(ogre,'?restoreConfig@Root@Ogre@@',C.c_bool,[C.c_void_p])(root)
m.function(ogre,'?initialise@Root@Ogre@@',C.c_void_p,[C.c_void_p,C.c_bool,C.POINTER(m.String),C.POINTER(m.String)])(root,False,C.byref(empty),C.byref(empty))
print('native Ogre GPU ready',flush=True)
params=m.WindowParameters()
window=m.function(ogre,'?createRenderWindow@Root@Ogre@@',C.c_void_p,[C.c_void_p,C.POINTER(m.String),C.c_uint32,C.c_uint32,C.c_bool,C.POINTER(m.WindowParameters)])(root,C.byref(m.String('native-batch-test')),64,64,False,C.byref(params));assert window
print('hidden native render window ready',flush=True)
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
print('native cube entity ready',flush=True)

for folder in ['common','deferred']:
 for path in (game/'data/materials'/folder).glob('*.hlsl'):shutil.copy2(path,stage/path.name)
# Make orientation, tangent handedness, camera direction and world position
# visible in the readback; a constant-color silhouette would miss these errors.
(stage/'probe-pixel.hlsl').write_text('float3 unit(float3 v){return v*rsqrt(max(dot(v,v),1e-8));} float4 main_ps(float3 normal:TEXCOORD1,float3 binormal:TEXCOORD2,float3 tangent:TEXCOORD3,float3 camera:TEXCOORD4,float3 world:TEXCOORD5) : SV_Target { return float4(0.5+0.1*unit(normal)+0.05*unit(tangent)+0.025*unit(binormal)+0.02*unit(camera)+0.001*world,1); }')
(stage/'probe.material').write_text("""
fragment_program Probe/Pixel hlsl { source probe-pixel.hlsl
 entry_point main_ps
 target ps_4_0
}
vertex_program Probe/Original hlsl { source objects.hlsl
 entry_point main_vs
 target vs_4_0
 preprocessor_defines GEOMETRY_PROBE
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
  fragment_program_ref Probe/Pixel {}
  texture_unit { texture_alias retained_alias }
 }}
}
""")
resource_manager=m.function(ogre,'?getSingletonPtr@ResourceGroupManager@',C.c_void_p,[])()
m.function(ogre,'?addResourceLocation@ResourceGroupManager@',None,[C.c_void_p,C.POINTER(m.String),C.POINTER(m.String),C.POINTER(m.String),C.c_bool,C.c_bool])(resource_manager,C.byref(m.String(str(stage))),C.byref(m.String('FileSystem')),C.byref(m.String('General')),False,True)
m.function(ogre,'?initialiseResourceGroup@ResourceGroupManager@',None,[C.c_void_p,C.POINTER(m.String)])(resource_manager,C.byref(m.String('General')))


setter(entity,C.byref(m.String('Probe/Material')),C.byref(m.String('General')))
loaded_material=m.Shared()
m.function(ogre,'?getByName@MaterialManager@',C.c_void_p,[C.c_void_p,C.POINTER(m.Shared),C.POINTER(m.String),C.POINTER(m.String)])(material_manager,C.byref(loaded_material),C.byref(m.String('Probe/Material')),C.byref(m.String('General')))
m.function(ogre,'?load@Resource@',None,[C.c_void_p,C.c_bool])(loaded_material.object,False)
print('PASS original material native load',flush=True)
def ptr(address):return C.c_void_p.from_address(address).value
def count(address):return C.c_size_t.from_address(address).value
get_buffer=m.function(ogre,'?getBuffer@VertexBufferBinding',C.c_void_p,[C.c_void_p,C.c_uint16])
add_element=m.function(ogre,'?addElement@VertexDeclaration',C.c_void_p,[C.c_void_p,C.c_uint16,C.c_size_t,C.c_uint32,C.c_uint32,C.c_uint16])
set_binding=m.function(ogre,'?setBinding@VertexBufferBinding',None,[C.c_void_p,C.c_uint16,C.POINTER(m.Shared)])
find_element=m.function(ogre,'?findElementBySemantic@VertexDeclaration',C.c_void_p,[C.c_void_p,C.c_uint32,C.c_uint16])
buffer_count=m.function(ogre,'?getBufferCount@VertexBufferBinding',C.c_size_t,[C.c_void_p])
manager=m.function(ogre,'?getSingletonPtr@HardwareBufferManager@',C.c_void_p,[])()
create_buffer=m.function(ogre,'?createVertexBuffer@HardwareBufferManager@Ogre@@',C.c_void_p,[C.c_void_p,C.POINTER(m.Shared),C.c_size_t,C.c_size_t,C.c_uint32,C.c_bool])
destroy_shared=m.function(ogre,'??1HardwareVertexBufferSharedPtr@',None,[C.POINTER(m.Shared)])
get_subentity=m.function(ogre,'?getSubEntity@Entity@Ogre@@QEAAPEAVSubEntity@2@_K@Z',C.c_void_p,[C.c_void_p,C.c_size_t])
mesh=ptr(m.function(ogre,'?getMesh@Entity@',C.c_void_p,[C.c_void_p])(entity))
submesh=m.function(ogre,'?getSubMesh@Mesh@Ogre@@QEBAPEAVSubMesh@2@G@Z',C.c_void_p,[C.c_void_p,C.c_uint16])(mesh,0)
construct=C.CFUNCTYPE(C.c_void_p,C.c_void_p,C.c_void_p,C.c_void_p)(base+0xa555d0)
add=C.CFUNCTYPE(None,C.c_void_p,C.c_void_p,C.c_void_p,C.c_void_p,C.c_void_p,C.c_void_p)(base+0xa55980)
build=C.CFUNCTYPE(None,C.c_void_p)(base+0xa572d0)
destroy=C.CFUNCTYPE(None,C.c_void_p)(base+0xa55860)
clear_geometry=C.CFUNCTYPE(None,C.c_void_p)(base+0xa576f0)
clear_subbatch=C.CFUNCTYPE(None,C.c_void_p)(base+0xa5b4e0)
build_subbatch=C.CFUNCTYPE(None,C.c_void_p)(base+0xa58930)
add_subentity=C.CFUNCTYPE(None,C.c_void_p,C.c_void_p,C.c_void_p,C.c_void_p,C.c_void_p,C.c_void_p,C.c_void_p)(base+0xa58590)
render_operation=C.CFUNCTYPE(None,C.c_void_p,C.c_void_p)(base+0xa5b6f0)
placements=[
 dict(position=[-70,-10,0],orientation_wxyz=[1,0,0,0],scale=[0.5]*3,color_components=[1]*4),
 dict(position=[5,8,17],orientation_wxyz=[0.5]*4,scale=[0.75]*3,color_components=[1]*4),
 dict(position=[44,2,-5],orientation_wxyz=[0,0,0,1],scale=[1.25]*3,color_components=[1]*4),
]
def submit(geometry,entries=placements):
 for p in entries:add(geometry,entity,(C.c_float*3)(*p['position']),(C.c_float*4)(*p['orientation_wxyz']),(C.c_float*3)(*p['scale']),(C.c_float*4)(*p['color_components']))
def batch_address(geometry):return ptr(ptr(ptr(C.addressof(geometry)+0x1a0))+0x40)
def read_buffer(buffer):
 size=count(buffer+8);output=C.create_string_buffer(size)
 m.virtual(buffer,5,None,[C.c_size_t,C.c_size_t,C.c_void_p])(buffer,0,size,output)
 return output.raw
def vertex_buffer(data,stream):return ptr(get_buffer(ptr(data+0x18),stream))

def instance_count(batch):
 operation=C.create_string_buffer(0x28);C.c_size_t.from_address(C.addressof(operation)+0x18).value=1
 render_operation(batch,operation)
 assert ptr(C.addressof(operation))==ptr(batch+0x58)
 assert ptr(C.addressof(operation)+0x10)==ptr(batch+0x60)
 if count(C.addressof(operation)+0x18)>1:
  assert C.c_bool.from_address(C.addressof(operation)+0x21).value is False
 return count(C.addressof(operation)+0x18)
# Proven native compositor/camera path; primary Forest MovableObject base is +0.
show=C.CFUNCTYPE(None,C.c_void_p,C.c_bool)(C.cast(getattr(ogre,'?setVisible@MovableObject@Ogre@@QEAAX_N@Z'),C.c_void_p).value)
create_node=m.function(ogre,'?createChildSceneNode@SceneNode@',C.c_void_p,[C.c_void_p,C.c_int,C.c_void_p,C.c_void_p])
attach=m.function(ogre,'?attachObject@SceneNode@',None,[C.c_void_p,C.c_void_p])
camera=m.function(ogre,'?createCamera@SceneManager@',C.c_void_p,[C.c_void_p,C.POINTER(m.String),C.c_bool,C.c_bool])(scene,C.byref(m.String('geometry-camera')),False,False)
dynamic_root=m.function(ogre,'?getRootSceneNode@SceneManager',C.c_void_p,[C.c_void_p,C.c_int])(scene,0)
camera_node=create_node(dynamic_root,0,(C.c_float*3)(0,0,400),(C.c_float*4)(1,0,0,0))
m.function(ogre,'?detachFromParent@MovableObject@',None,[C.c_void_p])(camera);attach(camera_node,camera)
m.function(ogre,'?setAspectRatio@Frustum@',None,[C.c_void_p,C.c_float])(camera,120/64)
compositor=m.function(ogre,'?getCompositorManager2@Root@',C.c_void_p,[C.c_void_p])(root)
workspace_id=C.c_uint32(0x7139a1cf)
m.function(ogre,'?createBasicWorkspaceDef@CompositorManager2@',None,[C.c_void_p,C.POINTER(C.c_uint32),C.c_void_p,C.c_uint32])(compositor,C.byref(workspace_id),(C.c_float*4)(0,0,0,1),0)
workspace=m.function(ogre,'?addWorkspace@CompositorManager2@Ogre@@QEAAPEAVCompositorWorkspace@2@PEAVSceneManager@2@PEAVRenderTarget',C.c_void_p,[C.c_void_p,C.c_void_p,C.c_void_p,C.c_void_p,C.c_uint32,C.c_bool,C.c_int])(compositor,scene,window,camera,workspace_id.value,True,-1)
assert workspace
render=m.function(ogre,'?renderOneFrame@Root@Ogre@@QEAA_NXZ',C.c_bool,[C.c_void_p])
faces=m.function(ogre,'?_getNumRenderedFaces@Camera@',C.c_uint32,[C.c_void_p])
batches=m.function(ogre,'?_getNumRenderedBatches@Camera@',C.c_uint32,[C.c_void_p])
assert render(root) and render(root)
assert (faces(camera),batches(camera))==(0,0)
print('PASS empty native compositor frame',flush=True)
# Native PixelBox constructor initializes its 56-byte POD. The pinned renderer
# RenderTexture/MultiRenderTarget vtables verify copyContentsToMemory slot26.
pixelbox_ctor=m.function(ogre,'??0PixelBox@Ogre@@QEAA@III',C.c_void_p,[C.c_void_p,C.c_uint32,C.c_uint32,C.c_uint32,C.c_int,C.c_void_p])
window_width=m.function(ogre,'?getWidth@RenderTarget@',C.c_uint32,[C.c_void_p])(window)
window_height=m.function(ogre,'?getHeight@RenderTarget@',C.c_uint32,[C.c_void_p])(window)
window_copy=m.virtual(window,26,None,[C.c_void_p,C.c_int])
baseline_pixels=None

def pixels():
 data=C.create_string_buffer(window_width*window_height*4);box=C.create_string_buffer(56)
 pixelbox_ctor(box,window_width,window_height,1,13,data) # PF_A8R8G8B8
 window_copy(window,box,2) # FB_AUTO
 return data.raw

def frame(label):
 global baseline_pixels
 assert render(root) and render(root)
 native_log=(stage/'probe.log').read_text(errors='replace')
 assert 'Exception when rendering material' not in native_log and 'OGRE EXCEPTION' not in native_log,label
 measured=(faces(camera),batches(camera));assert measured==(36,1),(label,measured)
 actual=pixels();colored=sum(actual[i:i+3]!=b'\0\0\0' for i in range(0,len(actual),4))
 assert colored>100,(label,colored)
 if baseline_pixels is None:baseline_pixels=actual
 else:assert actual==baseline_pixels,(label,sum(a!=b for a,b in zip(actual,baseline_pixels)))
 print('PASS actual native frame',label,'36 faces / 1 batch / byte-identical pixels',colored,'colored pixels',flush=True)
# The native addEntity converts the prefab's shared vertices. Prepare a proper
# tangent stream before constructing the baseline or optimized SubBatch clones.
seed=C.create_string_buffer(1024);construct(seed,scene,node);submit(seed,placements[:1]);destroy(seed)
source_vertex=ptr(submesh+8);source_index=ptr(submesh+0x10)
source_bytes=read_buffer(vertex_buffer(source_vertex,0));assert len(source_bytes)==24*32
normals=[struct.unpack_from('<3f',source_bytes,i*32+12) for i in range(24)]
tangents=[]
for nx,ny,nz in normals:
 tangent=(0,-nz,ny) if abs(nx)<0.9 else (-nz,0,nx)
 length=math.sqrt(sum(x*x for x in tangent));tangents.extend([x/length for x in tangent]+[1])
tangent_data=(C.c_float*96)(*tangents);tangent=m.Shared()
create_buffer(manager,C.byref(tangent),16,24,5,False);assert tangent.object
m.virtual(tangent.object,6,None,[C.c_size_t,C.c_size_t,C.c_void_p,C.c_bool])(tangent.object,0,C.sizeof(tangent_data),tangent_data,True)
add_element(ptr(source_vertex+0x10),1,0,3,9,0);set_binding(ptr(source_vertex+0x18),1,C.byref(tangent));destroy_shared(C.byref(tangent))
prototype_buffers=[vertex_buffer(source_vertex,i) for i in range(2)]
prototype_bytes=[read_buffer(b) for b in prototype_buffers]
prototype_index=ptr(source_index);prototype_index_bytes=read_buffer(prototype_index)
# Baseline goes through the uninstrumented shipped expanded builder.
baseline=C.create_string_buffer(1024);construct(baseline,scene,node);submit(baseline);build(baseline)
baseline_batch=batch_address(baseline);baseline_vertex=ptr(baseline_batch+0x58)
assert count(baseline_vertex+0x30)==72 and count(ptr(baseline_batch+0x60)+0x18)==108
baseline_bytes=read_buffer(vertex_buffer(baseline_vertex,0));assert len(baseline_bytes)==2304
expanded_bytes=sum(len(read_buffer(vertex_buffer(baseline_vertex,i))) for i in range(buffer_count(ptr(baseline_vertex+0x18))))+len(read_buffer(ptr(ptr(baseline_batch+0x60))))
print('PASS original baseline expands three native cube placements',flush=True)
frame('original expanded baseline');show(baseline,False)
host.dllStartPlugin()
host.rekenshi_plugin_setting_set.argtypes=[C.c_char_p,C.c_size_t,C.c_uint32,C.c_double]
def enabled(value):
 key=b'graphics.landscape_instancing';assert host.rekenshi_plugin_setting_set(key,len(key),1,float(value))==0
 print('instancing setting',value,flush=True)
def check_prototype():
 assert [read_buffer(b) for b in prototype_buffers]==prototype_bytes
 assert read_buffer(prototype_index)==prototype_index_bytes
 assert not any(m.function(ogre,'?getIsInstanceData@HardwareVertexBuffer',C.c_bool,[C.c_void_p])(b) for b in prototype_buffers)
def check_instanced(batch):
 vertex=ptr(batch+0x58);index=ptr(batch+0x60)
 assert count(vertex+0x30)==24 and count(index+0x18)==36,(count(vertex+0x30),count(index+0x18))
 assert instance_count(batch)==3
 assert [vertex_buffer(vertex,i) for i in range(2)]==prototype_buffers
 assert ptr(index)==prototype_index
 assert buffer_count(ptr(vertex+0x18))==3
 instance=read_buffer(vertex_buffer(vertex,2));assert len(instance)==144
 owner=ptr(batch+0x78);origin=list((C.c_float*3).from_address(owner+0x174))
 expected=[]
 for p in placements:
  affine=list(struct.unpack('<16f',cache_export.affine_record(p)))[:12]
  for i in range(3):affine[i*4+3]-=origin[i]
  expected.extend(affine)
 expected_bytes=struct.pack('<36f',*expected)
 assert instance==expected_bytes,(struct.unpack('<36f',instance),expected)
 # Compare actual Rust rows/prototype data against the original native expanded
 # positions; this checks native batch recentering, not a mirrored mock renderer.
 for i in range(3):
  row=struct.unpack_from('<12f',instance,i*48)
  for j in range(24):
   source=struct.unpack_from('<3f',prototype_bytes[0],j*32)
   actual=[sum(row[k*4+n]*source[n] for n in range(3))+row[k*4+3] for k in range(3)]
   original=struct.unpack_from('<3f',baseline_bytes,(i*24+j)*32)
   assert all(math.isclose(a,b,rel_tol=1e-5,abs_tol=2e-5) for a,b in zip(actual,original)),(i,j,actual,original)
 check_prototype()
 print('PASS actual Rust shares prototype buffers, uploads 144 affine bytes, returns three instances and matches native expanded positions',flush=True)
enabled(True)
geometry=C.create_string_buffer(1024);construct(geometry,scene,node);submit(geometry);build(geometry)
batch=batch_address(geometry);check_instanced(batch)
frame('actual Rust shared geometry')
# Disabled setting keeps existing optimized objects valid until native retirement.
enabled(False);assert instance_count(batch)==3
frame('disabled setting with existing shared geometry')
plain=C.create_string_buffer(1024);construct(plain,scene,node);submit(plain);build(plain)
plain_batch=batch_address(plain);assert count(ptr(plain_batch+0x58)+0x30)==72 and instance_count(plain_batch)==1
destroy(plain)
print('PASS disabling retains live instance counts and new builds fall back',flush=True)
# Extra adversarial coverage: native SubBatch::clear normally runs only in its
# destructor. Retained constructor prototype bindings also permit a first
# same-object expanded rebuild without leaking any instance declarations.
for cycle in range(2):
 clear_subbatch(batch)
 vertex=ptr(batch+0x58);index=ptr(batch+0x60)
 assert count(vertex+0x30)==count(index+0x18)==0
 assert not find_element(ptr(vertex+0x10),7,1)
 assert instance_count(batch)==1
 if cycle==0:
  assert [vertex_buffer(vertex,i) for i in range(2)]==prototype_buffers
 enabled(cycle==1)
 for p in placements:add_subentity(batch,get_subentity(entity,0),(C.c_float*3)(*p['position']),(C.c_float*4)(*p['orientation_wxyz']),(C.c_float*3)(*p['scale']),(C.c_float*4)(*p['color_components']),None)
 build_subbatch(batch)
 if cycle==1:check_instanced(batch)
 else:
  assert count(ptr(batch+0x58)+0x30)==72 and instance_count(batch)==1
  assert not find_element(ptr(ptr(batch+0x58)+0x10),7,1)
 check_prototype()
 frame('same-object rebuild '+str(cycle))
 print('PASS same native SubBatch clear/rebuild cycle',cycle,flush=True)

# Ordinary page retirement calls outer clear, destroys its SubBatches, then
# creates fresh ones on reloading. Exercise that observed native lifecycle.
for cycle in range(2):
 clear_geometry(geometry);check_prototype()
 submit(geometry);build(geometry);batch=batch_address(geometry)
 check_instanced(batch);frame('outer native clear/recreate '+str(cycle))
 print('PASS observed outer clear destroys/recreates native SubBatch',cycle,flush=True)
destroy(geometry);check_prototype()
# A nonwhite placement must take the original expanded/color path.
colored=[dict(p) for p in placements];colored[1]['color_components']=[0.25,0.5,0.75,1]
fallback=C.create_string_buffer(1024);construct(fallback,scene,node);submit(fallback,colored);build(fallback)
fallback_batch=batch_address(fallback)
assert count(ptr(fallback_batch+0x58)+0x30)==72 and instance_count(fallback_batch)==1
frame('nonwhite original fallback')
destroy(fallback);check_prototype()
print('PASS nonwhite placements fall back to original native builder',flush=True)
enabled(True)
stopped=C.create_string_buffer(1024);construct(stopped,scene,node);submit(stopped);build(stopped)
stopped_batch=batch_address(stopped);check_instanced(stopped_batch)
enabled(False);destroy(baseline);host.dllStopPlugin()
assert instance_count(stopped_batch)==3
frame('provider stopped with existing shared geometry')
destroy(stopped);check_prototype()
print('PASS stopped provider retains rendering and native retirement for live optimized geometry',flush=True)
(stage/'report.json').write_text(json.dumps(dict(instances=3,prototype_vertices=24,expanded_vertices=72,expanded_gpu_bytes=expanded_bytes,shared_prototype_gpu_bytes=sum(map(len,prototype_bytes))+len(prototype_index_bytes),new_instance_gpu_bytes=144,positions_match_native_expanded=True,same_subbatch_rebuild=True,outer_clear_recreates_subbatch=True,opt_out_preserves_existing_instances=True,provider_stop_preserves_existing_instances=True,rendered_frames=True,framebuffer_dimensions=[window_width,window_height],framebuffer_matches_native_expanded=True,framebuffer_sha256=hashlib.sha256(baseline_pixels).hexdigest()),indent=2))
release=m.function(ogre,'??1?$SharedPtr@VHardwareCounterBuffer@Ogre@@',None,[C.POINTER(m.Shared)])
for material in materials:release(C.byref(material))
release(C.byref(loaded_material))
m.function(ogre,'??1Root@Ogre@@QEAA@XZ',None,[C.c_void_p])(root)
print('PASS actual Rust instancing native clear/rebuild/disable/unload shutdown',stage,flush=True)
