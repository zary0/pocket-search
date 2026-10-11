// SPDX-License-Identifier: GPL-3.0-only
#include <stdlib.h>
static unsigned allocations, releases, fail_at;
static void *capture_malloc(size_t bytes){++allocations;return allocations==fail_at?NULL:malloc(bytes);}
static void capture_free(void *pointer){if(pointer)++releases;free(pointer);}
#define malloc capture_malloc
#define free capture_free
#include "../devices/chromatic/components/pocket_search_ui/pocket_frame_capture.h"
#undef malloc
#undef free
#include <assert.h>
int main(void){
    PS_Frame frame={0};assert(PS_FrameInit(&frame));
    uint8_t pixels[320*50];for(unsigned i=0;i<sizeof(pixels);++i)pixels[i]=(uint8_t)(i*7);
    assert(PS_FrameWrite(&frame,47,50,pixels));
    for(unsigned y=0;y<50;++y)assert(!memcmp(PS_FrameRow(&frame,y+47),pixels+y*320,320));
    assert(!PS_FrameWrite(&frame,140,5,pixels));assert(!PS_FrameWrite(&frame,145,0,pixels));assert(!PS_FrameWrite(&frame,0,1,NULL));assert(PS_FrameRow(&frame,144)==NULL);
    PS_FrameFree(&frame);assert(PS_FrameRow(&frame,0)==NULL);assert(!PS_FrameWrite(&frame,0,1,pixels));PS_FrameFree(&frame);
    for(unsigned fault=1;fault<=3;++fault){
        allocations=releases=0;fail_at=fault;
        PS_Frame failed={0};assert(!PS_FrameInit(&failed));assert(releases==fault-1);
        for(unsigned i=0;i<3;++i)assert(failed.bands[i]==NULL);
    }
    return 0;
}
