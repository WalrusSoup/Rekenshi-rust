import sys,pathlib
sys.path.insert(0,str(pathlib.Path(__file__).resolve().parent.parent/'target/re-tools'))
import pefile,capstone
p=pefile.PE('D:/SteamLibrary/steamapps/common/Kenshi/kenshi_x64.exe');md=capstone.Cs(capstone.CS_ARCH_X86,capstone.CS_MODE_64);md.detail=True
for ins in md.disasm(p.get_data(0x82ddc0,0x200),0x82ddc0):
    if ins.mnemonic=='lea' and ins.operands[-1].type==capstone.CS_OP_MEM and ins.operands[-1].mem.base==capstone.x86.X86_REG_RIP:
        rva=ins.address+ins.size+ins.operands[-1].mem.disp
        print(hex(ins.address),hex(rva),repr(p.get_data(rva,48).split(b'\0')[0]))
