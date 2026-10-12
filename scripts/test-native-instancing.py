"""Exercise the shipped Ogre HWBasic shared-geometry allocation path.

Creates an isolated native renderer, compiles an affine instancing material, and
checks native matrix upload, rendered face/batch counts, buffer reuse, and
entity/manager destruction. No game process or hooks are used. The Rust GPU
differential test separately verifies pixels. Writes stay under target/.
"""
import sys,ctypes as C,pathlib,shutil,os,json,uuid,hashlib,subprocess,struct,math
from importlib.machinery import SourceFileLoader
project=pathlib.Path(__file__).resolve().parent.parent
if len(sys.argv) not in (2,3):
 raise SystemExit('Usage: test-native-instancing.py <Kenshi directory>')
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
stage=project/'target/native-instancing-tests'/uuid.uuid4().hex;stage.mkdir(parents=True)
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


(stage/'probe.hlsl').write_text("""
float4x4 viewProj;
struct Output { float4 position : SV_Position; float4 color : COLOR; };
Output vertexMain(float3 position : POSITION, float4 row0 : TEXCOORD1,
                  float4 row1 : TEXCOORD2, float4 row2 : TEXCOORD3) {
 Output output; float4 local=float4(position,1);
 output.position=mul(viewProj,float4(dot(row0,local),dot(row1,local),dot(row2,local),1));
 output.color=float4(0.25,0.5,0.75,1); return output;
}
float4 pixelMain(Output input) : SV_Target { return input.color; }
""")
(stage/'probe.material').write_text("""
vertex_program NativeInstancing/Vertex hlsl
{
 source probe.hlsl
 entry_point vertexMain
 target vs_4_0
 default_params { param_named_auto viewProj viewproj_matrix }
}
fragment_program NativeInstancing/Pixel hlsl
{
 source probe.hlsl
 entry_point pixelMain
 target ps_4_0
}
material NativeInstancing/Material
{
 technique
 {
  pass
  {
   cull_hardware none
   vertex_program_ref NativeInstancing/Vertex {}
   fragment_program_ref NativeInstancing/Pixel {}
  }
 }
}
""")
resource_manager=m.function(ogre,'?getSingletonPtr@ResourceGroupManager@',C.c_void_p,[])()
m.function(ogre,'?addResourceLocation@ResourceGroupManager@',None,[C.c_void_p,C.POINTER(m.String),C.POINTER(m.String),C.POINTER(m.String),C.c_bool,C.c_bool])(resource_manager,C.byref(m.String(str(stage))),C.byref(m.String('FileSystem')),C.byref(m.String('General')),False,True)
m.function(ogre,'?initialiseResourceGroup@ResourceGroupManager@',None,[C.c_void_p,C.POINTER(m.String)])(resource_manager,C.byref(m.String('General')))
program_manager=m.function(ogre,'?getSingletonPtr@HighLevelGpuProgramManager@',C.c_void_p,[])()
programs=[]
for name in ['NativeInstancing/Vertex','NativeInstancing/Pixel']:
 program=m.Shared()
 m.function(ogre,'?getByName@HighLevelGpuProgramManager@',C.c_void_p,[C.c_void_p,C.POINTER(m.Shared),C.POINTER(m.String),C.POINTER(m.String)])(program_manager,C.byref(program),C.byref(m.String(name)),C.byref(m.String('General')))
 assert program.object,name
 m.function(ogre,'?load@Resource@',None,[C.c_void_p,C.c_bool])(program.object,False)
 assert m.function(ogre,'?isSupported@GpuProgram@',C.c_bool,[C.c_void_p])(program.object),name
 programs.append(program)
