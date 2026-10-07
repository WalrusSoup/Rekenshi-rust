"""Exercise internal plugins and live settings through the single host DLL."""
import ctypes as C
import pathlib
import shutil
import subprocess
import sys
import struct
import uuid

ROOT = pathlib.Path(__file__).resolve().parent.parent
def child(folder, game):
    ogre = C.WinDLL(str(folder / 'OgreMain_x64.dll'))
    mygui = C.WinDLL(str(game / 'MyGUIEngine_x64.dll'))
    compiler = C.WinDLL(str(folder / 'D3DCompiler_43.dll'))
    host = C.CDLL(str(folder / 'RE_Kenshi_Rust.dll'))
    host.rekenshi_shader_cache_hits.restype = C.c_uint64
    host.rekenshi_heightmap_cache_reads.restype = C.c_uint64
    setter = host.rekenshi_plugin_setting_set
    setter.argtypes = [C.c_char_p, C.c_size_t, C.c_uint32, C.c_double]
    legacy_setter = host.rekenshi_pack_setting_set
    legacy_setter.argtypes = setter.argtypes
    def set_value(key, kind, value, expected=0):
        key = key.encode()
        assert setter(key, len(key), kind, value) == expected
    compile_shader = compiler.D3DCompile
    compile_shader.argtypes = [C.c_void_p, C.c_size_t, C.c_char_p, C.c_void_p, C.c_void_p,
                              C.c_char_p, C.c_char_p, C.c_uint32, C.c_uint32,
                              C.POINTER(C.c_void_p), C.POINTER(C.c_void_p)]
    source = C.create_string_buffer(b'float4 main():SV_Target{return float4(0.1234,0.678,0.97,1);}')
    def compile():
        code, errors = C.c_void_p(), C.c_void_p()
        result = compile_shader(source, len(source.value), b'pack-test', None, None,
                                b'main', b'ps_5_0', 0, 0, C.byref(code), C.byref(errors))
        assert result == 0 and code.value
        for blob in [code, errors]:
            if blob.value:
                table = C.c_void_p.from_address(blob.value).value
                release = C.c_void_p.from_address(table + 16).value
                C.CFUNCTYPE(C.c_uint32, C.c_void_p)(release)(blob)
    host.dllStartPlugin()
    assert host.rekenshi_status() == 1 and host.rekenshi_pack_layers() == 6
    assert host.rekenshi_internal_plugins() == host.rekenshi_pack_layers()
    assert host.rekenshi_plugins_loaded() == 2
    startup_log = (folder / 'RE_Kenshi_Rust.log').read_text()
    assert 'post-tonemap effect registered' not in startup_log, 'default pack must apply no visual pass'
    assert 'rain-refinement: native streak geometry' not in startup_log, 'rain sample must remain unselected'
    initial_pack = (folder / 'RE_Kenshi_Rust/modpack.toml').read_text()
    assert 'graphics.color_grade' not in initial_pack and 'weather.rain_refinement' not in initial_pack
    compile()
    hits = host.rekenshi_shader_cache_hits()
    compile()
    assert host.rekenshi_shader_cache_hits() == hits + 1
    terrain = C.CDLL(str(folder / 'Plugin_Terrain_x64.dll'))
    open_tiff = C.CFUNCTYPE(C.c_void_p, C.c_char_p, C.c_int)(terrain._handle + 0x20880)
    read_pixel = C.CFUNCTYPE(C.c_size_t, C.c_void_p, C.c_int, C.c_int, C.c_void_p)(terrain._handle + 0x21640)
    destroy = C.CFUNCTYPE(C.c_void_p, C.c_void_p, C.c_uint32)(terrain._handle + 0x143f0)
    reader = open_tiff(str(folder / 'data/newland/land/fullmap.tif').encode(), 1)
    assert reader
    try:
        before = host.rekenshi_heightmap_cache_reads()
        output = C.c_uint16()
        assert read_pixel(reader, 17, 23, C.byref(output)) == 1 and output.value == 23*64+17
        assert host.rekenshi_heightmap_cache_reads() == before
        set_value('performance.heightmap_cache', 1, 1)
        assert read_pixel(reader, 17, 23, C.byref(output)) == 1 and output.value == 23*64+17
        assert host.rekenshi_heightmap_cache_reads() == before + 1
        set_value('performance.heightmap_cache', 1, 0)
        assert read_pixel(reader, 17, 23, C.byref(output)) == 1 and output.value == 23*64+17
        assert host.rekenshi_heightmap_cache_reads() == before + 1
    finally:
        destroy(reader, 1)
    other = open_tiff(str(folder / 'data/newland/land/other.tif').encode(), 1)
    assert other
    try:
        set_value('performance.heightmap_cache', 1, 1)
        before = host.rekenshi_heightmap_cache_reads()
        output = C.c_uint16()
        assert read_pixel(other,17,23,C.byref(output)) == 1 and output.value == 54321
        assert host.rekenshi_heightmap_cache_reads() == before, 'foreign terrain map must bypass'
        set_value('performance.heightmap_cache', 1, 0)
    finally:
        destroy(other, 1)
    set_value('performance.shader_cache', 1, 0)
    hits = host.rekenshi_shader_cache_hits()
    compile()
    compile()
    assert host.rekenshi_shader_cache_hits() == hits
    set_value('performance.shader_cache', 1, 1)
    compile()
    assert host.rekenshi_shader_cache_hits() == hits + 1
    # Retired effects are absent; the disabled sample can be enabled live.
    set_value('graphics.color_grade', 1, 1, 1)
    set_value('weather.rain_refinement', 1, 1, 1)
    set_value('samples.kenshi2_style', 1, 1)
    assert legacy_setter(b'samples.kenshi2_style', len(b'samples.kenshi2_style'), 1, 0) == 0
    set_value('samples.kenshi2_style.contrast', 2, 1.2)
    set_value('samples.kenshi2_style', 1, 1)
    set_value('samples.kenshi2_style.saturation', 2, 1.3)
    config = folder / 'RE_Kenshi_Rust/modpack.toml'
    saved = config.read_bytes()
    for key, kind, value in [('samples.kenshi2_style.contrast', 2, -1), ('samples.kenshi2_style.contrast', 1, 1),
                             ('samples.kenshi2_style.saturation', 2, float('nan')), ('unknown', 1, 1),
                             ('performance.shader_cache', 1, 2)]:
        set_value(key, kind, value, 1)
        assert config.read_bytes() == saved
    for key in ['performance.collider_cache', 'performance.dds_mip_skip']:
        set_value(key, 1, 1, 1)
        assert config.read_bytes() == saved
    set_value('performance.compressed_textures', 1, 1)
    set_value('performance.compressed_textures', 1, 0)
    saved = config.read_bytes()
    sample_config = folder / 'RE_Kenshi_Rust/settings/kenshi2-style.toml'
    sample_saved = sample_config.read_bytes()
    assert b'\"samples.kenshi2_style\" = true' in sample_saved
    host.dllStopPlugin()
    assert host.rekenshi_pack_layers() == 0
    assert host.rekenshi_internal_plugins() == 0
    host.dllStartPlugin()
    assert host.rekenshi_pack_layers() == 6 and config.read_bytes() == saved and sample_config.read_bytes() == sample_saved
    host.dllStopPlugin()
    print('PASS: single-DLL internal plugins, live shader/heightmap/sample style controls, availability, persistence, validation, compatibility aliases, stop/restart')

