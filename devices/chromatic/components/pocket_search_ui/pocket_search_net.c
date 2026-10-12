// SPDX-License-Identifier: GPL-3.0-only
// On-device HTTPS collection and Rust search. No companion service or USB proxy.
#include "pocket_search_net.h"
#include "pocket_typography.h"
#include "pocket_native_image.h"
#include "pocket_store.h"
#include "pocket_resource_policy.h"
#include "chromatic_wifi.h"
#include "network_time.h"
#include "esp_console.h"
#include "esp_crt_bundle.h"
#include "esp_http_client.h"
#include "esp_timer.h"
#include "mbedtls/sha256.h"
#include "esp_cpu.h"
#include "esp_private/esp_clk.h"
#include "esp_log.h"
#include "nvs.h"
#include "esp_heap_caps.h"
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include <stdio.h>
#include <string.h>
#include <strings.h>
#include <stdlib.h>
#include <stdatomic.h>
#include <time.h>

#define LOCAL_STATE_BYTES 22272
#define HTML_LIMIT 65536
#define ROBOTS_LIMIT 4096
#define COLLECTION_US 90000000
uint32_t ps_local_profiles(void);
static char robots_url[128];
static unsigned profile;
static portMUX_TYPE lock=portMUX_INITIALIZER_UNLOCKED;
static bool reading;
static _Alignas(8) uint8_t local_state[LOCAL_STATE_BYTES];
static uint8_t robots[ROBOTS_LIMIT+1];
static size_t robots_length;
static unsigned page_bytes;
static char conditional_etag[96],conditional_modified[96];
static unsigned robots_state,documents,requests,body_bytes;
static bool collected;
static _Atomic unsigned store_failures;
static char installed_font_url[256];
static _Atomic unsigned css_cache_hits,css_fetches,font_fetches,font_glyphs,font_bytes,css_rules,resource_fallbacks;
static const char probe_css[]="https://fonts.googleapis.com/css?family=VT323&text=WEB%20FONT%20TEST%20Pocket%20Search%20ABC%20xyz%20012345%20Small%20caption%20Downloaded%20TTF";
static const char probe_jp_css[]="https://fonts.googleapis.com/css2?family=Noto+Sans+JP%3Awght%40400&text=%E6%97%A5%E6%9C%AC%E8%AA%9E%E6%A4%9C%E7%B4%A2%E7%B5%90%E6%9E%9C";
static const char probe_jp_budget_css[]="https://fonts.googleapis.com/css2?family=Noto+Sans+JP%3Awght%40400&text=%E6%97%A5%E6%9C%AC%E8%AA%9E%E6%A4%9C%E7%B4%A2%E7%B5%90%E6%9E%9C%E3%81%8C%E3%81%B1";
static bool font_budget_probe;
static bool font_probe_japanese;
static unsigned font_cancel_check,font_checks;
static struct {bool running,ready,cancelled,configuring;PS_Request request;PS_Reply reply;} job;
enum {NO_WIFI=1,NO_CLOCK,NO_MEMORY,CONNECT_FAILED,HTTP_FAILED,TOO_LARGE,INCOMPLETE,WRONG_TYPE,CANCELLED,INVALID_IMAGE,STALE_COLLECTION,ROBOTS_DENIED,NO_DOCUMENTS};
typedef struct {PS_Reply *reply;bool html,plain,css,font,jpeg,png,identity;char etag[96],modified[64];} Events;
size_t ps_local_state_bytes(void);
int32_t ps_local_init(uint8_t*,size_t);
int32_t ps_local_next(uint8_t*,size_t,uint8_t*,size_t);
int32_t ps_local_feed(uint8_t*,size_t,const uint8_t*,size_t);
int32_t ps_local_finish(uint8_t*,size_t,int32_t);
int32_t ps_local_validators(uint8_t*,size_t,const uint8_t*,size_t,const uint8_t*,size_t);
int32_t ps_local_update_begin(uint8_t*,size_t,uint32_t,uint8_t*,size_t,uint8_t*);
int32_t ps_local_update_header(uint8_t*,size_t,int32_t,uint8_t*);
int32_t ps_local_update_restore(uint8_t*,size_t,uint8_t*,size_t,int32_t,uint32_t);
int32_t ps_local_cancel_transfer(uint8_t*,size_t);
int32_t ps_local_pending(uint8_t*,size_t);
int32_t ps_local_metadata(uint8_t*,size_t,uint32_t,uint32_t,uint32_t);
int32_t ps_local_count(uint8_t*,size_t);
int32_t ps_local_set_profile(uint8_t*,size_t,uint32_t);
int32_t ps_local_allowed(uint32_t,const uint8_t*,size_t);
int32_t ps_local_profile_text(uint32_t,uint32_t,uint8_t*,size_t);
int32_t ps_local_packet(uint8_t*,size_t,uint32_t,uint32_t,uint32_t,const uint8_t*,size_t,uint8_t*);
int32_t ps_local_robots(const uint8_t*,size_t,const uint8_t*,size_t);

int32_t ps_local_resource(uint8_t*,size_t,int32_t,uint8_t*);
int32_t ps_local_css(uint8_t*,size_t,const uint8_t*,size_t,const uint8_t*,size_t);
int32_t ps_local_reparse(uint8_t*,size_t,int32_t);
int32_t ps_local_css_cached(uint8_t*,size_t,const uint8_t*,size_t);
int32_t ps_local_css_count(uint8_t*,size_t);
size_t ps_local_backup_bytes(void);
int32_t ps_local_backup(uint8_t*,size_t,uint8_t*,size_t,int32_t);

