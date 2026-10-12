"""Exercise the real executable's native adapters without starting Kenshi.

All writes are staged under target/. EXE imports are bound in the test process;
its entry point and game loop are never invoked.
"""
import ctypes as C
import os, pathlib, shutil, struct, subprocess, sys, uuid
ROOT=pathlib.Path(__file__).resolve().parent.parent
sys.path.insert(0,str(ROOT/'target/re-tools'))
import pefile
CALL=C.CFUNCTYPE
class String(C.Structure):
    _fields_=[('storage',C.c_ubyte*16),('length',C.c_size_t),('capacity',C.c_size_t)]
    def __init__(self,value):
        super().__init__(); value=value.encode() if isinstance(value,str) else value
        self.length=len(value)
        if len(value)<16:
            self.capacity=15; C.memmove(C.addressof(self),value,len(value))
        else:
            self.buffer=C.create_string_buffer(value); self.capacity=len(value)
            C.c_void_p.from_address(C.addressof(self)).value=C.addressof(self.buffer)

class WindowParameterNode(C.Structure):
    # The shipped renderer's map uses 0x28-byte slots for each string, even
    # though String models only the 0x20-byte data/length/capacity prefix.
    # RenderSystem_Direct3D11_x64.dll SHA-256:
    # 5F224882DED5B6D3F68F14D6BB4E1762267BBB602B7B9733BD153663D1541BB9
    # RVA 0x304f0 reads key +0x18, 0x304f4 value +0x40, and 0x3052b nil +0x69.
    _fields_=[('left',C.c_void_p),('parent',C.c_void_p),('right',C.c_void_p),
              ('key',String),('key_padding',C.c_size_t),
              ('value',String),('value_padding',C.c_size_t),
              ('color',C.c_ubyte),('nil',C.c_ubyte),('pad',C.c_ubyte*6)]

class WindowParameters(C.Structure):
    """Borrowed single-entry VC100 map; keep its nodes alive during native reads."""
    _fields_=[('allocator',C.c_size_t),('head',C.c_void_p),('size',C.c_size_t)]

    def __init__(self):
        super().__init__()
        self._head=WindowParameterNode()
        self._entry=WindowParameterNode()
        head,entry=self._head,self._entry
        head.left=head.parent=head.right=C.addressof(entry)
        head.nil=head.color=1
        entry.left=entry.parent=entry.right=C.addressof(head)
        entry.color=1
        entry.key=String('hidden')
        entry.value=String('true')
        self.head=C.addressof(head)
        self.size=1

class Shared(C.Structure):
    _fields_=[('object',C.c_void_p),('info',C.c_void_p)]
class Result(C.Structure):
    _fields_=[('pixels',Shared),('metadata',Shared)]
def function(dll,needle,result,args):
    pe=pefile.PE(dll._name)
    names=[e.name.decode() for e in pe.DIRECTORY_ENTRY_EXPORT.symbols if e.name and needle in e.name.decode()]
    assert len(names)==1,(needle,names)
    address=C.cast(getattr(dll,names[0]),C.c_void_p).value
    return CALL(result,*args)(address)
def virtual(pointer,slot,result,args):
    table=C.c_void_p.from_address(pointer).value
    return CALL(result,C.c_void_p,*args)(C.c_void_p.from_address(table+slot*8).value)
