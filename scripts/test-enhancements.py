"""Real native DLL/compiler integration tests; stages only under target/."""
import ctypes as C
import pathlib
import shutil
import subprocess
import sys
import uuid

ROOT = pathlib.Path(__file__).resolve().parent.parent
CALL = C.CFUNCTYPE
LOG = CALL(None, C.c_uint64, C.c_void_p, C.c_size_t)
RESOLVE = CALL(C.c_void_p, C.c_char_p, C.c_char_p)
CLAIM = CALL(C.c_int32, C.c_uint64, C.c_void_p)
COUNT = CALL(C.c_uint64)
START = CALL(C.c_int32, C.c_void_p)
STOP = CALL(C.c_int32)

class Host(C.Structure):
    _fields_ = [('size', C.c_uint32), ('abi', C.c_uint32), ('token', C.c_uint64),
                ('log', LOG), ('resolve', RESOLVE), ('claim', CLAIM),
                ('release', CLAIM), ('count', COUNT)]

class Plugin(C.Structure):
    _fields_ = [('size', C.c_uint32), ('abi', C.c_uint32), ('id', C.c_char * 64),
                ('name', C.c_char * 96), ('version', C.c_char * 32),
                ('start', START), ('stop', STOP)]

class Macro(C.Structure):
    _fields_ = [('name', C.c_char_p), ('value', C.c_char_p)]

class HookInfo(C.Structure):
    _fields_ = [('size', C.c_uint32), ('id', C.c_uint32), ('address', C.c_void_p),
                ('owner', C.c_uint64), ('status', C.c_uint32), ('reserved', C.c_uint32)]

HOOK_RESOLVE = CALL(C.c_int32, C.c_uint32, C.POINTER(HookInfo), C.c_size_t)
HOOK_PUBLISH = CALL(C.c_int32, C.c_uint64, C.c_uint32, C.c_void_p)

class Hooks(C.Structure):
    _fields_ = [('size', C.c_uint32), ('abi', C.c_uint32),
                ('resolve', HOOK_RESOLVE), ('publish', HOOK_PUBLISH)]

GET_HOOKS = CALL(C.c_int32, C.c_uint32, C.POINTER(Hooks), C.c_size_t)

