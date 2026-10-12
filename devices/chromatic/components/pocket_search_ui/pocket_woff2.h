// SPDX-License-Identifier: GPL-3.0-only
#pragma once
#include <stddef.h>
#include <stdint.h>
#include <stdbool.h>
#ifdef __cplusplus
extern "C" {
#endif
enum { PS_FONT_WIRE=24576, PS_FONT_SFNT=24576, PS_FONT_PRIVATE=65536 };
enum { PS_FONT_OK=0, PS_FONT_MEMORY=3, PS_FONT_LIMIT=6, PS_FONT_UNSUPPORTED=8, PS_FONT_CANCELLED=9, PS_FONT_INVALID=10, PS_FONT_BUSY=15 };
typedef struct { bool (*cancel)(void *); uint64_t (*clock_ms)(void *); size_t (*free_heap)(void *); void *context; unsigned peak_bytes,allocations; } PS_FontBudget;
int PS_Woff2Decode(const uint8_t *,size_t,uint8_t **,size_t *,PS_FontBudget *);
void PS_Woff2Free(uint8_t *);
#ifdef __cplusplus
}
#endif
