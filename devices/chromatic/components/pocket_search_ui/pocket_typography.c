// SPDX-License-Identifier: GPL-3.0-only
// Fixed-cell typography: built-in OFL font or a locally rasterized ASCII atlas.
#include "pocket_typography.h"
#include "pocket_inline_image.h"
#include <string.h>
#include <stdint.h>
#include <stdatomic.h>
LV_FONT_DECLARE(pocket_jp);
static uint8_t web_atlas[867];
static atomic_bool web_ready=false;
static uint8_t glyphs[16][36]; /* one bounded scratch glyph per style; GUI-thread only */
static uint32_t cached_codes[16];static bool cached[16],cached_web[16];
static unsigned size(unsigned style){const unsigned sizes[]={12,16,14,10};return sizes[style&3];}
static unsigned width(uint32_t c,unsigned style){return size(style)/(c<128?2:1)+!!(style&4);}
static bool describe(const lv_font_t *font,lv_font_glyph_dsc_t *d,uint32_t c,uint32_t next){
    (void)next;unsigned style=(uintptr_t)font->dsc;
    if(c<32)return false;
    d->adv_w=d->box_w=width(c,style);d->box_h=size(style);d->ofs_x=d->ofs_y=0;d->bpp=1;d->is_placeholder=0;return true;
}
static bool ink(const uint8_t *source,unsigned sw,unsigned sh,unsigned x,unsigned y,unsigned tw,unsigned th){
    if(x>=tw)return false;
    unsigned bit=(y*sh/th)*sw+x*sw/tw;return (source[bit/8]>>(7-bit%8))&1;
}
static const uint8_t *bitmap(const lv_font_t *font,uint32_t c){
    unsigned style=(uintptr_t)font->dsc,w=width(c,style),h=size(style),plain=w-!!(style&4);
    bool web=(style&8)&&PS_WebFontReady()&&c>=32&&c<=126&&(web_atlas[855+(c-32)/8]&(1<<((c-32)%8)));
    if(cached[style]&&cached_codes[style]==c&&cached_web[style]==web)return glyphs[style];
    lv_font_glyph_dsc_t base={0};
    const uint8_t *source=NULL;
    if(web){
        base.box_w=6;base.box_h=12;source=web_atlas+(c-32)*9;
    }else{
    if(!lv_font_get_glyph_dsc(&pocket_jp,&base,c,0))return NULL;
    source=lv_font_get_glyph_bitmap(&pocket_jp,c);
    }
    if(!source)return NULL;
    if(!(style&4)&&base.box_w==w&&base.box_h==h)return source;
    uint8_t *out=glyphs[style];memset(out,0,sizeof(glyphs[0]));
    for(unsigned y=0;y<h;++y)for(unsigned x=0;x<w;++x){
        bool on=ink(source,base.box_w,base.box_h,x,y,plain,h);
        if((style&4)&&x)on|=ink(source,base.box_w,base.box_h,x-1,y,plain,h);
        if(on){unsigned bit=y*w+x;out[bit/8]|=1<<(7-bit%8);}
    }
    cached_codes[style]=c;cached[style]=true;cached_web[style]=web;return out;
}
#define FONT(s,h) {.get_glyph_dsc=describe,.get_glyph_bitmap=bitmap,.line_height=h,.base_line=2,.subpx=LV_FONT_SUBPX_NONE,.dsc=(const void*)(uintptr_t)s}
static const lv_font_t fonts[16]={FONT(0,14),FONT(1,18),FONT(2,16),FONT(3,12),FONT(4,14),FONT(5,18),FONT(6,16),FONT(7,12),FONT(8,14),FONT(9,18),FONT(10,16),FONT(11,12),FONT(12,14),FONT(13,18),FONT(14,16),FONT(15,12)};

