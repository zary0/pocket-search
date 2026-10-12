/* Pocket JP bitmap, derived from Noto Sans JP.
 * Copyright 2014-2021 Adobe (http://www.adobe.com/), Reserved Font Name 'Source'.
 * SPDX-License-Identifier: OFL-1.1
 * See POCKET-FONT-LICENSE.txt. Modified font name: Pocket JP.
 */
#include "lvgl.h"
#include <stdint.h>
#include <stdbool.h>
#include "pocket_jp_data.h"
static unsigned pop(uint8_t b){b-=((b>>1)&0x55);b=(b&0x33)+((b>>2)&0x33);return (b+(b>>4))&15;}
static int find(uint32_t c){
    if(c>65535||!(presence[c/8]&(1u<<(c%8))))return -1;
    unsigned page=c/256,rank=ranks[page],at=page*32;
    for(;at<c/8;++at)rank+=pop(presence[at]);
    rank+=pop(presence[at]&((1u<<(c%8))-1));return (int)rank;
}
static bool describe(const lv_font_t *font,lv_font_glyph_dsc_t *d,uint32_t c,uint32_t next){
    (void)font;(void)next;if(c<32)return false;if(find(c)<0)c='?';
    d->adv_w=d->box_w=c<127?6:12;d->box_h=12;d->ofs_x=d->ofs_y=0;d->bpp=1;d->is_placeholder=0;return true;
}
// LVGL GUI thread only. Decode at most eight 18-byte glyphs, no heap.
static const uint8_t *bitmap(const lv_font_t *font,uint32_t c){
    (void)font;static int cached=-1;static uint8_t glyph[18];
    int index=find(c);if(index<0)index=find('?');if(cached==index)return glyph;
    unsigned bit=checkpoints[(unsigned)index/8];
    for(unsigned g=(unsigned)index&~7u;g<=(unsigned)index;++g){
        for(unsigned b=0;b<18;++b){int node=0;for(unsigned depth=0;node>=0&&depth<13;++depth){unsigned value=(pixels[bit/8]>>(7-bit%8))&1;bit++;node=nodes[node*2+value];}glyph[b]=(uint8_t)(-node-1);}
    }
    cached=index;return glyph;
}
const lv_font_t pocket_jp = {.get_glyph_dsc=describe,.get_glyph_bitmap=bitmap,.line_height=14,.base_line=2,.subpx=LV_FONT_SUBPX_NONE};