def mapped_exe(folder,game):
    kernel=C.WinDLL('kernel32',use_last_error=True)
    kernel.LoadLibraryExW.argtypes=[C.c_wchar_p,C.c_void_p,C.c_uint32];kernel.LoadLibraryExW.restype=C.c_void_p
    base=kernel.LoadLibraryExW(str(folder/'kenshi_x64.exe'),None,0)
    assert base,C.get_last_error()
    kernel.GetProcAddress.argtypes=[C.c_void_p,C.c_void_p];kernel.GetProcAddress.restype=C.c_void_p
    kernel.VirtualProtect.argtypes=[C.c_void_p,C.c_size_t,C.c_uint32,C.POINTER(C.c_uint32)]
    pe=pefile.PE(str(folder/'kenshi_x64.exe')); keep=[]
    for descriptor in pe.DIRECTORY_ENTRY_IMPORT:
        name=descriptor.dll.decode(); path=folder/name
        if not path.exists():path=game/name
        dll=C.WinDLL(str(path) if path.exists() else name); keep.append(dll)
        image=pe.get_memory_mapped_image()
        for index,entry in enumerate(descriptor.imports):
            hint=struct.unpack_from('<Q',image,descriptor.struct.OriginalFirstThunk+index*8)[0]
            actual=image[hint+2:].split(b'\0',1)[0] if not hint>>63 else None
            symbol=C.create_string_buffer(actual) if actual else None
            address=kernel.GetProcAddress(dll._handle,C.cast(symbol,C.c_void_p) if symbol else entry.ordinal)
            assert address,(name,entry.name)
            slot=base+entry.address-pe.OPTIONAL_HEADER.ImageBase;old=C.c_uint32()
            assert kernel.VirtualProtect(slot,8,0x04,C.byref(old))
            C.c_void_p.from_address(slot).value=address
            assert kernel.VirtualProtect(slot,8,old.value,C.byref(old))
    return base,keep
