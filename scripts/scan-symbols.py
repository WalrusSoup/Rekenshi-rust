"""Inventory PE symbols on disk and generate a Rust export-name catalog.

Never loads game binaries. Names are discovery data, not verified FFI bindings.
Python 3.10+; install scripts/requirements-symbols.txt for the PE reader.
"""

from __future__ import annotations

import argparse
import ctypes
import hashlib
import json
import os
from pathlib import Path
import re
import struct
import sys
import uuid

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "target" / "re-tools"))
DEFAULT_MODULES = (
    ("kenshi", "kenshi_x64.exe"),
    ("mygui", "MyGUIEngine_x64.dll"),
    ("ogre", "OgreMain_x64.dll"),
    ("particle_universe", "Plugin_ParticleUniverse_x64.dll"),
    ("kenshi_lib", "KenshiLib.dll"),
)
EXECUTABLE = 0x20000000
RTTI_PATTERN = re.compile(rb"\.\?A[UV][\x21-\x7e]{1,1024}\x00")
RUST_KEYWORDS = frozenset("""
    mod self super crate type fn pub use const static as async await enum struct
    impl trait match loop move ref mut in where unsafe extern return let if else
    while for break continue true false dyn abstract become box do final macro
    override priv typeof unsized virtual yield try gen Self
""".split())


class Demangler:
    """Sequential calls to the system DbgHelp API, with a raw-name fallback."""

    def __init__(self, enabled: bool = True):
        self.function = None
        self.backend = "none"
        if enabled and os.name == "nt":
            # Restrict loading to the Windows system directory.
            self.library = ctypes.WinDLL("dbghelp.dll", winmode=0x800)
            self.function = self.library.UnDecorateSymbolName
            self.function.argtypes = [
                ctypes.c_char_p, ctypes.c_void_p, ctypes.c_uint32, ctypes.c_uint32
            ]
            self.function.restype = ctypes.c_uint32
            self.backend = "windows-dbghelp"

    def __call__(self, name: bytes) -> str | None:
        if self.function is None or not name.startswith(b"?"):
            return None
        output = ctypes.create_string_buffer(65536)
        if not self.function(name, output, len(output), 0):
            return None
        text = output.value.decode("ascii", errors="replace")
        return text if text != name.decode("ascii", errors="replace") else None


def rust_string(value: bytes) -> str:
    """Lossless C-string literal, including non-ASCII export-name bytes."""
    if b"\0" in value:
        raise ValueError("a C-string constant cannot contain an interior NUL")
    escapes = {34: '\\"', 92: "\\\\", 10: "\\n", 13: "\\r", 9: "\\t"}
    body = "".join(
        escapes.get(byte, chr(byte) if 32 <= byte <= 126 else f"\\x{byte:02x}")
        for byte in value
    )
    return f'c"{body}"'


def constant_name(name: bytes) -> str:
    """Readable scope + stable byte hash; overloads don't depend on scan order."""
    text = name.decode("ascii", errors="replace")
    if text.startswith("??0") or text.startswith("??1"):
        member = "CONSTRUCT" if text[2] == "0" else "DESTROY"
        scopes = text[3:].split("@@", 1)[0].split("@")
    elif text.startswith("?") and not text.startswith("??"):
        parts = text[1:].split("@@", 1)[0].split("@")
        member, scopes = parts[0], parts[1:]
    else:
        member, scopes = text, []
    stem = "_".join([*reversed(scopes), member])
    stem = re.sub(r"([a-z0-9])([A-Z])", r"\1_\2", stem)
    stem = re.sub(r"[^A-Za-z0-9]+", "_", stem).strip("_").upper()[:100]
    if not stem or stem[0].isdigit():
        stem = "EXPORT_" + stem
    return stem + "_" + hashlib.sha256(name).hexdigest()[:12].upper()


def section_at(pe, rva: int):
    for section in pe.sections:
        start = section.VirtualAddress
        if start <= rva < start + max(section.Misc_VirtualSize, section.SizeOfRawData):
            return section
    return None


def thunk_target(pe, entry: int, max_hops: int = 16) -> dict:
    """Follow only bounded E9 rel32 linker stubs in executable sections."""
    current, seen, chain = entry, set(), []
    for _ in range(max_hops):
        section = section_at(pe, current)
        if section is None or not section.Characteristics & EXECUTABLE:
            return {"chain": chain, "status": "outside-code"}
        if current in seen:
            return {"chain": chain, "status": "cycle"}
        seen.add(current)
        # Do not interpret bytes across section/file-backed boundaries.
        raw_end = section.VirtualAddress + section.SizeOfRawData
        if current + 5 > raw_end:
            return {"chain": chain, "status": "truncated"}
        code = pe.get_data(current, 5)
        if len(code) != 5:
            return {"chain": chain, "status": "truncated"}
        if code[0] != 0xE9:
            return {"chain": chain, "status": "resolved", "candidate_body_rva": current}
        current += 5 + struct.unpack("<i", code[1:])[0]
        chain.append(current)
    return {"chain": chain, "status": "hop-limit"}


