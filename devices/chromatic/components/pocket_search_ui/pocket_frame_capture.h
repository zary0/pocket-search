// SPDX-License-Identifier: GPL-3.0-only
// Development readback: one complete frame in three separately allocated bands.
#pragma once
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
enum {PS_FRAME_WIDTH=160,PS_FRAME_HEIGHT=144,PS_FRAME_BAND_ROWS=48,PS_FRAME_BANDS=3,PS_FRAME_STRIDE=320};
typedef struct {uint8_t *bands[PS_FRAME_BANDS];} PS_Frame;
static inline void PS_FrameFree(PS_Frame *frame){
    for(unsigned i=0;i<PS_FRAME_BANDS;++i){free(frame->bands[i]);frame->bands[i]=NULL;}
}
// Call with zeroed, exclusively owned frame; free only after GUI relinquishes it.
static inline bool PS_FrameInit(PS_Frame *frame){
    for(unsigned i=0;i<PS_FRAME_BANDS;++i){frame->bands[i]=malloc(PS_FRAME_BAND_ROWS*PS_FRAME_STRIDE);if(!frame->bands[i]){PS_FrameFree(frame);return false;}}
    return true;
}
static inline uint8_t *PS_FrameRow(PS_Frame *frame,unsigned y){
    if(y>=PS_FRAME_HEIGHT)return NULL;
    uint8_t *band=frame->bands[y/PS_FRAME_BAND_ROWS];
    return band?band+(y%PS_FRAME_BAND_ROWS)*PS_FRAME_STRIDE:NULL;
}
// Bounds are checked before any copy; full-width RGB565 rows only.
static inline bool PS_FrameWrite(PS_Frame *frame,unsigned y,unsigned rows,const uint8_t *pixels){
    if(!pixels||y>PS_FRAME_HEIGHT||rows>PS_FRAME_HEIGHT-y)return false;
    for(unsigned row=0;row<rows;++row)if(!PS_FrameRow(frame,y+row))return false;
    for(unsigned row=0;row<rows;++row)memcpy(PS_FrameRow(frame,y+row),pixels+row*PS_FRAME_STRIDE,PS_FRAME_STRIDE);
    return true;
}