def child(folder,game):
    os.chdir(folder); search=os.add_dll_directory(str(game))
    base,imports=mapped_exe(folder,game)
    ogre=C.WinDLL(str(folder/'OgreMain_x64.dll'))
    gui=C.WinDLL(str(folder/'MyGUIEngine_x64.dll'))
    host=C.CDLL(str(folder/'RE_Kenshi_Rust.dll'))
    for name in ['rekenshi_dds_skipped_bytes','rekenshi_collider_cache_hits','rekenshi_sic_decoded','rekenshi_texture_sidecar_hits']:
        getattr(host,name).restype=C.c_uint64
    host.rekenshi_pack_setting_set.argtypes=[C.c_char_p,C.c_size_t,C.c_uint32,C.c_double]
    def setting(key,value):
        key=key.encode(); assert host.rekenshi_pack_setting_set(key,len(key),1,float(value))==0,key
    print('Mapped executable imports; constructing headless Ogre root',flush=True)
    root=C.create_string_buffer(4096); empty=String(b''); log=String('native-performance.log')
    function(ogre,'??0Root@Ogre@@',C.c_void_p,[C.c_void_p,C.POINTER(String),C.POINTER(String),C.POINTER(String)])(root,C.byref(empty),C.byref(empty),C.byref(log))
    manager=function(ogre,'?getSingletonPtr@ResourceGroupManager@',C.c_void_p,[])()
    location=String(str(folder));kind=String('FileSystem');group=String('General')
    function(ogre,'?addResourceLocation@ResourceGroupManager@',None,[C.c_void_p,C.POINTER(String),C.POINTER(String),C.POINTER(String),C.c_bool,C.c_bool])(manager,C.byref(location),C.byref(kind),C.byref(group),False,True)
    archive=C.create_string_buffer(256)
    function(ogre,'??0FileSystemArchive@Ogre@@',C.c_void_p,[C.c_void_p,C.POINTER(String),C.POINTER(String),C.c_bool])(archive,C.byref(location),C.byref(kind),True)
    opener=function(ogre,'?open@FileSystemArchive@Ogre@@',C.c_void_p,[C.c_void_p,C.POINTER(Shared),C.POINTER(String),C.c_bool])
    release=function(ogre,'??1?$SharedPtr@VHardwareCounterBuffer@Ogre@@',None,[C.POINTER(Shared)])
    def stream(name):
        output=Shared(); opener(archive,C.byref(output),C.byref(String(name)),True); assert output.object;return output
    def read(value):
        size=C.c_size_t.from_address(value.object+0x30).value;output=C.create_string_buffer(size)
        virtual(value.object,10,None,[C.c_size_t])(value.object,0)
        assert virtual(value.object,3,C.c_size_t,[C.c_void_p,C.c_size_t])(value.object,output,size)==size
        return output.raw
    opened=CALL(None,C.c_void_p,C.POINTER(String),C.POINTER(String),C.c_void_p,C.POINTER(Shared))(base+0x82ddc0)
    quality=C.c_int32.from_address(base+0x213352c)
    expected={}
    for level in range(5):
        quality.value=level; source=stream('sample_HI.dds')
        opened(None,C.byref(String('sample_HI.dds')),C.byref(group),None,C.byref(source))
        expected[level]=read(source); release(C.byref(source))
    print('Native DDS baseline captured',flush=True)
    host.dllStartPlugin()
    assert host.rekenshi_pack_layers()==9,(host.rekenshi_pack_layers(),(folder/'RE_Kenshi_Rust.log').read_text())
    assert host.rekenshi_internal_plugins()==9
    setting('performance.dds_mip_skip',True)
    for level in range(5):
        quality.value=level;source=stream('sample_HI.dds');before=host.rekenshi_dds_skipped_bytes()
        print('DDS input',level,hex(C.c_void_p.from_address(source.object).value-ogre._handle),C.c_size_t.from_address(source.object+0x30).value,flush=True)
        opened(None,C.byref(String('sample_HI.dds')),C.byref(group),None,C.byref(source))
        assert read(source)==expected[level],level
        assert (host.rekenshi_dds_skipped_bytes()>before)==(level>0),(level,host.rekenshi_dds_skipped_bytes(),before)
        release(C.byref(source))
    setting('performance.dds_mip_skip',False)
    source=stream('sample_HI.dds');before=host.rekenshi_dds_skipped_bytes()
    opened(None,C.byref(String('sample_HI.dds')),C.byref(group),None,C.byref(source));assert read(source)==expected[4]
    assert host.rekenshi_dds_skipped_bytes()==before;release(C.byref(source))
    print('PASS: native DDS differential output, quality levels and bypass',flush=True)
    codec=function(ogre,'?getCodec@Codec@Ogre@@SAPEAV12@AEBV',C.c_void_p,[C.POINTER(String)])(C.byref(String('sic')))
    assert codec
    bridge=C.CDLL(str(ROOT/'target/native-unwind.dll'))
    bridge.reference_sic.argtypes=[C.c_void_p,C.c_size_t,C.c_void_p,C.c_size_t,C.c_size_t,C.c_int,C.c_int,C.c_int]
    bridge.reference_sic.restype=C.c_size_t
    for format,channels in enumerate([3,4,1,2]):
        raw=bytes((i*37+i//7)%256 for i in range(17*11*channels))
        for filter in range(7):
            for layout in range(2):
                encoded=C.create_string_buffer(len(raw)+4096)
                size=bridge.reference_sic(encoded,len(encoded),C.c_char_p(raw),17,11,format,filter,layout)
                assert size>16;(folder/'oracle.sic').write_bytes(encoded.raw[:size])
                source=stream('oracle.sic');result=Result()
                virtual(codec,3,C.c_void_p,[C.POINTER(Result),C.POINTER(Shared)])(codec,C.byref(result),C.byref(source))
                assert read(result.pixels)==raw,(format,filter,layout)
                release(C.byref(result.pixels));release(C.byref(result.metadata));release(C.byref(source))
    print('PASS: all 56 upstream SIC pixel/filter/layout combinations',flush=True)
    source=stream('sample.tga.sic'); result=Result()
    virtual(codec,3,C.c_void_p,[C.POINTER(Result),C.POINTER(Shared)])(codec,C.byref(result),C.byref(source))
    pixels=bytes((i*13)%256 for i in range(64*32*3))
    assert read(result.pixels)==pixels
    assert struct.unpack_from('<III',C.string_at(result.metadata.object,48),8)==(32,64,1)
    # Copy ref, release original first, then read through the surviving reference.
    copy=Shared(result.pixels.object,result.pixels.info)
    C.c_uint32.from_address(copy.info+0x20).value+=1
    release(C.byref(result.pixels));assert read(copy)==pixels
    release(C.byref(copy));release(C.byref(result.metadata));release(C.byref(source))
    image=C.create_string_buffer(64)
    function(ogre,'??0Image@Ogre@@QEAA@XZ',C.c_void_p,[C.c_void_p])(image)
    source=stream('sample.tga.sic')
    function(ogre,'?load@Image@Ogre@@QEAAAEAV12@AEAV',C.c_void_p,[C.c_void_p,C.POINTER(Shared),C.POINTER(String)])(image,C.byref(source),C.byref(String('sic')))
    release(C.byref(source))
    pointer=C.c_void_p.from_address(C.addressof(image)+0x30).value
    assert C.string_at(pointer,len(pixels))==pixels
    function(ogre,'??1Image@Ogre@@UEAA@XZ',None,[C.c_void_p])(image)
    bad=stream('bad.sic');bridge=C.CDLL(str(ROOT/'target/native-unwind.dll'))
    bridge.catch_decode.argtypes=[C.c_void_p]*4
    table=C.c_void_p.from_address(codec).value;target=C.c_void_p.from_address(table+24).value
    assert bridge.catch_decode(target,codec,C.byref(Result()),C.byref(bad))==0
    release(C.byref(bad))
    print('PASS: SIC pixels, metadata, copied references, Image adoption and C++ exception unwind',flush=True)
    setting('performance.compressed_textures',True)
    before=host.rekenshi_texture_sidecar_hits(); source=stream('sample.tga')
    assert read(source)==(folder/'sample.tga').read_bytes();release(C.byref(source))
    assert host.rekenshi_texture_sidecar_hits()==before+1
    (folder/'sample.tga.sic.toml').write_text('source_size=0\nmodified_secs=0\nmodified_nanos=0\n')
    source=stream('sample.tga');release(C.byref(source));assert host.rekenshi_texture_sidecar_hits()==before+1
    setting('performance.compressed_textures',False)
    print('PASS: converted sidecar substitution and stale-source fallback',flush=True)
    physics=C.WinDLL(str(folder/'PhysXLoader64.dll'))
    physics.NxCreatePhysicsSDK.argtypes=[C.c_uint32,C.c_void_p,C.c_void_p,C.c_void_p,C.c_void_p,C.c_char_p];physics.NxCreatePhysicsSDK.restype=C.c_void_p
    desc=(C.c_uint32*8)(65536,256,2048,0,3,32,0xffffffff,0)
    sdk=physics.NxCreatePhysicsSDK(0x02080400,None,None,C.byref(desc),None,b'BE9042F0-ADC4-4b12-A93E-DB8A731FDBD5');assert sdk
    load=CALL(C.c_void_p,C.POINTER(String),C.c_int32)(base+0x7e53f0)
    destroy=CALL(None,C.c_void_p)(base+0x1d99f0)
    setting('performance.collider_cache',True)
    first=load(C.byref(String('sample.bin')),0);assert first
    before=host.rekenshi_collider_cache_hits()
    second=load(C.byref(String('sample.bin')),0);assert second and second!=first
    assert host.rekenshi_collider_cache_hits()==before+1
    # Scene/actor/shape descriptors are independent; mutation never reaches cache.
    def shapes(collection):
        scene=C.c_void_p.from_address(C.c_void_p.from_address(collection+0xd8).value).value
        actor=C.c_void_p.from_address(C.c_void_p.from_address(scene+0x128).value).value
        return C.c_void_p.from_address(C.c_void_p.from_address(actor+0xf8).value).value
    shape=shapes(second); position=C.c_float.from_address(shape+0x34); original=position.value;position.value=9876.
    third=load(C.byref(String('sample.bin')),0);assert third and shapes(third)!=shape
    assert C.c_float.from_address(shapes(third)+0x34).value==original
    for collection in [first,second,third]:destroy(collection)
    setting('performance.collider_cache',False)
    collection=load(C.byref(String('sample.bin')),0);assert collection;destroy(collection)
    assert host.rekenshi_collider_cache_hits()==before+2
    host.dllStopPlugin();assert host.rekenshi_pack_layers()==0
    host.dllStartPlugin();assert host.rekenshi_pack_layers()==9;host.dllStopPlugin()
    physics.NxReleasePhysicsSDK.argtypes=[C.c_void_p];physics.NxReleasePhysicsSDK(sdk)
    function(ogre,'??1FileSystemArchive@Ogre@@UEAA@XZ',None,[C.c_void_p])(archive)
    function(ogre,'??1Root@Ogre@@QEAA@XZ',None,[C.c_void_p])(root)
    print('PASS: real PhysX collider clones, independent mutation, native releases, bypass and pack restart',flush=True)
def public_child(folder,game):
    os.chdir(folder); search=os.add_dll_directory(str(game))
    base,imports=mapped_exe(folder,game)
    ogre=C.WinDLL(str(folder/'OgreMain_x64.dll'))
    gui=C.WinDLL(str(folder/'MyGUIEngine_x64.dll'))
    host=C.CDLL(str(folder/'RE_Kenshi_Rust.dll'))
    for name in ['rekenshi_dds_skipped_bytes','rekenshi_collider_cache_hits','rekenshi_sic_decoded','rekenshi_texture_sidecar_hits']:
        getattr(host,name).restype=C.c_uint64
    host.rekenshi_pack_setting_set.argtypes=[C.c_char_p,C.c_size_t,C.c_uint32,C.c_double]
    def setting(key,value):
        key=key.encode(); assert host.rekenshi_pack_setting_set(key,len(key),1,float(value))==0,key
    print('Mapped executable imports; constructing headless Ogre root',flush=True)
    root=C.create_string_buffer(4096); empty=String(b''); log=String('native-performance.log')
    function(ogre,'??0Root@Ogre@@',C.c_void_p,[C.c_void_p,C.POINTER(String),C.POINTER(String),C.POINTER(String)])(root,C.byref(empty),C.byref(empty),C.byref(log))
    manager=function(ogre,'?getSingletonPtr@ResourceGroupManager@',C.c_void_p,[])()
    location=String(str(folder));kind=String('FileSystem');group=String('General')
    function(ogre,'?addResourceLocation@ResourceGroupManager@',None,[C.c_void_p,C.POINTER(String),C.POINTER(String),C.POINTER(String),C.c_bool,C.c_bool])(manager,C.byref(location),C.byref(kind),C.byref(group),False,True)
    archive=C.create_string_buffer(256)
    function(ogre,'??0FileSystemArchive@Ogre@@',C.c_void_p,[C.c_void_p,C.POINTER(String),C.POINTER(String),C.c_bool])(archive,C.byref(location),C.byref(kind),True)
    opener=function(ogre,'?open@FileSystemArchive@Ogre@@',C.c_void_p,[C.c_void_p,C.POINTER(Shared),C.POINTER(String),C.c_bool])
    release=function(ogre,'??1?$SharedPtr@VHardwareCounterBuffer@Ogre@@',None,[C.POINTER(Shared)])
    def stream(name):
        output=Shared(); opener(archive,C.byref(output),C.byref(String(name)),True); assert output.object;return output
    def read(value):
        size=C.c_size_t.from_address(value.object+0x30).value;output=C.create_string_buffer(size)
        virtual(value.object,10,None,[C.c_size_t])(value.object,0)
        assert virtual(value.object,3,C.c_size_t,[C.c_void_p,C.c_size_t])(value.object,output,size)==size
        return output.raw
    opened=CALL(None,C.c_void_p,C.POINTER(String),C.POINTER(String),C.c_void_p,C.POINTER(Shared))(base+0x82ddc0)
    quality=C.c_int32.from_address(base+0x213352c)
    quality.value=2; baseline=stream('sample_HI.dds')
    opened(None,C.byref(String('sample_HI.dds')),C.byref(group),None,C.byref(baseline))
    expected=read(baseline); release(C.byref(baseline))
    host.dllStartPlugin()
    assert host.rekenshi_pack_layers()==9 and host.rekenshi_plugins_loaded()==1, (folder/'RE_Kenshi_Rust.log').read_text()
    quality.value=0; source=stream('sample_HI.dds')
    opened(None,C.byref(String('sample_HI.dds')),C.byref(group),None,C.byref(source))
    assert read(source)==expected, 'independent mip rule must override built-in-off/native quality zero'
    release(C.byref(source))
    terrain=C.CDLL(str(folder/'Plugin_Terrain_x64.dll'))
    open_tiff=CALL(C.c_void_p,C.c_char_p,C.c_int)(terrain._handle+0x20880)
    read_pixel=CALL(C.c_size_t,C.c_void_p,C.c_int,C.c_int,C.c_void_p)(terrain._handle+0x21640)
    close=CALL(C.c_void_p,C.c_void_p,C.c_uint32)(terrain._handle+0x143f0)
    reader=open_tiff(str(folder/'data/newland/land/redirect.tif').encode(),1); assert reader
    pixel=C.c_uint16(); assert read_pixel(reader,17,23,C.byref(pixel))==1 and pixel.value==23*64+17+7
    codec=function(ogre,'?getCodec@Codec@Ogre@@SAPEAV12@AEBV',C.c_void_p,[C.POINTER(String)])(C.byref(String('rki')));assert codec
    (folder/'example.rki').write_bytes(b'RKI'+bytes([11,22,33,44]))
    source=stream('example.rki'); result=Result()
    virtual(codec,3,C.c_void_p,[C.POINTER(Result),C.POINTER(Shared)])(codec,C.byref(result),C.byref(source))
    assert read(result.pixels)==bytes([11,22,33,44])
    for value in [result.pixels,result.metadata,source]:release(C.byref(value))
    physics=C.WinDLL(str(folder/'PhysXLoader64.dll'))
    physics.NxCreatePhysicsSDK.argtypes=[C.c_uint32,C.c_void_p,C.c_void_p,C.c_void_p,C.c_void_p,C.c_char_p];physics.NxCreatePhysicsSDK.restype=C.c_void_p
    desc=(C.c_uint32*8)(65536,256,2048,0,3,32,0xffffffff,0)
    sdk=physics.NxCreatePhysicsSDK(0x02080400,None,None,C.byref(desc),None,b'BE9042F0-ADC4-4b12-A93E-DB8A731FDBD5'); assert sdk
    load=CALL(C.c_void_p,C.POINTER(String),C.c_int32)(base+0x7e53f0); destroy=CALL(None,C.c_void_p)(base+0x1d99f0)
    first=load(C.byref(String('sample.bin')),0); assert first
    before=host.rekenshi_collider_cache_hits();second=load(C.byref(String('sample.bin')),0); assert second and second!=first
    assert host.rekenshi_collider_cache_hits()==before+1, 'independent collider rule owns caching while built-in is off'
    for value in [first,second]:destroy(value)
    host.dllStopPlugin()
    assert read_pixel(reader,17,23,C.byref(pixel))==1 and pixel.value==23*64+17, 'owned read interception survived shutdown'
    close(reader,1)
    registered=function(ogre,'?isCodecRegistered@Codec@Ogre@@',C.c_bool,[C.POINTER(String)])
    assert not registered(C.byref(String('rki'))), 'native custom codec slot survived shutdown'
    physics.NxReleasePhysicsSDK.argtypes=[C.c_void_p];physics.NxReleasePhysicsSDK(sdk)
    function(ogre,'??1FileSystemArchive@Ogre@@UEAA@XZ',None,[C.c_void_p])(archive)
    function(ogre,'??1Root@Ogre@@QEAA@XZ',None,[C.c_void_p])(root)
    print('PASS: independent DLL controls native terrain, DDS, collider and custom codec; native allocation/release and shutdown cleanup',flush=True)

def stage(game):
    folder=ROOT/'target/performance-tests'/uuid.uuid4().hex;folder.mkdir(parents=True)
    for name in ['RE_Kenshi_Rust.dll','pack-config.exe','texture-pack.exe']:
        shutil.copy2(ROOT/'target/release'/name,folder/name)
    for name in ['kenshi_x64.exe','OgreMain_x64.dll','MyGUIEngine_x64.dll','D3DCompiler_43.dll','RenderSystem_Direct3D11_x64.dll','Plugin_Terrain_x64.dll','Plugin_ParticleUniverse_x64.dll','PhysXLoader64.dll','PhysXCore64.dll','PhysXCooking64.dll','PhysXDevice64.dll','cudart64_30_9.dll','physxcudart_20.dll']:
        shutil.copy2(game/name,folder/name)
    (folder/'RE_Kenshi_Rust').mkdir()
    (folder/'RE_Kenshi_Rust/plugins.toml').write_text('schema_version=1\n')
    heightmap=folder/'data/newland/land/fullmap.tif';heightmap.parent.mkdir(parents=True)
    fields=[(256,3,64),(257,3,64),(258,3,16),(259,3,1),(273,4,110),(277,3,1),(278,4,64),(279,4,8192)]
    tiff=b'II\x2a\0'+struct.pack('<IH',8,len(fields))+b''.join(struct.pack('<HHII',tag,kind,1,value) for tag,kind,value in fields)+b'\0'*4
    heightmap.write_bytes(tiff+struct.pack('<4096H',*range(4096)))
    subprocess.run([str(folder/'pack-config.exe'),str(folder/'RE_Kenshi_Rust')],check=True)
    header=bytearray(128);header[:4]=b'DDS '
    for offset,value in [(4,124),(8,0x21007),(12,64),(16,64),(28,7),(76,32),(80,4),(108,0x401008)]:struct.pack_into('<I',header,offset,value)
    header[84:88]=b'DXT1';payload=bytes(i%251 for i in range(2744));(folder/'sample_HI.dds').write_bytes(header+payload)
    tga=bytearray(18);tga[2]=2;struct.pack_into('<HH',tga,12,64,32);tga[16]=24;tga[17]=0x20
    (folder/'sample.tga').write_bytes(tga+bytes((i*13)%256 for i in range(64*32*3)))
    subprocess.run([str(folder/'texture-pack.exe'),str(folder/'sample.tga'),'--sidecar'],check=True)
    (folder/'bad.sic').write_bytes(b'bad!')
    source=game/'data/buildings/windmill/Small Wind Generator_COL.bin';shutil.copy2(source,folder/'sample.bin')
    return folder
if __name__=='__main__':
    if sys.argv[1]=='--child':child(pathlib.Path(sys.argv[2]),pathlib.Path(sys.argv[3]))
    elif sys.argv[1]=='--public-child':public_child(pathlib.Path(sys.argv[2]),pathlib.Path(sys.argv[3]))
    else:
        game=pathlib.Path(sys.argv[1]);folder=stage(game)
        print('Staged:',folder,flush=True)
        subprocess.run([sys.executable,__file__,'--child',str(folder),str(game)],check=True)
        folder=stage(game)
        plugins=folder/'RE_Kenshi_Rust/plugins'; plugins.mkdir(exist_ok=True)
        shutil.copy2(ROOT/'target/release/rekenshi_source_transform.dll',plugins)
        settings=plugins.parent/'settings';settings.mkdir(exist_ok=True)
        (settings/'source-transform.toml').write_text('schema_version=1\n[settings.values]\n"source_transform.enabled"=true\n"runtime_policy.enabled"=true\n')
        (plugins.parent/'plugins.toml').write_text('schema_version=1\ndiscover=true\n')
        subprocess.run([sys.executable,__file__,'--public-child',str(folder),str(game)],check=True)
