"""Native Options close/reopen/rebuild regression. Stages only under target/."""
import ctypes as C
import pathlib
import shutil
import subprocess
import sys
import uuid

ROOT = pathlib.Path(__file__).resolve().parent.parent

def resources():
    return (ROOT / 'tests/fixtures/menu-resources.xml').read_bytes()

def child(folder):
    ogre = C.CDLL(str(folder / 'OgreMain_x64.dll'))
    compiler = C.CDLL(str(folder / 'D3DCompiler_43.dll'))
    native = C.CDLL(str(folder / 'native-menu.dll'))
    host = C.CDLL(str(folder / 'RE_Kenshi_Rust.dll'))
    native.menu_setup.argtypes = [C.c_char_p]
    native.menu_hide.argtypes = [C.c_bool]
    native.menu_data.argtypes = [C.c_size_t]
    native.menu_data.restype = C.c_size_t
    native.menu_close_compatible.restype = C.c_bool
    native.menu_select.argtypes = [C.c_size_t]
    native.menu_resize.argtypes = [C.c_int, C.c_int]
    native.menu_find.argtypes = [C.c_char_p]
    for name in ['menu_tab', 'menu_find', 'menu_parent']:
        getattr(native, name).restype = C.c_void_p
    for name in ['menu_parent', 'menu_visible', 'menu_click', 'menu_checked', 'menu_width', 'menu_height', 'menu_top', 'menu_needs_mouse', 'menu_inherits_pick']:
        getattr(native, name).argtypes = [C.c_void_p]
    native.menu_visible.restype = native.menu_checked.restype = C.c_bool
    native.menu_needs_mouse.restype = native.menu_inherits_pick.restype = C.c_bool
    native.menu_count.restype = C.c_size_t
    native.menu_setup(resources())
    host.dllStartPlugin()
    assert host.rekenshi_status() == 1
    native.menu_frame()
    tab = native.menu_tab()
    assert tab, 'REKENSHI-RUST native Options tab is missing'
    assert native.menu_count() == 3, 'add exactly one tab'
    assert native.menu_close_compatible(), 'Kenshi close requires typed DatapanelGUI* data on every tab'
    assert native.menu_data(0) == 0x12340, 'vanilla panel ownership must remain intact'
    assert native.menu_data(2) == 0, 'custom tab must have a typed null panel, not own a vanilla panel'
    panel = native.menu_find(b'RKPack.panel')
    assert panel and native.menu_parent(panel) == tab, 'settings must belong to the native tab'
    assert not native.menu_find(b'RKPack.open') and not native.menu_find(b'RKPack.close')
    native.menu_select(2)
    native.menu_frame()
    assert native.menu_visible(panel)
    for _ in range(3):
        native.menu_hide(True); native.menu_frame()
        assert not native.menu_visible(panel), 'closing Options hides its settings'
        native.menu_hide(False); native.menu_frame()
        assert native.menu_visible(panel), 'reopening Options restores its settings'
        assert native.menu_count() == 3
    for _ in range(3):
        native.menu_close(); native.menu_frame()
        assert not native.menu_find(b'RKPack.panel'), 'closing destroys tabs and their settings'
        native.menu_reopen(); native.menu_frame()
        assert native.menu_close_compatible() and native.menu_count() == 3
        tab = native.menu_tab()
        panel = native.menu_find(b'RKPack.panel')
        assert panel and native.menu_parent(panel) == tab
        native.menu_select(2); native.menu_frame()
        assert native.menu_visible(panel), 'recreated tab works after native close/reopen'
    native.menu_select(0); native.menu_frame()
    assert not native.menu_visible(panel), 'switching tabs hides settings'
    native.menu_select(2); native.menu_frame()
    assert native.menu_visible(panel), 'switching back restores settings'
    first = native.menu_find(b'RKPack.toggle.0')
    assert native.menu_visible(first)
    mark = native.menu_find(b'RKPack.check.0')
    assert mark and native.menu_parent(mark) == first, 'checkbox owns its decorative indicator'
    assert native.menu_width(first) == native.menu_height(first) == 22, 'compact square checkbox'
    assert native.menu_needs_mouse(first), 'checkbox receives native input'
    for name in [b'RKPack.panel', b'RKPack.row.0', b'RKPack.label.0', b'RKPack.description.0', b'RKPack.inset.0', b'RKPack.check.0', b'RKPack.next.caption']:
        decoration = native.menu_find(name)
        assert decoration and not native.menu_needs_mouse(decoration), 'decoration must not intercept clicks: ' + str(name)
    assert native.menu_inherits_pick(panel), 'panel must permit picking its interactive descendants'
    native.menu_click(native.menu_find(b'RKPack.next')); native.menu_frame()
    assert not native.menu_visible(first), 'pagination hides the previous settings'
    native.menu_click(native.menu_find(b'RKPack.previous')); native.menu_frame()
    assert native.menu_visible(first), 'pagination restores the previous settings'
    checkbox = native.menu_find(b'RKPack.toggle.0')
    before = native.menu_checked(checkbox)
    assert native.menu_visible(mark) == before, 'indicator reflects the setting'
    native.menu_click(checkbox); native.menu_frame()
    assert native.menu_checked(checkbox) != before, 'native checkbox click applies setting'
    assert native.menu_visible(mark) != before, 'indicator changes with the native checkbox'
    native.menu_click(checkbox); native.menu_frame()
    assert native.menu_checked(checkbox) == before
    native.menu_resize(600, 400); native.menu_frame()
    assert native.menu_width(panel) <= 600, 'settings fit their resized native tab'
    footer = native.menu_find(b'RKPack.message')
    for i in range(30):
        card = native.menu_find(('RKPack.row.' + str(i)).encode())
        if card and native.menu_visible(card):
            assert native.menu_top(card) + native.menu_height(card) <= native.menu_top(footer), 'rows must fit above the footer'
    native.menu_resize(300, 220); native.menu_frame()
    assert not native.menu_visible(panel), 'too-small tabs must not leave overflowing controls'
    native.menu_resize(600, 400); native.menu_frame()
    assert native.menu_visible(panel), 'restoring space restores controls'
    native.menu_rebuild(); native.menu_frame()
    tab = native.menu_tab()
    assert tab and native.menu_count() == 3, 'rebuilt Options gets a fresh tab'
    panel = native.menu_find(b'RKPack.panel')
    assert panel and native.menu_parent(panel) == tab
    native.menu_select(2); native.menu_frame()
    assert native.menu_visible(panel)
    host.dllStopPlugin()
    native.menu_shutdown()
    print('PASS: native Options tab, typed close data, hide/reopen, close/recreate, tab switching, checkbox events, pagination, resize and GUI rebuild')

