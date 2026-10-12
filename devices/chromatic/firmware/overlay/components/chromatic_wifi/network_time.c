// SPDX-License-Identifier: GPL-3.0-only
#include "network_time.h"
#include "esp_console.h"
#include "esp_netif_sntp.h"
#include "esp_timer.h"
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include <stdio.h>
#include <time.h>

static portMUX_TYPE lock=portMUX_INITIALIZER_UNLOCKED;
static CW_TimeStatus state;
static bool initialized;
static void synchronized(struct timeval *tv)
{
    taskENTER_CRITICAL(&lock);
    state.synchronized=tv->tv_sec>=1704067200;state.sync_count++;state.error=ESP_OK;
    taskEXIT_CRITICAL(&lock);
}
void CW_TimeGetStatus(CW_TimeStatus *out)
{
    taskENTER_CRITICAL(&lock);*out=state;taskEXIT_CRITICAL(&lock);
    out->unix_time=time(NULL);
}
void CW_TimeNetworkReady(void)
{
    esp_err_t err;
    if(!initialized){
        esp_sntp_config_t config=ESP_NETIF_SNTP_DEFAULT_CONFIG("time.google.com");
        config.sync_cb=synchronized;config.wait_for_sync=false;
        err=esp_netif_sntp_init(&config);
        if(err==ESP_OK)initialized=true;
    }else err=esp_netif_sntp_start();
    taskENTER_CRITICAL(&lock);state.started=err==ESP_OK;state.error=err;taskEXIT_CRITICAL(&lock);
}
esp_err_t CW_TimeWait(unsigned timeout_ms)
{
    int64_t deadline=esp_timer_get_time()+(int64_t)timeout_ms*1000;
    while(1){
        CW_TimeStatus status;CW_TimeGetStatus(&status);
        if(status.synchronized&&status.unix_time>=1704067200)return ESP_OK;
        if(status.error!=ESP_OK)return status.error;
        if(esp_timer_get_time()>=deadline)return ESP_ERR_TIMEOUT;
        vTaskDelay(pdMS_TO_TICKS(200));
    }
}
static int clock_status(int argc,char **argv)
{
    (void)argc;(void)argv;CW_TimeStatus status;CW_TimeGetStatus(&status);
    printf("CWTIME {\"source\":\"SNTP_native_wifi\",\"server\":\"time.google.com\",\"started\":%s,\"synchronized\":%s,\"sync_count\":%u,\"unix_time\":%lld,\"error\":\"%s\"}\n",status.started?"true":"false",status.synchronized?"true":"false",status.sync_count,(long long)status.unix_time,esp_err_to_name(status.error));
    return 0;
}
esp_err_t CW_TimeInit(void)
{
    esp_console_cmd_t command={.command="wifi_time_status",.help="Read native Wi-Fi time synchronization status.",.func=clock_status};
    return esp_console_cmd_register(&command);
}
