"""Read-only direct-call inventory grouped by x64 unwind function ranges."""
import pathlib, sys, struct, bisect
sys.path.insert(0,str(pathlib.Path(__file__).resolve().parent.parent/'target/re-tools'))
import pefile
pe=pefile.PE(sys.argv[1]); image=pe.get_memory_mapped_image()
ranges=sorted((e.struct.BeginAddress,e.struct.EndAddress) for e in pe.DIRECTORY_ENTRY_EXCEPTION)
begins=[a for a,b in ranges]
targets=set(int(value,16) for value in sys.argv[2:])
for section in pe.sections:
 if not section.Characteristics&0x20000000: continue
 lo=section.VirtualAddress; hi=lo+section.Misc_VirtualSize
 for offset in range(lo,hi-5):
  if image[offset]!=0xe8: continue
  destination=offset+5+struct.unpack_from('<i',image,offset+1)[0]
  direct=destination
  if 0<=destination<len(image)-5 and image[destination]==0xe9:
   destination+=5+struct.unpack_from('<i',image,destination+1)[0]
  if destination in targets or direct in targets:
   index=bisect.bisect_right(begins,offset)-1
   start,end=ranges[index]
   print(f'call {offset:#x} via {direct:#x} -> {destination:#x}, function {start:#x}..{end:#x}')
