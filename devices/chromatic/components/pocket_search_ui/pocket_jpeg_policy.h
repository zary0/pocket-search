// SPDX-License-Identifier: GPL-3.0-only
#pragma once
#include <stddef.h>
#include <stdint.h>
// Admit only bounded, single-scan, 8-bit baseline JPEG before invoking ROM.
// This validates the envelope/table bounds, not entropy-decoder correctness.
static inline int PS_JPEGPolicy(const uint8_t *b,size_t n){
    if(!b||n<4||n>24576||b[0]!=255||b[1]!=216)return 0;
    size_t at=2;unsigned markers=0,components=0;
    while(at+4<=n&&++markers<=128){
        if(b[at++]!=255)return 0;
        while(at<n&&b[at]==255)++at;
        if(at>=n)return 0;
        unsigned tag=b[at++];if(tag==0||tag==216||tag==217||(tag>=208&&tag<=215))return 0;
        if(at+2>n)return 0;
        size_t size=((size_t)b[at]<<8)|b[at+1];
        if(size<2||size>n-at)return 0;
        if(tag==192){
            if(components||size<8||b[at+2]!=8)return 0;
            unsigned h=((unsigned)b[at+3]<<8)|b[at+4],w=((unsigned)b[at+5]<<8)|b[at+6];
            components=b[at+7];if(!w||!h||w>1024||h>1024||(components!=1&&components!=3)||size!=8+3*components)return 0;
            unsigned blocks=0;
            for(unsigned i=0;i<components;++i){unsigned sampling=b[at+9+3*i],x=sampling>>4,y=sampling&15;if(!x||!y||x>2||y>2||b[at+10+3*i]>3)return 0;blocks+=x*y;}
            if(blocks>6)return 0;
        }else if(tag>=193&&tag<=207&&tag!=196)return 0;
        else if(tag==219){
            size_t p=at+2;while(p<at+size){if(b[p]>3||at+size-p<65)return 0;p+=65;}
        }else if(tag==196){
            size_t p=at+2;while(p<at+size){if(at+size-p<17||(b[p]>>4)>1||(b[p]&15)>1)return 0;unsigned symbols=0,space=1;for(unsigned i=1;i<=16;++i){space*=2;if(b[p+i]>space)return 0;space-=b[p+i];symbols+=b[p+i];}if(!symbols||symbols>((b[p]>>4)?162:12)||symbols>at+size-p-17)return 0;for(unsigned i=0;i<symbols;++i){unsigned v=b[p+17+i];if((b[p]>>4)?(v!=0&&v!=240&&((v&15)==0||(v&15)>10)):v>11)return 0;}p+=17+symbols;}
        }else if(tag==221){if(size!=4)return 0;}
        else if(tag==218){
            if(!components||size!=6+2*components||b[at+2]!=components)return 0;
            for(unsigned i=0;i<components;++i){unsigned table=b[at+4+2*i];if((table>>4)>1||(table&15)>1)return 0;}
            if(b[at+size-3]!=0||b[at+size-2]!=63||b[at+size-1]!=0)return 0;
            at+=size;
            while(at<n){if(b[at++]!=255)continue;if(at>=n)return 0;unsigned value=b[at++];if(value==0||(value>=208&&value<=215))continue;if(value==217)return at==n;return 0;}
            return 0;
        }else if(!((tag>=224&&tag<=239)||tag==254))return 0;
        at+=size;
    }return 0;
}