def export_record(pe, symbol, demangle: Demangler, module: str) -> dict:
    name = symbol.name
    section = section_at(pe, symbol.address)
    if symbol.forwarder is not None:
        kind = "forwarder"
    elif section is not None and section.Characteristics & EXECUTABLE:
        kind = "code"
    elif section is not None:
        kind = "data"
    else:
        kind = "unmapped"
    record = {
        "name": name.decode("ascii", errors="backslashreplace") if name else None,
        "name_hex": name.hex() if name else None,
        "ordinal": symbol.ordinal,
        "rva": symbol.address,
        "kind": kind,
        "section": section.Name.rstrip(b"\0").decode("ascii", errors="replace") if section else None,
        "forwarder": symbol.forwarder.decode("ascii", errors="replace") if symbol.forwarder else None,
        "demangled": demangle(name) if name else None,
        "rust_constant": f"rekenshi_native::symbols::discovered::{module}::{constant_name(name)}" if name else None,
    }
    if kind == "code":
        record["entry_stub"] = thunk_target(pe, symbol.address)
    return record


def scan_module(path: Path, module: str, demangle: Demangler) -> dict:
    try:
        import pefile
    except ImportError as error:
        raise ValueError("install the scanner dependency: python -m pip install -r scripts/requirements-symbols.txt") from error
    data = path.read_bytes()
    try:
        pe = pefile.PE(data=data, fast_load=True)
        pe.parse_data_directories(directories=[
            pefile.DIRECTORY_ENTRY["IMAGE_DIRECTORY_ENTRY_EXPORT"],
            pefile.DIRECTORY_ENTRY["IMAGE_DIRECTORY_ENTRY_IMPORT"],
            pefile.DIRECTORY_ENTRY["IMAGE_DIRECTORY_ENTRY_DELAY_IMPORT"],
            pefile.DIRECTORY_ENTRY["IMAGE_DIRECTORY_ENTRY_DEBUG"],
        ])
    except pefile.PEFormatError as error:
        raise ValueError(f"{path.name}: invalid PE: {error}") from error
    try:
        exports = [export_record(pe, item, demangle, module)
                   for item in getattr(getattr(pe, "DIRECTORY_ENTRY_EXPORT", None), "symbols", [])]
        exports.sort(key=lambda item: (item["name_hex"] or "", item["ordinal"]))
        constants = [item["rust_constant"] for item in exports if item["name"]]
        if len(constants) != len(set(constants)):
            raise ValueError(f"{path.name}: duplicate export name or generated identifier collision")
        imports = []
        for attribute, delayed in (("DIRECTORY_ENTRY_IMPORT", False), ("DIRECTORY_ENTRY_DELAY_IMPORT", True)):
            for dependency in getattr(pe, attribute, []):
                for item in dependency.imports:
                    imports.append({
                        "dll": dependency.dll.decode("ascii", errors="backslashreplace"),
                        "name": item.name.decode("ascii", errors="backslashreplace") if item.name else None,
                        "ordinal": item.ordinal,
                        "iat_rva": item.address - pe.OPTIONAL_HEADER.ImageBase,
                        "delayed": delayed,
                        "demangled": demangle(item.name) if item.name else None,
                    })
        imports.sort(key=lambda item: (item["dll"], item["name"] or "", item["ordinal"] or 0, item["iat_rva"]))
        rtti = []
        for section in pe.sections:
            if not section.SizeOfRawData:
                continue
            raw = data[section.PointerToRawData:section.PointerToRawData + section.SizeOfRawData]
            for match in RTTI_PATTERN.finditer(raw):
                name = match.group()[:-1]
                rtti.append({
                    "name": name.decode("ascii"),
                    "rva": section.VirtualAddress + match.start(),
                    "demangled": demangle(b"??_R0" + name[1:] + b"@8"),
                })
        rtti.sort(key=lambda item: (item["name"], item["rva"]))
        debug = []
        for entry in getattr(pe, "DIRECTORY_ENTRY_DEBUG", []):
            if entry.struct.Type != 2:
                continue
            start = entry.struct.PointerToRawData
            raw = data[start:start + entry.struct.SizeOfData]
            if raw.startswith(b"RSDS") and len(raw) >= 24:
                debug.append({
                    "format": "RSDS",
                    "guid": str(uuid.UUID(bytes_le=raw[4:20])),
                    "age": struct.unpack("<I", raw[20:24])[0],
                    "pdb_reference": raw[24:].split(b"\0", 1)[0].decode("utf-8", errors="replace"),
                })
        return {
            "module": module,
            "file": path.name,
            "sha256": hashlib.sha256(data).hexdigest(),
            "file_bytes": len(data),
            "machine": f"0x{pe.FILE_HEADER.Machine:04x}",
            "image_base": pe.OPTIONAL_HEADER.ImageBase,
            "coff_symbol_count": pe.FILE_HEADER.NumberOfSymbols,
            "exports": exports,
            "imports": imports,
            "rtti_candidates": rtti,
            "debug_references": debug,
            "pe_warnings": pe.get_warnings(),
        }
    finally:
        pe.close()


