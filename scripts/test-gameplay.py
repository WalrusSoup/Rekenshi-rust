"""Exercise the real host and independent gameplay SDK DLL outside a game world.

Checks ABI negotiation, fingerprint rejection, inspector output and restart.
Does not claim that native gameplay calls were exercised. Uses a private fixture
directory and never modifies the supplied Kenshi installation.
"""
import ctypes as C
import json
import pathlib
import shutil
import subprocess
import sys
import time
import uuid

ROOT = pathlib.Path(__file__).resolve().parent.parent


class Gameplay(C.Structure):
    _fields_ = [("struct_size", C.c_uint32), ("version", C.c_uint32)] + [
        (name, C.c_void_p) for name in ("status", "snapshot", "enqueue", "take_result")
    ]


def wait_document(path, after=0):
    deadline = time.monotonic() + 10
    while time.monotonic() < deadline:
        try:
            if path.stat().st_mtime_ns > after:
                data = json.loads(path.read_text(encoding="utf-8"))
                if data["status"] == "unsupported":
                    return data
        except (OSError, ValueError):
            pass
        time.sleep(0.05)
    raise AssertionError(f"No unsupported-build inspector document: {path}")


def child(folder):
    ogre = C.CDLL(str(folder / "OgreMain_x64.dll"))
    host = C.CDLL(str(folder / "RE_Kenshi_Rust.dll"))
    getter = getattr(ogre, "?getSingletonPtr@Root@Ogre@@SAPEAV12@XZ")
    getter.restype = C.c_void_p
    original = getter()
    get = host.rekenshi_gameplay_api
    get.argtypes = [C.c_uint32, C.POINTER(Gameplay), C.c_size_t]
    api = Gameplay()
    assert get(999, C.byref(api), C.sizeof(api)) == 2
    assert get(1, C.byref(api), C.sizeof(api) - 1) == 2
    assert get(1, None, C.sizeof(api)) == 1
    assert get(1, C.byref(api), C.sizeof(api)) == 0
    assert api.struct_size == C.sizeof(api) and api.version == 1
    status = C.CFUNCTYPE(C.c_int, C.c_uint64, C.c_void_p)(api.status)
    assert status(0, None) == 1
    host.dllStartPlugin()
    assert host.rekenshi_status() == 1
    assert host.rekenshi_plugins_loaded() == 1
    assert host.rekenshi_plugins_failed() == 0
    assert getter() == original
    path = folder / "RE_Kenshi_Rust/squad-inspector.json"
    data = wait_document(path)
    assert data["characters"] == []
    assert data["capabilities"] == 0
    assert data["message"]
    host.dllStopPlugin()
    assert host.rekenshi_plugins_loaded() == 0
    stopped = path.stat().st_mtime_ns
    host.dllStartPlugin()
    assert host.rekenshi_plugins_loaded() == 1
    assert wait_document(path, stopped)["characters"] == []
    host.dllStopPlugin()
    assert getter() == original
    print("PASS: gameplay ABI negotiation, unsupported executable rejected, independent SDK inspector, stop/restart")


if __name__ == "__main__":
    if sys.argv[1] == "--child":
        child(pathlib.Path(sys.argv[2]))
    else:
        game = pathlib.Path(sys.argv[1])
        folder = ROOT / "target/gameplay-tests" / uuid.uuid4().hex
        mods = folder / "RE_Kenshi_Rust/plugins"
        mods.mkdir(parents=True)
        shutil.copy2(ROOT / "target/release/RE_Kenshi_Rust.dll", folder)
        shutil.copy2(ROOT / "target/release/rekenshi_squad_inspector.dll", mods)
        for name in ("OgreMain_x64.dll", "MyGUIEngine_x64.dll"):
            shutil.copy2(game / name, folder)
        subprocess.run([sys.executable, __file__, "--child", str(folder)], cwd=folder,
                       check=True, timeout=45)
