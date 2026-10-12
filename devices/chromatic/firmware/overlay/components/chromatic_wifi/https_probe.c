// SPDX-License-Identifier: GPL-3.0-only
// A bounded, device-side probe. USB carries the command/result only, never the page.
#include "chromatic_wifi.h"
#include "network_time.h"
#include "cJSON.h"
#include "esp_console.h"
#include "esp_crt_bundle.h"
#include "esp_heap_caps.h"
#include "esp_http_client.h"
#include "esp_system.h"
#include "esp_timer.h"
#include "esp_tls.h"
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include "lwip/netdb.h"
#include "lwip/sockets.h"
#include "mbedtls/sha256.h"
#include <errno.h>
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/time.h>
#include <unistd.h>

typedef struct { bool connected; int tls_code,tls_flags; } ProbeEvents;
static portMUX_TYPE probe_lock=portMUX_INITIALIZER_UNLOCKED;
static bool probe_running;

static esp_err_t probe_event(esp_http_client_event_t *event)
{
    ProbeEvents *p=event->user_data;
    if(event->event_id==HTTP_EVENT_ON_CONNECTED)p->connected=true;
    if(event->event_id==HTTP_EVENT_DISCONNECTED&&event->data){
        int code=0,flags=0;
        esp_tls_get_and_clear_last_error((esp_tls_error_handle_t)event->data,&code,&flags);
        if(code)p->tls_code=code;
        if(flags)p->tls_flags=flags;
    }
    return ESP_OK;
}

static bool probe_tcp(const struct sockaddr *address,socklen_t size,int *failure)
{
    int fd=socket(AF_INET,SOCK_STREAM,IPPROTO_TCP);
    if(fd<0){*failure=errno;return false;}
    int flags=fcntl(fd,F_GETFL,0);
    bool ok=flags>=0&&fcntl(fd,F_SETFL,flags|O_NONBLOCK)==0;
    if(ok&&connect(fd,address,size)!=0){
        if(errno!=EINPROGRESS){ok=false;*failure=errno;}
        else{
            fd_set writefds;FD_ZERO(&writefds);FD_SET(fd,&writefds);
            struct timeval timeout={.tv_sec=8};
            int ready=select(fd+1,NULL,&writefds,NULL,&timeout);
            int error=0;socklen_t len=sizeof(error);
            ok=ready>0&&getsockopt(fd,SOL_SOCKET,SO_ERROR,&error,&len)==0&&error==0;
            if(!ok)*failure=ready==0?ETIMEDOUT:error?error:errno;
        }
    }
    close(fd);return ok;
}

