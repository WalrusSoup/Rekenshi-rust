"""Read-only PE disassembly for documenting fingerprinted native adapters."""
import pathlib, sys
sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent.parent / 'target/re-tools'))
import pefile, capstone

path = pathlib.Path(sys.argv[1])
pe = pefile.PE(str(path))
base = pe.OPTIONAL_HEADER.ImageBase
code = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_64)
code.detail = True
for query in sys.argv[2:]:
    targets = [(int(query,16), query)] if query.startswith('0x') else [
        (entry.address,entry.name.decode()) for entry in pe.DIRECTORY_ENTRY_EXPORT.symbols
        if entry.name and query in entry.name.decode()]
    for rva, name in targets:
        print(f'\n{name} RVA={rva:#x}')
        entry=pe.get_data(rva,5)
        if entry[0]==0xe9:
            import struct
            rva+=5+struct.unpack('<i',entry[1:])[0]
            print(f'thunk body={rva:#x}')
        for ins in code.disasm(pe.get_data(rva, 1200), base+rva):
            print(f'{ins.address-base:08x} {ins.mnemonic:8} {ins.op_str}')
            if ins.mnemonic == 'ret': break
