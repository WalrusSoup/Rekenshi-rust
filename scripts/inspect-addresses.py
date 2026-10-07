import sys,pathlib,struct,re
sys.path.insert(0,str(pathlib.Path(__file__).resolve().parent.parent/'target/re-tools'))
import pefile
game=pathlib.Path('D:/SteamLibrary/steamapps/common/Kenshi');p=pefile.PE(str(game/'kenshi_x64.exe'))
for x in (0x16d0ea8,0x16ab37c,0x171cb25,0x167c3c5,0x171cb18,0x171cb13,0x168d9b7):
    print(hex(x),repr(p.get_data(x,40).split(b'\0')[0]))
data=(game/'RE_Kenshi/RVAs/Steam_1.0.65.br').read_bytes()
asm=pathlib.Path('target/references/KenshiLib/Source/kenshi/symbols.asm').read_text()
print('table length',len(data)//4)
for group in ('static_functions','PhysicsActual'):
    match=re.search('_'+group+'_base EQU (\d+)\*8',asm); index=int(match[1])
    print(group,index,[hex(x) for x in struct.unpack_from('<8I',data,index*4)])
