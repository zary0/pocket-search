// SPDX-License-Identifier: GPL-3.0-only
#pragma once
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
void PS_StoreInit(void);
bool PS_StoreLoad(uint8_t *state,size_t size,unsigned profile);
bool PS_StoreSave(uint8_t *state,size_t size);
bool PS_StoreReady(void);
unsigned PS_StoreGeneration(void);
