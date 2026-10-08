"""Real compiler interception across host/provider/consumer Rust DLL boundaries."""
import ctypes as C
import pathlib
import shutil
import subprocess
import sys
import uuid

ROOT = pathlib.Path(__file__).resolve().parent.parent

def child(folder, game):
    C.WinDLL(str(game / 'MyGUIEngine_x64.dll'))
    C.WinDLL(str(folder / 'OgreMain_x64.dll'))
    compiler = C.WinDLL(str(folder / 'D3DCompiler_43.dll'))
    host = C.CDLL(str(folder / 'RE_Kenshi_Rust.dll'))
    sample = C.CDLL(str(folder / 'RE_Kenshi_Rust/plugins/rekenshi_source_transform.dll'))
    sample.rekenshi_source_transform_calls.restype = C.c_uint64
    sample.rekenshi_source_transform_late.restype = C.c_uint64
    sample.rekenshi_source_transform_ready.restype = C.c_uint64
    compile_shader = compiler.D3DCompile
    compile_shader.argtypes = [C.c_void_p, C.c_size_t, C.c_char_p, C.c_void_p, C.c_void_p,
                              C.c_char_p, C.c_char_p, C.c_uint32, C.c_uint32,
                              C.POINTER(C.c_void_p), C.POINTER(C.c_void_p)]
    def compile(source):
        source_buffer = C.create_string_buffer(source)
        code, errors = C.c_void_p(), C.c_void_p()
        result = compile_shader(source_buffer, len(source), b'platform-test', None, None,
                                b'main', b'ps_5_0', 0, 0, C.byref(code), C.byref(errors))
        try:
            assert result == 0 and code.value, result
            table = C.c_void_p.from_address(code.value).value
            get_data = C.CFUNCTYPE(C.c_void_p, C.c_void_p)(C.c_void_p.from_address(table + 24).value)
            get_size = C.CFUNCTYPE(C.c_size_t, C.c_void_p)(C.c_void_p.from_address(table + 32).value)
            return C.string_at(get_data(code), get_size(code))
        finally:
            for blob in (code, errors):
                if blob.value:
                    table = C.c_void_p.from_address(blob.value).value
                    C.CFUNCTYPE(C.c_uint32, C.c_void_p)(C.c_void_p.from_address(table + 16).value)(blob)
    source = b'// example-source-transform\nfloat4 main():SV_Target{return float4(1,0,0,1);}'
    reference = source.replace(b'float4(1,0,0,1)', b'float4(0,1,0,1)')
    rejected = source.replace(b'// example-source-transform', b'// reject-source-transform')
    original, expected, original_rejected = compile(source), compile(reference), compile(rejected)
    assert original != expected
    host.dllStartPlugin()
    assert host.rekenshi_plugins_loaded() == 2, (folder / 'RE_Kenshi_Rust.log').read_text()
    assert sample.rekenshi_source_transform_late() == 0
    assert sample.rekenshi_source_transform_ready() == 0
    # Explicitly emulate the serial delivery point: this headless process has
    # no supported PlayerInterface hook and this observer touches copied data only.
    assert sample.rekenshi_plugin_dispatch(1) == 0
    assert sample.rekenshi_source_transform_ready() == 1
    assert sample.rekenshi_plugin_dispatch(1) == 0
    assert sample.rekenshi_source_transform_ready() == 1
    assert compile(source) == expected
    before = sample.rekenshi_source_transform_calls()
    # The transform still runs before the cache lookup; bytecode reflects its output.
    assert compile(source) == expected
    assert sample.rekenshi_source_transform_calls() == before + 1
    assert compile(rejected) == original_rejected
    setter = host.rekenshi_plugin_setting_set
    setter.argtypes = [C.c_char_p, C.c_size_t, C.c_uint32, C.c_double]
    key = b'source_transform.enabled'
    assert setter(key, len(key), 1, 0) == 0
    assert compile(source) == original
    assert setter(key, len(key), 1, 1) == 0
    assert sample.rekenshi_source_transform_late() == 1
    assert compile(source) == expected
    host.dllStopPlugin()
    before = sample.rekenshi_source_transform_calls()
    assert compile(source) == original
    assert sample.rekenshi_source_transform_calls() == before
    # Retained trampolines and broker handles must support a new provider generation.
    host.dllStartPlugin()
    assert host.rekenshi_plugins_loaded() == 2
    assert sample.rekenshi_plugin_dispatch(1) == 0
    assert sample.rekenshi_source_transform_ready() == 2
    assert compile(source) == expected
    host.dllStopPlugin()
    print('PASS: independent compiler-provider and source-policy DLLs; transformed cache identity, original fallback, cleanup and recreation')

if __name__ == '__main__':
    if sys.argv[1] == '--child':
        child(pathlib.Path(sys.argv[2]), pathlib.Path(sys.argv[3]))
    else:
        game = pathlib.Path(sys.argv[1])
        folder = ROOT / 'target/platform-tests' / uuid.uuid4().hex
        plugins = folder / 'RE_Kenshi_Rust/plugins'
        plugins.mkdir(parents=True)
        shutil.copy2(ROOT / 'target/release/RE_Kenshi_Rust.dll', folder)
        for name in ('OgreMain_x64.dll', 'D3DCompiler_43.dll', 'RenderSystem_Direct3D11_x64.dll'):
            shutil.copy2(game / name, folder)
        for name in ('rekenshi_shader_cache.dll', 'rekenshi_source_transform.dll'):
            shutil.copy2(ROOT / 'target/release' / name, plugins)
        (plugins.parent / 'plugins.toml').write_text('schema_version=1\n' + ''.join(
            f'[[plugins]]\nfile="{name}"\nenabled=true\n' for name in
            ('rekenshi_shader_cache.dll', 'rekenshi_source_transform.dll')))
        subprocess.run([sys.executable, __file__, '--child', str(folder), str(game)], check=True)
