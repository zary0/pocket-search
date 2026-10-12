// SPDX-License-Identifier: GPL-3.0-only
#include "../devices/chromatic/components/pocket_search_ui/pocket_native_image.h"
#include "../devices/chromatic/components/pocket_search_ui/pocket_image_policy.h"
#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <zlib.h>
void PS_ImageTestFailure(unsigned);
unsigned PS_ImageTestRemaining(void);
static unsigned calls,cancel_at;static int64_t time_us,time_step;
int64_t esp_timer_get_time(void){return time_us;}
static bool cancelled(void *context){(void)context;++calls;time_us+=time_step;return cancel_at&&calls>=cancel_at;}
static unsigned color(const PS_NativeImage *image,unsigned x,unsigned y){size_t at=((size_t)y*image->width+x)*2;return image->pixels[at]|(unsigned)image->pixels[at+1]<<8;}
static void reject(uint8_t *bytes,size_t length,int expected){PS_NativeImage image;calls=cancel_at=0;time_us=time_step=0;int error=PS_ExtendedImage(bytes,length,&image,cancelled,NULL,NULL);assert(error==expected&&!image.pixels&&PS_ImageTestRemaining()==0);}
static void be32(uint8_t *b,uint32_t v){for(unsigned i=0;i<4;++i)b[i]=(uint8_t)(v>>(24-8*i));}
static size_t chunk(uint8_t *b,const char *type,const uint8_t *bytes,size_t length){be32(b,(uint32_t)length);memcpy(b+4,type,4);if(length)memcpy(b+8,bytes,length);be32(b+8+length,PS_PNGCRC(b+4,4+length));return length+12;}
static size_t small_png(uint8_t b[256],uint8_t filter,size_t raw_length,bool bad_adler){
    uint8_t header[13]={0,0,0,1,0,0,0,1,8,2,0,0,0},raw[5]={filter,255,0,0,99},compressed[32];uLongf n=sizeof(compressed);assert(compress(compressed,&n,raw,raw_length)==Z_OK);if(bad_adler)compressed[n-1]^=1;
    memcpy(b,"\x89PNG\r\n\x1a\n",8);size_t at=8;at+=chunk(b+at,"IHDR",header,13);at+=chunk(b+at,"IDAT",compressed,n);at+=chunk(b+at,"IEND",NULL,0);return at;
}
static void invalid_png(void){
    uint8_t b[256];size_t n=small_png(b,0,4,false);PS_NativeImage image;assert(!PS_ExtendedImage(b,n,&image,NULL,NULL,NULL));assert(color(&image,0,0)==0xf800);free(image.pixels);
    b[29]^=1;reject(b,n,PS_IMAGE_MALFORMED);
    n=small_png(b,5,4,false);reject(b,n,PS_IMAGE_MALFORMED);
    n=small_png(b,0,4,true);reject(b,n,PS_IMAGE_MALFORMED);
    n=small_png(b,0,5,false);reject(b,n,PS_IMAGE_MALFORMED); // inflated bytes exceed exact geometry
    n=small_png(b,0,3,false);reject(b,n,PS_IMAGE_MALFORMED);
    n=small_png(b,0,4,false);b[24]=16;be32(b+29,PS_PNGCRC(b+12,17));reject(b,n,PS_IMAGE_UNSUPPORTED);
    n=small_png(b,0,4,false);b[28]=1;be32(b+29,PS_PNGCRC(b+12,17));reject(b,n,PS_IMAGE_UNSUPPORTED);
    n=small_png(b,0,4,false);be32(b+16,257);be32(b+29,PS_PNGCRC(b+12,17));reject(b,n,PS_IMAGE_LIMIT);
    n=small_png(b,0,4,false);b[n++]=0;reject(b,n,PS_IMAGE_MALFORMED);
    n=small_png(b,0,4,false);memmove(b+45,b+33,n-33);chunk(b+33,"acTL",NULL,0);reject(b,n+12,PS_IMAGE_UNSUPPORTED);
    n=small_png(b,0,4,false);memmove(b+45,b+33,n-33);chunk(b+33,"WHAT",NULL,0);reject(b,n+12,PS_IMAGE_UNSUPPORTED);
}
static uint8_t *load(const char *name,size_t *n){FILE *f=fopen(name,"rb");assert(f);assert(!fseek(f,0,SEEK_END));long length=ftell(f);assert(length>0&&length<24577);*n=(size_t)length;rewind(f);uint8_t *b=malloc(*n);assert(b&&fread(b,1,*n,f)==*n);fclose(f);return b;}
static void check(const char *name,unsigned format){
    size_t n;uint8_t *b=load(name,&n);PS_NativeImage image;
    calls=cancel_at=0;time_us=0;PS_ImageTestFailure(0);
    int error=PS_ExtendedImage(b,n,&image,cancelled,NULL,NULL);
    if(error){fprintf(stderr,"decode %s error=%d peak=%u alloc=%u\n",name,error,image.decoder_peak_bytes,image.decoder_allocations);abort();}
    assert(image.pixels&&image.width&&image.height&&image.width<=160&&image.height<=80&&image.format==format);
    assert(image.decoder_peak_bytes<=65536&&PS_ImageTestRemaining()==0);
    if(strstr(name,"bars.png")){const unsigned colors[]={0xf800,0x7fef,0xffff,0xfff7,0x07ff,0};assert(image.width==160&&image.height==26);for(unsigned y=0;y<image.height;++y)for(unsigned x=0;x<image.width;++x)assert(color(&image,x,y)==colors[(x*192/image.width)/32]);}
    if(strstr(name,"palette.png")){assert(image.width==17&&image.height==9);for(unsigned y=0;y<9;++y)for(unsigned x=0;x<17;++x)assert(color(&image,x,y)==(x%2?0xffff:0xf800));}
    unsigned allocations=image.decoder_allocations,stop_calls=calls;
    printf("DECODE %s %ux%u peak=%u allocations=%u\n",name,image.width,image.height,image.decoder_peak_bytes,allocations);free(image.pixels);
    for(unsigned i=1;i<=allocations;++i){calls=cancel_at=0;PS_ImageTestFailure(i);error=PS_ExtendedImage(b,n,&image,cancelled,NULL,NULL);assert(error==PS_IMAGE_MEMORY&&!image.pixels&&PS_ImageTestRemaining()==0);}
    PS_ImageTestFailure(0);
    for(unsigned i=1;i<=stop_calls;++i){calls=0;cancel_at=i;error=PS_ExtendedImage(b,n,&image,cancelled,NULL,NULL);assert(error==PS_IMAGE_CANCELLED&&!image.pixels&&PS_ImageTestRemaining()==0);}
    cancel_at=0;
    for(size_t i=0;i<n;++i){calls=0;error=PS_ExtendedImage(b,i,&image,cancelled,NULL,NULL);assert(error!=PS_IMAGE_OK&&!image.pixels&&PS_ImageTestRemaining()==0);}
    time_us=0;time_step=1000000;error=PS_ExtendedImage(b,n,&image,cancelled,NULL,NULL);assert(error==PS_IMAGE_CANCELLED&&!image.pixels&&PS_ImageTestRemaining()==0);time_step=time_us=0;
    // Deterministic malformed entropy/chunk mutations remain inside all bounds.
    uint32_t seed=7;for(unsigned i=0;i<400;++i){seed=seed*1664525u+1013904223u;size_t at=seed%n;uint8_t old=b[at];b[at]^=(uint8_t)(1u<<(i%8));calls=0;error=PS_ExtendedImage(b,n,&image,cancelled,NULL,NULL);assert(PS_ImageTestRemaining()==0&&image.decoder_peak_bytes<=65536);if(!error){assert(image.pixels);free(image.pixels);}else assert(!image.pixels);b[at]=old;}
    free(b);
}
int main(int argc,char **argv){assert(argc>=3);invalid_png();check(argv[1],PS_FORMAT_PNG);check(argv[2],PS_FORMAT_PROGRESSIVE);for(int i=3;i<argc;++i)check(argv[i],strstr(argv[i],".png")?PS_FORMAT_PNG:PS_FORMAT_PROGRESSIVE);return 0;}
