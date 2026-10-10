// SPDX-License-Identifier: GPL-3.0-only
#include "pocket_search_origin.h"
#include "nvs.h"
#include <assert.h>
#include <stdio.h>
#include <string.h>

// The storage fixture models separate staged/committed data and failures.
static char stored[256],staged[256];
static size_t stored_size;
static bool exists;
static esp_err_t open_error,set_error,commit_error,get_error;
static unsigned writes,commits,opens,closes;
esp_err_t nvs_open(const char *space,nvs_open_mode_t mode,nvs_handle_t *handle){
    assert(!strcmp(space,"pocket_search"));
    if(open_error)return open_error;
    if(mode==NVS_READONLY&&!exists)return ESP_ERR_NVS_NOT_FOUND;
    *handle=1;++opens;return ESP_OK;
}
esp_err_t nvs_get_str(nvs_handle_t handle,const char *key,char *value,size_t *size){
    assert(handle==1&&!strcmp(key,"origin"));
    if(get_error)return get_error;
    if(!exists)return ESP_ERR_NVS_NOT_FOUND;
    if(*size<stored_size){*size=stored_size;return ESP_ERR_NVS_INVALID_LENGTH;}
    memcpy(value,stored,stored_size);*size=stored_size;return ESP_OK;
}
esp_err_t nvs_set_str(nvs_handle_t handle,const char *key,const char *value){
    assert(handle==1&&!strcmp(key,"origin"));++writes;
    if(set_error)return set_error;
    strcpy(staged,value);return ESP_OK;
}
esp_err_t nvs_commit(nvs_handle_t handle){
    assert(handle==1);++commits;
    if(commit_error)return commit_error;
    strcpy(stored,staged);stored_size=strlen(stored)+1;exists=true;return ESP_OK;
}
void nvs_close(nvs_handle_t handle){assert(handle==1);++closes;}
static void empty(void){
    memset(stored,0,sizeof(stored));memset(staged,0,sizeof(staged));stored_size=0;
    exists=false;open_error=set_error=commit_error=get_error=0;writes=commits=opens=closes=0;
}
static void restored(const char *expected){
    // A fresh RAM buffer stands in for a restarted process/device.
    char value[PS_ORIGIN_BYTES];memset(value,'x',sizeof(value));
    assert(PS_OriginLoad(value)==ESP_OK&&!strcmp(value,expected));
}
int main(void){
    const char *first="https://search.example.org",*second="https://other.example.org";
    const char *bad[]={"","http://search.example.org","https://","https://user:pass@example.org",
        "https://example.org:443","https://example.org/","https://example.org?q=x","https://example.org#x",
        "https://.example.org","https://example..org","https://example.org.","https://-example.org",
        "https://example-.org","https://exa mple.org","https://example.org\n","https://\xc3\xa9.org"};
    empty();assert(!PS_OriginValid(NULL));assert(PS_OriginValid(first));
    for(size_t i=0;i<sizeof(bad)/sizeof(*bad);++i){
        assert(!PS_OriginValid(bad[i]));assert(PS_OriginSave(bad[i])==ESP_ERR_INVALID_ARG);
    }
    assert(writes==0&&commits==0&&opens==0);
    char value[PS_ORIGIN_BYTES];memset(value,'x',sizeof(value));
    assert(PS_OriginLoad(value)==ESP_ERR_NVS_NOT_FOUND&&value[0]==0);
    assert(PS_OriginSave(first)==ESP_OK);restored(first);
    unsigned prior_writes=writes,prior_commits=commits;
    assert(PS_OriginSave(first)==ESP_OK&&writes==prior_writes&&commits==prior_commits);
    assert(PS_OriginSave(second)==ESP_OK);restored(second);assert(opens==closes);
    open_error=ESP_FAIL;memset(value,'x',sizeof(value));
    assert(PS_OriginLoad(value)==ESP_FAIL&&value[0]==0);assert(PS_OriginSave(first)==ESP_FAIL);
    open_error=0;set_error=ESP_FAIL;assert(PS_OriginSave(first)==ESP_FAIL);set_error=0;restored(second);
    commit_error=ESP_FAIL;assert(PS_OriginSave(first)==ESP_FAIL);commit_error=0;restored(second);
    get_error=ESP_FAIL;memset(value,'x',sizeof(value));assert(PS_OriginLoad(value)==ESP_FAIL&&value[0]==0);get_error=0;
    // Corrupt/oversized saved data fails closed, with a cleared output and closed handle.
    memset(stored,'a',sizeof(stored));stored_size=sizeof(stored);
    assert(PS_OriginLoad(value)==ESP_ERR_NVS_INVALID_LENGTH&&value[0]==0);
    strcpy(stored,"http://unsafe.example.org");stored_size=strlen(stored)+1;
    assert(PS_OriginLoad(value)==ESP_ERR_INVALID_ARG&&value[0]==0);
    strcpy(stored,first);stored_size=strlen(stored)+2;stored[stored_size-1]=0;
    assert(PS_OriginLoad(value)==ESP_ERR_INVALID_ARG&&value[0]==0);
    assert(PS_OriginSave(first)==ESP_OK);restored(first);assert(opens==closes);
    // Label and full-buffer limits are checked before any storage write.
    char long_name[PS_ORIGIN_BYTES+1];memset(long_name,'a',sizeof(long_name));memcpy(long_name,"https://",8);long_name[PS_ORIGIN_BYTES]=0;
    assert(!PS_OriginValid(long_name));long_name[8+64]=0;assert(!PS_OriginValid(long_name));
    long_name[8+63]=0;assert(PS_OriginValid(long_name));
    assert(PS_OriginLoad(NULL)==ESP_ERR_INVALID_ARG);
    puts("Saved HTTPS origin validation, restart restoration, no-op writes and storage failures passed.");
}
