// SPDX-License-Identifier: GPL-3.0-only
// Wi-Fi fetches scene/image data; USB configures the origin and synthetic controls.
#include "pocket_search_net.h"
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
#include <ctype.h>
static portMUX_TYPE lock=portMUX_INITIALIZER_UNLOCKED;
static char origin[192];
static struct {bool running,ready,cancelled;PS_Request request;PS_Reply reply;char endpoint[192];} job;
enum {NO_WIFI=1,NO_CLOCK,NO_MEMORY,CONNECT_FAILED,HTTP_FAILED,TOO_LARGE,INCOMPLETE,WRONG_TYPE,CANCELLED,INVALID_IMAGE,STALE_COLLECTION};
typedef struct {PS_Reply *reply;bool correct_type,generation_seen,sample_seen,collection_error;const char *expected;} Events;
static esp_err_t event(esp_http_client_event_t *e){
    Events *events=e->user_data;
    if(e->event_id==HTTP_EVENT_ON_CONNECTED)events->reply->tls_verified=true;
    if(e->event_id==HTTP_EVENT_ON_HEADER&&e->header_key&&!strcasecmp(e->header_key,"Content-Type"))events->correct_type=e->header_value&&!strcmp(e->header_value,events->expected);
    if(e->event_id==HTTP_EVENT_ON_HEADER&&e->header_key&&!strcasecmp(e->header_key,"X-Pocket-Collection")){
        if(events->generation_seen||!ps_generation_valid(e->header_value))events->collection_error=true;
        else memcpy(events->reply->generation,e->header_value,strlen(e->header_value)+1);
        events->generation_seen=true;
    }
    if(e->event_id==HTTP_EVENT_ON_HEADER&&e->header_key&&!strcasecmp(e->header_key,"X-Pocket-Sample")){
        if(events->sample_seen||!e->header_value||(strcmp(e->header_value,"0")&&strcmp(e->header_value,"1")))events->collection_error=true;
        else events->reply->sample=!strcmp(e->header_value,"1");
        events->sample_seen=true;
    }
    return ESP_OK;
}
static bool cancelled(void){taskENTER_CRITICAL(&lock);bool value=job.cancelled;taskEXIT_CRITICAL(&lock);return value;}
static void memory(PS_Reply *r){
    unsigned free=(unsigned)heap_caps_get_free_size(MALLOC_CAP_8BIT);
    unsigned block=(unsigned)heap_caps_get_largest_free_block(MALLOC_CAP_8BIT);
    if(free<r->minimum_free_heap_bytes)r->minimum_free_heap_bytes=free;
    if(block<r->largest_free_block_bytes)r->largest_free_block_bytes=block;
    r->stack_hwm=(unsigned)uxTaskGetStackHighWaterMark(NULL);
}
static void publish(const PS_Reply *reply,bool final){
    taskENTER_CRITICAL(&lock);
    if(final&&job.ready){
        // Preserve the pending bytes and report failure if the GUI missed its deadline.
        job.reply.error=reply->error?reply->error:INCOMPLETE;
        job.reply.finished=true;
        job.reply.elapsed_ms=reply->elapsed_ms;
    }else{job.reply=*reply;job.ready=true;}
    if(final)job.running=false;
    taskEXIT_CRITICAL(&lock);
}
static bool consumed(int64_t start){
    while(esp_timer_get_time()-start<45000000){
        taskENTER_CRITICAL(&lock);bool ready=job.ready;taskEXIT_CRITICAL(&lock);
        if(!ready)return true;
        vTaskDelay(pdMS_TO_TICKS(10));
    }return false;
}
static bool exact(esp_http_client_handle_t client,uint8_t *bytes,size_t size,int64_t start){
    size_t used=0;
    while(used<size&&esp_timer_get_time()-start<45000000&&!cancelled()){
        int n=esp_http_client_read(client,(char*)bytes+used,size-used);
        if(n<=0)return false;
        used+=(size_t)n;
    }return used==size;
}
static void worker(void *arg){
    (void)arg;PS_Reply result={.sequence=job.request.sequence};
    result.heap_before=(unsigned)heap_caps_get_free_size(MALLOC_CAP_8BIT);result.minimum_free_heap_bytes=result.heap_before;result.largest_free_block_bytes=(unsigned)heap_caps_get_largest_free_block(MALLOC_CAP_8BIT);
    CW_Status wifi;CW_GetStatus(&wifi);int64_t start=esp_timer_get_time();esp_http_client_handle_t client=NULL;
    char url[672],encoded[289];const char hex[]="0123456789ABCDEF";size_t at=0;
    bool image=job.request.kind==3;
    Events events={.reply=&result,.expected=image?"application/vnd.pocket-search.pci1":"application/vnd.pocket-search.pcs1"};
    if(wifi.state!=CW_CONNECTED){result.error=NO_WIFI;goto done;}
    esp_err_t clock_error;int64_t clock_deadline=esp_timer_get_time()+20000000;
    do{
        if(cancelled()){result.error=CANCELLED;goto done;}
        clock_error=CW_TimeWait(200); // No SNTP restart: bounded polling of existing state.
    }while(clock_error==ESP_ERR_TIMEOUT&&esp_timer_get_time()<clock_deadline);
    if(clock_error!=ESP_OK){result.error=NO_CLOCK;goto done;}result.clock_synced=true;
    if(cancelled()){result.error=CANCELLED;goto done;}
    for(const unsigned char *s=(const unsigned char*)job.request.query;*s;++s){encoded[at++]='%';encoded[at++]=hex[*s>>4];encoded[at++]=hex[*s&15];}encoded[at]=0;
    int written=job.request.kind==1?snprintf(url,sizeof(url),"%s/v1/chromatic/search?q=%s&result=%u",job.endpoint,encoded,job.request.page):snprintf(url,sizeof(url),"%s/v1/chromatic/%s?id=%lu&page=%u",job.endpoint,image?"image":"page",(unsigned long)job.request.document,job.request.page);
    if(written<0||(size_t)written>=sizeof(url)){result.error=TOO_LARGE;goto done;}
    if(job.request.generation[0]){size_t used=(size_t)written;written=snprintf(url+used,sizeof(url)-used,"&generation=%s",job.request.generation);if(written<0||(size_t)written>=sizeof(url)-used){result.error=TOO_LARGE;goto done;}}
    esp_http_client_config_t config={.url=url,.timeout_ms=10000,.crt_bundle_attach=esp_crt_bundle_attach,.skip_cert_common_name_check=false,.disable_auto_redirect=true,.buffer_size=512,.buffer_size_tx=512,.event_handler=event,.user_data=&events};
    client=esp_http_client_init(&config);if(!client){result.error=NO_MEMORY;goto done;}
    esp_http_client_set_header(client,"Accept",events.expected);esp_http_client_set_header(client,"Accept-Encoding","identity");
    if(esp_http_client_open(client,0)!=ESP_OK){result.error=CONNECT_FAILED;goto done;}
    int64_t length=esp_http_client_fetch_headers(client);result.http_status=esp_http_client_get_status_code(client);memory(&result);
    if(result.http_status!=200){result.error=result.http_status==409?STALE_COLLECTION:HTTP_FAILED;goto done;}
    if(events.collection_error||!events.generation_seen||!events.sample_seen||(job.request.generation[0]&&strcmp(job.request.generation,result.generation))){result.error=STALE_COLLECTION;goto done;}
    if(length>(image?PS_IMAGE_WIRE_BYTES:512)){result.error=TOO_LARGE;goto done;}
    if(!events.correct_type){result.error=WRONG_TYPE;goto done;}
    if(!image){
        while(result.length<512&&esp_timer_get_time()-start<45000000&&!cancelled()){
            int n=esp_http_client_read(client,(char*)result.bytes+result.length,512-result.length);
            if(n<0){result.error=INCOMPLETE;goto done;}
            if(!n)break;
            result.length+=(size_t)n;
        }
        result.wire_bytes=(unsigned)result.length;
        if(!esp_http_client_is_complete_data_received(client)){result.error=cancelled()?CANCELLED:INCOMPLETE;goto done;}
    }else{
        unsigned y=0,width=0,height=0;
        while(y<height||!height){
            if(!exact(client,result.bytes,24,start)){result.error=cancelled()?CANCELLED:INCOMPLETE;goto done;}
            size_t payload=(size_t)result.bytes[18]|((size_t)result.bytes[19]<<8);
            if(payload>2048){result.error=TOO_LARGE;goto done;}
            if(!exact(client,result.bytes+24,payload,start)){result.error=cancelled()?CANCELLED:INCOMPLETE;goto done;}
            result.length=24+payload;PS_ImageView view;
            if(ps_image_decode(result.bytes,result.length,&view)!=0||view.document_id!=job.request.document||view.page_index!=job.request.page||view.y!=y||(height&&(view.width!=width||view.height!=height))){result.error=INVALID_IMAGE;goto done;}
            width=view.width;height=view.height;y+=view.rows;result.wire_bytes+=(unsigned)result.length;memory(&result);
            if(y==height){
                uint8_t extra;
                if(!esp_http_client_is_complete_data_received(client)){
                    int n=esp_http_client_read(client,(char*)&extra,1);
                    if(n!=0||!esp_http_client_is_complete_data_received(client)){result.error=INCOMPLETE;goto done;}
                }
                break;
            }
            result.finished=false;publish(&result,false);
            if(!consumed(start)){result.error=INCOMPLETE;goto done;}
            if(cancelled()){result.error=CANCELLED;goto done;}
        }
    }
done:
    // A timeout during GUI consumption must not overwrite an unconsumed stripe.
    (void)consumed(start);
    memory(&result);if(client)esp_http_client_cleanup(client);
    result.finished=true;result.elapsed_ms=(unsigned)((esp_timer_get_time()-start)/1000);
    printf("SEARCH_NET {\"sequence\":%u,\"error\":%d,\"http_status\":%d,\"bytes\":%u,\"wire_bytes\":%u,\"tls_verified\":%s,\"clock_synced\":%s,\"elapsed_ms\":%u,\"heap_before\":%u,\"minimum_free_heap_bytes\":%u,\"minimum_largest_free_block_bytes\":%u,\"stack_hwm\":%u}\n",result.sequence,result.error,result.http_status,(unsigned)result.length,result.wire_bytes,result.tls_verified?"true":"false",result.clock_synced?"true":"false",result.elapsed_ms,result.heap_before,result.minimum_free_heap_bytes,result.largest_free_block_bytes,result.stack_hwm);
    taskENTER_CRITICAL(&lock);memset(job.request.query,0,sizeof(job.request.query));taskEXIT_CRITICAL(&lock);
    publish(&result,true);vTaskDelete(NULL);
}
bool PS_NetConfigured(void){taskENTER_CRITICAL(&lock);bool configured=origin[0]!=0;taskEXIT_CRITICAL(&lock);return configured;}
bool PS_NetStart(const PS_Request *request){
    if(!request||!memchr(request->query,0,sizeof(request->query))||!memchr(request->generation,0,sizeof(request->generation))||(request->generation[0]&&!ps_generation_valid(request->generation))||request->kind<1||request->kind>3)return false;
    taskENTER_CRITICAL(&lock);bool allowed=!job.running&&!job.ready&&origin[0];if(allowed){job.running=true;job.cancelled=false;job.request=*request;memcpy(job.endpoint,origin,sizeof(origin));}taskEXIT_CRITICAL(&lock);
    if(!allowed)return false;
    if(xTaskCreatePinnedToCore(worker,"search_https",10240,NULL,2,NULL,0)==pdPASS)return true;
    taskENTER_CRITICAL(&lock);job.running=false;memset(job.request.query,0,sizeof(job.request.query));taskEXIT_CRITICAL(&lock);return false;
}
bool PS_NetPoll(PS_Reply *reply){taskENTER_CRITICAL(&lock);bool ready=job.ready;if(ready){*reply=job.reply;memset(job.reply.bytes,0,sizeof(job.reply.bytes));job.ready=false;}taskEXIT_CRITICAL(&lock);return ready;}
void PS_NetCancel(unsigned sequence){taskENTER_CRITICAL(&lock);if(job.running&&job.request.sequence==sequence)job.cancelled=true;taskEXIT_CRITICAL(&lock);}
static int configure(int argc,char **argv){
    if(argc!=2)return 1;
    const char *value=argv[1];size_t n=strlen(value);if(n<12||n>=sizeof(origin)||strncmp(value,"https://",8))return 1;
    for(size_t i=8;i<n;++i)if(!isalnum((unsigned char)value[i])&&value[i]!='-'&&value[i]!='.')return 1;
    taskENTER_CRITICAL(&lock);bool allowed=!job.running&&!job.ready;if(allowed)memcpy(origin,value,n+1);taskEXIT_CRITICAL(&lock);
    if(allowed)printf("SEARCH_ENDPOINT_CONFIGURED\n");
    return allowed?0:1;
}
void PS_NetRegister(void){const esp_console_cmd_t command={.command="search_endpoint",.help="Experimental HTTPS origin in RAM only",.func=configure};ESP_ERROR_CHECK(esp_console_cmd_register(&command));esp_log_level_set("HTTP_CLIENT",ESP_LOG_ERROR);}
