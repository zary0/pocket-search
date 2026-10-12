// SPDX-License-Identifier: GPL-3.0-only
#pragma once
#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>
#include <string.h>
#define PS_STORE_BYTES 18432u
#define PS_STORE_OFFSET 0x210000u
#define PS_STORE_REGION 0x20000u
#define PS_STORE_BANK 32768u
static inline uint32_t PS_StoreCRC(const void *data,size_t n){const uint8_t *p=data;uint32_t crc=UINT32_MAX;while(n--){crc^=*p++;for(unsigned b=0;b<8;++b)crc=(crc>>1)^((0u-(crc&1u))&0xedb88320u);}return ~crc;}
static inline void PS_StorePut(uint8_t *p,uint32_t v){for(unsigned i=0;i<4;++i)p[i]=(uint8_t)(v>>(i*8));}
static inline uint32_t PS_StoreGet(const uint8_t *p){return (uint32_t)p[0]|(uint32_t)p[1]<<8|(uint32_t)p[2]<<16|(uint32_t)p[3]<<24;}
static inline void PS_StoreOwner(uint8_t h[32]){memset(h,0,32);memcpy(h,"PSOWN001",8);PS_StorePut(h+8,1);PS_StorePut(h+12,PS_STORE_OFFSET);PS_StorePut(h+16,PS_STORE_REGION);PS_StorePut(h+20,PS_STORE_BANK);PS_StorePut(h+28,PS_StoreCRC(h,28));}
static inline bool PS_StoreOwned(const uint8_t h[32]){uint8_t expected[32];PS_StoreOwner(expected);return !memcmp(h,expected,32);}
static inline void PS_StoreHeader(uint8_t h[32],uint32_t generation,uint32_t length,uint32_t crc){memset(h,0,32);memcpy(h,"PSBANK01",8);PS_StorePut(h+8,generation);PS_StorePut(h+12,length);PS_StorePut(h+16,crc);PS_StorePut(h+24,PS_StoreCRC(h,24));PS_StorePut(h+28,0xc04d17edu);}
static inline bool PS_StoreValid(const uint8_t h[32]){return !memcmp(h,"PSBANK01",8)&&PS_StoreGet(h+12)>0&&PS_StoreGet(h+12)<=PS_STORE_BYTES&&PS_StoreGet(h+20)==0&&PS_StoreGet(h+24)==PS_StoreCRC(h,24)&&PS_StoreGet(h+28)==0xc04d17edu;}
static inline bool PS_StoreNewer(uint32_t a,uint32_t b){return a!=b&&(uint32_t)(a-b)<0x80000000u;}
