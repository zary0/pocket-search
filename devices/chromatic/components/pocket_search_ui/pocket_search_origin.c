// SPDX-License-Identifier: GPL-3.0-only
#include "pocket_search_origin.h"
#include "nvs.h"
#include <stddef.h>
#include <string.h>

static bool alphanumeric(unsigned char c){
    return (c>='a'&&c<='z')||(c>='A'&&c<='Z')||(c>='0'&&c<='9');
}
bool PS_OriginValid(const char *value){
    if(!value)return false;
    size_t n=0;
    while(n<PS_ORIGIN_BYTES&&value[n])++n;
    if(n<=8||n==PS_ORIGIN_BYTES||memcmp(value,"https://",8))return false;
    size_t label=0;
    for(size_t i=8;i<n;++i){
        unsigned char c=(unsigned char)value[i];
        if(c=='.'){
            if(!label||value[i-1]=='-')return false;
            label=0;
        }else{
            if(!alphanumeric(c)&&c!='-')return false;
            if((!label&&c=='-')||++label>63)return false;
        }
    }
    return label&&value[n-1]!='-';
}
esp_err_t PS_OriginLoad(char value[PS_ORIGIN_BYTES]){
    if(!value)return ESP_ERR_INVALID_ARG;
    memset(value,0,PS_ORIGIN_BYTES);
    nvs_handle_t handle;
    esp_err_t error=nvs_open("pocket_search",NVS_READONLY,&handle);
    if(error!=ESP_OK)return error;
    size_t size=PS_ORIGIN_BYTES;
    error=nvs_get_str(handle,"origin",value,&size);
    nvs_close(handle);
    if(error==ESP_OK&&(!size||size>PS_ORIGIN_BYTES||value[size-1]||strlen(value)+1!=size||!PS_OriginValid(value)))error=ESP_ERR_INVALID_ARG;
    if(error!=ESP_OK)memset(value,0,PS_ORIGIN_BYTES);
    return error;
}
esp_err_t PS_OriginSave(const char *value){
    if(!PS_OriginValid(value))return ESP_ERR_INVALID_ARG;
    char current[PS_ORIGIN_BYTES];
    if(PS_OriginLoad(current)==ESP_OK&&!strcmp(value,current))return ESP_OK;
    nvs_handle_t handle;
    esp_err_t error=nvs_open("pocket_search",NVS_READWRITE,&handle);
    if(error!=ESP_OK)return error;
    error=nvs_set_str(handle,"origin",value);
    if(error==ESP_OK)error=nvs_commit(handle);
    nvs_close(handle);
    if(error==ESP_OK){
        error=PS_OriginLoad(current);
        if(error==ESP_OK&&strcmp(current,value))error=ESP_FAIL;
    }
    return error;
}
