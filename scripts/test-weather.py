"""Stage native DLLs and exercise rain render hooks with captured submissions.

The stage substitutes only the two original render bodies with capture callbacks,
so no GPU/game loop is needed. Exported native getters/setters and our installed
detours run normally. All file/memory changes are confined to this child process.
"""
import ctypes as C
import pathlib, shutil, struct, subprocess, sys, uuid

ROOT = pathlib.Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / 'target/re-tools'))
import pefile
from importlib.util import spec_from_file_location, module_from_spec
spec = spec_from_file_location('native_test', ROOT / 'scripts/test-performance.py')
native = module_from_spec(spec); spec.loader.exec_module(native)
String, function = native.String, native.function

def child(folder, game, with_sample, public=False):
    import os
    os.chdir(folder); search = os.add_dll_directory(str(game))
    ogre = C.WinDLL(str(folder / 'OgreMain_x64.dll'))
    particles = C.WinDLL(str(folder / 'Plugin_ParticleUniverse_x64.dll'))
    host = C.CDLL(str(folder / 'RE_Kenshi_Rust.dll'))
    setter = host.rekenshi_pack_setting_set
    setter.argtypes = [C.c_char_p, C.c_size_t, C.c_uint32, C.c_double]
    def setting(key, value):
        key = key.encode()
        assert setter(key, len(key), 1 if isinstance(value, bool) else 2, float(value)) == 0, key
    queue = function(particles, '?_updateRenderQueue@BillboardRenderer@', None, [C.c_void_p]*5)
    inject = function(ogre, '?injectBillboard@BillboardSet@', None, [C.c_void_p]*3)
    draw_set = C.create_string_buffer(1024)
    renderer = C.create_string_buffer(512)
    technique = C.create_string_buffer(1536)
    system = C.create_string_buffer(1024)
    template = String('Kenshi_Rain_RED')
    C.c_void_p.from_buffer(technique, 0x108).value = C.addressof(system)
    def template_name(value):
        nonlocal template
        template = String(value)
        C.memmove(C.addressof(system)+0x380, C.byref(template), 32)
    template_name('Kenshi_Rain_RED')
    billboard = C.create_string_buffer(0x58)
    C.c_void_p.from_buffer(renderer, 0x40).value = C.addressof(technique)
    C.c_void_p.from_buffer(renderer, 0xd8).value = C.addressof(draw_set)
    C.c_ubyte.from_buffer(draw_set, 0x215).value = 1
    C.c_float.from_buffer(draw_set, 0x1d0).value = 1
    C.c_float.from_buffer(draw_set, 0x1d4).value = 10
    material = String('Kenshi_Rain_RED')
    def material_name(value):
        nonlocal material
        material = String(value)
        C.memmove(C.addressof(technique)+0x4e8, C.byref(material), 32)
    material_name('Kenshi_Rain_RED')
    struct.pack_into('<3f', billboard, 0x1c, 0, 0, 20)
    struct.pack_into('<4f', billboard, 0x40, 0.8, 0.4, 0.2, 1)
    captured = []
    camera = C.create_string_buffer(2048)
    camera_table = C.create_string_buffer(256)
    @C.CFUNCTYPE(None, C.c_void_p)
    def camera_update(this):
        pass
    C.c_void_p.from_buffer(camera).value = C.addressof(camera_table)
    C.c_void_p.from_buffer(camera_table, 0x88).value = C.cast(camera_update,C.c_void_p).value
    @C.CFUNCTYPE(None, C.c_void_p, C.c_void_p, C.c_void_p)
    def capture(this, value, camera):
        captured.append((C.string_at(value, 0x58),
                         C.c_float.from_address(this+0x1d0).value,
                         C.c_float.from_address(this+0x1d4).value))
    nested = False
    @C.CFUNCTYPE(None, *([C.c_void_p]*5))
    def render(this, queue_ptr, camera, lod, pool):
        nonlocal nested
        if nested:
            nested = False
            template_name('CampFire')
            queue(renderer,None,camera,None,None)
            template_name('kenshi_weather_dust1')
        inject(draw_set, billboard, camera)
    kernel = C.WinDLL('kernel32', use_last_error=True)
    kernel.VirtualProtect.argtypes = [C.c_void_p, C.c_size_t, C.c_uint32, C.POINTER(C.c_uint32)]
    kernel.FlushInstructionCache.argtypes = [C.c_void_p, C.c_void_p, C.c_size_t]
    kernel.GetCurrentProcess.restype = C.c_void_p
    def replace(body, callback):
        pointer = C.cast(body, C.c_void_p).value
        patch = b'\x48\xb8'+struct.pack('<Q', C.cast(callback, C.c_void_p).value)+b'\xff\xe0'
        old = C.c_uint32()
        assert kernel.VirtualProtect(pointer, len(patch), 0x40, C.byref(old))
        C.memmove(pointer, patch, len(patch))
        assert kernel.VirtualProtect(pointer, len(patch), old.value, C.byref(old))
        assert kernel.FlushInstructionCache(kernel.GetCurrentProcess(), pointer, len(patch))
    replace(queue, render); replace(inject, capture)
    host.dllStartPlugin()
    log = (folder/'RE_Kenshi_Rust.log').read_text()
    assert 'weather: shared native particle renderer' in log, log
    configure_rain = C.CFUNCTYPE(C.c_int32, C.c_uint64, C.c_uint32, C.c_float, C.c_float)
    class WeatherV1(C.Structure):
        _fields_ = [('struct_size', C.c_uint32), ('abi_version', C.c_uint32),
                    ('configure_rain', configure_rain)]
    negotiate = host.rekenshi_weather_api
    negotiate.argtypes = [C.c_uint32, C.POINTER(WeatherV1), C.c_size_t]
    api = WeatherV1()
    assert negotiate(1, C.byref(api), C.sizeof(api)) == 0
    assert api.struct_size == C.sizeof(api) and api.abi_version == 1
    # The extracted renderer must still ask the host to authorize ownership.
    unknown_token = (1 << 64) - 1
    assert api.configure_rain(unknown_token, 1, 0.25, 1.) == 1
    assert api.configure_rain(unknown_token, 0, 0.25, 1.) == 1
    def stopped_weather():
        assert negotiate(1, C.byref(WeatherV1()), C.sizeof(api)) == 1
        assert api.configure_rain(unknown_token, 1, 0.25, 1.) == 1
    if with_sample:
        assert 'rain-refinement: native streak geometry' in log, log
    original = billboard.raw
    def submit(camera_ptr=None):
        queue(renderer, None, camera_ptr, None, None)
        assert billboard.raw == original, 'source billboard was mutated'
        assert C.c_float.from_buffer(draw_set,0x1d0).value == 1
        assert C.c_float.from_buffer(draw_set,0x1d4).value == 10
        return captured.pop()
    if public:
        setting('weather.tone_down',False)
        assert host.rekenshi_plugins_loaded()==1,log
        result,width,height=submit()
        assert abs(struct.unpack_from('<f',result,0x40)[0]-0.123)<1e-6
        assert (width,height)==(1,10)
        host.dllStopPlugin(); assert submit()==(original,1,10)
        print('PASS: independent copied particle interceptor, native source/dimension restoration and owner shutdown')
        return
    if with_sample:
        setting('weather.tone_down', False)
    else:
        # Default built-in reduces additive rain RGB, retaining alpha and geometry.
        reduced, width, height = submit()
        assert (width, height) == (1, 10)
        assert all(abs(a-b)<1e-6 for a,b in zip(struct.unpack_from('<4f', reduced,0x40),(0.28,0.14,0.07,1)))
        setting('weather.tone_down', False)
        assert submit() == (original,1,10)
        setting('weather.tone_down', True)
        # Alpha-blended dust and fog soften alpha, keeping their native colour.
        for material_value, template_value in [('DustMite','Permanent-dust-storm'),('Kenshi_mist_cloud','fogislands'),('kenshi_ash_particle1','kenshi_weather_ash2')]:
            material_name(material_value); template_name(template_value)
            reduced, _, _ = submit()
            assert all(abs(a-b)<1e-6 for a,b in zip(struct.unpack_from('<4f', reduced,0x40),(0.8,0.4,0.2,0.35)))
        # Shared additive smoke dims weather only, not fire or smoke from other systems.
        material_name('kenshi_smoke2_add'); template_name('kenshi_weather_dust1')
        assert abs(struct.unpack_from('<f',submit()[0],0x40)[0]-0.28)<1e-6
        for name in ('kenshi_twister_fire1','CampFire','Dustkick01'):
            template_name(name); assert submit() == (original,1,10)
        template_name('kenshi_weather_dust1')
        nested=True
        reduced,_,_=submit()
        assert captured.pop() == (original,1,10), 'nested fire inherited weather tint'
        assert abs(struct.unpack_from('<f',reduced,0x40)[0]-0.28)<1e-6
        material_name('Kenshi_Rain_RED'); template_name('Kenshi_Rain_RED')
        setting('weather.tone_down', False)
        assert submit() == (original,1,10)
        setting('weather.tone_down', True)
        host.dllStopPlugin()
        assert submit() == (original,1,10)
        host.dllStartPlugin()
        assert submit()[0] != original
        host.dllStopPlugin()
        stopped_weather()
        print('PASS: built-in weather default, rain/dust/fog, shared smoke exclusion, live persisted toggle, exact restoration and restart')
        return
    refined, width, height = submit()
    assert abs(width-0.12)<1e-6 and abs(height-1.8)<1e-6
    assert all(abs(a-b)<1e-6 for a,b in zip(struct.unpack_from('<4f',refined,0x40),(0.2,0.1,0.05,1)))
    # Real native Camera::getDerivedPosition reads its returned vector after
    # calling the virtual view-update entry (a no-op on our fixture camera).
    struct.pack_into('<3f',camera,0x564,0,0,17)
    assert struct.unpack_from('<3f',submit(camera)[0],0x40) == (0,0,0)
    struct.pack_into('<3f',camera,0x564,0,0,12.5)
    assert all(abs(a-b)<1e-6 for a,b in zip(struct.unpack_from('<3f',submit(camera)[0],0x40),(0.1,0.05,0.025)))
    setting('weather.rain_visibility', 0.)
    assert struct.unpack_from('<3f',submit()[0],0x40) == (0,0,0)
    setting('weather.rain_visibility', 0.5)
    setting('weather.rain_size', 2.)
    _, width, height = submit()
    assert abs(width-0.24)<1e-6 and abs(height-3.6)<1e-6
    # Explicit per-particle dimensions (legacy kenshi_rain1) also refine.
    C.c_ubyte.from_buffer(billboard).value=1
    struct.pack_into('<2f',billboard,0x14,0.15,16)
    original=billboard.raw
    refined, _, _=submit()
    assert all(abs(a-b)<1e-6 for a,b in zip(struct.unpack_from('<2f',refined,0x14),(0.054,3.6)))
    material_name('Kenshi-rainsplash_Basepart')
    assert submit()[0] == original, 'splashes must bypass'
    material_name('Kenshi_Rain_RED')
    setting('weather.rain_refinement',False)
    assert submit() == (original,1,10)
    setting('weather.rain_refinement',True)
    assert submit()[0] != original
    separate, width, height = submit()
    setting('weather.tone_down', True)
    combined, combined_width, combined_height = submit()
    assert (combined_width,combined_height) == (width,height)
    assert all(abs(a*0.35-b)<1e-6 for a,b in zip(struct.unpack_from('<3f',separate,0x40),struct.unpack_from('<3f',combined,0x40)))
    setting('weather.tone_down', False)
    # Outside a rain renderer scope, Ogre billboard calls remain exact.
    inject(draw_set,billboard,None)
    assert captured.pop() == (original,1,10)
    host.dllStopPlugin()
    assert submit() == (original,1,10)
    host.dllStartPlugin()
    assert submit()[0] != original
    host.dllStopPlugin()
    stopped_weather()
    print('PASS: native rain scopes, geometry/color, explicit dimensions, live controls, source restoration, unrelated effects, stop/restart')