// One immutable face per boot. Acquire/release publication avoids GUI/worker races.
bool PS_WebFontReady(void){return atomic_load_explicit(&web_ready,memory_order_acquire);}
int PS_WebFontLoad(const uint8_t *bytes,size_t length){
    if(PS_WebFontReady())return -1;
    int n=ps_webfont(bytes,length,web_atlas);
    if(n>0)atomic_store_explicit(&web_ready,true,memory_order_release);
    return n;
}
void PS_TypographyDraw(lv_event_t *event,const uint8_t *packet,const PS_RichView *view){
    if(lv_event_get_code(event)!=LV_EVENT_DRAW_MAIN)return;
    lv_obj_t *object=lv_event_get_target(event);lv_area_t area;lv_obj_get_coords(object,&area);
    lv_draw_ctx_t *ctx=lv_event_get_draw_ctx(event);lv_draw_label_dsc_t dsc;lv_draw_label_dsc_init(&dsc);
    for(unsigned row=0;row<view->row_count;++row){
        const PS_RichRow *r=&view->rows[row];unsigned x=0,glyph=0,at=0;
        const uint8_t *text=packet+r->text.offset,*styles=packet+r->styles.offset;
        while(at<r->text.length){
            unsigned style=(styles[glyph/2]>>((glyph%2)*4))&15;uint32_t c=text[at++];
            unsigned remaining=c<128?0:(c&0xe0)==0xc0?1:(c&0xf0)==0xe0?2:3;
            if(remaining){c&=(1u<<(6-remaining))-1;while(remaining--)c=(c<<6)|(text[at++]&63);}
            dsc.font=&fonts[style];dsc.color=lv_color_hex((style&3)==3?0x5f6368:0x202124);
            lv_point_t p={area.x1+(lv_coord_t)x,area.y1+r->y+r->height-dsc.font->line_height};
            lv_draw_letter(ctx,&dsc,&p,c);x+=width(c,style);++glyph;
        }
    }
}
static lv_color_t rgb565(uint16_t value){return lv_color_make(((value>>11)&31)*255/31,((value>>5)&63)*255/63,(value&31)*255/31);}
static void rect(lv_draw_ctx_t *ctx,lv_coord_t x,lv_coord_t y,unsigned w,unsigned h,lv_color_t color){
    if(!w||!h)return;
    lv_draw_rect_dsc_t d;lv_draw_rect_dsc_init(&d);d.bg_color=color;d.bg_opa=LV_OPA_COVER;d.border_width=0;d.radius=0;
    lv_area_t a={x,y,x+(lv_coord_t)w-1,y+(lv_coord_t)h-1};lv_draw_rect(ctx,&d,&a);
}
void PS_FlowDraw(lv_event_t *event,const uint8_t *packet,const PS_FlowView *view,const lv_img_dsc_t *image){
    if(lv_event_get_code(event)!=LV_EVENT_DRAW_MAIN)return;
    lv_obj_t *object=lv_event_get_target(event);lv_area_t area;lv_obj_get_coords(object,&area);
    lv_draw_ctx_t *ctx=lv_event_get_draw_ctx(event);const uint8_t *palette=packet+view->palette.offset;
    rect(ctx,area.x1,area.y1,160,92,rgb565((uint16_t)palette[2]|((uint16_t)palette[3]<<8)));
    lv_draw_label_dsc_t dsc;lv_draw_label_dsc_init(&dsc);
    bool first_image=true;
    for(unsigned row=0;row<view->row_count;++row){
        const PS_FlowRow *r=&view->rows[row];lv_coord_t bx=area.x1+r->box_x,by=area.y1+r->y;
        if(r->flags&1)rect(ctx,bx,by,r->box_width,r->height,rgb565(r->background));
        if(r->stroke){
            lv_color_t color=rgb565(r->border);rect(ctx,bx,by,r->stroke,r->height,color);rect(ctx,bx+r->box_width-r->stroke,by,r->stroke,r->height,color);
            if(r->edges&1)rect(ctx,bx,by,r->box_width,r->stroke,color);
            if(r->edges&2)rect(ctx,bx,by+r->height-r->stroke,r->box_width,r->stroke,color);
        }
        if((r->flags&4)&&first_image){
            first_image=false;
            if(image){
                unsigned bottom=(r->edges&2)?r->stroke:0;
                PS_InlineFit fit=PS_InlineFitBox(image->header.w,image->header.h,r->width,r->height-r->text_y-bottom);
                if(fit.width&&fit.height){
                    lv_coord_t x=area.x1+r->x+(r->width-fit.width)/2,y=by+r->text_y+(r->height-r->text_y-bottom-fit.height)/2;
                    lv_draw_img_dsc_t d;lv_draw_img_dsc_init(&d);d.zoom=fit.zoom;d.pivot=(lv_point_t){0,0};d.antialias=0;
                    lv_area_t a={x,y,x+(lv_coord_t)image->header.w-1,y+(lv_coord_t)image->header.h-1};
                    lv_area_t box={area.x1+r->x,by+r->text_y,area.x1+r->x+r->width-1,by+r->height-bottom-1},clip;
                    const lv_area_t *previous=ctx->clip_area;
                    if(_lv_area_intersect(&clip,previous,&box)){ctx->clip_area=&clip;lv_draw_img(ctx,&d,&a,image);ctx->clip_area=previous;}
                    continue;
                }
            }
        }
        unsigned x=r->x,glyph=0,at=0;const uint8_t *text=packet+r->text.offset,*codes=packet+r->codes.offset;
        while(at<r->text.length){
            unsigned code=codes[glyph],style=code&15;const uint8_t *paint=palette+(code>>4)*5;uint32_t c=text[at++];
            unsigned remaining=c<128?0:(c&0xe0)==0xc0?1:(c&0xf0)==0xe0?2:3;
            if(remaining){c&=(1u<<(6-remaining))-1;while(remaining--)c=(c<<6)|(text[at++]&63);}
            unsigned advance=width(c,style);lv_coord_t gx=area.x1+(lv_coord_t)x,gy=by+r->text_y;
            if(paint[4]&1)rect(ctx,gx,gy,advance,size(style)+2,rgb565((uint16_t)paint[2]|((uint16_t)paint[3]<<8)));
            dsc.font=&fonts[style];dsc.color=rgb565((uint16_t)paint[0]|((uint16_t)paint[1]<<8));
            lv_point_t p={gx,gy};lv_draw_letter(ctx,&dsc,&p,c);
            if(paint[4]&2)rect(ctx,gx,gy+size(style)+1,advance,1,dsc.color);
            x+=advance;++glyph;
        }
    }
}
