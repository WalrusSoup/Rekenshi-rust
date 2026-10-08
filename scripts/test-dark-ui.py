"""Test whole-game UI colours/lifetimes with the installed native MyGUI DLL.

Uses real SimpleText/SubSkin objects but no graphics device. In-game visual QA
is still needed for the original atlas artwork; no screenshot is simulated.
"""
import ctypes as C
import pathlib
import shutil
import subprocess
import sys
import uuid
import xml.etree.ElementTree as E

ROOT = pathlib.Path(__file__).resolve().parent.parent

def child(folder,public=False):
    C.CDLL(str(folder / 'OgreMain_x64.dll'))
    C.CDLL(str(folder / 'D3DCompiler_43.dll'))
    native = C.CDLL(str(folder / 'native-menu.dll'))
    host = C.CDLL(str(folder / 'RE_Kenshi_Rust.dll'))
    resources = E.parse(ROOT / 'tests/fixtures/menu-resources.xml').getroot()
    resources.extend(E.parse(ROOT / 'tests/fixtures/dark-ui-resources.xml').getroot())
    native.menu_setup.argtypes = [C.c_char_p]
    native.theme_create.argtypes = [C.c_char_p, C.c_char_p]
    native.theme_create.restype = C.c_void_p
    native.theme_destroy.argtypes = [C.c_void_p]
    native.theme_set_colour.argtypes = native.theme_set_text.argtypes = [C.c_void_p] + [C.c_float] * 4
    native.theme_text_colour.argtypes = [C.c_void_p, C.POINTER(C.c_float)]
    native.theme_set_enabled.argtypes = [C.c_void_p, C.c_bool]
    native.theme_visible.argtypes = [C.c_void_p, C.c_bool]
    native.theme_draw_colour.restype = C.c_uint32
    native.theme_draw_colour_at.argtypes = [C.c_size_t]
    native.theme_draw_colour_at.restype = C.c_uint32
    setter = host.rekenshi_plugin_setting_set
    setter.argtypes = [C.c_char_p, C.c_size_t, C.c_uint32, C.c_double]
    def set_value(key, kind, value):
        key = key.encode()
        assert setter(key, len(key), kind, value) == 0, (key, (folder / 'RE_Kenshi_Rust.log').read_text())
        native.menu_frame()
    def colour(widget):
        result = (C.c_float * 4)()
        native.theme_text_colour(widget, result)
        return list(result)
    def close(actual, expected):
        assert all(abs(a-b) < 0.002 for a,b in zip(actual, expected)), (actual, expected)
    def tint():
        packed = native.theme_draw_colour()
        assert packed, 'native renderer must produce panel vertices'
        return [(packed >> shift) & 255 for shift in (16, 8, 0)]
    native.menu_setup(E.tostring(resources))
    host.dllStartPlugin()
    assert host.rekenshi_status() == 1
    assert 'dark UI: native skin creation' in (folder / 'RE_Kenshi_Rust.log').read_text()
    native.theme_render_setup()
    panel = native.theme_create(b'Kenshi_HealthPanelBottomSkin', b'ThemeHud')
    if public:
        assert host.rekenshi_plugins_loaded()==1,(folder/'RE_Kenshi_Rust.log').read_text()
        set_value('ui.dark_mode',1,0)
        native.theme_set_colour(panel,0.9,0.6,0.4,1.)
        assert abs(tint()[0]-63)<=1,tint()
        sample=C.CDLL(str(folder/'RE_Kenshi_Rust/plugins/rekenshi_source_transform.dll'))
        sample.rekenshi_widget_events.restype=C.c_uint64
        assert sample.rekenshi_widget_events()==0
        assert sample.rekenshi_plugin_dispatch(1)==0
        assert sample.rekenshi_widget_events()>=2
        native.theme_destroy(panel)
        assert sample.rekenshi_plugin_dispatch(1)==0
        assert sample.rekenshi_widget_events()>=3
        host.dllStopPlugin();native.menu_shutdown()
        print('PASS: independent owned theme color, deferred copied skin/initialization/destruction observations')
        return
    assert all(abs(a-b) <= 1 for a,b in zip(tint(), [135, 153, 171])), tint()
    # Default skin text, including state changes, flows through the final text path.
    initial = colour(panel)
    assert min(initial[:3]) > 0.8, initial
    native.theme_set_enabled(panel, False)
    assert 0.6 < min(colour(panel)[:3]) < 0.8, colour(panel)
    native.theme_set_enabled(panel, True)
    assert min(colour(panel)[:3]) > 0.8, colour(panel)
    native.theme_set_text(panel, 0.078, 0.031, 0.024, 0.4)
    assert min(colour(panel)[:3]) > 0.8 and abs(colour(panel)[3]-0.4) < 0.002, colour(panel)
    for _ in range(5):
        set_value('ui.dark_mode', 1, 0)
        assert tint() == [255, 255, 255], tint()
        close(colour(panel), [0.078, 0.031, 0.024, 0.4])
        set_value('ui.dark_mode', 1, 1)
        assert all(abs(a-b) <= 1 for a,b in zip(tint(), [135, 153, 171])), tint()
        assert min(colour(panel)[:3]) > 0.8
    native.theme_set_text(panel, 0.5, 0, 0, 0.6)
    close(colour(panel), [0.9, 0, 0, 0.6])
    set_value('ui.dark_mode', 1, 0)
    close(colour(panel), [0.5, 0, 0, 0.6])
    set_value('ui.dark_mode', 1, 1)
    native.theme_set_text(panel, 0.85, 0.4, 0.2, 0.8)
    close(colour(panel), [0.85, 0.4, 0.2, 0.8])
    # Native coloured status fills must keep their original vertex colours.
    native.theme_visible(panel, False)
    bar = native.theme_create(b'Kenshi_LifeBarSkin', b'ThemeLifeBar')
    native.theme_set_colour(bar, 0.2, 0.8, 0.1, 1)
    expected_bar = tint()
    assert all(abs(a-b) <= 1 for a,b in zip(expected_bar, [51, 204, 25])), expected_bar
    set_value('ui.panel_brightness', 2, 0.60)
    assert tint() == expected_bar
    set_value('ui.dark_mode', 1, 0)
    assert tint() == expected_bar
    set_value('ui.dark_mode', 1, 1)
    native.theme_destroy(bar)
    group = native.theme_create(b'Kenshi_GenericWindowSkin', b'ThemeGroup')
    child_colour = native.theme_draw_colour_at(6)
    assert child_colour, 'skin child must produce vertices'
    for enabled in [0, 1, 0, 1]:
        set_value('ui.dark_mode', 1, enabled)
        assert native.theme_draw_colour_at(6) == child_colour, 'live tint must preserve native skin-child colours'
    native.theme_destroy(group)
    set_value('ui.panel_brightness', 2, 0.16)
    tick = native.theme_create(b'Kenshi_TickButton1Skin', b'ThemeTick')
    assert all(abs(a-b) <= 1 for a,b in zip(tint(), [112,127,142])), tint()
    native.theme_destroy(tick)
    native.theme_visible(panel, True)
    # Destruction, skin teardown and address reuse cannot leave dangling tint entries.
    for i in range(120):
        widget = native.theme_create(b'Kenshi_HealthPanelBottomSkin', ('Temporary' + str(i)).encode())
        native.theme_set_colour(widget, 0.8, 0.6, 0.4, 0.7)
        native.theme_destroy(widget)
        if i % 10 == 0:
            set_value('ui.panel_brightness', 2, 0.16 if i % 20 == 0 else 0.60)
    native.theme_destroy(panel)
    set_value('ui.dark_mode', 1, 0)
    host.dllStopPlugin()
    native.menu_shutdown()
    print('PASS: native HUD output vertex tints, status fills/skin-child colours, default/manual/disabled text, alpha, reversible live toggle, brightness, destruction and address reuse')

