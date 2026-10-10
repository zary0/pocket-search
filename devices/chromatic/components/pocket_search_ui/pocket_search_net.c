// SPDX-License-Identifier: GPL-3.0-only
// Native Wi-Fi HTTPS only: USB configures an origin, never supplies page contents.
#include "pocket_search_net.h"
#include "chromatic_wifi.h"
#include "network_time.h"
#include "esp_console.h"
#include "esp_crt_bundle.h"
#include "esp_http_client.h"
#include "esp_timer.h"
#include "esp_log.h"
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include <stdio.h>
#include <string.h>
#include <strings.h>
#include <ctype.h>
static portMUX_TYPE lock=portMUX_INITIALIZER_UNLOCKED;
static char origin[192];
static struct {bool running,ready;PS_Request request;PS_Reply reply;char endpoint[192];} job;
enum {NO_WIFI=1,NO_CLOCK,NO_MEMORY,CONNECT_FAILED,HTTP_FAILED,TOO_LARGE,INCOMPLETE,WRONG_TYPE};
typedef struct {PS_Reply *reply;bool correct_type;} Events;
static esp_err_t event(esp_http_client_event_t *e){Events *events=e->user_data;if(e->event_id==HTTP_EVENT_ON_CONNECTED)events->reply->tls_verified=true;if(e->event_id==HTTP_EVENT_ON_HEADER&&e->header_key&&!strcasecmp(e->header_key,"Content-Type"))events->correct_type=e->header_value&&!strcmp(e->header_value,"application/vnd.pocket-search.pcs1");return ESP_OK;}
static void worker(void *arg){
    (void)arg;PS_Reply result={.sequence=job.request.sequence};CW_Status wifi;CW_GetStatus(&wifi);int64_t start=esp_timer_get_time();esp_http_client_handle_t client=NULL;
    char url[576],encoded[289];const char hex[]="0123456789ABCDEF";size_t at=0;Events events={.reply=&result};
    if(wifi.state!=CW_CONNECTED){result.error=NO_WIFI;goto done;}
    if(CW_TimeWait(20000)!=ESP_OK){result.error=NO_CLOCK;goto done;}result.clock_synced=true;
    for(const unsigned char *s=(const unsigned char*)job.request.query;*s;++s){encoded[at++]='%';encoded[at++]=hex[*s>>4];encoded[at++]=hex[*s&15];}encoded[at]=0;
    int written=job.request.kind==1?snprintf(url,sizeof(url),"%s/v1/chromatic/search?q=%s&result=%u",job.endpoint,encoded,job.request.page):snprintf(url,sizeof(url),"%s/v1/chromatic/page?id=%lu&page=%u",job.endpoint,(unsigned long)job.request.document,job.request.page);
    if(written<0||(size_t)written>=sizeof(url)){result.error=TOO_LARGE;goto done;}
    esp_http_client_config_t config={.url=url,.timeout_ms=10000,.crt_bundle_attach=esp_crt_bundle_attach,.skip_cert_common_name_check=false,.disable_auto_redirect=true,.buffer_size=512,.buffer_size_tx=512,.event_handler=event,.user_data=&events};
    client=esp_http_client_init(&config);if(!client){result.error=NO_MEMORY;goto done;}
    esp_http_client_set_header(client,"Accept","application/vnd.pocket-search.pcs1");esp_http_client_set_header(client,"Accept-Encoding","identity");
    if(esp_http_client_open(client,0)!=ESP_OK){result.error=CONNECT_FAILED;goto done;}
    int64_t length=esp_http_client_fetch_headers(client);result.http_status=esp_http_client_get_status_code(client);
    if(result.http_status!=200){result.error=HTTP_FAILED;goto done;}
    if(length>512){result.error=TOO_LARGE;goto done;}
    if(!events.correct_type){result.error=WRONG_TYPE;goto done;}
    while(result.length<sizeof(result.bytes)&&esp_timer_get_time()-start<45000000){int n=esp_http_client_read(client,(char*)result.bytes+result.length,sizeof(result.bytes)-result.length);if(n<0){result.error=INCOMPLETE;goto done;}if(!n)break;result.length+=(size_t)n;}
    if(!esp_http_client_is_complete_data_received(client)){result.error=INCOMPLETE;goto done;}
done:
    if(client)esp_http_client_cleanup(client);
    result.elapsed_ms=(unsigned)((esp_timer_get_time()-start)/1000);
    printf("SEARCH_NET {\"sequence\":%u,\"error\":%d,\"http_status\":%d,\"bytes\":%u,\"tls_verified\":%s,\"clock_synced\":%s,\"elapsed_ms\":%u}\n",result.sequence,result.error,result.http_status,(unsigned)result.length,result.tls_verified?"true":"false",result.clock_synced?"true":"false",result.elapsed_ms);
    taskENTER_CRITICAL(&lock);job.reply=result;memset(job.request.query,0,sizeof(job.request.query));job.running=false;job.ready=true;taskEXIT_CRITICAL(&lock);vTaskDelete(NULL);
}
bool PS_NetConfigured(void){taskENTER_CRITICAL(&lock);bool configured=origin[0]!=0;taskEXIT_CRITICAL(&lock);return configured;}
bool PS_NetStart(const PS_Request *request){
    if(!request||!memchr(request->query,0,sizeof(request->query))||!(request->kind==1||request->kind==2))return false;
    taskENTER_CRITICAL(&lock);bool allowed=!job.running&&!job.ready&&origin[0];if(allowed){job.running=true;job.request=*request;memcpy(job.endpoint,origin,sizeof(origin));}taskEXIT_CRITICAL(&lock);
    if(!allowed)return false;
    if(xTaskCreatePinnedToCore(worker,"search_https",8192,NULL,2,NULL,0)==pdPASS)return true;
    taskENTER_CRITICAL(&lock);job.running=false;memset(job.request.query,0,sizeof(job.request.query));taskEXIT_CRITICAL(&lock);return false;
}
bool PS_NetPoll(PS_Reply *reply){taskENTER_CRITICAL(&lock);bool ready=job.ready;if(ready){*reply=job.reply;memset(job.reply.bytes,0,sizeof(job.reply.bytes));job.ready=false;}taskEXIT_CRITICAL(&lock);return ready;}
static int configure(int argc,char **argv){
    if(argc!=2)return 1;
    const char *value=argv[1];size_t n=strlen(value);if(n<12||n>=sizeof(origin)||strncmp(value,"https://",8))return 1;
    // An HTTPS origin only: no credentials, path, fragment, query, or alternate port.
    for(size_t i=8;i<n;++i)if(!isalnum((unsigned char)value[i])&&value[i]!='-'&&value[i]!='.')return 1;
    taskENTER_CRITICAL(&lock);bool allowed=!job.running&&!job.ready;if(allowed)memcpy(origin,value,n+1);taskEXIT_CRITICAL(&lock);
    if(allowed)printf("SEARCH_ENDPOINT_CONFIGURED\n");
    return allowed?0:1;
}
void PS_NetRegister(void){const esp_console_cmd_t command={.command="search_endpoint",.help="Set an experimental HTTPS origin in RAM only",.func=configure};ESP_ERROR_CHECK(esp_console_cmd_register(&command));esp_log_level_set("HTTP_CLIENT",ESP_LOG_ERROR);}
