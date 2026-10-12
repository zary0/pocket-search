// SPDX-License-Identifier: GPL-3.0-only
#include "../devices/chromatic/components/pocket_search_ui/pocket_inline_image.h"
#include <assert.h>
int main(void){
    PS_InlineImage image={0};
    assert(!PS_InlineNeedsLoad(&image,0,1));assert(!PS_InlineNeedsLoad(&image,1,0));assert(!PS_InlineNeedsLoad(&image,1,25));
    assert(PS_InlineNeedsLoad(&image,1,3));PS_InlineBegin(&image,1,3);
    // Repeated scroll windows over the same slot never queue another download.
    assert(!PS_InlineNeedsLoad(&image,1,3));PS_InlineFinish(&image,0);
    assert(PS_InlineMatches(&image,1,3));assert(!PS_InlineNeedsLoad(&image,1,3));
    assert(!PS_InlineNeedsLoad(&image,1,0));assert(PS_InlineNeedsLoad(&image,1,4));assert(PS_InlineNeedsLoad(&image,2,3));
    PS_InlineBegin(&image,1,4);PS_InlineInterrupt(&image);
    assert(image.error==9&&image.stage==PS_INLINE_FAILED);assert(!PS_InlineNeedsLoad(&image,1,4));
    // Explicit retry may replace cancellation/error without poisoning another slot.
    PS_InlineBegin(&image,1,4);PS_InlineFinish(&image,14);
    assert(!PS_InlineNeedsLoad(&image,1,4));assert(PS_InlineNeedsLoad(&image,1,3));
    PS_InlineBegin(&image,1,4);PS_InlineFinish(&image,0);PS_InlineInterrupt(&image);assert(image.stage==PS_INLINE_READY);
    PS_InlineFit f=PS_InlineFitBox(64,80,154,42);assert(f.width==33&&f.height==41&&f.zoom==134);
    f=PS_InlineFitBox(160,10,60,48);assert(f.width==60&&f.height==3);
    f=PS_InlineFitBox(8,6,154,42);assert(f.width==8&&f.height==6&&f.zoom==256);
    assert(!PS_InlineFitBox(0,80,160,42).zoom);assert(!PS_InlineFitBox(64,81,160,42).zoom);
    assert(!PS_InlineFitBox(160,80,160,0).zoom);assert(!PS_InlineFitBox(160,80,161,42).zoom);
    for(unsigned w=1;w<=160;++w)for(unsigned h=1;h<=80;++h)for(unsigned box=1;box<=86;++box){
        f=PS_InlineFitBox(w,h,154,box);assert(f.zoom<=256&&f.width<=154&&f.height<=box&&f.width<=w&&f.height<=h);
    }
    return 0;
}
