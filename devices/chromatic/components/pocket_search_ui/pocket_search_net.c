// SPDX-License-Identifier: GPL-3.0-only
// On-device HTTPS collection and Rust search. No companion service or USB proxy.
#include "pocket_search_net.h"
#include "pocket_typography.h"
#include "pocket_resource_policy.h"
#include "chromatic_wifi.h"
#include "network_time.h"
#include "esp_console.h"
#include "esp_crt_bundle.h"
#include "esp_http_client.h"
#include "esp_timer.h"
#include "esp_log.h"
#include "esp_heap_caps.h"
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include <stdio.h>
#include <string.h>
#include <strings.h>
#include <stdlib.h>
#include <stdatomic.h>

#define LOCAL_STATE_BYTES 16640
#define HTML_LIMIT 65536
#define ROBOTS_LIMIT 2048
#define COLLECTION_US 90000000
static const char base[]="https://www-formal.stanford.edu/jmc/whatisai/";
static const char robots_url[]="https://www-formal.stanford.edu/robots.txt";
static portMUX_TYPE lock=portMUX_INITIALIZER_UNLOCKED;
static _Alignas(8) uint8_t local_state[LOCAL_STATE_BYTES];
static uint8_t robots[ROBOTS_LIMIT+1];
static size_t robots_length;
static unsigned robots_state,documents,requests,body_bytes;
static bool collected;
static char installed_font_url[256];
static _Atomic unsigned css_cache_hits,css_fetches,font_fetches,font_glyphs,font_bytes,css_rules,resource_fallbacks;
static const char probe_css[]="https://fonts.googleapis.com/css?family=VT323&text=WEB%20FONT%20TEST%20Pocket%20Search%20ABC%20xyz%20012345%20Small%20caption%20Downloaded%20TTF";
static struct {bool running,ready,cancelled,configuring;PS_Request request;PS_Reply reply;} job;
enum {NO_WIFI=1,NO_CLOCK,NO_MEMORY,CONNECT_FAILED,HTTP_FAILED,TOO_LARGE,INCOMPLETE,WRONG_TYPE,CANCELLED,INVALID_IMAGE,STALE_COLLECTION,ROBOTS_DENIED,NO_DOCUMENTS};
typedef struct {PS_Reply *reply;bool html,plain,css,font,identity;} Events;
size_t ps_local_state_bytes(void);
int32_t ps_local_init(uint8_t*,size_t);
int32_t ps_local_next(uint8_t*,size_t,uint8_t*,size_t);
int32_t ps_local_feed(uint8_t*,size_t,const uint8_t*,size_t);
int32_t ps_local_finish(uint8_t*,size_t,int32_t);
int32_t ps_local_count(uint8_t*,size_t);
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
static esp_err_t event(esp_http_client_event_t *e){
    Events *v=e->user_data;
    if(e->event_id==HTTP_EVENT_ON_CONNECTED)v->reply->tls_verified=true;
    if(e->event_id==HTTP_EVENT_ON_HEADER&&e->header_key){
        if(!strcasecmp(e->header_key,"Content-Type")){v->html=media(e->header_value,"text/html");v->plain=media(e->header_value,"text/plain");v->css=media(e->header_value,"text/css");v->font=media(e->header_value,"font/ttf")||media(e->header_value,"application/octet-stream")||media(e->header_value,"application/x-font-ttf")||media(e->header_value,"application/font-sfnt");}
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
    if(strncmp(url,base,sizeof(base)-1))return false;
    const char *s=url+sizeof(base)-1;if(strncmp(s,"node",4))return false;s+=4;
    unsigned n=0;while(*s>='0'&&*s<='9'){++s;++n;}return n>=1&&n<=2&&!strcmp(s,".html");
}
static int fetch(const char *url,bool robot,PS_Reply *r,int64_t start){
    Events events={.reply=r,.identity=true};int error=0;size_t total=0;esp_http_client_handle_t client=NULL;
    const size_t limit=robot?ROBOTS_LIMIT:HTML_LIMIT;
    esp_http_client_config_t config={.url=url,.timeout_ms=8000,.crt_bundle_attach=esp_crt_bundle_attach,.skip_cert_common_name_check=false,.disable_auto_redirect=true,.buffer_size=512,.buffer_size_tx=512,.event_handler=event,.user_data=&events};
    client=esp_http_client_init(&config);if(!client)return NO_MEMORY;
    esp_http_client_set_header(client,"Accept",robot?"text/plain":"text/html");esp_http_client_set_header(client,"Accept-Encoding","identity");
    esp_http_client_set_header(client,"User-Agent","PocketSearchBot/0.1 (bounded handheld research; https://github.com/zary0/pocket-search)");
    taskENTER_CRITICAL(&lock);++requests;taskEXIT_CRITICAL(&lock);
    if(esp_http_client_open(client,0)!=ESP_OK){error=CONNECT_FAILED;goto done;}
    int64_t length=esp_http_client_fetch_headers(client);r->http_status=esp_http_client_get_status_code(client);memory(r);
    // A confirmed missing robots resource permits this bounded crawl. Other
    // errors, redirects and unsupported/oversize policies fail closed.
    if(robot&&(r->http_status==404||r->http_status==410)){robots_state=2;goto done;}
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
done:
    taskENTER_CRITICAL(&lock);body_bytes+=(unsigned)total;taskEXIT_CRITICAL(&lock);
    r->wire_bytes+=(unsigned)total;memory(r);esp_http_client_cleanup(client);return error;
}
// Exact public authorities, no redirect, port, userinfo, local address or arbitrary host.
static int resource(const char *url,bool font,uint8_t **out,size_t *out_length,PS_Reply *r,int64_t start){
    *out=NULL;*out_length=0;if(!PS_ResourceURL(url,font))return WRONG_TYPE;
    Events events={.reply=r,.identity=true};int error=0;size_t total=0;uint8_t *body=NULL;
    size_t limit=font?24576:3072;
    esp_http_client_config_t config={.url=url,.timeout_ms=8000,.crt_bundle_attach=esp_crt_bundle_attach,.skip_cert_common_name_check=false,.disable_auto_redirect=true,.buffer_size=512,.buffer_size_tx=512,.event_handler=event,.user_data=&events};
    esp_http_client_handle_t client=esp_http_client_init(&config);if(!client)return NO_MEMORY;
    esp_http_client_set_header(client,"Accept",font?"font/ttf,application/octet-stream":"text/css");
    esp_http_client_set_header(client,"Accept-Encoding","identity");esp_http_client_set_header(client,"User-Agent","PocketSearchBot/0.1");
    taskENTER_CRITICAL(&lock);++requests;if(font)++font_fetches;else ++css_fetches;taskEXIT_CRITICAL(&lock);
    if(cancelled()||esp_timer_get_time()-start>=COLLECTION_US){error=CANCELLED;goto done;}
    if(esp_http_client_open(client,0)!=ESP_OK){error=CONNECT_FAILED;goto done;}
    int64_t length=esp_http_client_fetch_headers(client);r->http_status=esp_http_client_get_status_code(client);memory(r);
    if(r->http_status!=200){error=HTTP_FAILED;goto done;}
    if(!events.identity||(font?!events.font:!events.css)){error=WRONG_TYPE;goto done;}
    if(length>(int64_t)limit){error=TOO_LARGE;goto done;}
    // Allocate after the TLS handshake, and release raw font bytes before the next socket.
    size_t capacity=length>0?(size_t)length:limit;body=malloc(capacity);if(!body){error=NO_MEMORY;goto done;}
    while(!cancelled()&&esp_timer_get_time()-start<COLLECTION_US){
        int n=esp_http_client_read(client,(char*)r->bytes,512);if(n<0){error=INCOMPLETE;goto done;}if(!n)break;
        if(total+(size_t)n>capacity){error=TOO_LARGE;goto done;}memcpy(body+total,r->bytes,(size_t)n);total+=(size_t)n;memory(r);
    }
    if(cancelled()){error=CANCELLED;goto done;}
    if(!esp_http_client_is_complete_data_received(client)){error=INCOMPLETE;goto done;}
    *out=body;*out_length=total;body=NULL;
done:
    free(body);r->wire_bytes+=(unsigned)total;taskENTER_CRITICAL(&lock);body_bytes+=(unsigned)total;taskEXIT_CRITICAL(&lock);
    memory(r);esp_http_client_cleanup(client);return error;
}
static bool load_face(PS_Reply *r,int64_t start,int *cause){
    char url[256];if(ps_local_resource(local_state,sizeof(local_state),1,(uint8_t*)url)<=0){if(cause)*cause=WRONG_TYPE;return false;}
    // Existing readers retain the same immutable atlas, even during refresh.
    if(PS_WebFontReady()){bool same=!strcmp(url,installed_font_url);if(!same&&cause)*cause=WRONG_TYPE;return same;}
    uint8_t *body=NULL;size_t length=0;
    int error=resource(url,true,&body,&length,r,start);if(error){if(cause)*cause=error;++resource_fallbacks;return false;}
    int n=PS_WebFontLoad(body,length);free(body);
    if(n<=0){if(cause)*cause=WRONG_TYPE;++resource_fallbacks;return false;}
    memcpy(installed_font_url,url,strlen(url)+1);font_glyphs=(unsigned)n;font_bytes=(unsigned)length;memory(r);return true;
}
static bool load_css(const char *url,PS_Reply *r,int64_t start,bool *ready,int *cause){
    if(ps_local_css_cached(local_state,sizeof(local_state),(const uint8_t*)url,strlen(url))==1){
        ++css_cache_hits;*ready=load_face(r,start,cause);return ps_local_css_count(local_state,sizeof(local_state))>0||*ready;
    }
    uint8_t *body=NULL;size_t length=0;int error=resource(url,false,&body,&length,r,start);
    if(error){if(cause)*cause=error;++resource_fallbacks;return false;}
    int rules=ps_local_css(local_state,sizeof(local_state),body,length,(const uint8_t*)url,strlen(url));free(body);
    if(rules<0){if(cause)*cause=WRONG_TYPE;++resource_fallbacks;return false;}css_rules+=(unsigned)rules;
    *ready=load_face(r,start,cause);return rules>0||*ready;
}
static void worker(void *arg){
    (void)arg;PS_Reply r={.sequence=job.request.sequence,.local=true};int64_t start=esp_timer_get_time();int crawl_error=0;
    r.heap_before=(unsigned)heap_caps_get_free_size(MALLOC_CAP_8BIT);r.minimum_free_heap_bytes=r.heap_before;r.largest_free_block_bytes=(unsigned)heap_caps_get_largest_free_block(MALLOC_CAP_8BIT);
    // Cached search/reader work never requires a socket, Wi-Fi, clock sync or
    // another device. Verified flags describe the retained source provenance.
    if((!collected&&job.request.kind==1)||job.request.kind==255){
        CW_Status wifi;CW_GetStatus(&wifi);if(wifi.state!=CW_CONNECTED){r.error=NO_WIFI;goto done;}
        esp_err_t clock_error;int64_t clock_deadline=start+20000000;
        do{if(cancelled()){r.error=CANCELLED;goto done;}clock_error=CW_TimeWait(200);}while(clock_error==ESP_ERR_TIMEOUT&&esp_timer_get_time()<clock_deadline);
        if(clock_error!=ESP_OK){r.error=NO_CLOCK;goto done;}
        r.clock_synced=true;
        if(job.request.kind==255){
            bool ready=false;
            int cause=0;if(!load_css(probe_css,&r,start,&ready,&cause)||!ready)r.error=cause?cause:WRONG_TYPE;
            goto done;
        }
        if(!robots_state){crawl_error=fetch(robots_url,true,&r,start);if(crawl_error)robots_state=3;}
        if(robots_state==3){r.error=ROBOTS_DENIED;goto done;}
        char url[128];
        while(esp_timer_get_time()-start<COLLECTION_US&&!cancelled()){
            int next=ps_local_next(local_state,sizeof(local_state),(uint8_t*)url,sizeof(url));if(next<=0)break;
            const char *path=strchr(url+8,'/');
            bool allowed=valid_url(url)&&path&&(robots_state==2||ps_local_robots(robots,robots_length,(const uint8_t*)path,strlen(path))==1);
            if(!allowed){(void)ps_local_finish(local_state,sizeof(local_state),0);crawl_error=ROBOTS_DENIED;continue;}
            // One request at a time, at least one second between page starts.
            for(unsigned i=0;i<10&&!cancelled();++i)vTaskDelay(pdMS_TO_TICKS(100));
            if(cancelled()){(void)ps_local_finish(local_state,sizeof(local_state),0);break;}
            int error=fetch(url,false,&r,start);
            char css_url[256];bool ready=false;
            if(!error&&ps_local_resource(local_state,sizeof(local_state),0,(uint8_t*)css_url)>0&&load_css(css_url,&r,start,&ready,NULL)){
                // No retained DOM: refetch once with the bounded cascade, then publish.
                size_t saved_length=ps_local_backup_bytes();uint8_t *saved=saved_length<=3072?calloc(1,saved_length):NULL;
                if(saved&&ps_local_backup(local_state,sizeof(local_state),saved,saved_length,0)==0){
                    ps_local_reparse(local_state,sizeof(local_state),ready?1:0);
                    for(unsigned i=0;i<10&&!cancelled();++i)vTaskDelay(pdMS_TO_TICKS(100));
                    error=fetch(url,false,&r,start);
                    if(error&&!cancelled()){
                        ps_local_backup(local_state,sizeof(local_state),saved,saved_length,1);++resource_fallbacks;error=0;
                    }
                }else ++resource_fallbacks;
                free(saved);
            }
            (void)ps_local_finish(local_state,sizeof(local_state),error==0);
            taskENTER_CRITICAL(&lock);documents=(unsigned)ps_local_count(local_state,sizeof(local_state));taskEXIT_CRITICAL(&lock);
            if(error)crawl_error=error;
        }
        if(cancelled()){r.error=CANCELLED;goto done;}
        collected=true;
    }
    if(!documents){r.error=crawl_error==ROBOTS_DENIED?ROBOTS_DENIED:NO_DOCUMENTS;goto done;}
    r.tls_verified=true;r.clock_synced=true;r.sample=false;
    memset(r.generation,'0',64);r.generation[64]=0;
    int64_t search_start=esp_timer_get_time();
    int n=ps_local_packet(local_state,sizeof(local_state),job.request.kind,job.request.document,job.request.page,(const uint8_t*)job.request.query,strlen(job.request.query),r.bytes);
    r.compute_us=(unsigned)(esp_timer_get_time()-search_start);
    if(n<=0){r.error=INCOMPLETE;goto done;}r.length=(size_t)n;
done:
    memory(&r);r.finished=true;r.elapsed_ms=(unsigned)((esp_timer_get_time()-start)/1000);
    printf("SEARCH_NET {\"sequence\":%u,\"execution_mode\":\"on-device\",\"service_required\":false,\"error\":%d,\"http_status\":%d,\"bytes\":%u,\"wire_bytes\":%u,\"documents\":%u,\"compute_us\":%u,\"tls_verified\":%s,\"clock_synced\":%s,\"elapsed_ms\":%u,\"heap_before\":%u,\"minimum_free_heap_bytes\":%u,\"minimum_largest_free_block_bytes\":%u,\"stack_hwm\":%u}\n",r.sequence,r.error,r.http_status,(unsigned)r.length,r.wire_bytes,documents,r.compute_us,r.tls_verified?"true":"false",r.clock_synced?"true":"false",r.elapsed_ms,r.heap_before,r.minimum_free_heap_bytes,r.largest_free_block_bytes,r.stack_hwm);
    taskENTER_CRITICAL(&lock);memset(job.request.query,0,sizeof(job.request.query));taskEXIT_CRITICAL(&lock);
    publish(&r);vTaskDelete(NULL);
}
bool PS_NetConfigured(void){return true;}
bool PS_NetStart(const PS_Request *request){
    if(!request||!memchr(request->query,0,sizeof(request->query))||request->kind<1||(request->kind>2&&request->kind!=255))return false;
    taskENTER_CRITICAL(&lock);bool allowed=!job.running&&!job.ready&&!job.configuring;if(allowed){job.running=true;job.cancelled=false;job.request=*request;}taskEXIT_CRITICAL(&lock);
    if(!allowed)return false;
    if(xTaskCreatePinnedToCore(worker,"local_search",10240,NULL,2,NULL,0)==pdPASS)return true;
    taskENTER_CRITICAL(&lock);job.running=false;memset(job.request.query,0,sizeof(job.request.query));taskEXIT_CRITICAL(&lock);return false;
}
bool PS_NetPoll(PS_Reply *reply){taskENTER_CRITICAL(&lock);bool ready=job.ready;if(ready){*reply=job.reply;memset(job.reply.bytes,0,sizeof(job.reply.bytes));job.ready=false;}taskEXIT_CRITICAL(&lock);return ready;}
void PS_NetCancel(unsigned sequence){taskENTER_CRITICAL(&lock);if(job.running&&job.request.sequence==sequence)job.cancelled=true;taskEXIT_CRITICAL(&lock);}
bool PS_NetRefresh(void){
    taskENTER_CRITICAL(&lock);bool allowed=!job.running&&!job.ready&&!job.configuring;if(allowed)job.configuring=true;taskEXIT_CRITICAL(&lock);
    if(!allowed)return false;
    int error=ps_local_init(local_state,sizeof(local_state));
    taskENTER_CRITICAL(&lock);documents=requests=body_bytes=robots_state=0;robots_length=0;collected=false;job.configuring=false;taskEXIT_CRITICAL(&lock);return error==0;
}
static int status(int argc,char **argv){
    if(argc==2&&!strcmp(argv[1],"fonts")){PS_Request request={.kind=255};return PS_NetStart(&request)?0:1;}
    if(argc==2&&!strcmp(argv[1],"refresh"))return PS_NetRefresh()?0:1;
    if(argc!=2||strcmp(argv[1],"status"))return 1;
    taskENTER_CRITICAL(&lock);unsigned count=documents,attempts=requests,bytes=body_bytes;bool running=job.running;taskEXIT_CRITICAL(&lock);
    printf("SEARCH_LOCAL {\"execution_mode\":\"on-device\",\"service_required\":false,\"documents\":%u,\"requests\":%u,\"body_bytes\":%u,\"running\":%s,\"state_bytes\":%u,\"state_used_bytes\":%u,\"robots_buffer_bytes\":%u,\"maximum_documents\":4,\"text_bytes_per_document\":2560,\"persistent_index\":false,\"native_images\":false}\n",count,attempts,bytes,running?"true":"false",(unsigned)sizeof(local_state),(unsigned)ps_local_state_bytes(),(unsigned)sizeof(robots));printf("SEARCH_RESOURCES {\"css_cache_hits\":%u,\"css_requests\":%u,\"font_requests\":%u,\"css_rules\":%u,\"font_ready\":%s,\"font_glyphs\":%u,\"font_source_bytes\":%u,\"atlas_bytes\":867,\"fallbacks\":%u,\"server_required\":false}\n",css_cache_hits,css_fetches,font_fetches,css_rules,PS_WebFontReady()?"true":"false",font_glyphs,font_bytes,resource_fallbacks);return 0;
}
static int legacy(int argc,char **argv){if(argc!=2||strcmp(argv[1],"status"))return 1;printf("SEARCH_ENDPOINT_STATE {\"configured\":true,\"service_required\":false,\"execution_mode\":\"on-device\"}\n");return 0;}
void PS_NetRegister(void){
    ESP_ERROR_CHECK(ps_local_init(local_state,sizeof(local_state))?ESP_FAIL:ESP_OK);
    const esp_console_cmd_t local={.command="search_local",.help="On-device status/refresh/fonts (explicit public font diagnostic); no page data over USB",.func=status};
    const esp_console_cmd_t old={.command="search_endpoint",.help="Legacy status only; no companion service is used",.func=legacy};
    ESP_ERROR_CHECK(esp_console_cmd_register(&local));ESP_ERROR_CHECK(esp_console_cmd_register(&old));esp_log_level_set("HTTP_CLIENT",ESP_LOG_ERROR);
}
