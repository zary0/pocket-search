// SPDX-License-Identifier: GPL-3.0-only
#pragma once
#include <stdbool.h>
#include <string.h>
static inline bool PS_ResourceURL(const char *url,bool font){
    if(!url||strlen(url)>255||strpbrk(url,"\\#\r\n\t "))return false;
    const char *page="https://www-formal.stanford.edu/";
    if(!strncmp(url,page,strlen(page)))return true;
    const char *face="https://fonts.gstatic.com/";
    if(font)return !strncmp(url,face,strlen(face));
    const char *css="https://fonts.googleapis.com/css?",*css2="https://fonts.googleapis.com/css2?";
    return !strncmp(url,css,strlen(css))||!strncmp(url,css2,strlen(css2));
}
