// SPDX-License-Identifier: GPL-3.0-only
// Original JPEG-envelope fixtures; no ROM decode or physical-image claim.
#include "../devices/chromatic/components/pocket_search_ui/pocket_jpeg_policy.h"
#include <assert.h>
#include <string.h>
static const uint8_t original[]={255,216,255,192,0,17,8,0,2,0,3,3,1,17,0,2,17,0,3,17,0,255,218,0,12,3,1,0,2,0,3,0,0,63,0,0,255,217};
int main(void){
    assert(PS_JPEGPolicy(original,sizeof(original)));
    for(size_t n=0;n<sizeof(original);++n)assert(!PS_JPEGPolicy(original,n));
    uint8_t data[128];memcpy(data,original,sizeof(original));
    data[3]=194;assert(!PS_JPEGPolicy(data,sizeof(original))); // progressive
    memcpy(data,original,sizeof(original));data[8]=0;assert(!PS_JPEGPolicy(data,sizeof(original))); // zero height
    memcpy(data,original,sizeof(original));data[9]=5;assert(!PS_JPEGPolicy(data,sizeof(original))); // dimension >1024
    memcpy(data,original,sizeof(original));data[13]=65;assert(!PS_JPEGPolicy(data,sizeof(original))); // unsupported sampling
    memcpy(data,original,sizeof(original));data[33]=62;assert(!PS_JPEGPolicy(data,sizeof(original))); // non-baseline scan
    memcpy(data,original,sizeof(original));data[27]=0x22;assert(!PS_JPEGPolicy(data,sizeof(original))); // unsupported Huffman table
    memcpy(data,original,sizeof(original));data[4]=255;assert(!PS_JPEGPolicy(data,sizeof(original))); // oversize marker
    memcpy(data,original,sizeof(original));data[sizeof(original)]=0;assert(!PS_JPEGPolicy(data,sizeof(original)+1)); // trailing data
    unsigned random=1;
    for(unsigned i=0;i<10000;++i){for(unsigned j=0;j<sizeof(data);++j){random=random*1664525u+1013904223u;data[j]=(uint8_t)(random>>24);}data[0]=255;data[1]=216;(void)PS_JPEGPolicy(data,i%sizeof(data));}
    return 0;
}
