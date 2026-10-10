// SPDX-License-Identifier: GPL-3.0-only
#include "../devices/chromatic/components/pocket_search_ui/pocket_search_input.h"
#include <assert.h>
int main(void) {
    char text[PS_INPUT_CAPACITY] = {0};
    for (unsigned i=0; i<32; ++i) assert(ps_append(text,96,"あ"));
    assert(strlen(text)==96); char original[PS_INPUT_CAPACITY]; memcpy(original,text,sizeof(text));
    assert(!ps_append(text,96,"a")); assert(!memcmp(text,original,sizeof(text)));
    ps_erase(text); assert(strlen(text)==93); assert(ps_append(text,96,"漢"));
    for(unsigned i=0;i<32;++i) ps_erase(text);
    assert(!text[0]);
    assert(ps_append(text,96,"😀é")); ps_erase(text); assert(!strcmp(text,"😀")); ps_erase(text); assert(!text[0]);
    assert(!ps_append(text,96,"\xf0\x80\x80\x80")); assert(!text[0]);
    assert(!ps_append(text,96,"\xed\xa0\x80")); assert(!ps_append(text,96,"\xf4\x90\x80\x80"));
    assert(!ps_append(text,96,"\xe3\x81")); assert(!ps_append(text,96,"\x80")); assert(!ps_append(text,96,"\n"));
    assert(ps_append(text,32,"01234567890123456789012345678901")); assert(!ps_append(text,32,"2"));
    memset(text,'x',sizeof(text)); assert(!ps_append(text,96,"a")); ps_erase(text); assert(text[96]=='x');
    return 0;
}