def rust_files(catalog: dict) -> dict[str, str]:
    modules = catalog["modules"]
    root = [
        "//! Generated export names, not verified callable bindings.",
        "//! Regenerate with `scripts/scan-symbols.py scan`; see `docs/native-symbols.md`.",
        "#![forbid(unsafe_code)]", "",
    ]
    files = {}
    for module in modules:
        key = module["module"]
        # Generated declarations stay byte-for-byte reproducible through cargo fmt.
        root.extend(["#[rustfmt::skip]", f"pub mod {key};", ""])
        lines = [
            f"//! Generated exports from `{module['file']}`.",
            "//! An exported name does not establish a safe function signature.",
            "use std::ffi::CStr;", "",
            f"pub const FILE: &str = {json.dumps(module['file'], ensure_ascii=False)};",
            f"pub const SCANNED_SHA256: &str = \"{module['sha256']}\";", "",
        ]
        names = []
        for item in module["exports"]:
            if item["name_hex"] is None:
                continue
            identifier = item["rust_constant"].rsplit("::", 1)[1]
            declaration = item["demangled"] or item["name"]
            # A single comment line; no syntax from an untrusted DLL is emitted as code.
            declaration = " ".join(declaration.splitlines())
            lines.extend([
                f"/// {item['kind']}: `{declaration}`",
                f"pub const {identifier}: &CStr = {rust_string(bytes.fromhex(item['name_hex']))};", "",
            ])
            names.append(identifier)
        lines.append("/// All named exports, including data and forwarders; not a list of callable hooks.")
        lines.append("pub const EXPORTED_NAMES: &[&CStr] = &[")
        lines.extend(f"    {name}," for name in names)
        lines.extend([
            "];", "",
            f"pub const ORDINAL_ONLY_EXPORT_COUNT: usize = {sum(item['name'] is None for item in module['exports'])};", "",
        ])
        files[f"{key}.rs"] = "\n".join(lines)
    files["mod.rs"] = "\n".join(root)
    return files


def summary(catalog: dict) -> str:
    lines = ["# Native symbol scan", "", "Names are discovery data; verify ABI and owning binary before writing a binding.", "",
             "| File | Named exports | Ordinal only | Imports | RTTI candidates |", "| --- | ---: | ---: | ---: | ---: |"]
    for module in catalog["modules"]:
        named = sum(item["name"] is not None for item in module["exports"])
        lines.append(f"| {module['file']} | {named} | {len(module['exports']) - named} | {len(module['imports'])} | {len(module['rtti_candidates'])} |")
    lines.extend(["", "Imports name dependency functions; RTTI strings name types. Neither identifies a callable local function.",
                  "Debug records reference build-time PDBs; this scanner does not retrieve or parse those files.", ""])
    for module in catalog["modules"]:
        lines.extend([f"## {module['file']}", "", f"SHA256: `{module['sha256']}`", "",
                      f"COFF symbols: {module['coff_symbol_count']}", ""])
        for record in module["debug_references"]:
            lines.extend([f"PDB reference: `{record['pdb_reference']}` (GUID `{record['guid']}`, age {record['age']})", ""])
    return "\n".join(lines)


def write_files(files: dict[Path, str], check: bool) -> bool:
    mismatches = []
    for path, content in files.items():
        encoded = content.encode("utf-8")
        if path.is_file() and path.read_bytes() == encoded:
            continue
        if check:
            mismatches.append(str(path))
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(encoded)
    if mismatches:
        print("Catalog drift (no files changed):\n" + "\n".join(mismatches), file=sys.stderr)
    return not mismatches


