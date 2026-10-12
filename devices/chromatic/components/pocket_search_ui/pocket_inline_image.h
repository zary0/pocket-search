// SPDX-License-Identifier: GPL-3.0-only
#pragma once
#include <stdbool.h>
#include <stdint.h>
// One image belongs to a semantic slot, not a scroll offset. A failed/cancelled
// slot requires explicit retry; adjacent viewports must not create a fetch loop.
enum { PS_INLINE_EMPTY, PS_INLINE_LOADING, PS_INLINE_READY, PS_INLINE_FAILED };
typedef struct { uint32_t document; uint8_t slot,stage; int error; } PS_InlineImage;
static inline bool PS_InlineMatches(const PS_InlineImage *s,uint32_t document,unsigned slot){
    return document&&slot&&s->document==document&&s->slot==slot;
}
static inline bool PS_InlineNeedsLoad(const PS_InlineImage *s,uint32_t document,unsigned slot){
    return document&&slot&&slot<=24&&(!PS_InlineMatches(s,document,slot)||s->stage==PS_INLINE_EMPTY);
}
static inline void PS_InlineBegin(PS_InlineImage *s,uint32_t document,unsigned slot){
    *s=(PS_InlineImage){.document=document,.slot=(uint8_t)slot,.stage=PS_INLINE_LOADING};
}
static inline void PS_InlineFinish(PS_InlineImage *s,int error){
    s->stage=error?PS_INLINE_FAILED:PS_INLINE_READY;s->error=error;
}
static inline void PS_InlineInterrupt(PS_InlineImage *s){
    if(s->stage==PS_INLINE_LOADING)PS_InlineFinish(s,9);
}
// Fit inside the retained box, preserving aspect ratio and avoiding upscaling.
typedef struct { unsigned width,height,zoom; } PS_InlineFit;
static inline PS_InlineFit PS_InlineFitBox(unsigned width,unsigned height,unsigned box_width,unsigned box_height){
    if(!width||!height||!box_width||!box_height||width>160||height>80||box_width>160||box_height>92)return (PS_InlineFit){0};
    unsigned zoom=256;
    if(width>box_width)zoom=box_width*256/width;
    if(height*zoom/256>box_height)zoom=box_height*256/height;
    if(!zoom)return (PS_InlineFit){0};
    return (PS_InlineFit){width*zoom/256,height*zoom/256,zoom};
}