if __name__ == '__main__':
    if sys.argv[1] == '--child':
        child(pathlib.Path(sys.argv[2]),len(sys.argv)>3 and sys.argv[3]=='public')
    else:
        folder = ROOT / 'target/dark-ui-tests' / uuid.uuid4().hex
        folder.mkdir(parents=True)
        game = pathlib.Path(sys.argv[1])
        for file in ['MyGUIEngine_x64.dll', 'OgreMain_x64.dll', 'D3DCompiler_43.dll']:
            shutil.copy2(game / file, folder / file)
        for source in [ROOT / 'target/release/RE_Kenshi_Rust.dll', ROOT / 'target/native-menu.dll']:
            shutil.copy2(source, folder / source.name)
        config = folder / 'RE_Kenshi_Rust'
        config.mkdir()
        shutil.copy2(ROOT / 'config/modpack.toml', config / 'modpack.toml')
        (config / 'plugins.toml').write_text('schema_version=1\ndiscover=false\nplugins=[]\n', encoding='utf-8')
        subprocess.run([sys.executable, __file__, '--child', str(folder)], cwd=folder, check=True)

        plugins=config/'plugins';plugins.mkdir(exist_ok=True)
        shutil.copy2(ROOT/'target/release/rekenshi_source_transform.dll',plugins)
        settings=config/'settings';settings.mkdir(exist_ok=True)
        (settings/'source-transform.toml').write_text('schema_version=1\n[settings.values]\n"source_transform.enabled"=true\n"runtime_policy.enabled"=true\n')
        (config/'plugins.toml').write_text('schema_version=1\ndiscover=true\n')
        subprocess.run([sys.executable,__file__,'--child',str(folder),'public'],cwd=folder,check=True)
