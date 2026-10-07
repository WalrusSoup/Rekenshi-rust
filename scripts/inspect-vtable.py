"""Locate a Microsoft x64 RTTI vtable in a PE without running it."""
import sys,pathlib,struct
sys.path.insert(0,str(pathlib.Path(__file__).resolve().parent.parent/'target/re-tools'))
import pefile
p=pefile.PE(sys.argv[1]); b=p.get_memory_mapped_image()
name=sys.argv[2].encode()+b'\0'; td=b.find(name)-16
assert td>=0
needle=struct.pack('<I',td); pos=0
while True:
    pos=b.find(needle,pos)
    if pos<0:break
    col=pos-12
    if struct.unpack_from('<I',b,col)[0]==1 and struct.unpack_from('<I',b,col+20)[0]==col:
        print('COL',hex(col)); ref=struct.pack('<Q',p.OPTIONAL_HEADER.ImageBase+col); q=b.find(ref)
        while q>=0:
            print('vtable',hex(q+8),[hex(x-p.OPTIONAL_HEADER.ImageBase) for x in struct.unpack_from('<4Q',b,q+8)]); q=b.find(ref,q+1)
    pos+=1
