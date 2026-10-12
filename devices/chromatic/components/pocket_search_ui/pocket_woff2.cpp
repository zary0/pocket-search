// SPDX-License-Identifier: GPL-3.0-only
#include "pocket_woff2.h"
#include "pocket_woff2_budget.hpp"
#include <woff2/decode.h>
#include <woff2/output.h>
#include <atomic>
#include <cstdlib>
#include <cstring>
#include <cstddef>
namespace {
struct alignas(std::max_align_t) Header{size_t bytes;};
struct Stopped{int error;};
struct Budget{PS_FontBudget *callbacks;size_t live=0;uint64_t start=0;int error=0;};
Budget *active=nullptr;
std::atomic_flag ownership=ATOMIC_FLAG_INIT;
uint32_t be32(const uint8_t *p){return uint32_t(p[0])<<24|uint32_t(p[1])<<16|uint32_t(p[2])<<8|p[3];}
uint16_t be16(const uint8_t *p){return uint16_t(p[0])<<8|p[1];}
void *brotli_alloc(void*,size_t n){try{return PSFontAlloc(n);}catch(...){return nullptr;}}
void brotli_free(void*,void *p){PSFontFree(p);}
struct BrotliDelete{void operator()(BrotliDecoderState *p)const noexcept{BrotliDecoderDestroyInstance(p);}};
class Output:public woff2::WOFF2Out {
    uint8_t *bytes;size_t capacity,used=0;
public:
    Output(uint8_t *b,size_t n):bytes(b),capacity(n){}
    bool Write(const void *p,size_t n)override{return Write(p,used,n);}
    bool Write(const void *p,size_t offset,size_t n)override{PSFontCheckpoint();if(offset>capacity||n>capacity-offset)return false;if(n)memcpy(bytes+offset,p,n);used=std::max(used,offset+n);return true;}
    size_t Size()override{return used;}
};
}
void PSFontCheckpoint(){
    if(!active)throw Stopped{PS_FONT_BUSY};
    auto c=active->callbacks;
    if((c->cancel&&c->cancel(c->context))||(c->clock_ms&&c->clock_ms(c->context)-active->start>=5000)){
        active->error=PS_FONT_CANCELLED;throw Stopped{PS_FONT_CANCELLED};
    }
}
void *PSFontAlloc(size_t n){
    PSFontCheckpoint();
    if(n>PS_FONT_PRIVATE-sizeof(Header)||active->live>PS_FONT_PRIVATE-n-sizeof(Header)){
        active->error=PS_FONT_MEMORY;throw std::bad_alloc();
    }
    if(active->callbacks->free_heap&&active->callbacks->free_heap(active->callbacks->context)<n+sizeof(Header)+16384){active->error=PS_FONT_MEMORY;throw std::bad_alloc();}
    Header *p=static_cast<Header*>(malloc(n+sizeof(Header)));
    if(!p){active->error=PS_FONT_MEMORY;throw std::bad_alloc();}
    p->bytes=n+sizeof(Header);active->live+=p->bytes;
    active->callbacks->peak_bytes=std::max(active->callbacks->peak_bytes,unsigned(active->live));++active->callbacks->allocations;
    return p+1;
}
void PSFontFree(void *p){if(!p)return;Header *h=static_cast<Header*>(p)-1;if(active)active->live-=h->bytes;free(h);}
BrotliDecoderResult PSBrotliDecode(size_t n,const uint8_t *input,size_t *out_size,uint8_t *output){
    std::unique_ptr<BrotliDecoderState,BrotliDelete> decoder(BrotliDecoderCreateInstance(brotli_alloc,brotli_free,nullptr));
    if(!decoder){if(active->error)throw Stopped{active->error};return BROTLI_DECODER_RESULT_ERROR;}
    size_t source=0,dest=0,total=0;
    while(true){
        PSFontCheckpoint();size_t ai=std::min(size_t(256),n-source),ao=std::min(size_t(256),*out_size-dest);
        const uint8_t *pi=input+source;uint8_t *po=output+dest;size_t oldi=ai,oldo=ao;
        auto result=BrotliDecoderDecompressStream(decoder.get(),&ai,&pi,&ao,&po,&total);source+=oldi-ai;dest+=oldo-ao;
        if(active->error)throw Stopped{active->error};
        if(result==BROTLI_DECODER_RESULT_SUCCESS){if(source!=n||dest!=*out_size)return BROTLI_DECODER_RESULT_ERROR;return result;}
        if(result==BROTLI_DECODER_RESULT_ERROR||(oldi==ai&&oldo==ao)||(source==n&&result==BROTLI_DECODER_RESULT_NEEDS_MORE_INPUT)||(dest==*out_size&&result==BROTLI_DECODER_RESULT_NEEDS_MORE_OUTPUT))return BROTLI_DECODER_RESULT_ERROR;
    }
}
extern "C" int PS_Woff2Decode(const uint8_t *b,size_t n,uint8_t **out,size_t *out_n,PS_FontBudget *callbacks){
    if(!out||!out_n||!callbacks)return PS_FONT_INVALID;
    *out=nullptr;*out_n=0;callbacks->peak_bytes=callbacks->allocations=0;
    if(!b||n<48||memcmp(b,"wOF2",4))return PS_FONT_INVALID;
    if(n>PS_FONT_WIRE)return PS_FONT_LIMIT;
    if(be32(b+4)!=0x10000)return PS_FONT_UNSUPPORTED;
    unsigned tables=be16(b+12);size_t size=be32(b+16);
    if(be32(b+8)!=n||!tables||tables>24||be16(b+14)||size<12+tables*16||!be32(b+20)||be32(b+20)>n)return PS_FONT_INVALID;
    if(size>PS_FONT_SFNT)return PS_FONT_LIMIT;
    if(be32(b+28)||be32(b+32)||be32(b+36)||be32(b+40)||be32(b+44))return PS_FONT_UNSUPPORTED;
    if(ownership.test_and_set(std::memory_order_acquire))return PS_FONT_BUSY;
    Budget budget{callbacks};if(callbacks->clock_ms)budget.start=callbacks->clock_ms(callbacks->context);active=&budget;
    uint8_t *bytes=nullptr;int error=PS_FONT_INVALID;
    try{
        bytes=static_cast<uint8_t*>(PSFontAlloc(size));memset(bytes,0,size);Output output(bytes,size);
        if(woff2::ConvertWOFF2ToTTF(b,n,&output)&&output.Size()==size){PSFontCheckpoint();*out=bytes;*out_n=size;bytes=nullptr;error=PS_FONT_OK;}
    }catch(const Stopped &s){error=s.error;}catch(const std::bad_alloc&){error=PS_FONT_MEMORY;}catch(...){error=PS_FONT_INVALID;}
    PSFontFree(bytes);active=nullptr;ownership.clear(std::memory_order_release);return error;
}
extern "C" void PS_Woff2Free(uint8_t *p){if(p)free(reinterpret_cast<Header*>(p)-1);}
