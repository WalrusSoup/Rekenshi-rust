"""Mix separately compiled SDK DLLs, real hooks, graphics and generated settings."""
import ctypes as C
import pathlib, shutil, subprocess, sys, uuid
ROOT = pathlib.Path(__file__).resolve().parent.parent

def disabled_child(folder):
    ogre=C.CDLL(str(folder/'OgreMain_x64.dll'))
    native=C.CDLL(str(folder/'native-menu.dll'))
    native.menu_setup.argtypes=[C.c_char_p]
    native.menu_setup((ROOT/'tests/fixtures/menu-resources.xml').read_bytes())
    host=C.CDLL(str(folder/'RE_Kenshi_Rust.dll'))
    counter=C.CDLL(str(folder/'RE_Kenshi_Rust/plugins/rekenshi_callback_counter.dll'))
    counter.rekenshi_counter_calls.restype=C.c_uint64
    getter=getattr(ogre,'?getSingletonPtr@Root@Ogre@@SAPEAV12@XZ')
    getter.restype=C.c_void_p
    original=getter()
    host.dllStartPlugin()
    assert host.rekenshi_plugins_loaded()==1
    assert host.rekenshi_plugins_failed()==0
    for _ in range(5): assert getter()==original
    assert counter.rekenshi_counter_calls()==0
    host.rekenshi_plugin_setting_set.argtypes=[C.c_char_p,C.c_size_t,C.c_uint32,C.c_double]
    assert host.rekenshi_plugin_setting_set(b'callback_counter.enabled',24,1,1)==1
    host.dllStopPlugin(); native.menu_shutdown()
    print('PASS: explicit disabled DLL overrides folder discovery, including filename case')

def child(folder):
    ogre=C.CDLL(str(folder/'OgreMain_x64.dll'))
    native=C.CDLL(str(folder/'native-menu.dll'))
    native.menu_setup.argtypes=[C.c_char_p]
    native.menu_setup((ROOT/'tests/fixtures/menu-resources.xml').read_bytes())
    host=C.CDLL(str(folder/'RE_Kenshi_Rust.dll'))
    counter=C.CDLL(str(folder/'RE_Kenshi_Rust/plugins/rekenshi_callback_counter.dll'))
    counter.rekenshi_counter_calls.restype=C.c_uint64
    counter.rekenshi_counter_frames.restype=C.c_uint64
    counter.rekenshi_counter_delta.restype=C.c_float
    getter=getattr(ogre,'?getSingletonPtr@Root@Ogre@@SAPEAV12@XZ')
    getter.restype=C.c_void_p
    original=getter()
    host.dllStartPlugin()
    assert host.rekenshi_status()==1
    assert host.rekenshi_plugins_loaded()==2, (folder/'RE_Kenshi_Rust.log').read_text()
    assert host.rekenshi_plugins_failed()==0
    setter=host.rekenshi_plugin_setting_set
    setter.argtypes=[C.c_char_p,C.c_size_t,C.c_uint32,C.c_double]
    def setting(key,kind,value,expected=0):
        raw=key.encode(); assert setter(raw,len(raw),kind,value)==expected
    before=counter.rekenshi_counter_calls()
    for _ in range(5): assert getter()==original
    assert counter.rekenshi_counter_calls()==before+5
    before=counter.rekenshi_counter_frames()
    native.menu_frame()
    assert counter.rekenshi_counter_frames()==before+1
    assert abs(counter.rekenshi_counter_delta()-0.6)<1e-6
    setting('samples.kenshi2_style',1,1)
    setting('samples.kenshi2_style.ink',2,0.7)
    setting('callback_counter.enabled',1,0)
    before=counter.rekenshi_counter_calls(); getter()
    assert counter.rekenshi_counter_calls()==before
    setting('samples.kenshi2_style',1,0)
    setting('samples.kenshi2_style',1,1)
    settings=folder/'RE_Kenshi_Rust/settings'
    owned={p.name:p.read_bytes() for p in settings.glob('*.toml')}
    assert b'0.7' in owned['kenshi2-style.toml']
    assert b'false' in owned['callback-counter.toml']
    for key,kind,value in [('samples.kenshi2_style.ink',2,2),('samples.kenshi2_style',2,1),('callback_counter.enabled',1,2)]:
        setting(key,kind,value,1)
        assert {p.name:p.read_bytes() for p in settings.glob('*.toml')}==owned
    host.dllStopPlugin()
    before=counter.rekenshi_counter_calls()
    for _ in range(3):getter()
    native.menu_frame()
    assert counter.rekenshi_counter_calls()==before
    assert host.rekenshi_plugins_loaded()==0
    host.dllStartPlugin()
    assert host.rekenshi_plugins_loaded()==2
    getter(); assert counter.rekenshi_counter_calls()==before, 'disabled setting must survive restart'
    setting('callback_counter.enabled',1,1)
    getter(); assert counter.rekenshi_counter_calls()==before+1
    host.dllStopPlugin(); native.menu_shutdown()
    log=(folder/'RE_Kenshi_Rust.log').read_text()
    assert 'post-tonemap effect registered' in log
    print('PASS: folder discovery, two independent SDK DLLs, Ogre/UI callbacks, original forwarding, graphics updates/removal, isolated settings, validation and restart')

if __name__=='__main__':
    if sys.argv[1]=='--child': child(pathlib.Path(sys.argv[2]))
    elif sys.argv[1]=='--disabled-child': disabled_child(pathlib.Path(sys.argv[2]))
    else:
        game=pathlib.Path(sys.argv[1]); folder=ROOT/'target/sdk-tests'/uuid.uuid4().hex
        mods=folder/'RE_Kenshi_Rust/plugins';mods.mkdir(parents=True)
        for name in ['RE_Kenshi_Rust.dll','rekenshi_kenshi2_style.dll','rekenshi_callback_counter.dll']:
            shutil.copy2(ROOT/'target/release'/name,(folder if name=='RE_Kenshi_Rust.dll' else mods)/name)
        shutil.copy2(ROOT/'target/native-menu.dll',folder/'native-menu.dll')
        for name in ['MyGUIEngine_x64.dll','OgreMain_x64.dll','D3DCompiler_43.dll','RenderSystem_Direct3D11_x64.dll','Plugin_Terrain_x64.dll']:
            shutil.copy2(game/name,folder/name)
        # No manifest: drop-in folder discovery is the default.
        (mods.parent/'modpack.toml').write_text('schema_version=1\n[settings.values]\n')
        subprocess.run([sys.executable,__file__,'--child',str(folder)],cwd=folder,check=True)
        (mods.parent/'plugins.toml').write_text("schema_version=1\ndiscover=true\n[[plugins]]\nfile='REKENSHI_CALLBACK_COUNTER.DLL'\nenabled=false\n")
        subprocess.run([sys.executable,__file__,'--disabled-child',str(folder)],cwd=folder,check=True)