if __name__ == '__main__':
    if sys.argv[1] == '--child':
        child(pathlib.Path(sys.argv[2]), pathlib.Path(sys.argv[3]))
    else:
        game = pathlib.Path(sys.argv[1])
        folder = ROOT / 'target/modpack-tests' / uuid.uuid4().hex
        mods = folder / 'RE_Kenshi_Rust/plugins'
        mods.mkdir(parents=True)
        for name in ['RE_Kenshi_Rust.dll', 'pack-config.exe']:
            shutil.copy2(ROOT / 'target/release' / name, folder / name)
        for name in ['rekenshi_diagnostics.dll','rekenshi_kenshi2_style.dll']:
            shutil.copy2(ROOT / 'target/release' / name, mods / name)
        for name in ['OgreMain_x64.dll', 'D3DCompiler_43.dll', 'RenderSystem_Direct3D11_x64.dll', 'Plugin_ParticleUniverse_x64.dll']:
            shutil.copy2(game / name, folder / name)
        shutil.copy2(game / 'Plugin_Terrain_x64.dll', folder / 'Plugin_Terrain_x64.dll')
        heightmap = folder / 'data/newland/land/fullmap.tif'
        heightmap.parent.mkdir(parents=True)
        fields = [(256,3,64),(257,3,64),(258,3,16),(259,3,1),(273,4,110),
                  (277,3,1),(278,4,64),(279,4,64*64*2)]
        header = b'II\x2a\x00' + struct.pack('<IH',8,len(fields))
        header += b''.join(struct.pack('<HHII',tag,kind,1,value) for tag,kind,value in fields) + b'\0'*4
        heightmap.write_bytes(header + struct.pack('<4096H',*range(4096)))
        heightmap.with_name('other.tif').write_bytes(header + struct.pack('<4096H',*[54321]*4096))
        config = folder / 'RE_Kenshi_Rust/plugins.toml'
        config.write_text('schema_version=1\n' + ''.join(f'[[plugins]]\nfile="{name}"\nenabled=true\n' for name in
            ['rekenshi_diagnostics.dll', 'rekenshi_performance.dll', 'rekenshi_graphics.dll', 'rekenshi_color_grade.dll']))
        subprocess.run([str(folder / 'pack-config.exe'), str(mods.parent)], check=True)
        assert 'rekenshi_performance.dll' not in config.read_text()
        assert (mods.parent / 'plugins.toml.before-modpack').exists()
        # A manual DLL upgrade may bypass the installer. Boot must also migrate
        # retired visual keys before validating the remaining plugin settings.
        pack = mods.parent / 'modpack.toml'
        pack.write_text(pack.read_text() + '\n"graphics.color_grade"=true\n"weather.rain_refinement"=true\n')
        subprocess.run([sys.executable, __file__, '--child', str(folder), str(game)], check=True)
        backup = (mods.parent / 'modpack.toml.before-dll-plugins').read_text()
        assert 'graphics.color_grade' in backup and 'weather.rain_refinement' in backup
