// SPDX-License-Identifier: GPL-3.0-only
#include <assert.h>
#include "../devices/chromatic/components/pocket_search_ui/pocket_resource_policy.h"
int main(void){
    assert(PS_ResourceURL("https://www-formal.stanford.edu/jmc/site.css",false));
    assert(PS_ResourceURL("https://www-formal.stanford.edu/jmc/font.ttf",true));
    assert(PS_ResourceURL("https://fonts.googleapis.com/css?family=VT323",false));
    assert(PS_ResourceURL("https://fonts.gstatic.com/l/font?kit=public",true));
    const char *bad[]={"http://www-formal.stanford.edu/site.css","https://www-formal.stanford.edu.evil/site.css","https://www-formal.stanford.edux/site.css","https://www-formal.stanford.edu:443/site.css","https://fonts.gstatic.com@127.0.0.1/x","https://fonts.gstatic.com.evil/x","https://127.0.0.1/x","https://192.168.1.1/x","https://[::1]/x","https://fonts.gstatic.com\\evil/x","https://fonts.googleapis.com/css?family=x#fragment"};
    for(unsigned i=0;i<sizeof(bad)/sizeof(*bad);++i){assert(!PS_ResourceURL(bad[i],false));assert(!PS_ResourceURL(bad[i],true));}
    assert(!PS_ResourceURL("https://fonts.googleapis.com/css?family=x",true));
    assert(!PS_ResourceURL("https://fonts.gstatic.com/l/font",false));
    assert(!PS_ResourceURL(NULL,false));return 0;
}
