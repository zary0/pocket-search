// SPDX-License-Identifier: GPL-3.0-only
#include "../devices/chromatic/components/pocket_search_ui/pocket_store_format.h"
#include <assert.h>
int main(void){
    uint8_t h[32],owner[32];PS_StoreOwner(owner);assert(PS_StoreOwned(owner));for(unsigned i=0;i<32;++i){owner[i]^=1;assert(!PS_StoreOwned(owner));owner[i]^=1;}
    assert(PS_StoreCRC("123456789",9)==0xcbf43926);
    PS_StoreHeader(h,UINT32_MAX,PS_STORE_BYTES,123);assert(PS_StoreValid(h));assert(PS_StoreNewer(0,UINT32_MAX));assert(!PS_StoreNewer(UINT32_MAX,0));
    for(unsigned cut=0;cut<32;++cut){uint8_t torn[32];memset(torn,255,32);memcpy(torn,h,cut);assert(!PS_StoreValid(torn));}
    for(unsigned i=0;i<32;++i){h[i]^=1;assert(!PS_StoreValid(h));h[i]^=1;}
    PS_StoreHeader(h,1,PS_STORE_BYTES+1,123);assert(!PS_StoreValid(h));PS_StoreHeader(h,1,0,123);assert(!PS_StoreValid(h));
    return 0;
}