static void probe_task(void *arg)
{
    (void)arg;CW_Status wifi;CW_GetStatus(&wifi);
    int64_t started=esp_timer_get_time();
    cJSON *result=cJSON_CreateObject();
    esp_http_client_handle_t client=NULL;
    struct addrinfo *resolved=NULL;
    char *body=NULL;
    ProbeEvents events={0};
    const char *stage="wifi",*error="not_connected";
    bool success=false,dns=false,tcp=false,complete=false;
    int dns_error=0,tcp_error=0,status=0;
    size_t received=0;
    if(!result)goto done;
    cJSON_AddStringToObject(result,"url","https://example.com/");
    cJSON_AddStringToObject(result,"transport","native_wifi");
    cJSON_AddStringToObject(result,"clock_source","SNTP_native_wifi");
    cJSON_AddStringToObject(result,"ntp_server","time.google.com");
    cJSON_AddBoolToObject(result,"host_clock_supplied",false);
    cJSON_AddNumberToObject(result,"free_heap_before",esp_get_free_heap_size());
    cJSON_AddNumberToObject(result,"largest_block_before",heap_caps_get_largest_free_block(MALLOC_CAP_8BIT));
    if(wifi.state!=CW_CONNECTED)goto report;
    stage="clock";
    esp_err_t sync=CW_TimeWait(40000);
    CW_TimeStatus clock;CW_TimeGetStatus(&clock);
    cJSON_AddNumberToObject(result,"clock_unix",clock.unix_time);
    cJSON_AddNumberToObject(result,"ntp_sync_count",clock.sync_count);
    cJSON_AddBoolToObject(result,"clock_synchronized",clock.synchronized);
    if(sync!=ESP_OK){error=esp_err_to_name(sync);goto report;}
    started=esp_timer_get_time();
    stage="dns";
    struct addrinfo hints={.ai_family=AF_INET,.ai_socktype=SOCK_STREAM};
    dns_error=getaddrinfo("example.com","443",&hints,&resolved);
    if(dns_error||!resolved){error="dns_failed";goto report;}
    dns=true;
    char ip[16];inet_ntop(AF_INET,&((struct sockaddr_in*)resolved->ai_addr)->sin_addr,ip,sizeof(ip));
    cJSON_AddStringToObject(result,"resolved_ipv4",ip);
    stage="tcp";
    tcp=probe_tcp(resolved->ai_addr,resolved->ai_addrlen,&tcp_error);
    if(!tcp){error="tcp_failed";goto report;}
    stage="tls";
    esp_http_client_config_t config={.url="https://example.com/",.timeout_ms=12000,
        .crt_bundle_attach=esp_crt_bundle_attach,.skip_cert_common_name_check=false,
        .disable_auto_redirect=true,.buffer_size=512,.buffer_size_tx=512,
        .event_handler=probe_event,.user_data=&events};
    client=esp_http_client_init(&config);
    if(!client){error="http_client_no_memory";goto report;}
    esp_http_client_set_header(client,"Accept","text/html");
    esp_http_client_set_header(client,"Accept-Encoding","identity");
    esp_err_t err=esp_http_client_open(client,0);
    if(err!=ESP_OK){error=esp_err_to_name(err);goto report;}
    stage="http";
    if(esp_http_client_fetch_headers(client)<0&&!esp_http_client_is_chunked_response(client)){
        error="headers_failed";goto report;
    }
    status=esp_http_client_get_status_code(client);
    if(status!=200){error="unexpected_http_status";goto report;}
    stage="body";
    body=calloc(4097,1);
    if(!body){error="body_no_memory";goto report;}
    while(received<4096&&esp_timer_get_time()-started<45000000){
        int n=esp_http_client_read(client,body+received,(4096-received)>256?256:4096-received);
        if(n<0){error="body_read_failed";goto report;}
        if(n==0)break;
        received+=n;
    }
    complete=esp_http_client_is_complete_data_received(client);
    if(!complete){error="body_incomplete_or_limit";goto report;}
    unsigned char digest[32];char hex[65];
    if(mbedtls_sha256((unsigned char*)body,received,digest,0)!=0){error="digest_failed";goto report;}
    for(int i=0;i<32;i++)snprintf(hex+i*2,3,"%02x",digest[i]);
    cJSON_AddStringToObject(result,"body_sha256",hex);
    bool title=strstr(body,"<title>Example Domain</title>")!=NULL;
    cJSON_AddBoolToObject(result,"example_domain_title",title);
    if(!title){error="unexpected_body";goto report;}
    stage="complete";error="ESP_OK";success=true;
report:
    if(client){esp_http_client_cleanup(client);client=NULL;}
    cJSON_AddBoolToObject(result,"ok",success);
    cJSON_AddStringToObject(result,"stage",stage);
    cJSON_AddStringToObject(result,"error",error);
    cJSON_AddBoolToObject(result,"dns_resolved",dns);
    cJSON_AddNumberToObject(result,"dns_error",dns_error);
    cJSON_AddBoolToObject(result,"tcp_connected",tcp);
    cJSON_AddNumberToObject(result,"tcp_errno",tcp_error);
    cJSON_AddBoolToObject(result,"tls_verified",events.connected);
    cJSON_AddNumberToObject(result,"tls_code",events.tls_code);
    cJSON_AddNumberToObject(result,"tls_verify_flags",events.tls_flags);
    cJSON_AddNumberToObject(result,"http_status",status);
    cJSON_AddNumberToObject(result,"bytes_received",received);
    cJSON_AddBoolToObject(result,"body_complete",complete);
    cJSON_AddNumberToObject(result,"elapsed_ms",(esp_timer_get_time()-started)/1000);
    cJSON_AddNumberToObject(result,"free_heap_after",esp_get_free_heap_size());
    char *text=cJSON_PrintUnformatted(result);
    if(text){printf("\nCWHTTPS %s\n",text);fflush(stdout);free(text);}
done:
    if(resolved)freeaddrinfo(resolved);
    free(body);cJSON_Delete(result);
    taskENTER_CRITICAL(&probe_lock);probe_running=false;taskEXIT_CRITICAL(&probe_lock);
    vTaskDelete(NULL);
}

static int probe_command(int argc,char **argv)
{
    (void)argv;if(argc!=1)return 1;
    taskENTER_CRITICAL(&probe_lock);
    if(probe_running){taskEXIT_CRITICAL(&probe_lock);return 1;}
    probe_running=true;taskEXIT_CRITICAL(&probe_lock);
    if(xTaskCreatePinnedToCore(probe_task,"https_probe",8192,NULL,2,NULL,0)!=pdPASS){
        taskENTER_CRITICAL(&probe_lock);probe_running=false;taskEXIT_CRITICAL(&probe_lock);return 1;
    }
    return 0;
}

esp_err_t CW_HTTPSInit(void)
{
    esp_console_cmd_t command={.command="wifi_https_probe",.help="Fetch verified HTTPS example.com after native Wi-Fi time synchronization.",.func=probe_command};
    return esp_console_cmd_register(&command);
}