print('PASS native Ogre compiles and supports affine instancing vertex/pixel programs',flush=True)
create=m.function(ogre,'?createInstanceManager@SceneManager@Ogre@@',C.c_void_p,[C.c_void_p,C.POINTER(m.String),C.POINTER(m.String),C.POINTER(m.String),C.c_int,C.c_size_t,C.c_uint16,C.c_uint16])
manager=create(scene,C.byref(m.String('probe')),C.byref(m.String('Prefab_Cube')),C.byref(m.String('General')),2,3,0,0)
assert manager
new=m.function(ogre,'?createInstancedEntity@InstanceManager@',C.c_void_p,[C.c_void_p,C.POINTER(m.String),C.c_int])
entities=[new(manager,C.byref(m.String('NativeInstancing/Material')),1) for _ in range(3)]
assert all(entities)
get_owner=m.function(ogre,'?_getOwner@InstancedEntity@',C.c_void_p,[C.c_void_p])
owner=get_owner(entities[0]);assert all(get_owner(e)==owner for e in entities)
op=C.create_string_buffer(0x28)
m.function(ogre,'?getRenderOperation@InstanceBatch@',None,[C.c_void_p,C.c_void_p])(owner,op)
get_buffer=m.function(ogre,'?getBuffer@VertexBufferBinding@',C.c_void_p,[C.c_void_p,C.c_uint16])
data=C.c_void_p.from_buffer(op).value
assert C.c_size_t.from_address(data+0x30).value==24
binding=C.c_void_p.from_address(data+0x18).value
buffers=[]
for slot in range(2):
 shared=get_buffer(binding,slot);buffer=C.c_void_p.from_address(shared).value
 buffers.append((buffer,C.c_size_t.from_address(buffer+8).value))
assert [size for pointer,size in buffers]==[768,144],buffers
mesh_shared=m.function(ogre,'?getMesh@Entity@',C.c_void_p,[C.c_void_p])(entity)
mesh=C.c_void_p.from_address(mesh_shared).value
submesh=m.function(ogre,'?getSubMesh@Mesh@Ogre@@QEBAPEAVSubMesh@2@G@Z',C.c_void_p,[C.c_void_p,C.c_uint16])(mesh,0)
source_data=C.c_void_p.from_address(submesh+8).value
source_binding=C.c_void_p.from_address(source_data+0x18).value
source_buffer=C.c_void_p.from_address(get_buffer(source_binding,0)).value
def read_buffer(pointer):
 size=C.c_size_t.from_address(pointer+8).value
 output=C.create_string_buffer(size)
 m.virtual(pointer,5,None,[C.c_size_t,C.c_size_t,C.c_void_p])(pointer,0,size,output)
 return output.raw
assert read_buffer(source_buffer)==read_buffer(buffers[0][0]), 'native batch mesh copy must preserve source bytes'
batch_mesh_buffer=buffers[0][0]
print('PASS 3 native instances share one 24-vertex cube buffer: 768 mesh bytes + 144 affine bytes',flush=True)
print('Native manager clones source buffer once:',source_buffer!=batch_mesh_buffer,flush=True)
# Exercise the engine's native transform upload and frame submission.
create_node=m.function(ogre,'?createChildSceneNode@SceneNode@',C.c_void_p,[C.c_void_p,C.c_int,C.c_void_p,C.c_void_p])
attach=m.function(ogre,'?attachObject@SceneNode@',None,[C.c_void_p,C.c_void_p])
positions=[(-120.,0.,0.),(0.,0.,0.),(120.,0.,0.)]
orientations=[(1.,0.,0.,0.),(math.sqrt(.5),0.,0.,math.sqrt(.5)),(math.sqrt(.5),0.,math.sqrt(.5),0.)]
scales=[(1.,1.,1.),(.75,1.25,.5),(1.2,.5,1.)]
children=[]
for value,position,orientation,scale in zip(entities,positions,orientations,scales):
 child=create_node(node,1,(C.c_float*3)(*position),(C.c_float*4)(*orientation))
 assert child
 C.CFUNCTYPE(None,C.c_void_p,C.c_void_p)(C.cast(getattr(ogre,'?setScale@Node@Ogre@@QEAAXAEBVVector3@2@@Z'),C.c_void_p).value)(child,(C.c_float*3)(*scale))
 attach(child,value);children.append(child)