def child(folder, warm):
    modules = {}
    claims = {}
    logs = []
    compiler = C.WinDLL(str(folder / 'D3DCompiler_43.dll'))
    targets = {3: C.cast(compiler.D3DCompile, C.c_void_p).value,
               4: C.cast(compiler.D3DCreateBlob, C.c_void_p).value}
    @HOOK_RESOLVE
    def hook_resolve(id, output, size):
        if size != C.sizeof(HookInfo) or id not in range(1, 6):
            return 2
        address = targets.get(id)
        if not address:
            return 4
        owner = claims.get(address)
        output[0] = HookInfo(size, id, address, owner or 0, 2 if owner is not None else 1, 0)
        return 0
    @HOOK_PUBLISH
    def hook_publish(token, id, address):
        if id != 5:
            return 2
        if claims.get(address) != token:
            return 1
        if id in targets and targets[id] != address:
            return 3
        targets[id] = address
        return 0
    @GET_HOOKS
    def get_hooks(version, output, size):
        if version != 1 or size != C.sizeof(Hooks):
            return 2
        output[0] = Hooks(size, 1, hook_resolve, hook_publish)
        return 0
    @LOG
    def log(token, ptr, length):
        message = C.string_at(ptr, length).decode('utf8', errors='replace')
        logs.append(message)
        print(f'[{token}] {message}', flush=True)
    @RESOLVE
    def resolve(module, name):
        if module.lower() == b're_kenshi_rust.dll' and name == b'rekenshi_hooks_api':
            return C.cast(get_hooks, C.c_void_p).value
        dll = modules.get(module.decode().lower())
        return C.cast(getattr(dll, name.decode()), C.c_void_p).value if dll else None
    @CLAIM
    def claim(token, ptr):
        if ptr in claims:
            return 3
        claims[ptr] = token
        return 0
    @CLAIM
    def release(token, ptr):
        if claims.get(ptr) != token:
            return 1
        del claims[ptr]
        return 0
    @COUNT
    def count():
        return 0
    sessions = []
    def start(file, token):
        dll = C.WinDLL(str(folder / 'RE_Kenshi_Rust' / 'plugins' / file))
        modules[file] = dll
        dll.rekenshi_plugin_entry.argtypes = [C.c_uint32, C.POINTER(Plugin), C.c_size_t]
        dll.rekenshi_plugin_entry.restype = C.c_int32
        plugin = Plugin()
        assert dll.rekenshi_plugin_entry(1, C.byref(plugin), C.sizeof(plugin)) == 0
        host = Host(C.sizeof(Host), 1, token, log, resolve, claim, release, count)
        assert plugin.start(C.byref(host)) == 0, logs
        sessions.append((plugin, host, dll))
        return dll
    perf = start('rekenshi_shader_cache.dll', 1)
    perf.rekenshi_shader_cache_hits.restype = C.c_uint64
    compiler.D3DCompile.argtypes = [C.c_void_p, C.c_size_t, C.c_char_p, C.POINTER(Macro), C.c_void_p, C.c_char_p, C.c_char_p, C.c_uint32, C.c_uint32, C.POINTER(C.c_void_p), C.POINTER(C.c_void_p)]
    compiler.D3DCompile.restype = C.c_int32
    def blob(ptr):
        if not ptr.value:
            return None
        vt = C.cast(ptr, C.POINTER(C.POINTER(C.c_void_p))).contents
        get_data = C.WINFUNCTYPE(C.c_void_p, C.c_void_p)(vt[3])
        get_size = C.WINFUNCTYPE(C.c_size_t, C.c_void_p)(vt[4])
        release_blob = C.WINFUNCTYPE(C.c_uint32, C.c_void_p)(vt[2])
        result = C.string_at(get_data(ptr), get_size(ptr))
        release_blob(ptr)
        return result
    def compile(source, defines=None, flags=0, name=b'smoke.hlsl', include=None):
        output, errors = C.c_void_p(), C.c_void_p()
        result = compiler.D3DCompile(source, len(source), name, defines, include, b'main', b'ps_5_0', flags, 0, C.byref(output), C.byref(errors))
        return result, blob(output), blob(errors)
    source = b'float4 main():SV_Target { return float4(0.2,0.4,0.6,1); }'
    h0 = perf.rekenshi_shader_cache_hits()
    first = compile(source)
    h1 = perf.rekenshi_shader_cache_hits()
    second = compile(source)
    assert first == second and first[0] >= 0 and first[1]
    assert perf.rekenshi_shader_cache_hits() == h1 + 1
    assert (h1 > h0) == warm, 'cache must survive a fresh process'
    before = perf.rekenshi_shader_cache_hits()
    variants = [compile(source, flags=1 << 11), compile(source, name=b'other.hlsl')]
    if not warm:
        assert perf.rekenshi_shader_cache_hits() == before, 'flags/source name must invalidate'
    macros = (Macro * 2)(Macro(b'VALUE', b'0.2'), Macro(None, None))
    macro_source = b'float4 main():SV_Target { return VALUE; }'
    a = compile(macro_source, defines=macros)
    macros[0].value = b'0.8'
    b = compile(macro_source, defines=macros)
    assert a[1] != b[1], 'macro values must invalidate'
    warning_source = b'float4 main():SV_Target { float x=float2(1,2); return x; }'
    warnings = compile(warning_source)
    assert compile(warning_source) == warnings, 'warning blob must be preserved'
    invalid = compile(b'not valid HLSL')
    assert invalid[0] < 0 and invalid[2]
    # Unknown native include handlers are passed through, never substituted.
    OPEN = C.WINFUNCTYPE(C.c_int32, C.c_void_p, C.c_uint32, C.c_char_p, C.c_void_p, C.POINTER(C.c_void_p), C.POINTER(C.c_uint32))
    CLOSE = C.WINFUNCTYPE(C.c_int32, C.c_void_p, C.c_void_p)
    buffers, opened = {}, [0]
    @OPEN
    def inc_open(this, kind, name, parent, output, length):
        content = b'#define INCLUDED 0.25' if opened[0] == 0 else b'#define INCLUDED 0.75'
        storage = C.create_string_buffer(content)
        address = C.addressof(storage)
        buffers[address] = storage
        output[0] = address
        length[0] = len(content)
        opened[0] += 1
        return 0
    @CLOSE
    def inc_close(this, pointer):
        del buffers[pointer]
        return 0
    class IncludeVtable(C.Structure):
        _fields_ = [('open', OPEN), ('close', CLOSE)]
    table = IncludeVtable(inc_open, inc_close)
    handler = C.pointer(C.pointer(table))
    before = perf.rekenshi_shader_cache_hits()
    inc_source = b'#include "custom.h"\nfloat4 main():SV_Target { return INCLUDED; }'
    ia, ib = compile(inc_source, include=handler), compile(inc_source, include=handler)
    assert ia[0] >= 0 and ib[0] >= 0 and ia[1] != ib[1]
    assert not buffers and opened[0] == 2 and perf.rekenshi_shader_cache_hits() == before
    assert compile(source, include=1)[0] >= 0, 'standard-include sentinel must never be dereferenced'
    # A corrupted cache record must invoke the compiler and repair the entry.
    shader_files = list((folder / 'RE_Kenshi_Rust' / 'cache' / 'shaders').glob('*.shader'))
    if not warm:
        for file in shader_files:
            file.write_bytes(b'corrupted record')
        before = perf.rekenshi_shader_cache_hits()
        assert compile(source) == first and perf.rekenshi_shader_cache_hits() == before
        assert compile(source) == first and perf.rekenshi_shader_cache_hits() == before + 1
    graphics = start('rekenshi_graphics.dll', 2)
    color = start('rekenshi_color_grade.dll', 3)
    for plugin, host, dll in reversed(sessions):
        assert plugin.stop() == 0
    # Native plugin callbacks must support stop/restart with retained trampolines.
    claims.clear()
    for plugin, host, dll in sessions:
        assert plugin.start(C.byref(host)) == 0
    for plugin, host, dll in reversed(sessions):
        assert plugin.stop() == 0
    print('PASS native cache identity, hit/miss, macros, warnings, corruption, include passthrough, graphics registration, lifecycle', flush=True)

