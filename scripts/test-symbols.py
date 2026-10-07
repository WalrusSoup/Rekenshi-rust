"""Scanner regression tests using a small synthetic PE, without a game install."""

import contextlib
import importlib.util
import io
from pathlib import Path
import struct
import subprocess
import sys
import tempfile
import unittest

SCRIPT = Path(__file__).with_name("scan-symbols.py")
SPEC = importlib.util.spec_from_file_location("scan_symbols", SCRIPT)
scanner = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(scanner)


def fixture() -> bytes:
    """PE32+ with code/data/ordinal/forwarder exports, imports, RTTI and RSDS."""
    image = bytearray(0xC00)

    def pack(offset, format, *values):
        struct.pack_into("<" + format, image, offset, *values)

    def put(offset, data):
        image[offset:offset + len(data)] = data

    put(0, b"MZ")
    pack(0x3C, "I", 0x80)
    put(0x80, b"PE\0\0")
    pack(0x84, "HHIIIHH", 0x8664, 2, 0, 0, 0, 0xF0, 0x2022)
    optional = 0x98
    pack(optional, "H", 0x20B)
    pack(optional + 16, "I", 0x1000)
    pack(optional + 20, "I", 0x1000)
    pack(optional + 24, "Q", 0x180000000)
    pack(optional + 32, "II", 0x1000, 0x200)
    pack(optional + 56, "II", 0x3000, 0x200)
    pack(optional + 68, "H", 3)
    pack(optional + 108, "I", 16)
    pack(optional + 112, "II", 0x2000, 0x200)  # Exports
    pack(optional + 120, "II", 0x2500, 40)  # Imports
    pack(optional + 112 + 6 * 8, "II", 0x2400, 28)  # Debug
    pack(optional + 112 + 13 * 8, "II", 0x2700, 64)  # Delay imports
    sections = optional + 0xF0
    pack(sections, "8sIIIIIIHHI", b".text", 0x200, 0x1000, 0x200, 0x200, 0, 0, 0, 0, 0x60000020)
    pack(sections + 40, "8sIIIIIIHHI", b".rdata", 0x800, 0x2000, 0x800, 0x400, 0, 0, 0, 0, 0x40000040)
    put(0x200, b"\xe9" + struct.pack("<i", 11))  # Linker thunk -> RVA 0x1010
    put(0x210, b"\x90\xc3\x90\x90\x90")
    put(0x220, b"\xc3\x90\x90\x90\x90")
    pack(0x400, "IIHHIIIIIII", 0, 0, 0, 0, 0x2050, 10, 4, 3, 0x2080, 0x20A0, 0x20B0)
    put(0x450, b"fixture.dll\0")
    pack(0x480, "IIII", 0x1000, 0x1020, 0x2300, 0x2180)
    pack(0x4A0, "III", 0x2100, 0x2110, 0x2120)
    pack(0x4B0, "HHH", 3, 2, 0)
    put(0x500, b"Api\0")
    put(0x510, b"MyData\0")
    put(0x520, b"?paint@Widget@MyGUI@@QEAAHXZ\0")
    put(0x580, b"other.RealApi\0")
    put(0x700, b"DATA")
    put(0x780, b".?AVWidget@MyGUI@@\0")
    debug = b"RSDS" + bytes(range(16)) + struct.pack("<I", 2) + b"C:\\build\\fixture.pdb\0"
    pack(0x800, "IIHHIIII", 0, 0, 0, 0, 2, len(debug), 0x2450, 0x850)
    put(0x850, debug)
    pack(0x900, "IIIII", 0x2640, 0, 0, 0x2600, 0x2660)
    put(0xA00, b"dependency.dll\0")
    pack(0xA40, "QQQ", 0x2680, 0x8000000000000011, 0)
    pack(0xA60, "QQQ", 0x2680, 0x8000000000000011, 0)
    put(0xA80, b"\0\0ImportedTick\0")
    pack(0xB00, "IIIIIIII", 1, 0x2760, 0, 0x27A0, 0x2780, 0, 0, 0)
    put(0xB60, b"delayed.dll\0")
    pack(0xB80, "QQ", 0x27C0, 0)
    pack(0xBA0, "QQ", 0x27C0, 0)
    put(0xBC0, b"\0\0DelayedTick\0")
    return bytes(image)


class ScannerTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.binary = self.root / "fixture.dll"
        self.binary.write_bytes(fixture())

    def scan(self):
        return scanner.scan_module(self.binary, "fixture", scanner.Demangler(False))

    def test_inventory_preserves_export_kinds_and_ordinals(self):
        module = self.scan()
        records = {item["name"]: item for item in module["exports"]}
        self.assertEqual(len(records), 4)
        self.assertEqual(records["Api"]["kind"], "forwarder")
        self.assertEqual(records["Api"]["forwarder"], "other.RealApi")
        self.assertNotIn("entry_stub", records["Api"])
        self.assertEqual(records["MyData"]["kind"], "data")
        self.assertNotIn("entry_stub", records["MyData"])
        self.assertEqual(records[None]["ordinal"], 11)
        self.assertIsNone(records[None]["rust_constant"])
        code = records["?paint@Widget@MyGUI@@QEAAHXZ"]
        self.assertEqual(code["rva"], 0x1000)
        self.assertEqual(code["entry_stub"], {
            "chain": [0x1010], "status": "resolved", "candidate_body_rva": 0x1010
        })
        generated = scanner.rust_files({"modules": [module]})["fixture.rs"]
        self.assertEqual(generated.count(": &CStr ="), 3)
        self.assertIn("ORDINAL_ONLY_EXPORT_COUNT: usize = 1", generated)

    def test_imports_rtti_and_debug_are_separate_from_exports(self):
        module = self.scan()
        imports = module["imports"]
        self.assertEqual(len(imports), 3)
        ordinal = next(item for item in imports if item["name"] is None)
        self.assertEqual(ordinal["ordinal"], 17)
        self.assertEqual(ordinal["dll"], "dependency.dll")
        delayed = next(item for item in imports if item["delayed"])
        self.assertEqual(delayed["name"], "DelayedTick")
        self.assertEqual(delayed["iat_rva"], 0x27A0)
        self.assertEqual(module["rtti_candidates"], [
            {"name": ".?AVWidget@MyGUI@@", "rva": 0x2380, "demangled": None}
        ])
        debug = module["debug_references"][0]
        self.assertEqual(debug["pdb_reference"], "C:\\build\\fixture.pdb")
        self.assertEqual(debug["guid"], "03020100-0504-0706-0809-0a0b0c0d0e0f")
        self.assertEqual(debug["age"], 2)

    def test_thunk_cycles_outside_code_and_truncation_never_claim_a_body(self):
        variants = (
            (0x200, -5, "cycle"),
            (0x200, 0x2000 - 0x1005, "outside-code"),
            (0x200, 0x11FE - 0x1005, "truncated"),
        )
        for offset, displacement, status in variants:
            with self.subTest(status=status):
                image = bytearray(fixture())
                image[offset:offset + 5] = b"\xe9" + struct.pack("<i", displacement)
                self.binary.write_bytes(image)
                code = next(item for item in self.scan()["exports"] if item["rva"] == 0x1000)
                self.assertEqual(code["entry_stub"]["status"], status)
                self.assertNotIn("candidate_body_rva", code["entry_stub"])

    def test_names_are_stable_and_distinguish_overloads(self):
        names = [b"?paint@Widget@MyGUI@@QEAAHXZ", b"?paint@Widget@MyGUI@@QEAAHH@Z"]
        identifiers = [scanner.constant_name(name) for name in names]
        self.assertNotEqual(*identifiers)
        self.assertTrue(all(name.startswith("MY_GUI_WIDGET_PAINT_") for name in identifiers))
        self.assertEqual(identifiers, list(reversed([scanner.constant_name(name) for name in reversed(names)])))
        for name in [*names, b"123", b"\xff\\quote\""]:
            self.assertRegex(scanner.constant_name(name), r"^[A-Z][A-Z0-9_]*$")
        self.assertEqual(scanner.rust_string(b'a"\\\xff\n'), 'c"a\\"\\\\\\xff\\n"')
        with self.assertRaisesRegex(ValueError, "NUL"):
            scanner.rust_string(b"bad\0name")

    def test_cli_scan_check_and_query_are_repeatable_and_read_only(self):
        output = self.root / "inventory"
        rust = self.root / "rust"
        command = [sys.executable, str(SCRIPT), "scan", "--game", str(self.root),
                   "--module", "fixture=fixture.dll", "--output", str(output),
                   "--rust-output", str(rust), "--no-demangle"]
        subprocess.run(command, check=True, capture_output=True, text=True)
        before = {path: path.read_bytes() for path in self.root.rglob("*") if path.is_file()}
        subprocess.run(command + ["--check"], check=True, capture_output=True, text=True)
        after = {path: path.read_bytes() for path in self.root.rglob("*") if path.is_file()}
        self.assertEqual(before, after)
        query = subprocess.run([sys.executable, str(SCRIPT), "query", "paint", "--catalog", str(output / "catalog.json")],
                               check=True, capture_output=True, text=True)
        self.assertIn("[code] RVA 0x1000", query.stdout)
        self.assertIn("symbols::discovered::fixture::MY_GUI_WIDGET_PAINT_", query.stdout)
        generated = rust / "fixture.rs"
        generated.write_text("manual drift\n", encoding="utf-8")
        result = subprocess.run(command + ["--check"], capture_output=True, text=True)
        self.assertEqual(result.returncode, 1)
        self.assertIn("Catalog drift", result.stderr)
        self.assertEqual(generated.read_text(encoding="utf-8"), "manual drift\n")
        # Changed bytes in a game update must also change the recorded fingerprint.
        image = bytearray(self.binary.read_bytes())
        image[0x700] ^= 1
        self.binary.write_bytes(image)
        self.assertEqual(subprocess.run(command + ["--check"], capture_output=True).returncode, 1)

    def test_invalid_input_does_not_generate_partial_files(self):
        self.binary.write_bytes(b"this is not a PE")
        with self.assertRaisesRegex(ValueError, "invalid PE"):
            self.scan()
        with self.assertRaisesRegex(ValueError, "unique"):
            scanner.module_inputs(self.root, ["fixture=fixture.dll", "fixture=fixture.dll"])
        with self.assertRaisesRegex(ValueError, "Rust module"):
            scanner.module_inputs(self.root, ["../escape=fixture.dll"])
        with self.assertRaisesRegex(ValueError, "Rust module"):
            scanner.module_inputs(self.root, ["type=fixture.dll"])

    def test_default_inputs_support_gog_and_optional_kenshi_lib(self):
        for _, name in scanner.DEFAULT_MODULES:
            if name in ("kenshi_x64.exe", "KenshiLib.dll"):
                continue
            (self.root / name).write_bytes(fixture())
        (self.root / "kenshi_GOG_x64.exe").write_bytes(fixture())
        stderr = io.StringIO()
        with contextlib.redirect_stderr(stderr):
            inputs = scanner.module_inputs(self.root, None)
        self.assertEqual(len(inputs), 4)
        self.assertEqual(dict(inputs)["kenshi"].name, "kenshi_GOG_x64.exe")
        self.assertIn("KenshiLib.dll absent", stderr.getvalue())
        (self.root / "KenshiLib.dll").write_bytes(fixture())
        self.assertEqual(len(scanner.module_inputs(self.root, None)), 5)


if __name__ == "__main__":
    unittest.main()
