"""Match relocation-independent instruction bytes between reference PEs."""
import sys,pathlib,re,struct
sys.path.insert(0,str(pathlib.Path(__file__).resolve().parent.parent/'target/re-tools'))
import pefile,capstone
game=pathlib.Path('D:/SteamLibrary/steamapps/common/Kenshi')
old=pefile.PE(str(game/'RE_Kenshi/kenshi_x64.exe'))
new=pefile.PE(str(game/'kenshi_x64.exe'))
md=capstone.Cs(capstone.CS_ARCH_X86,capstone.CS_MODE_64); md.detail=True
image=new.get_memory_mapped_image()
for addr in sys.argv[1:]:
    rva=int(addr,16); sample=old.get_data(rva,160); mask=bytearray(b'\xff'*len(sample))
    for ins in md.disasm(sample,old.OPTIONAL_HEADER.ImageBase+rva):
        offset=ins.address-old.OPTIONAL_HEADER.ImageBase-rva
        if ins.mnemonic in ('call','jmp') or ins.mnemonic.startswith('j'):
            if ins.imm_size: mask[offset+ins.imm_offset:offset+ins.imm_offset+ins.imm_size]=bytes(ins.imm_size)
        if any(op.type==capstone.CS_OP_MEM and op.mem.base==capstone.x86.X86_REG_RIP for op in ins.operands):
            mask[offset+ins.disp_offset:offset+ins.disp_offset+ins.disp_size]=bytes(ins.disp_size)
    spans=list(re.finditer(b'\xff+',mask)); anchor=max(spans,key=lambda m:m.end()-m.start()); start=anchor.start()
    needle=sample[anchor.start():anchor.end()]; hits=[]; pos=0
    while True:
        pos=image.find(needle,pos)
        if pos<0:break
        candidate=pos-start
        if candidate>=0 and all(not m or a==b for a,b,m in zip(sample,image[candidate:candidate+len(sample)],mask)):hits.append(candidate)
        pos+=1
    print(addr,'matches',[hex(v) for v in hits])
for match in re.finditer(rb'\.\?AV[^\x00]{3,100}\x00',image):
    name=match.group().decode(errors='replace')
    if any(s.lower() in name.lower() for s in ('loadinglistener','DDS','NxuPhysics','PhysicsActual')):
        print(hex(match.start()),name)