camera=m.function(ogre,'?createCamera@SceneManager@',C.c_void_p,[C.c_void_p,C.POINTER(m.String),C.c_bool,C.c_bool])(scene,C.byref(m.String('probe-camera')),False,False)
dynamic_root=m.function(ogre,'?getRootSceneNode@SceneManager',C.c_void_p,[C.c_void_p,C.c_int])(scene,0)
camera_node=create_node(dynamic_root,0,(C.c_float*3)(0,0,400),(C.c_float*4)(1,0,0,0))
m.function(ogre,'?detachFromParent@MovableObject@',None,[C.c_void_p])(camera)
attach(camera_node,camera)
m.function(ogre,'?setAspectRatio@Frustum@',None,[C.c_void_p,C.c_float])(camera,120/64)
compositor=m.function(ogre,'?getCompositorManager2@Root@',C.c_void_p,[C.c_void_p])(root)
workspace_id=C.c_uint32(0x7139a1ce)
m.function(ogre,'?createBasicWorkspaceDef@CompositorManager2@',None,[C.c_void_p,C.POINTER(C.c_uint32),C.c_void_p,C.c_uint32])(compositor,C.byref(workspace_id),(C.c_float*4)(0,0,0,1),0)
workspace=m.function(ogre,'?addWorkspace@CompositorManager2@Ogre@@QEAAPEAVCompositorWorkspace@2@PEAVSceneManager@2@PEAVRenderTarget',C.c_void_p,[C.c_void_p,C.c_void_p,C.c_void_p,C.c_void_p,C.c_uint32,C.c_bool,C.c_int])(compositor,scene,window,camera,workspace_id.value,True,-1)
assert workspace
render=m.function(ogre,'?renderOneFrame@Root@Ogre@@QEAA_NXZ',C.c_bool,[C.c_void_p])
assert render(root)
assert render(root)
instance_data=read_buffer(buffers[1][0])
transforms=[struct.unpack_from('<12f',instance_data,index*48) for index in range(3)]
full=m.function(ogre,'?_getFullTransform@Node@',C.c_void_p,[C.c_void_p])
expected_rows=[struct.unpack('<16f',C.string_at(full(child),64))[:12] for child in children]
for actual in transforms:
 assert any(all(abs(a-b)<2e-5 for a,b in zip(actual,expected)) for expected in expected_rows),(actual,expected_rows)
print('PASS native matrix upload preserves position, rotation and nonuniform scale',flush=True)
translations=sorted((row[3],row[7],row[11]) for row in transforms)
assert translations==sorted(positions),translations
faces=m.function(ogre,'?_getNumRenderedFaces@Camera@',C.c_uint32,[C.c_void_p])(camera)
batches=m.function(ogre,'?_getNumRenderedBatches@Camera@',C.c_uint32,[C.c_void_p])(camera)
assert faces==36 and batches==1,(faces,batches)
print('PASS native frame uploaded 3 placement matrices and rendered',faces,'faces in',batches,'batches',flush=True)
(stage/'report.json').write_text(json.dumps(dict(instances=3,vertices=24,mesh_buffer_bytes=768,instance_buffer_bytes=144,source_cloned_once=source_buffer!=batch_mesh_buffer,shader_supported=True,rendered_frame=True,instancing_culling_method=1,rendered_faces=faces,rendered_batches=batches,uploaded_affine_rows=transforms),indent=2))
# Destroy one entity, recreate it, and require slot reuse without a new batch.
destroy_entity=m.function(ogre,'?destroyInstancedEntity@SceneManager@',None,[C.c_void_p,C.c_void_p])
destroy_entity(scene,entities.pop())
replacement=new(manager,C.byref(m.String('NativeInstancing/Material')),1)
assert replacement and get_owner(replacement)==owner
entities.append(replacement)
assert C.c_void_p.from_address(get_buffer(binding,0)).value==batch_mesh_buffer
assert C.c_void_p.from_address(get_buffer(binding,1)).value==buffers[1][0]
print('PASS entity removal/recreation reuses existing mesh and instance buffers',flush=True)
for value in entities:destroy_entity(scene,value)
getattr(ogre,'?destroyInstanceManager@SceneManager@Ogre@@UEAAXPEAVInstanceManager@2@@Z')(C.c_void_p(scene),C.c_void_p(manager))
release=m.function(ogre,'??1?$SharedPtr@VHardwareCounterBuffer@Ogre@@',None,[C.POINTER(m.Shared)])
for value in programs+materials:release(C.byref(value))
m.function(ogre,'??1Root@Ogre@@QEAA@XZ',None,[C.c_void_p])(root)
print('PASS native instance manager and renderer shut down cleanly; artifacts',stage,flush=True)