static bool media(const char *value,const char *type){size_t n=strlen(type);return value&&!strncasecmp(value,type,n)&&(value[n]==0||value[n]==';'||value[n]==' ');}
static void validator(char *out,size_t capacity,const char *value){if(!value)return;size_t n=strlen(value);if(n>=capacity)return;for(size_t i=0;i<n;++i)if((unsigned char)value[i]<32||(unsigned char)value[i]>126)return;memcpy(out,value,n+1);}
static esp_err_t event(esp_http_client_event_t *e){
    Events *v=e->user_data;
    if(e->event_id==HTTP_EVENT_ON_CONNECTED)v->reply->tls_verified=true;
    if(e->event_id==HTTP_EVENT_ON_HEADER&&e->header_key){
        if(!strcasecmp(e->header_key,"Content-Type")){v->jpeg=media(e->header_value,"image/jpeg");v->png=media(e->header_value,"image/png");v->html=media(e->header_value,"text/html");v->plain=media(e->header_value,"text/plain");v->css=media(e->header_value,"text/css");v->font=media(e->header_value,"font/woff2")||media(e->header_value,"font/ttf")||media(e->header_value,"application/octet-stream")||media(e->header_value,"application/x-font-ttf")||media(e->header_value,"application/font-sfnt");}
        if(!strcasecmp(e->header_key,"ETag"))validator(v->etag,sizeof(v->etag),e->header_value);
        if(!strcasecmp(e->header_key,"Last-Modified"))validator(v->modified,sizeof(v->modified),e->header_value);
        if(!strcasecmp(e->header_key,"Content-Encoding")&&(!e->header_value||strcasecmp(e->header_value,"identity")))v->identity=false;
    }return ESP_OK;
}
static bool cancelled(void){taskENTER_CRITICAL(&lock);bool v=job.cancelled;taskEXIT_CRITICAL(&lock);return v;}
static void memory(PS_Reply *r){
    unsigned free=(unsigned)heap_caps_get_free_size(MALLOC_CAP_8BIT),block=(unsigned)heap_caps_get_largest_free_block(MALLOC_CAP_8BIT);
    if(free<r->minimum_free_heap_bytes)r->minimum_free_heap_bytes=free;
    if(block<r->largest_free_block_bytes)r->largest_free_block_bytes=block;
    r->stack_hwm=(unsigned)uxTaskGetStackHighWaterMark(NULL);
}
static void publish(const PS_Reply *r){taskENTER_CRITICAL(&lock);job.reply=*r;job.ready=true;job.running=false;taskEXIT_CRITICAL(&lock);}
static bool valid_url(const char *url){
    return ps_local_allowed(profile,(const uint8_t*)url,strlen(url))==1;
}
static int fetch(const char *url,bool robot,PS_Reply *r,int64_t start){
    Events events={.reply=r,.identity=true};int error=0;size_t total=0;esp_http_client_handle_t client=NULL;
    const size_t limit=robot?ROBOTS_LIMIT:HTML_LIMIT;
    esp_http_client_config_t config={.url=url,.timeout_ms=8000,.crt_bundle_attach=esp_crt_bundle_attach,.skip_cert_common_name_check=false,.disable_auto_redirect=true,.buffer_size=512,.buffer_size_tx=512,.event_handler=event,.user_data=&events};
    client=esp_http_client_init(&config);if(!client)return NO_MEMORY;
    esp_http_client_set_header(client,"Accept",robot?"text/plain":"text/html");esp_http_client_set_header(client,"Accept-Encoding","identity");
    esp_http_client_set_header(client,"User-Agent","PocketSearchBot/0.1 (bounded handheld research; https://github.com/zary0/pocket-search)");
    if(!robot){if(*conditional_etag)esp_http_client_set_header(client,"If-None-Match",conditional_etag);else if(*conditional_modified)esp_http_client_set_header(client,"If-Modified-Since",conditional_modified);}
    taskENTER_CRITICAL(&lock);++requests;taskEXIT_CRITICAL(&lock);
    if(esp_http_client_open(client,0)!=ESP_OK){error=CONNECT_FAILED;goto done;}
    int64_t length=esp_http_client_fetch_headers(client);r->http_status=esp_http_client_get_status_code(client);memory(r);
    // A confirmed missing robots resource permits this bounded crawl. Other
    // errors, redirects and unsupported/oversize policies fail closed.
    if(robot&&(r->http_status==404||r->http_status==410)){robots_state=2;goto done;}
    if(!robot&&r->http_status==304&&(*conditional_etag||*conditional_modified))goto done;
    if(r->http_status!=200){error=HTTP_FAILED;goto done;}
    if(!events.identity||(robot?!events.plain:!events.html)){error=WRONG_TYPE;goto done;}
    if(length>(int64_t)limit){error=TOO_LARGE;goto done;}
    while(esp_timer_get_time()-start<COLLECTION_US&&!cancelled()){
        int n=esp_http_client_read(client,(char*)r->bytes,512);
        if(n<0){error=INCOMPLETE;goto done;}
        if(!n)break;
        total+=(size_t)n;if(total>limit){error=TOO_LARGE;goto done;}
        if(robot)memcpy(robots+total-(size_t)n,r->bytes,(size_t)n);
        else if(ps_local_feed(local_state,sizeof(local_state),r->bytes,(size_t)n)){error=INCOMPLETE;goto done;}
        memory(r);
    }
    if(cancelled()){error=CANCELLED;goto done;}
    if(!esp_http_client_is_complete_data_received(client)){error=INCOMPLETE;goto done;}
    if(robot){robots_length=total;robots_state=1;}
    else if(ps_local_validators(local_state,sizeof(local_state),(const uint8_t*)events.etag,strlen(events.etag),(const uint8_t*)events.modified,strlen(events.modified)))error=INCOMPLETE;
done:
    taskENTER_CRITICAL(&lock);body_bytes+=(unsigned)total;taskEXIT_CRITICAL(&lock);
    r->wire_bytes+=(unsigned)total;if(!robot)page_bytes=(unsigned)total;memory(r);esp_http_client_cleanup(client);return error;
}
// Exact public authorities, no redirect, port, userinfo, local address or arbitrary host.
static bool image_url_allowed(const char *url){
    char base[128];if(ps_local_profile_text(profile,1,(uint8_t*)base,sizeof(base))<=0)return false;
    const char *end=strchr(base+8,'/');if(!end)return false;size_t authority=(size_t)(end-base)+1;
    if(strncmp(url,base,authority)||strstr(url+authority,"..")||strstr(url+authority,"//"))return false;
    for(const unsigned char *p=(const unsigned char*)url+authority;*p;++p)if(!((*p>='a'&&*p<='z')||(*p>='A'&&*p<='Z')||(*p>='0'&&*p<='9')||strchr("/-_.",*p)))return false;
    const char *path=url+authority-1;return robots_state==2||(robots_state==1&&ps_local_robots(robots,robots_length,(const uint8_t*)path,strlen(path))==1);
}
static int resource(const char *url,bool font,bool image,uint8_t **out,size_t *out_length,PS_Reply *r,int64_t start){
    *out=NULL;*out_length=0;
    if(!image&&!strncmp(url,"https://www.w3.org/",19)&&!image_url_allowed(url))return WRONG_TYPE;
    if(image?!image_url_allowed(url):!PS_ResourceURL(url,font))return image?ROBOTS_DENIED:WRONG_TYPE;
    Events events={.reply=r,.identity=true};int error=0;size_t total=0;uint8_t *body=NULL;
    size_t limit=image?PS_NATIVE_IMAGE_WIRE:font?24576:3072;
    esp_http_client_config_t config={.url=url,.timeout_ms=8000,.crt_bundle_attach=esp_crt_bundle_attach,.skip_cert_common_name_check=false,.disable_auto_redirect=true,.buffer_size=512,.buffer_size_tx=512,.event_handler=event,.user_data=&events};
    esp_http_client_handle_t client=esp_http_client_init(&config);if(!client)return NO_MEMORY;
    esp_http_client_set_header(client,"Accept",image?"image/jpeg,image/png":font?"font/woff2,font/ttf,application/octet-stream":"text/css");
    esp_http_client_set_header(client,"Accept-Encoding","identity");esp_http_client_set_header(client,"User-Agent",!font&&!image&&!strncmp(url,"https://fonts.googleapis.com/",29)?"Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36":"PocketSearchBot/0.1");
    taskENTER_CRITICAL(&lock);++requests;if(!image){if(font)++font_fetches;else ++css_fetches;}taskEXIT_CRITICAL(&lock);
    if(cancelled()||esp_timer_get_time()-start>=COLLECTION_US){error=CANCELLED;goto done;}
    if(esp_http_client_open(client,0)!=ESP_OK){error=CONNECT_FAILED;goto done;}
    int64_t length=esp_http_client_fetch_headers(client);r->http_status=esp_http_client_get_status_code(client);memory(r);
    if(r->http_status!=200){error=HTTP_FAILED;goto done;}
    if(!events.identity||(image?!(events.jpeg||events.png):font?!events.font:!events.css)){error=WRONG_TYPE;goto done;}
    if(length>(int64_t)limit){error=TOO_LARGE;goto done;}
    // Allocate after the TLS handshake, and release raw font bytes before the next socket.
    size_t capacity=length>0?(size_t)length:limit;if(image&&heap_caps_get_free_size(MALLOC_CAP_8BIT)<capacity+16384){error=NO_MEMORY;goto done;}body=malloc(capacity);if(!body){error=NO_MEMORY;goto done;}
    while(!cancelled()&&esp_timer_get_time()-start<COLLECTION_US){
        int n=esp_http_client_read(client,(char*)r->bytes,512);if(n<0){error=INCOMPLETE;goto done;}if(!n)break;
        if(total+(size_t)n>capacity){error=TOO_LARGE;goto done;}memcpy(body+total,r->bytes,(size_t)n);total+=(size_t)n;memory(r);
    }
    if(cancelled()){error=CANCELLED;goto done;}
    if(!esp_http_client_is_complete_data_received(client)){error=INCOMPLETE;goto done;}
    if(image&&((events.jpeg&&(total<2||body[0]!=255||body[1]!=216))||(events.png&&(total<8||memcmp(body,"\x89PNG\r\n\x1a\n",8))))){error=WRONG_TYPE;goto done;}
    *out=body;*out_length=total;body=NULL;
done:
    free(body);r->wire_bytes+=(unsigned)total;taskENTER_CRITICAL(&lock);body_bytes+=(unsigned)total;taskEXIT_CRITICAL(&lock);
    memory(r);esp_http_client_cleanup(client);return error;
}
static bool font_cancel(void *context){(void)context;++font_checks;return cancelled()||(font_cancel_check&&font_checks>=font_cancel_check);}
static uint64_t font_clock(void *context){(void)context;static int64_t yielded;int64_t now=esp_timer_get_time();if(now-yielded>=10000){vTaskDelay(1);yielded=now;}return (uint64_t)(now/1000);}
static size_t font_heap(void *context){memory(context);return heap_caps_get_free_size(MALLOC_CAP_8BIT);}
static bool load_face(PS_Reply *r,int64_t start,int *cause){
    char url[256];if(ps_local_resource(local_state,sizeof(local_state),1,(uint8_t*)url)<=0){if(cause)*cause=WRONG_TYPE;return false;}
    // Existing readers retain the same immutable atlas, even during refresh.
    if(PS_WebFontReady()){bool same=!strcmp(url,installed_font_url);if(!same&&cause)*cause=WRONG_TYPE;return same;}
    uint8_t *body=NULL;size_t length=0;
    bool tls_monitor=heap_caps_monitor_local_minimum_free_size_start()==ESP_OK;r->tls_heap_window=tls_monitor;
    int error=resource(url,true,false,&body,&length,r,start);
    if(tls_monitor){r->tls_allocator_minimum=(unsigned)heap_caps_get_minimum_free_size(MALLOC_CAP_8BIT);heap_caps_monitor_local_minimum_free_size_stop();}
    if(error){if(cause)*cause=error;++resource_fallbacks;return false;}
    bool decode_monitor=heap_caps_monitor_local_minimum_free_size_start()==ESP_OK;r->decode_heap_window=decode_monitor;r->decode_heap_before=(unsigned)heap_caps_get_free_size(MALLOC_CAP_8BIT);
    font_checks=0;PS_FontBudget budget={.cancel=font_cancel,.clock_ms=font_clock,.free_heap=font_heap,.context=r};
    int64_t begin=esp_timer_get_time();int n=PS_WebFontLoad(body,length,&budget);r->compute_us=(unsigned)(esp_timer_get_time()-begin);r->decoder_peak_bytes=budget.peak_bytes;r->decoder_allocations=budget.allocations;
    if(decode_monitor){r->decode_allocator_minimum=(unsigned)heap_caps_get_minimum_free_size(MALLOC_CAP_8BIT);heap_caps_monitor_local_minimum_free_size_stop();}
    char digest[65]={0},atlas_digest[65]={0};uint8_t hash[32];mbedtls_sha256(body,length,hash,0);for(unsigned i=0;i<32;++i)sprintf(digest+i*2,"%02x",hash[i]);
    if(n>0){mbedtls_sha256(PS_WebFontAtlas(),PS_WebFontAtlasBytes(),hash,0);for(unsigned i=0;i<32;++i)sprintf(atlas_digest+i*2,"%02x",hash[i]);}
    printf("SEARCH_FONT {\"error\":%d,\"wire_bytes\":%u,\"private_peak_bytes\":%u,\"allocations\":%u,\"decode_raster_us\":%lld,\"atlas_bytes\":%u,\"glyphs\":%d,\"source_sha256\":\"%s\",\"atlas_sha256\":\"%s\",\"checks\":%u}\n",n<0?-n:n?0:10,(unsigned)length,budget.peak_bytes,budget.allocations,(long long)(esp_timer_get_time()-begin),(unsigned)PS_WebFontAtlasBytes(),n>0?n:0,digest,atlas_digest,font_checks);
    free(body);
    if(n<=0){if(cause)*cause=n<0?-n:WRONG_TYPE;++resource_fallbacks;return false;}
    memcpy(installed_font_url,url,strlen(url)+1);font_glyphs=(unsigned)n;font_bytes=(unsigned)length;memory(r);return true;
}
static bool load_css(const char *url,PS_Reply *r,int64_t start,bool *ready,int *cause){
    if(ps_local_css_cached(local_state,sizeof(local_state),(const uint8_t*)url,strlen(url))==1){
        ++css_cache_hits;*ready=load_face(r,start,cause);return ps_local_css_count(local_state,sizeof(local_state))>0||*ready;
    }
    uint8_t *body=NULL;size_t length=0;int error=resource(url,false,false,&body,&length,r,start);
    if(error){if(cause)*cause=error;++resource_fallbacks;return false;}
    int rules=ps_local_css(local_state,sizeof(local_state),body,length,(const uint8_t*)url,strlen(url));free(body);
    if(rules<0){if(cause)*cause=WRONG_TYPE;++resource_fallbacks;return false;}css_rules+=(unsigned)rules;
    *ready=load_face(r,start,cause);return rules>0||*ready;
}
int32_t ps_local_image_url(uint8_t*,size_t,uint32_t,uint32_t,uint8_t*,size_t);
int32_t ps_local_image_slot(uint8_t*,size_t,uint32_t,uint32_t);
static bool image_cancelled(void *context){(void)context;return cancelled();}
static void image_memory(void *context){memory(context);}
static bool image_digest(const uint8_t *bytes,size_t length,char hex[65]){
    uint8_t digest[32];if(mbedtls_sha256(bytes,length,digest,0))return false;
    const char digits[]="0123456789abcdef";for(unsigned i=0;i<32;++i){hex[i*2]=digits[digest[i]>>4];hex[i*2+1]=digits[digest[i]&15];}hex[64]=0;return true;
}
static void worker(void *arg){
    (void)arg;PS_Reply r={.sequence=job.request.sequence,.local=true};int64_t start=esp_timer_get_time();int crawl_error=0;
    // ESP-IDF records each heap's low watermark at allocation time. Their sum
    // is a conservative phase minimum, unlike the periodic sample above.
    bool phase_monitor=false;
    if(job.request.kind==3||job.request.kind==7){r.tls_heap_window=heap_caps_monitor_local_minimum_free_size_start()==ESP_OK;phase_monitor=r.tls_heap_window;}
    conditional_etag[0]=conditional_modified[0]=0;
    r.heap_before=(unsigned)heap_caps_get_free_size(MALLOC_CAP_8BIT);r.minimum_free_heap_bytes=r.heap_before;r.largest_free_block_bytes=(unsigned)heap_caps_get_largest_free_block(MALLOC_CAP_8BIT);
    // Cached search/reader work never requires a socket, Wi-Fi, clock sync or
    // another device. Verified flags describe the retained source provenance.
    if((!collected&&job.request.kind==1)||job.request.kind==255){
        CW_Status wifi;CW_GetStatus(&wifi);if(wifi.state!=CW_CONNECTED){if(documents&&job.request.kind==1)goto cached;r.error=NO_WIFI;goto done;}
        esp_err_t clock_error;int64_t clock_deadline=start+20000000;
        do{if(cancelled()){r.error=CANCELLED;goto done;}clock_error=CW_TimeWait(200);}while(clock_error==ESP_ERR_TIMEOUT&&esp_timer_get_time()<clock_deadline);
        if(clock_error!=ESP_OK){r.error=NO_CLOCK;goto done;}
        r.clock_synced=true;
        if(job.request.kind==255){
            bool ready=false;
            int cause=0;if(!load_css(font_probe_japanese?(font_budget_probe?probe_jp_budget_css:probe_jp_css):probe_css,&r,start,&ready,&cause)||!ready)r.error=cause?cause:WRONG_TYPE;
            goto done;
        }
        if(!robots_state){
            crawl_error=fetch(robots_url,true,&r,start);
            // Cancellation and transport failure do not establish a robots
            // denial. Fail this request closed, but allow an explicit retry.
            if(crawl_error){r.error=cancelled()?CANCELLED:crawl_error;goto done;}
        }
        if(robots_state==3){r.error=ROBOTS_DENIED;goto done;}
        char url[128];
        while(esp_timer_get_time()-start<COLLECTION_US&&!cancelled()){
            int next=ps_local_next(local_state,sizeof(local_state),(uint8_t*)url,sizeof(url));if(next<=0)break;
            const char *path=strchr(url+8,'/');
            bool allowed=valid_url(url)&&path&&(robots_state==2||ps_local_robots(robots,robots_length,(const uint8_t*)path,strlen(path))==1);
            if(!allowed){(void)ps_local_finish(local_state,sizeof(local_state),0);crawl_error=ROBOTS_DENIED;continue;}
            // One request at a time, at least one second between page starts.
            for(unsigned i=0;i<10&&!cancelled();++i)vTaskDelay(pdMS_TO_TICKS(100));
            if(cancelled()){(void)ps_local_cancel_transfer(local_state,sizeof(local_state));break;}
            int error=fetch(url,false,&r,start);unsigned original_page_bytes=page_bytes;
            char css_url[256];bool ready=false;
            if(!error&&ps_local_resource(local_state,sizeof(local_state),0,(uint8_t*)css_url)>0&&load_css(css_url,&r,start,&ready,NULL)){
                // No retained DOM: refetch once with the bounded cascade, then publish.
                size_t saved_length=ps_local_backup_bytes();uint8_t *saved=saved_length<=4608?calloc(1,saved_length):NULL;
                if(saved&&ps_local_backup(local_state,sizeof(local_state),saved,saved_length,0)==0){
                    ps_local_reparse(local_state,sizeof(local_state),ready?1:0);
                    for(unsigned i=0;i<10&&!cancelled();++i)vTaskDelay(pdMS_TO_TICKS(100));
                    error=fetch(url,false,&r,start);
                    if(error&&!cancelled()){
                        ps_local_backup(local_state,sizeof(local_state),saved,saved_length,1);++resource_fallbacks;error=0;page_bytes=original_page_bytes;
                    }
                }else ++resource_fallbacks;
                free(saved);
            }
            if(cancelled()){(void)ps_local_cancel_transfer(local_state,sizeof(local_state));break;}
            if(!error&&ps_local_metadata(local_state,sizeof(local_state),page_bytes,(uint32_t)time(NULL),200))error=INCOMPLETE;
            (void)ps_local_finish(local_state,sizeof(local_state),error==0);
            taskENTER_CRITICAL(&lock);documents=(unsigned)ps_local_count(local_state,sizeof(local_state));taskEXIT_CRITICAL(&lock);
            if(error)crawl_error=error;
            else if(!PS_StoreSave(local_state,sizeof(local_state)))++store_failures;
        }
        if(cancelled()){r.error=CANCELLED;goto done;}
        collected=true;
    }
    if(job.request.kind==5){
        CW_Status wifi;CW_GetStatus(&wifi);if(wifi.state!=CW_CONNECTED){r.error=NO_WIFI;goto done;}
        if(CW_TimeWait(1000)!=ESP_OK){r.error=NO_CLOCK;goto done;}r.clock_synced=true;
        if(!robots_state){r.error=fetch(robots_url,true,&r,start);if(r.error)goto done;}
        size_t saved_size=ps_local_backup_bytes();uint8_t *saved=calloc(1,saved_size);if(!saved){r.error=NO_MEMORY;goto done;}
        unsigned count=documents,checked=0;
        for(unsigned id=1;id<=count&&!cancelled()&&esp_timer_get_time()-start<COLLECTION_US;++id){
            char url[128];if(ps_local_update_begin(local_state,sizeof(local_state),id,saved,saved_size,(uint8_t*)url)<=0){r.error=INCOMPLETE;break;}
            conditional_etag[0]=conditional_modified[0]=0;
            (void)ps_local_update_header(saved,saved_size,0,(uint8_t*)conditional_etag);(void)ps_local_update_header(saved,saved_size,1,(uint8_t*)conditional_modified);
            int error=image_url_allowed(url)?0:ROBOTS_DENIED;
            for(unsigned i=0;i<10&&!cancelled();++i)vTaskDelay(pdMS_TO_TICKS(100));
            if(!error&&!cancelled())error=fetch(url,false,&r,start);
            if(cancelled())error=CANCELLED;
            if(error)(void)ps_local_update_restore(local_state,sizeof(local_state),saved,saved_size,0,0);
            else if(r.http_status==304)(void)ps_local_update_restore(local_state,sizeof(local_state),saved,saved_size,1,(uint32_t)time(NULL));
            else if(ps_local_metadata(local_state,sizeof(local_state),page_bytes,(uint32_t)time(NULL),200)||ps_local_finish(local_state,sizeof(local_state),1)!=1){(void)ps_local_update_restore(local_state,sizeof(local_state),saved,saved_size,0,0);error=INCOMPLETE;}
            if(!error&&!PS_StoreSave(local_state,sizeof(local_state)))++store_failures;
            if(error){r.error=error;break;}
            ++checked;
        }
        if(checked<count&&!r.error)r.error=cancelled()?CANCELLED:INCOMPLETE;
        conditional_etag[0]=conditional_modified[0]=0;free(saved);if(r.error)goto done;
    }
cached:
    if(!documents){r.error=crawl_error==ROBOTS_DENIED?ROBOTS_DENIED:NO_DOCUMENTS;goto done;}
    if(job.request.kind==6){
        uint64_t reference_us=0,cached_us=0,decode_cycles=0;bool equal=true;unsigned count=0;uint32_t cpu_before=esp_clk_cpu_freq();
        for(unsigned i=0;i<31&&!cancelled();++i){
            int64_t before=esp_timer_get_time();int n=ps_local_packet(local_state,sizeof(local_state),6,job.request.document,job.request.page,(const uint8_t*)"",0,r.bytes);unsigned ref=(unsigned)(esp_timer_get_time()-before);
            before=esp_timer_get_time();int m=ps_local_packet(local_state,sizeof(local_state),2,job.request.document,job.request.page,(const uint8_t*)"",0,r.bytes+512);unsigned fast=(unsigned)(esp_timer_get_time()-before);
            if(n<=0||n!=m||memcmp(r.bytes,r.bytes+512,(size_t)n)){equal=false;break;}
            r.length=(size_t)m;
            PS_FlowView view;uint32_t cycles=esp_cpu_get_cycle_count();int decoded=ps_flow_decode(r.bytes+512,(size_t)m,&view);cycles=esp_cpu_get_cycle_count()-cycles;
            if(decoded){equal=false;break;}
            if(i){reference_us+=ref;cached_us+=fast;decode_cycles+=cycles;++count;}memory(&r);
        }
        uint32_t cpu_after=esp_clk_cpu_freq();
        printf("SEARCH_COMPARE {\"execution_mode\":\"physical-native-cpu\",\"repeats\":%u,\"identical_packets\":%s,\"reference_total_us\":%llu,\"cached_total_us\":%llu,\"decoder_total_cycles\":%llu,\"decoder_allocations\":0,\"cpu_hz_before\":%lu,\"cpu_hz_after\":%lu,\"core\":%u,\"packet_bytes\":%u,\"source_bytes_over_usb\":0}\n",count,equal?"true":"false",(unsigned long long)reference_us,(unsigned long long)cached_us,(unsigned long long)decode_cycles,(unsigned long)cpu_before,(unsigned long)cpu_after,(unsigned)xPortGetCoreID(),(unsigned)r.length);
        r.error=equal&&count==30&&cpu_before==cpu_after?0:INCOMPLETE;goto done;
    }
    if(job.request.kind==3||job.request.kind==7){
        char url[256];
        if(job.request.kind==7){
            // Diagnostic-only original public source files. No caller URL, proxy
            // pixels or host resizing. The selected authority/robots still apply.
            const char *probes[]={"https://www.w3.org/People/Raggett/dsr-46c.jpg","https://www.w3.org/assets/logos/w3c-2025-transitional/w3c-72x48.png","https://www.w3.org/People/Raggett/rib.jpg"};
            if(job.request.document>=3){r.error=INVALID_IMAGE;goto done;}strcpy(url,probes[job.request.document]);
        }else if(ps_local_image_url(local_state,sizeof(local_state),job.request.document,job.request.page,(uint8_t*)url,sizeof(url))<=0){r.error=INVALID_IMAGE;goto done;}
        CW_Status wifi;CW_GetStatus(&wifi);if(wifi.state!=CW_CONNECTED){r.error=NO_WIFI;goto done;}
        if(CW_TimeWait(1000)!=ESP_OK){r.error=NO_CLOCK;goto done;}
        r.clock_synced=true;if(!robots_state){r.error=fetch(robots_url,true,&r,start);if(r.error)goto done;}uint8_t *body=NULL;size_t length=0;r.error=resource(url,false,true,&body,&length,&r,start);
        if(!r.error){
            if(phase_monitor){r.tls_allocator_minimum=(unsigned)heap_caps_get_minimum_free_size(MALLOC_CAP_8BIT);heap_caps_monitor_local_minimum_free_size_stop();phase_monitor=false;}
            if(!image_digest(body,length,r.image_source_sha256)){r.error=INVALID_IMAGE;free(body);goto done;}
            r.decode_heap_window=heap_caps_monitor_local_minimum_free_size_start()==ESP_OK;phase_monitor=r.decode_heap_window;r.decode_heap_before=(unsigned)heap_caps_get_free_size(MALLOC_CAP_8BIT);
            PS_NativeImage image={0};int64_t decode_start=esp_timer_get_time();int error=PS_NativeDecode(body,length,&image,image_cancelled,image_memory,&r);r.compute_us=(unsigned)(esp_timer_get_time()-decode_start);r.image_format=image.format;r.decoder_peak_bytes=image.decoder_peak_bytes;r.decoder_allocations=image.decoder_allocations;
            if(phase_monitor){r.decode_allocator_minimum=(unsigned)heap_caps_get_minimum_free_size(MALLOC_CAP_8BIT);heap_caps_monitor_local_minimum_free_size_stop();phase_monitor=false;}
            if(!error){r.image_pixels=image.pixels;r.image_width=image.width;r.image_height=image.height;image_digest(image.pixels,(size_t)image.width*image.height*2,r.image_pixels_sha256);}
            else r.error=error==PS_IMAGE_CANCELLED?CANCELLED:error==PS_IMAGE_MEMORY?NO_MEMORY:error==PS_IMAGE_LIMIT?TOO_LARGE:error==PS_IMAGE_UNSUPPORTED?PS_NET_IMAGE_UNSUPPORTED:INVALID_IMAGE;
        }free(body);goto done;
    }
    r.tls_verified=true;r.clock_synced=true;r.sample=false;
    memset(r.generation,'0',64);r.generation[64]=0;
    int64_t search_start=esp_timer_get_time();
    int n=ps_local_packet(local_state,sizeof(local_state),job.request.kind==5?1:job.request.kind,job.request.document,job.request.page,(const uint8_t*)job.request.query,strlen(job.request.query),r.bytes);
    r.compute_us=(unsigned)(esp_timer_get_time()-search_start);
    if(n<=0){r.error=INCOMPLETE;goto done;}r.length=(size_t)n;
    if(job.request.kind==2){int slot=ps_local_image_slot(local_state,sizeof(local_state),job.request.document,job.request.page);if(slot>0&&slot<=24)r.image_slot=(uint8_t)slot;}
done:
    if(phase_monitor){r.tls_allocator_minimum=(unsigned)heap_caps_get_minimum_free_size(MALLOC_CAP_8BIT);heap_caps_monitor_local_minimum_free_size_stop();}
    if(cancelled()){if(r.image_pixels){free(r.image_pixels);r.image_pixels=NULL;}r.error=CANCELLED;}
    memory(&r);r.finished=true;r.elapsed_ms=(unsigned)((esp_timer_get_time()-start)/1000);
    printf("SEARCH_NET {\"sequence\":%u,\"execution_mode\":\"on-device\",\"service_required\":false,\"error\":%d,\"http_status\":%d,\"bytes\":%u,\"wire_bytes\":%u,\"documents\":%u,\"image_bytes\":%u,\"image_format\":%u,\"image_width\":%u,\"image_height\":%u,\"image_source_sha256\":\"%s\",\"image_pixels_sha256\":\"%s\",\"decoder_peak_bytes\":%u,\"decoder_allocations\":%u,\"tls_heap_window\":%s,\"tls_allocator_minimum_free_bytes\":%u,\"decode_heap_window\":%s,\"decode_heap_before_bytes\":%u,\"decode_allocator_minimum_free_bytes\":%u,\"compute_us\":%u,\"tls_verified\":%s,\"clock_synced\":%s,\"elapsed_ms\":%u,\"heap_before\":%u,\"minimum_free_heap_bytes\":%u,\"minimum_largest_free_block_bytes\":%u,\"stack_hwm\":%u}\n",r.sequence,r.error,r.http_status,(unsigned)r.length,r.wire_bytes,documents,(unsigned)r.image_width*r.image_height*2,r.image_format,r.image_width,r.image_height,r.image_source_sha256,r.image_pixels_sha256,r.decoder_peak_bytes,r.decoder_allocations,r.tls_heap_window?"true":"false",r.tls_allocator_minimum,r.decode_heap_window?"true":"false",r.decode_heap_before,r.decode_allocator_minimum,r.compute_us,r.tls_verified?"true":"false",r.clock_synced?"true":"false",r.elapsed_ms,r.heap_before,r.minimum_free_heap_bytes,r.largest_free_block_bytes,r.stack_hwm);
    taskENTER_CRITICAL(&lock);memset(job.request.query,0,sizeof(job.request.query));taskEXIT_CRITICAL(&lock);
    publish(&r);vTaskDelete(NULL);
}
bool PS_NetConfigured(void){return true;}
bool PS_NetBusy(void){taskENTER_CRITICAL(&lock);bool busy=job.running||job.ready||job.configuring;taskEXIT_CRITICAL(&lock);return busy;}
bool PS_NetReadViewport(uint32_t document,uint16_t page,uint8_t bytes[512],size_t *length,uint8_t *slot){
    if(!bytes||!length||!slot)return false;
    // Image jobs only read LocalSearch. Hold a read reservation across both
    // encodings so console refresh/profile/start cannot publish a new corpus.
    taskENTER_CRITICAL(&lock);bool allowed=!job.configuring&&!reading&&(!job.running||job.request.kind==3||job.request.kind==7);if(allowed)reading=true;taskEXIT_CRITICAL(&lock);
    if(!allowed)return false;
    int n=ps_local_packet(local_state,sizeof(local_state),2,document,page,(const uint8_t*)"",0,bytes);
    int key=n>0?ps_local_image_slot(local_state,sizeof(local_state),document,page):0;
    taskENTER_CRITICAL(&lock);reading=false;taskEXIT_CRITICAL(&lock);
    if(n<=0||n>512||key<0||key>24)return false;
    *length=(size_t)n;*slot=(uint8_t)key;return true;
}
bool PS_NetStart(const PS_Request *request){
    if(!request||!memchr(request->query,0,sizeof(request->query))||request->kind<1||(request->kind>7&&request->kind!=255))return false;
    taskENTER_CRITICAL(&lock);bool allowed=!reading&&!job.running&&!job.ready&&!job.configuring;if(allowed){job.running=true;job.cancelled=false;job.request=*request;}taskEXIT_CRITICAL(&lock);
    if(!allowed)return false;
    if(xTaskCreatePinnedToCore(worker,"local_search",16384,NULL,2,NULL,0)==pdPASS)return true;
    taskENTER_CRITICAL(&lock);job.running=false;memset(job.request.query,0,sizeof(job.request.query));taskEXIT_CRITICAL(&lock);return false;
}
bool PS_NetPoll(PS_Reply *reply){taskENTER_CRITICAL(&lock);bool ready=job.ready;if(ready){*reply=job.reply;memset(job.reply.bytes,0,sizeof(job.reply.bytes));job.reply.image_pixels=NULL;job.ready=false;}taskEXIT_CRITICAL(&lock);return ready;}
void PS_NetCancel(unsigned sequence){taskENTER_CRITICAL(&lock);if(job.running&&job.request.sequence==sequence)job.cancelled=true;taskEXIT_CRITICAL(&lock);}
bool PS_NetRefresh(void){
    taskENTER_CRITICAL(&lock);bool allowed=!reading&&!job.running&&!job.ready&&!job.configuring;if(allowed)job.configuring=true;taskEXIT_CRITICAL(&lock);
    if(!allowed)return false;
    int error=ps_local_set_profile(local_state,sizeof(local_state),profile);
    if(!error&&!PS_StoreSave(local_state,sizeof(local_state)))++store_failures;
    taskENTER_CRITICAL(&lock);documents=requests=body_bytes=robots_state=0;robots_length=0;collected=false;job.configuring=false;taskEXIT_CRITICAL(&lock);return error==0;
}
unsigned PS_NetProfileCount(void){return ps_local_profiles();}
unsigned PS_NetProfile(void){taskENTER_CRITICAL(&lock);unsigned value=profile;taskEXIT_CRITICAL(&lock);return value;}
bool PS_NetProfileName(unsigned value,char *out,size_t capacity){return ps_local_profile_text(value,0,(uint8_t*)out,capacity)>0;}
bool PS_NetSelectProfile(unsigned value){
    if(value>=PS_NetProfileCount())return false;
    taskENTER_CRITICAL(&lock);bool allowed=!reading&&!job.running&&!job.ready&&!job.configuring;if(allowed)job.configuring=true;taskEXIT_CRITICAL(&lock);
    if(!allowed)return false;
    nvs_handle_t handle;esp_err_t error=nvs_open("pocket_native",NVS_READWRITE,&handle);
    if(error==ESP_OK){error=nvs_set_u8(handle,"profile",(uint8_t)value);if(error==ESP_OK)error=nvs_commit(handle);nvs_close(handle);}
    if(error==ESP_OK)error=ps_local_set_profile(local_state,sizeof(local_state),value)?ESP_FAIL:ESP_OK;
    if(error==ESP_OK)error=ps_local_profile_text(value,3,(uint8_t*)robots_url,sizeof(robots_url))>0?ESP_OK:ESP_FAIL;
    if(error==ESP_OK&&!PS_StoreSave(local_state,sizeof(local_state)))++store_failures;
    taskENTER_CRITICAL(&lock);if(error==ESP_OK){profile=value;documents=requests=body_bytes=robots_state=0;robots_length=0;collected=false;}job.configuring=false;taskEXIT_CRITICAL(&lock);
    return error==ESP_OK;
}
static int status(int argc,char **argv){
    if(argc==3&&!strcmp(argv[1],"profile")){char *end;unsigned long value=strtoul(argv[2],&end,10);return *argv[2]&&!*end&&value<PS_NetProfileCount()&&PS_NetSelectProfile((unsigned)value)?0:1;}
    if(argc==3&&!strcmp(argv[1],"compare")){char *end;unsigned long id=strtoul(argv[2],&end,10);PS_Request request={.kind=6,.document=(uint32_t)id};return *argv[2]&&!*end&&id>0&&id<=4&&PS_NetStart(&request)?0:1;}
    if(argc==2&&(!strcmp(argv[1],"fonts")||!strcmp(argv[1],"jfonts")||!strcmp(argv[1],"jfonts-budget"))){if(PS_NetBusy())return 1;font_cancel_check=0;font_budget_probe=!strcmp(argv[1],"jfonts-budget");font_probe_japanese=strcmp(argv[1],"fonts")!=0;PS_Request request={.kind=255};return PS_NetStart(&request)?0:1;}
    if(argc==3&&!strcmp(argv[1],"jfonts-cancel")){char *end;unsigned long checks=strtoul(argv[2],&end,10);if(!*argv[2]||*end||!checks||checks>100000||PS_NetBusy())return 1;font_probe_japanese=true;font_budget_probe=false;font_cancel_check=(unsigned)checks;PS_Request request={.kind=255};return PS_NetStart(&request)?0:1;}
    if(argc==2&&!strcmp(argv[1],"font-cancel")){taskENTER_CRITICAL(&lock);bool allowed=job.running&&job.request.kind==255&&job.request.sequence==0;if(allowed)job.cancelled=true;taskEXIT_CRITICAL(&lock);return allowed?0:1;}
    if(argc==2&&!strcmp(argv[1],"refresh"))return PS_NetRefresh()?0:1;
    if(argc!=2||strcmp(argv[1],"status"))return 1;
    taskENTER_CRITICAL(&lock);unsigned count=documents,attempts=requests,bytes=body_bytes;bool running=job.running;taskEXIT_CRITICAL(&lock);
    printf("SEARCH_LOCAL {\"execution_mode\":\"on-device\",\"service_required\":false,\"documents\":%u,\"requests\":%u,\"body_bytes\":%u,\"running\":%s,\"state_bytes\":%u,\"state_used_bytes\":%u,\"robots_buffer_bytes\":%u,\"maximum_documents\":4,\"text_bytes_per_document\":2560,\"persistent_index\":%s,\"store_generation\":%u,\"store_failures\":%u,\"native_images\":true,\"native_image_formats\":\"baseline-jpeg,png,progressive-jpeg\",\"source_profile\":%u}\n",count,attempts,bytes,running?"true":"false",(unsigned)sizeof(local_state),(unsigned)ps_local_state_bytes(),(unsigned)sizeof(robots),PS_StoreReady()?"true":"false",PS_StoreGeneration(),store_failures,PS_NetProfile());printf("SEARCH_RESOURCES {\"css_cache_hits\":%u,\"css_requests\":%u,\"font_requests\":%u,\"css_rules\":%u,\"font_ready\":%s,\"font_glyphs\":%u,\"font_source_bytes\":%u,\"atlas_bytes\":8708,\"fallbacks\":%u,\"server_required\":false}\n",css_cache_hits,css_fetches,font_fetches,css_rules,PS_WebFontReady()?"true":"false",font_glyphs,font_bytes,resource_fallbacks);return 0;
}
static int legacy(int argc,char **argv){if(argc!=2||strcmp(argv[1],"status"))return 1;printf("SEARCH_ENDPOINT_STATE {\"configured\":true,\"service_required\":false,\"execution_mode\":\"on-device\"}\n");return 0;}
void PS_NetRegister(void){
    ESP_ERROR_CHECK(ps_local_init(local_state,sizeof(local_state))?ESP_FAIL:ESP_OK);
    nvs_handle_t handle;uint8_t saved=0;if(nvs_open("pocket_native",NVS_READONLY,&handle)==ESP_OK){if(nvs_get_u8(handle,"profile",&saved)!=ESP_OK||saved>=PS_NetProfileCount())saved=0;nvs_close(handle);}
    profile=saved;ESP_ERROR_CHECK(ps_local_set_profile(local_state,sizeof(local_state),profile)?ESP_FAIL:ESP_OK);
    ESP_ERROR_CHECK(ps_local_profile_text(profile,3,(uint8_t*)robots_url,sizeof(robots_url))>0?ESP_OK:ESP_FAIL);
    PS_StoreInit();if(PS_StoreLoad(local_state,sizeof(local_state),profile)){documents=(unsigned)ps_local_count(local_state,sizeof(local_state));collected=ps_local_pending(local_state,sizeof(local_state))==0;}
    const esp_console_cmd_t local={.command="search_local",.help="On-device status/refresh/fonts (explicit public font diagnostic); no page data over USB",.func=status};
    const esp_console_cmd_t old={.command="search_endpoint",.help="Legacy status only; no companion service is used",.func=legacy};
    ESP_ERROR_CHECK(esp_console_cmd_register(&local));ESP_ERROR_CHECK(esp_console_cmd_register(&old));esp_log_level_set("HTTP_CLIENT",ESP_LOG_ERROR);
}
