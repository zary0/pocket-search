// SPDX-License-Identifier: GPL-3.0-only
#include "../devices/chromatic/components/pocket_search_ui/pocket_search_collection.h"
#include <assert.h>
#include <string.h>
int main(void){
    assert(!ps_generation_valid(NULL));assert(!ps_generation_valid(""));
    assert(ps_generation_valid("original-samples-v1"));
    assert(!ps_generation_valid("a&generation=b"));assert(!ps_generation_valid("x\r\ny"));
    char token[66];memset(token,'a',64);token[64]=0;assert(ps_generation_valid(token));
    token[64]='a';token[65]=0;assert(!ps_generation_valid(token));
    return 0;
}
