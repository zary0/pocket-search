// SPDX-License-Identifier: GPL-3.0-only
// Bounded RGB565 output using the existing ESP32 ROM JPEG decoder.
#include "pocket_native_image.h"
#include "pocket_jpeg_policy.h"
#include "rom/tjpgd.h"
#include "esp_timer.h"
#include <stdlib.h>
#include <string.h>
typedef struct {
    const uint8_t *bytes;size_t length,offset;
    PS_NativeImage image;unsigned source_width,source_height;
    PS_ImageCancelled cancelled;void *context;int64_t deadline;bool interrupted;
} Decoder;
static bool stop(Decoder *d){if((d->cancelled&&d->cancelled(d->context))||esp_timer_get_time()>=d->deadline){d->interrupted=true;return true;}return false;}
static UINT input(JDEC *jpeg,BYTE *buffer,UINT requested){
    Decoder *d=jpeg->device;if(stop(d))return 0;
    size_t n=requested;if(n>d->length-d->offset)n=d->length-d->offset;
    if(buffer)memcpy(buffer,d->bytes+d->offset,n);
    d->offset+=n;return (UINT)n;
}
static UINT output(JDEC *jpeg,void *pixels,JRECT *rect){
    Decoder *d=jpeg->device;if(stop(d))return 0;
    if(!pixels||rect->right<rect->left||rect->bottom<rect->top||rect->right>=d->source_width||rect->bottom>=d->source_height)return 0;
    const uint8_t *rgb=pixels;unsigned stride=rect->right-rect->left+1;
    unsigned y0=(rect->top*d->image.height+d->source_height-1)/d->source_height;
    unsigned y1=((rect->bottom+1)*d->image.height+d->source_height-1)/d->source_height;
    unsigned x0=(rect->left*d->image.width+d->source_width-1)/d->source_width;
    unsigned x1=((rect->right+1)*d->image.width+d->source_width-1)/d->source_width;
    for(unsigned y=y0;y<y1;++y){
        unsigned sy=y*d->source_height/d->image.height;if(sy<rect->top||sy>rect->bottom)continue;
        for(unsigned x=x0;x<x1;++x){
            unsigned sx=x*d->source_width/d->image.width;if(sx<rect->left||sx>rect->right)continue;
            const uint8_t *p=rgb+((sy-rect->top)*stride+sx-rect->left)*3;
            uint16_t value=(uint16_t)((p[0]>>3)<<11)|((uint16_t)(p[1]>>2)<<5)|(p[2]>>3);
            size_t at=((size_t)y*d->image.width+x)*2;d->image.pixels[at]=(uint8_t)value;d->image.pixels[at+1]=(uint8_t)(value>>8);
        }
    }return 1;
}
void PS_NativeImageFree(PS_NativeImage *image){if(image){free(image->pixels);memset(image,0,sizeof(*image));}}
int PS_NativeJPEG(const uint8_t *bytes,size_t length,PS_NativeImage *out,PS_ImageCancelled cancelled,PS_ImageMemory observe,void *context){
    if(!out||!bytes||length<4)return PS_IMAGE_MALFORMED;
    memset(out,0,sizeof(*out));
    if(length>PS_NATIVE_IMAGE_WIRE)return PS_IMAGE_LIMIT;
    if(bytes[0]!=0xff||bytes[1]!=0xd8)return PS_IMAGE_UNSUPPORTED;
    if(!PS_JPEGPolicy(bytes,length))return PS_IMAGE_UNSUPPORTED;
    Decoder d={.bytes=bytes,.length=length,.cancelled=cancelled,.context=context,.deadline=esp_timer_get_time()+5000000};
    if(stop(&d))return PS_IMAGE_CANCELLED;
    void *pool=malloc(4096);if(!pool)return PS_IMAGE_MEMORY;
    JDEC jpeg={0};JRESULT result=jd_prepare(&jpeg,input,pool,4096,&d);
    int error=PS_IMAGE_MALFORMED;
    if(result!=JDR_OK){if(result==JDR_FMT3||result==JDR_FMT2)error=PS_IMAGE_UNSUPPORTED;goto done;}
    if(!jpeg.width||!jpeg.height||jpeg.width>1024||jpeg.height>1024){error=PS_IMAGE_LIMIT;goto done;}
    d.source_width=jpeg.width;d.source_height=jpeg.height;
    unsigned w=jpeg.width,h=jpeg.height;
    if(w>PS_NATIVE_IMAGE_WIDTH){h=h*PS_NATIVE_IMAGE_WIDTH/w;w=PS_NATIVE_IMAGE_WIDTH;}
    if(h>PS_NATIVE_IMAGE_HEIGHT){w=w*PS_NATIVE_IMAGE_HEIGHT/h;h=PS_NATIVE_IMAGE_HEIGHT;}
    if(!w)w=1;
    if(!h)h=1;
    d.image.width=(uint16_t)w;d.image.height=(uint16_t)h;
    d.image.pixels=malloc((size_t)w*h*2);if(!d.image.pixels){error=PS_IMAGE_MEMORY;goto done;}
    memset(d.image.pixels,0xff,(size_t)w*h*2);if(observe)observe(context);
    result=jd_decomp(&jpeg,output,0);
    if(result==JDR_OK){*out=d.image;d.image.pixels=NULL;error=PS_IMAGE_OK;}
done:
    if(d.interrupted)error=PS_IMAGE_CANCELLED;
    PS_NativeImageFree(&d.image);free(pool);return error;
}
