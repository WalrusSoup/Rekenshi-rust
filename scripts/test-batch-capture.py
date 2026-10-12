"""Check forest batch provenance and GPU bytes with the shipped native builder.

The executable is mapped with imports bound; its entry point is never run.
All writes stay under target/. The renderer uses a hidden 64x64 window.
"""
import sys,ctypes as C,pathlib,shutil,os,json,uuid,hashlib,subprocess,struct,math
from importlib.machinery import SourceFileLoader
project=pathlib.Path(__file__).resolve().parent.parent
if len(sys.argv) not in (2,3):
 raise SystemExit('Usage: test-batch-capture.py <Kenshi directory>')
if len(sys.argv)==2:
 subprocess.run([sys.executable,str(pathlib.Path(__file__).resolve()),sys.argv[1],'--child'],cwd=project,check=True,timeout=120)
 raise SystemExit(0)
assert sys.argv[2]=='--child'
os.chdir(project)
assert C.sizeof(C.c_void_p)==8
m=SourceFileLoader('fixture',str(project/'scripts/test-performance.py')).load_module()
cache_export=SourceFileLoader('instance_cache',str(project/'scripts/export-instance-cache.py')).load_module()
game=pathlib.Path(sys.argv[1]).resolve()
# These private test calls are permitted only for the inspected native binaries.
for name,expected in [
 ('kenshi_x64.exe','A596AB4E407C67B58599C54FFB32DC1BF2B64510CDEBD3FA9359EF05A576AEB1'),
 ('OgreMain_x64.dll','7D3888D21398647519F10A89B1F300D8606F761AD159E6021A797B1B9B3DF5F6'),
 ('RenderSystem_Direct3D11_x64.dll','5F224882DED5B6D3F68F14D6BB4E1762267BBB602B7B9733BD153663D1541BB9')]:
 assert hashlib.sha256((game/name).read_bytes()).hexdigest().upper()==expected,name
stage=project/'target/batch-native-tests'/uuid.uuid4().hex;stage.mkdir(parents=True)
for n in ['kenshi_x64.exe','OgreMain_x64.dll','MyGUIEngine_x64.dll','RenderSystem_Direct3D11_x64.dll','D3DCompiler_43.dll']:shutil.copy2(game/n,stage/n)
shutil.copy2(project/'target/release/RE_Kenshi_Rust.dll',stage/'RE_Kenshi_Rust.dll')
r=stage/'RE_Kenshi_Rust';r.mkdir();(r/'plugins.toml').write_text('schema_version=1\ndiscover=false\nplugins=[]\n');(r/'modpack.toml').write_text('schema_version=1\n[settings.values]\n"graphics.resource_capture"=true\n')
os.chdir(stage);search=os.add_dll_directory(str(game));base,imports=m.mapped_exe(stage,game)
ogre=C.WinDLL(str(stage/'OgreMain_x64.dll'));renderer=C.WinDLL(str(stage/'RenderSystem_Direct3D11_x64.dll'));host=C.CDLL(str(stage/'RE_Kenshi_Rust.dll'))
root=C.create_string_buffer(4096);empty=m.String('');log=m.String('probe.log');config=m.String('ogre.cfg');pathlib.Path('ogre.cfg').write_text('Render System=Direct3D11 Rendering Subsystem\n[Direct3D11 Rendering Subsystem]\n')
m.function(ogre,'??0Root@Ogre@@',C.c_void_p,[C.c_void_p,C.POINTER(m.String),C.POINTER(m.String),C.POINTER(m.String)])(root,C.byref(empty),C.byref(config),C.byref(log))
renderer.dllStartPlugin();assert m.function(ogre,'?restoreConfig@Root@Ogre@@',C.c_bool,[C.c_void_p])(root)
m.function(ogre,'?initialise@Root@Ogre@@',C.c_void_p,[C.c_void_p,C.c_bool,C.POINTER(m.String),C.POINTER(m.String)])(root,False,C.byref(empty),C.byref(empty))
print('native Ogre GPU ready',flush=True)
params=m.WindowParameters()
window=m.function(ogre,'?createRenderWindow@Root@Ogre@@',C.c_void_p,[C.c_void_p,C.POINTER(m.String),C.c_uint32,C.c_uint32,C.c_bool,C.POINTER(m.WindowParameters)])(root,C.byref(m.String('native-batch-test')),64,64,False,C.byref(params));assert window
print('hidden native render window ready',flush=True)
scene=m.function(ogre,'?createSceneManager@Root@Ogre@@QEAAPEAVSceneManager@2@G',C.c_void_p,[C.c_void_p,C.c_uint16,C.c_size_t,C.c_int,C.POINTER(m.String)])(root,1,1,0,C.byref(empty));assert scene
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
# Native build traverses geometry+1a0 -> leftmost node -> value+40.
# HardwareBuffer virtual slot 5 reads its bytes without retaining ownership.
get_vertex_buffer=m.function(ogre,'?getBuffer@VertexBufferBinding',C.c_void_p,[C.c_void_p,C.c_uint16])
def read_vertex_data(data,binding_index=0):
 binding=C.c_void_p.from_address(data+0x18).value
 shared=get_vertex_buffer(binding,binding_index)
 buffer=C.c_void_p.from_address(shared).value
 size=C.c_size_t.from_address(buffer+8).value
 output=C.create_string_buffer(size)
 m.virtual(buffer,5,None,[C.c_size_t,C.c_size_t,C.c_void_p])(buffer,0,size,output)
 return output.raw
