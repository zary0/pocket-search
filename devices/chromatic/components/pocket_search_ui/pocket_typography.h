// SPDX-License-Identifier: GPL-3.0-only
#pragma once
#include "lvgl.h"
#include "pocket_search_net.h"
void PS_TypographyDraw(lv_event_t *event,const uint8_t *packet,const PS_RichView *view);

bool PS_WebFontReady(void);
int PS_WebFontLoad(const uint8_t *bytes,size_t length);