if __name__ == '__main__':
    if sys.argv[1] == '--child':
        child(pathlib.Path(sys.argv[2]), sys.argv[3] == 'warm')
    else:
        game = pathlib.Path(sys.argv[1])
        stage = ROOT / 'target' / 'enhancement-tests' / uuid.uuid4().hex
        mods = stage / 'RE_Kenshi_Rust' / 'plugins'
        mods.mkdir(parents=True)
        for file in ['rekenshi_shader_cache.dll', 'rekenshi_graphics.dll', 'rekenshi_color_grade.dll']:
            shutil.copy2(ROOT / 'target' / 'release' / file, mods / file)
        shutil.copy2(ROOT / 'target' / 'release' / 'rekenshi_diagnostics.dll', mods / 'rekenshi_diagnostics.dll')
        shutil.copy2(ROOT / 'target' / 'release' / 'RE_Kenshi_Rust.dll', stage / 'RE_Kenshi_Rust.dll')
        (stage / 'RE_Kenshi_Rust' / 'plugins.toml').write_text('schema_version=1\n' + ''.join(
            f'[[plugins]]\nfile="{name}"\nenabled=true\n' for name in
            ['rekenshi_diagnostics.dll', 'rekenshi_shader_cache.dll', 'rekenshi_graphics.dll', 'rekenshi_color_grade.dll']))
        for file in ['D3DCompiler_43.dll', 'RenderSystem_Direct3D11_x64.dll']:
            shutil.copy2(game / file, stage / file)
        subprocess.run([str(ROOT / 'target' / 'release' / 'plugin-smoke.exe'),
                        str(stage / 'RE_Kenshi_Rust.dll'), str(game), '4', '0'], check=True)
        for phase in ['cold', 'warm']:
            subprocess.run([sys.executable, __file__, '--child', str(stage), phase], check=True)
        print(f'PASS independent-process persistence; staged files retained in {stage}')
