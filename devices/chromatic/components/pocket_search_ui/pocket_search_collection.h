// SPDX-License-Identifier: GPL-3.0-only
#pragma once
#include <stdbool.h>
#include <stddef.h>
#define PS_GENERATION_BYTES 65
static inline bool ps_generation_valid(const char *value) {
    if(!value || !value[0]) return false;
    for(size_t i=0;i<PS_GENERATION_BYTES;++i) {
        unsigned char c=(unsigned char)value[i];
        if(!c) return true;
        if(!((c>='a'&&c<='z')||(c>='A'&&c<='Z')||(c>='0'&&c<='9')||c=='-')) return false;
    }
    return false;
}