def batch_address(geometry):
 address=C.addressof(geometry)
 head=C.c_void_p.from_address(address+0x1a0).value
 entry=C.c_void_p.from_address(head).value
 batch=C.c_void_p.from_address(entry+0x40).value
 return batch
def read_vertices(geometry,binding_index=0):
 batch=batch_address(geometry)
 data=C.c_void_p.from_address(batch+0x58).value
 return read_vertex_data(data,binding_index)
mesh_shared=m.function(ogre,'?getMesh@Entity@',C.c_void_p,[C.c_void_p])(entity)
mesh=C.c_void_p.from_address(mesh_shared).value
source_submesh=m.function(ogre,'?getSubMesh@Mesh@Ogre@@QEBAPEAVSubMesh@2@G@Z',C.c_void_p,[C.c_void_p,C.c_uint16])(mesh,0)
baseline_geometry=C.create_string_buffer(1024)
C.CFUNCTYPE(C.c_void_p,C.c_void_p,C.c_void_p,C.c_void_p)(base+0xa555d0)(baseline_geometry,scene,node)
add_baseline=C.CFUNCTYPE(None,C.c_void_p,C.c_void_p,C.c_void_p,C.c_void_p,C.c_void_p,C.c_void_p)(base+0xa55980)
# Deliberately distinct, exactly representable floats exercise field offsets,
# quaternion component order, nonuniform scale, and renderer color swizzling.
placements=[
 dict(position=[0,0,0],orientation_wxyz=[1,0,0,0],scale=[1,1,1],color_components=[1,1,1,1]),
 dict(position=[12,-2,7],orientation_wxyz=[0.5,0.5,0.5,0.5],scale=[2,0.5,1.5],color_components=[0.25,0.5,0.75,1]),
 dict(position=[-8,3,-4],orientation_wxyz=[0,0,0,1],scale=[0.75,1.25,2],color_components=[0.75,0.25,0.5,1]),
]
def submit(add,geometry):
 for p in placements:
  add(geometry,entity,(C.c_float*3)(*p['position']),(C.c_float*4)(*p['orientation_wxyz']),(C.c_float*3)(*p['scale']),(C.c_float*4)(*p['color_components']))
submit(add_baseline,baseline_geometry)
# addEntity prepares an unshared native submesh before queueing it.
source_bytes=read_vertex_data(C.c_void_p.from_address(source_submesh+8).value)
C.CFUNCTYPE(None,C.c_void_p)(base+0xa572d0)(baseline_geometry)
baseline_bytes=read_vertices(baseline_geometry)
baseline_color_bytes=read_vertices(baseline_geometry,1)
assert len(baseline_color_bytes)==72*4
native_transform=(C.c_float*16)()
C.CFUNCTYPE(None,C.c_void_p,C.c_void_p)(base+0xa5b920)(batch_address(baseline_geometry),native_transform)
output_transform_rows=[list(native_transform[i:i+4]) for i in range(0,16,4)]
(stage/'source-vertices.bin').write_bytes(source_bytes)
(stage/'expanded-vertices.bin').write_bytes(baseline_bytes)
(stage/'placements.json').write_text(json.dumps(placements))
(stage/'native-output-transform.json').write_text(json.dumps(output_transform_rows))
assert len(source_bytes)==24*32 and len(baseline_bytes)==3*len(source_bytes)
for placement_index,p in enumerate(placements):
 record=struct.unpack('<16f',cache_export.affine_record(p))
 for vertex_index in range(24):
  source=struct.unpack_from('<3f',source_bytes,vertex_index*32)
  expected=[sum(record[row*4+j]*source[j] for j in range(3))+record[row*4+3] for row in range(3)]
  actual=struct.unpack_from('<3f',baseline_bytes,(placement_index*24+vertex_index)*32)
  world=[sum(row[j]*actual[j] for j in range(3))+row[3] for row in output_transform_rows[:3]]
  assert all(math.isclose(a,b,rel_tol=1e-6,abs_tol=1e-5) for a,b in zip(world,expected)),(placement_index,vertex_index,actual,world,expected,output_transform_rows)