if __name__ == '__main__':
    if sys.argv[1] == '--child':
        child(pathlib.Path(sys.argv[2]))
    else:
        folder = ROOT / 'target/menu-tests' / uuid.uuid4().hex
        folder.mkdir(parents=True)
        game = pathlib.Path(sys.argv[1])
        for file in ['MyGUIEngine_x64.dll', 'OgreMain_x64.dll', 'D3DCompiler_43.dll']:
            shutil.copy2(game / file, folder / file)
        config = folder / 'RE_Kenshi_Rust'
        config.mkdir()
        shutil.copy2(ROOT / 'config/modpack.toml', config / 'modpack.toml')
        host = pathlib.Path(sys.argv[2]) if len(sys.argv) > 2 else ROOT / 'target/release/RE_Kenshi_Rust.dll'
        for source in [host, ROOT / 'target/native-menu.dll']:
            shutil.copy2(source, folder / source.name)
        (config / 'plugins').mkdir(exist_ok=True)
        shutil.copy2(ROOT / 'target/release/rekenshi_kenshi2_style.dll', config / 'plugins/rekenshi_kenshi2_style.dll')
        (config / 'plugins.toml').write_text('schema_version=1\ndiscover=true\nplugins=[]\n')
        subprocess.run([sys.executable, __file__, '--child', str(folder)], cwd=folder, check=True)
