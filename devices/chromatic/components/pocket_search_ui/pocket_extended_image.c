// SPDX-License-Identifier: GPL-3.0-only
// Bounded native PNG and progressive JPEG. One serialized image worker owns the
// allocator. Source bytes are never replaced with host-decoded pixels.
#include "pocket_native_image.h"
#include "pocket_image_policy.h"
#include "esp_timer.h"
#include <stdlib.h>
#include <stdatomic.h>
#include <stddef.h>
#include <string.h>
#ifdef PS_IMAGE_HOST
#include <zlib.h>
#else
#include "miniz.h"
#include "esp_heap_caps.h"
#endif
typedef struct {
    const uint8_t *bytes;size_t length,offset,allocated,peak;
    unsigned allocations;bool cancelled,memory;int64_t deadline;
    PS_ImageCancelled cancel;PS_ImageMemory observe;void *context;
} Budget;
typedef union {struct {size_t size;} value;max_align_t alignment;} Allocation;
static Budget *active;static atomic_bool decoding;
#ifdef PS_IMAGE_HOST
static unsigned fail_allocation,last_remaining;
void PS_ImageTestFailure(unsigned n){fail_allocation=n;}
unsigned PS_ImageTestRemaining(void){return last_remaining;}
#endif
static bool stopped(Budget *b){
    if((b->cancel&&b->cancel(b->context))||esp_timer_get_time()>=b->deadline)b->cancelled=true;
    return b->cancelled;
}
static bool available(Budget *b,size_t size){
    if(stopped(b))return false;
    if(size>PS_EXTENDED_IMAGE_ALLOCATIONS-b->allocated){b->memory=true;return false;}
#ifndef PS_IMAGE_HOST
    if(heap_caps_get_free_size(MALLOC_CAP_8BIT)<size+16384){b->memory=true;return false;}
#endif
    return true;
}
static void accounted(Budget *b,size_t size){
    b->allocated+=size;if(b->allocated>b->peak)b->peak=b->allocated;++b->allocations;
    if(b->observe)b->observe(b->context);
}
static void *bounded_malloc(size_t size){
    Budget *b=active;if(!b||size>PS_EXTENDED_IMAGE_ALLOCATIONS-sizeof(Allocation)||!available(b,size+sizeof(Allocation)))return NULL;
#ifdef PS_IMAGE_HOST
    if(fail_allocation&&b->allocations+1==fail_allocation){b->memory=true;return NULL;}
#endif
    Allocation *p=malloc(sizeof(*p)+size);if(!p){b->memory=true;return NULL;}p->value.size=size+sizeof(*p);accounted(b,p->value.size);return p+1;
}
static void bounded_free(void *ptr){if(ptr){Allocation *p=(Allocation*)ptr-1;active->allocated-=p->value.size;free(p);}}
static __attribute__((unused)) void *bounded_realloc(void *ptr,size_t size){
    void *next=bounded_malloc(size);if(next){if(ptr){Allocation *p=(Allocation*)ptr-1;size_t old=p->value.size-sizeof(*p);memcpy(next,ptr,old<size?old:size);}bounded_free(ptr);}return next;
}
#define STBI_ONLY_JPEG
#define STBI_NO_STDIO
#define STBI_NO_SIMD
#define STBI_NO_HDR
#define STBI_NO_LINEAR
#define STBI_NO_THREAD_LOCALS
#define STBI_MAX_DIMENSIONS 128
#define STBI_MALLOC bounded_malloc
#define STBI_REALLOC bounded_realloc
#define STBI_FREE bounded_free
#define STB_IMAGE_STATIC
#define STB_IMAGE_IMPLEMENTATION
// Upstream static API includes unused entry points in this private translation unit.
#if defined(__GNUC__)
#pragma GCC diagnostic push
#pragma GCC diagnostic ignored "-Wunused-function"
#pragma GCC diagnostic ignored "-Wunused-parameter"
#endif
#include "vendor/stb_image.h"
#if defined(__GNUC__)
#pragma GCC diagnostic pop
#endif
static int input_read(void *arg,char *out,int requested){
    Budget *b=arg;if(requested<0||stopped(b))return 0;size_t n=(size_t)requested;if(n>b->length-b->offset)n=b->length-b->offset;memcpy(out,b->bytes+b->offset,n);b->offset+=n;return (int)n;
}
static void input_skip(void *arg,int n){Budget *b=arg;if(stopped(b))return;if(n<0||(size_t)n>b->length-b->offset){b->offset=b->length;return;}b->offset+=(size_t)n;}
static int input_eof(void *arg){Budget *b=arg;return stopped(b)||b->offset==b->length;}
static int output_allocate(Budget *b,unsigned w,unsigned h,PS_NativeImage *out){
    unsigned width=w,height=h;if(width>160){height=height*160/width;width=160;}if(height>80){width=width*80/height;height=80;}if(!width)width=1;if(!height)height=1;
    size_t size=(size_t)width*height*2;if(!available(b,size))return b->cancelled?PS_IMAGE_CANCELLED:PS_IMAGE_MEMORY;
#ifdef PS_IMAGE_HOST
    if(fail_allocation&&b->allocations+1==fail_allocation){b->memory=true;return PS_IMAGE_MEMORY;}
#endif
    out->pixels=malloc(size);if(!out->pixels){b->memory=true;return PS_IMAGE_MEMORY;}accounted(b,size);out->width=(uint16_t)width;out->height=(uint16_t)height;return PS_IMAGE_OK;
}
static void pixel(PS_NativeImage *out,unsigned x,unsigned y,unsigned r,unsigned g,unsigned b,unsigned alpha){
    r=(r*alpha+255*(255-alpha)+127)/255;g=(g*alpha+255*(255-alpha)+127)/255;b=(b*alpha+255*(255-alpha)+127)/255;
    uint16_t v=(uint16_t)((r>>3)<<11)|((uint16_t)(g>>2)<<5)|(b>>3);size_t at=((size_t)y*out->width+x)*2;out->pixels[at]=(uint8_t)v;out->pixels[at+1]=(uint8_t)(v>>8);
}
static int progressive(Budget *b,PS_NativeImage *out){
    unsigned w=0,h=0;int error=PS_ProgressivePolicy(b->bytes,b->length,&w,&h);if(error)return error;
    stbi__jpeg *jpeg=bounded_malloc(sizeof(*jpeg));if(!jpeg)return b->cancelled?PS_IMAGE_CANCELLED:PS_IMAGE_MEMORY;
    memset(jpeg,0,sizeof(*jpeg));stbi__context input={0};stbi_io_callbacks callbacks={input_read,input_skip,input_eof};stbi__start_callbacks(&input,&callbacks,b);jpeg->s=&input;stbi__setup_jpeg(jpeg);
    if(!stbi__decode_jpeg_image(jpeg)){error=PS_IMAGE_MALFORMED;goto done;}
    if(stopped(b)){error=PS_IMAGE_CANCELLED;goto done;}
    if(!jpeg->progressive||input.img_x!=w||input.img_y!=h||(input.img_n!=1&&input.img_n!=3)){error=PS_IMAGE_MALFORMED;goto done;}
    // IDCT has finished: release all coefficient arrays before allocating output.
    // Read the decoded component planes directly; no full-size interleaved RGB copy.
    for(int c=0;c<input.img_n;++c){bounded_free(jpeg->img_comp[c].raw_coeff);jpeg->img_comp[c].raw_coeff=NULL;jpeg->img_comp[c].coeff=NULL;}
    error=output_allocate(b,w,h,out);if(error)goto done;
    bool rgb=input.img_n==3&&(jpeg->rgb==3||(jpeg->app14_color_transform==0&&!jpeg->jfif));
    for(unsigned y=0;y<out->height;++y){
        if(stopped(b)){error=PS_IMAGE_CANCELLED;goto done;}unsigned sy=y*h/out->height;
        for(unsigned x=0;x<out->width;++x){
            unsigned sx=x*w/out->width;uint8_t channels[3]={0},result[4]={0};
            for(int c=0;c<input.img_n;++c){unsigned cx=sx*jpeg->img_comp[c].h/jpeg->img_h_max,cy=sy*jpeg->img_comp[c].v/jpeg->img_v_max;if(cx>=(unsigned)jpeg->img_comp[c].w2||cy>=(unsigned)jpeg->img_comp[c].h2){error=PS_IMAGE_MALFORMED;goto done;}channels[c]=jpeg->img_comp[c].data[(size_t)cy*jpeg->img_comp[c].w2+cx];}
            if(input.img_n==1)pixel(out,x,y,channels[0],channels[0],channels[0],255);
            else {if(rgb)memcpy(result,channels,3);else jpeg->YCbCr_to_RGB_kernel(result,channels,channels+1,channels+2,1,4);pixel(out,x,y,result[0],result[1],result[2],255);}
        }
    }
done:
    stbi__cleanup_jpeg(jpeg);bounded_free(jpeg);return b->cancelled?PS_IMAGE_CANCELLED:b->memory?PS_IMAGE_MEMORY:error;
}
static int paeth(int a,int b,int c){int p=a+b-c,pa=abs(p-a),pb=abs(p-b),pc=abs(p-c);return pa<=pb&&pa<=pc?a:pb<=pc?b:c;}
static int png_inflate(Budget *b,const PS_PNGInfo *info,uint8_t *raw){
    size_t at=8,total=0,consumed=0;int error=PS_IMAGE_MALFORMED;
#ifdef PS_IMAGE_HOST
    z_stream stream={0};if(inflateInit(&stream)!=Z_OK)return PS_IMAGE_MEMORY;
#else
    tinfl_decompressor *stream=bounded_malloc(sizeof(*stream));if(!stream)return b->cancelled?PS_IMAGE_CANCELLED:PS_IMAGE_MEMORY;tinfl_init(stream);
#endif
    while(at+12<=b->length){
        size_t size=PS_ImageBE32(b->bytes+at);const uint8_t *p=b->bytes+at+8;
        if(!memcmp(b->bytes+at+4,"IDAT",4)&&size){
            size_t offset=0;
            while(offset<size){
                if(stopped(b)){error=PS_IMAGE_CANCELLED;goto done;}size_t input=size-offset,output=info->filtered_bytes-total;
#ifdef PS_IMAGE_HOST
                stream.next_in=(Bytef*)(p+offset);stream.avail_in=(uInt)input;stream.next_out=raw+total;stream.avail_out=(uInt)output;int status=inflate(&stream,Z_NO_FLUSH);input-=stream.avail_in;output-=stream.avail_out;
                bool complete=status==Z_STREAM_END;bool bad=status!=Z_OK&&status!=Z_STREAM_END;
#else
                unsigned flags=TINFL_FLAG_PARSE_ZLIB_HEADER|TINFL_FLAG_USING_NON_WRAPPING_OUTPUT_BUF;if(consumed+input<info->idat_bytes)flags|=TINFL_FLAG_HAS_MORE_INPUT;
                tinfl_status status=tinfl_decompress(stream,p+offset,&input,raw,raw+total,&output,flags);
                bool complete=status==TINFL_STATUS_DONE;bool bad=status<0;
#endif
                offset+=input;consumed+=input;total+=output;
                if(complete){error=total==info->filtered_bytes&&consumed==info->idat_bytes?PS_IMAGE_OK:PS_IMAGE_MALFORMED;goto done;}
                if(bad||(!input&&!output)||total>info->filtered_bytes){error=PS_IMAGE_MALFORMED;goto done;}
            }
        }
        at+=size+12;
    }
done:
#ifdef PS_IMAGE_HOST
    inflateEnd(&stream);
#else
    bounded_free(stream);
#endif
    return error;
}
static int png(Budget *b,PS_NativeImage *out){
    PS_PNGInfo info;int error=PS_PNGPolicy(b->bytes,b->length,&info);if(error)return error;
    uint8_t *raw=bounded_malloc(info.filtered_bytes);if(!raw)return b->cancelled?PS_IMAGE_CANCELLED:PS_IMAGE_MEMORY;
    error=png_inflate(b,&info,raw);if(error)goto done;
    unsigned stride=info.row_bytes+1,bpp=(info.channels*info.depth+7)/8;
    for(unsigned y=0;y<info.height;++y){
        if(stopped(b)){error=PS_IMAGE_CANCELLED;goto done;}
        uint8_t *row=raw+(size_t)y*stride+1,*prev=y?row-stride:NULL;unsigned filter=row[-1];if(filter>4){error=PS_IMAGE_MALFORMED;goto done;}
        for(unsigned x=0;x<info.row_bytes;++x){int a=x>=bpp?row[x-bpp]:0,p=prev?prev[x]:0,c=prev&&x>=bpp?prev[x-bpp]:0;row[x]=(uint8_t)(row[x]+(filter==1?a:filter==2?p:filter==3?(a+p)/2:filter==4?paeth(a,p,c):0));}
        if(info.color==3){for(unsigned x=0;x<info.width;++x){unsigned shift=8-info.depth-(x*info.depth%8),index=(row[x*info.depth/8]>>shift)&((1u<<info.depth)-1);if(index>=info.palette_entries){error=PS_IMAGE_MALFORMED;goto done;}}}
    }
    error=output_allocate(b,info.width,info.height,out);if(error)goto done;
    for(unsigned y=0;y<out->height;++y){
        if(stopped(b)){error=PS_IMAGE_CANCELLED;goto done;}const uint8_t *row=raw+(size_t)(y*info.height/out->height)*stride+1;
        for(unsigned x=0;x<out->width;++x){
            unsigned sx=x*info.width/out->width,r=0,g=0,blue=0,a=255;const uint8_t *p=row+sx*info.channels;
            if(info.color==0||info.color==3){unsigned shift=8-info.depth-(sx*info.depth%8),v=(row[sx*info.depth/8]>>shift)&((1u<<info.depth)-1);if(info.color==3){const uint8_t *q=info.palette+v*3;r=q[0];g=q[1];blue=q[2];if(v<info.transparent_bytes)a=info.transparency[v];}else{r=g=blue=v*255/((1u<<info.depth)-1);if(info.transparency&&v==info.transparency[1])a=0;}}
            else if(info.color==4){r=g=blue=p[0];a=p[1];}
            else {r=p[0];g=p[1];blue=p[2];if(info.color==6)a=p[3];else if(info.transparency&&r==info.transparency[1]&&g==info.transparency[3]&&blue==info.transparency[5])a=0;}
            pixel(out,x,y,r,g,blue,a);
        }
    }
done:
    bounded_free(raw);return error;
}
int PS_ExtendedImage(const uint8_t *bytes,size_t length,PS_NativeImage *out,PS_ImageCancelled cancel,PS_ImageMemory observe,void *context){
    if(!out)return PS_IMAGE_MALFORMED;
    memset(out,0,sizeof(*out));if(!bytes||length<4)return PS_IMAGE_MALFORMED;if(length>PS_NATIVE_IMAGE_WIRE)return PS_IMAGE_LIMIT;
    if(atomic_exchange(&decoding,true))return PS_IMAGE_MEMORY;
    Budget budget={.bytes=bytes,.length=length,.cancel=cancel,.observe=observe,.context=context,.deadline=esp_timer_get_time()+5000000};active=&budget;
    out->format=length>=8&&!memcmp(bytes,"\x89PNG\r\n\x1a\n",8)?PS_FORMAT_PNG:PS_FORMAT_PROGRESSIVE;
    int error=out->format==PS_FORMAT_PNG?png(&budget,out):progressive(&budget,out);
    if(stopped(&budget))error=PS_IMAGE_CANCELLED;
    if(error&&out->pixels){budget.allocated-=(size_t)out->width*out->height*2;free(out->pixels);out->pixels=NULL;out->width=out->height=0;}
    out->decoder_peak_bytes=(unsigned)budget.peak;out->decoder_allocations=budget.allocations;
#ifdef PS_IMAGE_HOST
    last_remaining=(unsigned)(budget.allocated-(out->pixels?(size_t)out->width*out->height*2:0));
#endif
    active=NULL;atomic_store(&decoding,false);return error;
}
