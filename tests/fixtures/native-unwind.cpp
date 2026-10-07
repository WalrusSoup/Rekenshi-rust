// Test-only bridge: ensure native Ogre exceptions cross Rust C-unwind frames.
#include <cstdint>
#include <cstring>
#include "../../target/references/RE_Kenshi/SIC.h"
// Test-only oracle; the upstream implementation is never linked into the pack.
extern "C" __declspec(dllexport) size_t reference_sic(void* out, size_t capacity, const void* input, size_t width, size_t height, int format, int filter, int layout) {
    return SIC::Compress(out,capacity,input,width,height,static_cast<SIC::EncodeType>(format),static_cast<SIC::FilterType>(filter),static_cast<SIC::SICHeader::RearrangeType::Enum>(layout),3);
}
extern "C" __declspec(dllexport) int catch_decode(void* target, void* self, void* output, void* input) {
    using Function = void* (*)(void*,void*,void*);
    try { reinterpret_cast<Function>(target)(self,output,input); return 1; }
    catch (...) { return 0; }
}