if __name__ == '__main__':
    if sys.argv[1] == '--child':
        child(pathlib.Path(sys.argv[2]), pathlib.Path(sys.argv[3]), sys.argv[4]=='sample',sys.argv[4]=='public')
    else:
        game = pathlib.Path(sys.argv[1])
        folder = ROOT/'target/weather-tests'/uuid.uuid4().hex
        root = folder/'RE_Kenshi_Rust'; root.mkdir(parents=True)
        shutil.copy2(ROOT/'target/release/RE_Kenshi_Rust.dll',folder/'RE_Kenshi_Rust.dll')
        for name in ['OgreMain_x64.dll','Plugin_ParticleUniverse_x64.dll','D3DCompiler_43.dll',
                     'RenderSystem_Direct3D11_x64.dll','Plugin_Terrain_x64.dll']:
            shutil.copy2(game/name,folder/name)
        (root/'plugins.toml').write_text('schema_version=1\nplugins=[]\n')
        (root/'modpack.toml').write_text('schema_version=1\n[settings.values]\n')
        (root/'plugins').mkdir()
        subprocess.run([sys.executable,__file__,'--child',str(folder),str(game),'base'],check=True)
        shutil.copy2(ROOT/'target/release/rekenshi_rain_refinement.dll',root/'plugins/rekenshi_rain_refinement.dll')
        (root/'plugins.toml').write_text('schema_version=1\ndiscover=true\nplugins=[]\n')
        subprocess.run([sys.executable,__file__,'--child',str(folder),str(game),'sample'],check=True)

        shutil.copy2(ROOT/'target/release/rekenshi_source_transform.dll',root/'plugins/rekenshi_source_transform.dll')
        settings=root/'settings';settings.mkdir(exist_ok=True)
        (settings/'source-transform.toml').write_text('schema_version=1\n[settings.values]\n"source_transform.enabled"=true\n"runtime_policy.enabled"=true\n')
        (root/'plugins.toml').write_text('schema_version=1\ndiscover=false\n[[plugins]]\nfile="rekenshi_source_transform.dll"\nenabled=true\n')
        subprocess.run([sys.executable,__file__,'--child',str(folder),str(game),'public'],check=True)