print('PASS baked instance affine transforms match all 72 native GPU positions with native world transform',output_transform_rows,flush=True)
print('native uninstrumented baseline saved',len(baseline_bytes),flush=True)
geometry=C.create_string_buffer(1024)
C.CFUNCTYPE(C.c_void_p,C.c_void_p,C.c_void_p,C.c_void_p)(base+0xa555d0)(geometry,scene,node)
print('native forest geometry constructed',flush=True)
host.dllStartPlugin()
add=C.CFUNCTYPE(None,C.c_void_p,C.c_void_p,C.c_void_p,C.c_void_p,C.c_void_p,C.c_void_p)(base+0xa55980)
submit(add,geometry)
print('three native submesh submissions queued',flush=True)
C.CFUNCTYPE(None,C.c_void_p)(base+0xa572d0)(geometry)
print('native forest build completed',flush=True)
assert read_vertices(geometry)==baseline_bytes
assert read_vertices(geometry,1)==baseline_color_bytes
print('PASS byte-identical expanded geometry and color buffers with capture enabled',flush=True)
host.rekenshi_plugin_setting_set.argtypes=[C.c_char_p,C.c_size_t,C.c_uint32,C.c_double]
key=b'graphics.resource_capture';assert host.rekenshi_plugin_setting_set(key,len(key),1,0.)==0
report=json.loads(next((r/'graphics-captures').glob('*/summary.json')).read_text())
assert report['batch_attribution_unavailable'] is None,report['batch_attribution_unavailable']
assert report['batch_snapshot_failures_process_total']==0
assert all(report[k]==0 for k in ('untracked_creations','attachment_failures','dropped_events','io_errors'))
buffers=[x for x in report['resources'] if x.get('batch') and x['description']['bind_flags']&1]
assert buffers,report
assert sorted(b['estimated_payload_bytes'] for b in buffers)==[288,2304]
for b in buffers:
 q=b['batch'];assert q['queued_submeshes']==3,q;assert sum(s['copies'] for s in q['sources'])==3,q;assert sum(s['copies']*s['vertices_per_copy'] for s in q['sources'])==q['output_vertices'],q
 assert q['effective_material'],q
 assert all(s['mesh']=='Prefab_Cube' and s['vertices_per_copy']==24 and s['indices_per_copy']==36 for s in q['sources']),q
 assert b['estimated_payload_bytes'] in (len(baseline_bytes),len(baseline_color_bytes))
 assert q['omitted_submeshes']==0
 assert q['placement_schema_version']==1 and q['placement_space']=='builder_input'
 assert q['placement_color_order']=='renderer_native'
 assert q['batch_origin']==[row[3] for row in output_transform_rows[:3]],q
 assert q['omitted_placements']==q['invalid_placements']==0
 assert len(q['sources'])==1 and q['sources'][0]['placements']==placements,q
 print('PASS exact native positions, wxyz rotations, scales, and color components',flush=True)
 print('PASS native provenance',b['estimated_payload_bytes'],q,flush=True)
cache_data,cache_metadata=cache_export.bake(report,hashlib.sha256(next((r/'graphics-captures').glob('*/summary.json')).read_bytes()).hexdigest())
verified_metadata,cache_payload=cache_export.read_cache(cache_data)
assert cache_metadata==verified_metadata
assert verified_metadata['record_count']==3 and len(cache_payload)==192
assert verified_metadata['chunks'][0]['effective_material']==buffers[0]['batch']['effective_material']
assert cache_payload==b''.join(cache_export.affine_record(p) for p in placements)
(stage/'native-placement-cache.instances').write_bytes(cache_data)
print('PASS actual native placement capture exported to 192-byte instance payload',flush=True)
host.dllStopPlugin();C.CFUNCTYPE(None,C.c_void_p)(base+0xa55860)(geometry)
C.CFUNCTYPE(None,C.c_void_p)(base+0xa55860)(baseline_geometry)
release=m.function(ogre,'??1?$SharedPtr@VHardwareCounterBuffer@Ogre@@',None,[C.POINTER(m.Shared)])
for material in materials:release(C.byref(material))
m.function(ogre,'??1Root@Ogre@@QEAA@XZ',None,[C.c_void_p])(root)
print('PASS shutdown; report',next((r/'graphics-captures').glob('*/summary.json')),flush=True)