def module_inputs(game: Path, overrides: list[str] | None) -> list[tuple[str, Path]]:
    result = []
    if overrides:
        for specification in overrides:
            key, separator, file = specification.partition("=")
            if not separator or not re.fullmatch(r"[a-z][a-z0-9_]*", key) or key in RUST_KEYWORDS:
                raise ValueError("--module expects a Rust module name and file, e.g. mygui=MyGUIEngine_x64.dll")
            path = Path(file)
            result.append((key, path if path.is_absolute() else game / path))
    else:
        for key, file in DEFAULT_MODULES:
            path = game / file
            if key == "kenshi" and not path.is_file():
                path = game / "kenshi_GOG_x64.exe"
            if key == "kenshi_lib" and not path.is_file():
                print("KenshiLib.dll absent; scanning the four game binaries.", file=sys.stderr)
                continue
            result.append((key, path))
    if len({key for key, _ in result}) != len(result):
        raise ValueError("module names must be unique")
    for _, path in result:
        if not path.is_file():
            raise ValueError(f"binary not found: {path}")
    return sorted(result)


def scan(args) -> int:
    inputs = module_inputs(args.game, args.module)
    demangle = Demangler(not args.no_demangle)
    catalog = {
        "schema_version": 1,
        "demangler": demangle.backend,
        "modules": [scan_module(path, key, demangle) for key, path in inputs],
    }
    files = {
        args.output / "catalog.json": json.dumps(catalog, indent=2, ensure_ascii=True) + "\n",
        args.output / "summary.md": summary(catalog),
    }
    files.update({args.rust_output / name: content for name, content in rust_files(catalog).items()})
    if not write_files(files, args.check):
        return 1
    for module in catalog["modules"]:
        named = sum(item["name"] is not None for item in module["exports"])
        print(f"{module['file']}: {named} named, {len(module['exports']) - named} ordinal-only exports; {len(module['imports'])} imports; {len(module['rtti_candidates'])} RTTI candidates")
    print("Catalog verified." if args.check else f"Wrote {args.output / 'catalog.json'} and {args.rust_output}")
    return 0


def query(args) -> int:
    catalog = json.loads(args.catalog.read_text(encoding="utf-8"))
    if catalog.get("schema_version") != 1:
        raise ValueError("unsupported catalog schema; rescan with this tool")
    needle, matches = args.text.casefold(), []
    groups = ("exports", "imports", "rtti_candidates") if args.kind == "all" else (args.kind,)
    for module in catalog["modules"]:
        if args.module and args.module not in (module["module"], module["file"]):
            continue
        for group in groups:
            for item in module[group]:
                searchable = " ".join(str(item.get(key) or "") for key in ("name", "demangled", "rust_constant", "dll"))
                if needle in searchable.casefold():
                    matches.append((module, group, item))
    for module, group, item in matches[:args.limit]:
        kind = item.get("kind", group)
        address = item.get("rva", item.get("iat_rva"))
        print(f"{module['file']} [{kind}] RVA 0x{address:x}: {item['name'] or ('ordinal ' + str(item['ordinal']))}")
        if item.get("demangled"):
            print("  " + item["demangled"])
        if item.get("rust_constant"):
            print("  " + item["rust_constant"])
        if group == "imports":
            print("  imported from " + item["dll"])
        if item.get("forwarder"):
            print("  forwards to " + item["forwarder"])
    print(f"{len(matches)} matches; showing {min(len(matches), args.limit)}.")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    scanner = commands.add_parser("scan", help="scan binaries and regenerate JSON, summary and Rust constants")
    scanner.add_argument("--game", required=True, type=Path, help="Kenshi install directory")
    scanner.add_argument("--module", action="append", help="override default modules: rust_name=file; repeatable")
    scanner.add_argument("--output", type=Path, default=ROOT / "target/native-symbols")
    scanner.add_argument("--rust-output", type=Path, default=ROOT / "crates/native/src/symbols/discovered")
    scanner.add_argument("--no-demangle", action="store_true", help="portable raw-name output without Windows DbgHelp")
    scanner.add_argument("--check", action="store_true", help="fail on drift without writing any files")
    scanner.set_defaults(run=scan)
    search = commands.add_parser("query", help="search the catalog without rescanning")
    search.add_argument("text")
    search.add_argument("--catalog", type=Path, default=ROOT / "target/native-symbols/catalog.json")
    search.add_argument("--module", help="filter by Rust module key or binary filename")
    search.add_argument("--kind", choices=("exports", "imports", "rtti_candidates", "all"), default="exports")
    search.add_argument("--limit", type=int, default=30)
    search.set_defaults(run=query)
    args = parser.parse_args()
    if args.command == "query" and args.limit < 1:
        parser.error("--limit must be positive")
    try:
        return args.run(args)
    except (ValueError, OSError) as error:
        print(f"Symbol scan failed: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
