// SPDX-License-Identifier: GPL-3.0-only
#pragma once
#include "pocket_native_image.h"
#include <string.h>
typedef struct {unsigned width,height,depth,color,channels,row_bytes;size_t filtered_bytes,idat_bytes;const uint8_t *palette,*transparency;unsigned palette_entries,transparent_bytes;} PS_PNGInfo;
static inline uint32_t PS_ImageBE32(const uint8_t *p){return (uint32_t)p[0]<<24|(uint32_t)p[1]<<16|(uint32_t)p[2]<<8|p[3];}
static inline uint32_t PS_PNGCRC(const uint8_t *p,size_t n){
    uint32_t crc=~0u;for(size_t i=0;i<n;++i){crc^=p[i];for(unsigned j=0;j<8;++j)crc=(crc>>1)^(0xedb88320u&(0u-(crc&1)));}return ~crc;
}
// Verify the complete envelope, chunk order and CRCs before any decoder allocation.
static inline int PS_PNGPolicy(const uint8_t *b,size_t n,PS_PNGInfo *out){
    if(!out||!b||n<8)return PS_IMAGE_MALFORMED;
    *out=(PS_PNGInfo){0};if(n>PS_NATIVE_IMAGE_WIRE)return PS_IMAGE_LIMIT;
    if(memcmp(b,"\x89PNG\r\n\x1a\n",8))return PS_IMAGE_UNSUPPORTED;
    size_t at=8;unsigned chunks=0;bool header=false,data=false,ended_data=false,plte=false,trns=false;
    while(n-at>=12&&++chunks<=256){
        size_t size=PS_ImageBE32(b+at);if(size>n-at-12)return PS_IMAGE_MALFORMED;
        const uint8_t *tag=b+at+4,*p=b+at+8;
        for(unsigned i=0;i<4;++i)if(!((tag[i]>='A'&&tag[i]<='Z')||(tag[i]>='a'&&tag[i]<='z')))return PS_IMAGE_MALFORMED;
        if(tag[2]&32||PS_PNGCRC(tag,size+4)!=PS_ImageBE32(p+size))return PS_IMAGE_MALFORMED;
        if(!header&&memcmp(tag,"IHDR",4))return PS_IMAGE_MALFORMED;
        if(!memcmp(tag,"IHDR",4)){
            if(header||size!=13)return PS_IMAGE_MALFORMED;
            header=true;out->width=PS_ImageBE32(p);out->height=PS_ImageBE32(p+4);out->depth=p[8];out->color=p[9];
            if(!out->width||!out->height)return PS_IMAGE_MALFORMED;
            if(out->width>256||out->height>256||out->width*out->height>PS_PNG_MAX_PIXELS)return PS_IMAGE_LIMIT;
            if(p[10]||p[11]||p[12])return PS_IMAGE_UNSUPPORTED;
            if(out->color==0||out->color==3){if(out->depth!=1&&out->depth!=2&&out->depth!=4&&out->depth!=8)return PS_IMAGE_UNSUPPORTED;out->channels=1;}
            else {if(out->depth!=8)return PS_IMAGE_UNSUPPORTED;out->channels=out->color==2?3:out->color==4?2:out->color==6?4:0;if(!out->channels)return PS_IMAGE_UNSUPPORTED;}
            out->row_bytes=(out->width*out->channels*out->depth+7)/8;
            out->filtered_bytes=(out->row_bytes+1)*out->height;if(out->filtered_bytes>PS_PNG_MAX_FILTERED_BYTES)return PS_IMAGE_LIMIT;
        }else if(!memcmp(tag,"PLTE",4)){
            if(plte||data||trns||!size||size%3||size>768||out->color==0||out->color==4)return PS_IMAGE_MALFORMED;
            plte=true;out->palette=p;out->palette_entries=(unsigned)size/3;
            if(out->color==3&&out->palette_entries>(1u<<out->depth))return PS_IMAGE_MALFORMED;
        }else if(!memcmp(tag,"tRNS",4)){
            if(trns||data)return PS_IMAGE_MALFORMED;
            if(out->color==3){if(!plte||!size||size>out->palette_entries)return PS_IMAGE_MALFORMED;}
            else if(out->color==0){if(size!=2||p[0]||p[1]>=(1u<<out->depth))return PS_IMAGE_MALFORMED;}
            else if(out->color==2){if(size!=6||p[0]||p[2]||p[4])return PS_IMAGE_MALFORMED;}
            else return PS_IMAGE_MALFORMED;
            trns=true;out->transparency=p;out->transparent_bytes=(unsigned)size;
        }else if(!memcmp(tag,"IDAT",4)){
            if(ended_data||(out->color==3&&!plte))return PS_IMAGE_MALFORMED;
            data=true;out->idat_bytes+=size;
        }else if(!memcmp(tag,"IEND",4)){
            if(size||!data||!out->idat_bytes||at+12!=n)return PS_IMAGE_MALFORMED;
            return PS_IMAGE_OK;
        }else{
            if(!(tag[0]&32))return PS_IMAGE_UNSUPPORTED;
            if(!memcmp(tag,"acTL",4)||!memcmp(tag,"fcTL",4)||!memcmp(tag,"fdAT",4))return PS_IMAGE_UNSUPPORTED;
            if(data)ended_data=true;
        }
        at+=size+12;
    }return PS_IMAGE_MALFORMED;
}
// Progressive scans are bounded and must follow valid successive approximation
// order. Entropy correctness still belongs to the separately bounded decoder.
static inline int PS_ProgressivePolicy(const uint8_t *b,size_t n,unsigned *width,unsigned *height){
    if(!b||!width||!height||n<4)return PS_IMAGE_MALFORMED;
    if(n>PS_NATIVE_IMAGE_WIRE)return PS_IMAGE_LIMIT;
    if(b[0]!=255||b[1]!=216)return PS_IMAGE_UNSUPPORTED;
    size_t at=2;unsigned markers=0,components=0,scans=0;uint8_t ids[3]={0},successive[3][64];memset(successive,255,sizeof(successive));
    while(at+2<=n&&++markers<=256){
        if(b[at++]!=255)return PS_IMAGE_MALFORMED;
        while(at<n&&b[at]==255)++at;
        if(at>=n)return PS_IMAGE_MALFORMED;
        unsigned tag=b[at++];
        if(tag==217){if(at!=n||!components||!scans)return PS_IMAGE_MALFORMED;for(unsigned i=0;i<components;++i)if(successive[i][0]==255)return PS_IMAGE_MALFORMED;return PS_IMAGE_OK;}
        if(at+2>n)return PS_IMAGE_MALFORMED;
        size_t size=(unsigned)b[at]*256+b[at+1];if(size<2||size>n-at)return PS_IMAGE_MALFORMED;
        if(tag==194){
            if(components||scans||size<8||b[at+2]!=8)return PS_IMAGE_UNSUPPORTED;
            *height=(unsigned)b[at+3]*256+b[at+4];*width=(unsigned)b[at+5]*256+b[at+6];components=b[at+7];
            if(!*width||!*height||(components!=1&&components!=3)||size!=8+3*components)return PS_IMAGE_MALFORMED;
            if(*width>128||*height>128||(*width)*(*height)>PS_PROGRESSIVE_MAX_PIXELS)return PS_IMAGE_LIMIT;
            unsigned blocks=0;for(unsigned i=0;i<components;++i){ids[i]=b[at+8+3*i];for(unsigned j=0;j<i;++j)if(ids[i]==ids[j])return PS_IMAGE_MALFORMED;unsigned sample=b[at+9+3*i],x=sample>>4,y=sample&15;if(!x||!y||x>2||y>2||b[at+10+3*i]>3)return PS_IMAGE_UNSUPPORTED;blocks+=x*y;}if(blocks>6)return PS_IMAGE_UNSUPPORTED;
        }else if(tag==219){
            size_t p=at+2;while(p<at+size){if(b[p]>3||at+size-p<65)return PS_IMAGE_UNSUPPORTED;p+=65;}
        }else if(tag==196){
            size_t p=at+2;while(p<at+size){if(at+size-p<17||(b[p]>>4)>1||(b[p]&15)>3)return PS_IMAGE_MALFORMED;unsigned symbols=0,space=1;for(unsigned i=1;i<=16;++i){space*=2;if(b[p+i]>space)return PS_IMAGE_MALFORMED;space-=b[p+i];symbols+=b[p+i];}if(!symbols||symbols>((b[p]>>4)?162:12)||symbols>at+size-p-17)return PS_IMAGE_MALFORMED;for(unsigned i=0;i<symbols;++i){unsigned v=b[p+17+i];if((b[p]>>4)?((v&15)>10):v>11)return PS_IMAGE_MALFORMED;}p+=17+symbols;}
        }else if(tag==221){if(size!=4)return PS_IMAGE_MALFORMED;}
        else if(tag==218){
            if(!components||++scans>32||size<6)return PS_IMAGE_MALFORMED;
            unsigned count=b[at+2],ss=b[at+size-3],se=b[at+size-2],ah=b[at+size-1]>>4,al=b[at+size-1]&15;
            if(!count||count>components||size!=6+2*count||ss>se||se>63||(ss==0&&se!=0)||(ss&&count!=1)||al>13||(ah&&ah!=al+1))return PS_IMAGE_MALFORMED;
            unsigned seen=0;
            for(unsigned i=0;i<count;++i){unsigned id=b[at+3+2*i],table=b[at+4+2*i],c=0;while(c<components&&ids[c]!=id)++c;if(c==components||seen&(1u<<c)||(table>>4)>3||(table&15)>3)return PS_IMAGE_MALFORMED;seen|=1u<<c;if(ss&&successive[c][0]==255)return PS_IMAGE_MALFORMED;for(unsigned k=ss;k<=se;++k){if(ah?successive[c][k]!=ah:successive[c][k]!=255)return PS_IMAGE_MALFORMED;successive[c][k]=(uint8_t)al;}}
            at+=size;
            while(at<n){if(b[at++]!=255)continue;size_t mark=at-1;while(at<n&&b[at]==255)++at;if(at==n)return PS_IMAGE_MALFORMED;unsigned value=b[at];if(value==0||(value>=208&&value<=215)){++at;continue;}at=mark;break;}
            continue;
        }else if(!((tag>=224&&tag<=239)||tag==254))return PS_IMAGE_UNSUPPORTED;
        at+=size;
    }return PS_IMAGE_MALFORMED;
}
