// SPDX-License-Identifier: GPL-3.0-only
#include "chromatic_wifi.h"
#include "network_time.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "cJSON.h"
#include "esp_console.h"
#include "esp_event.h"
#include "esp_netif.h"
#include "esp_system.h"
#include "esp_timer.h"
#include "esp_wifi.h"
#include "freertos/FreeRTOS.h"
#include "freertos/queue.h"
#include "freertos/task.h"
#include "nvs.h"
esp_err_t CW_HTTPSInit(void);

typedef struct {
    uint8_t version;
    uint8_t enabled;
    char ssid[33];
    char password[64];
} Credentials;
typedef enum { CMD_ON, CMD_OFF, CMD_SCAN, CMD_JOIN, CMD_FORGET } Action;
typedef struct { Action action; Credentials credentials; } Request;

static portMUX_TYPE s_lock = portMUX_INITIALIZER_UNLOCKED;
static CW_Status s_status;
static QueueHandle_t s_queue;
static Credentials s_credentials;
static bool s_network_ready;
static bool s_driver_ready;
static bool s_started;
static bool s_connect_wanted;
static int64_t s_last_console_activity;
static esp_netif_t *s_netif;
static esp_event_handler_instance_t s_wifi_handler;
static esp_event_handler_instance_t s_ip_handler;

void CW_GetStatus(CW_Status *status)
{
    taskENTER_CRITICAL(&s_lock);
    *status = s_status;
    taskEXIT_CRITICAL(&s_lock);
}

bool CW_IsEnabled(void)
{
    taskENTER_CRITICAL(&s_lock);
    bool enabled = s_status.enabled;
    taskEXIT_CRITICAL(&s_lock);
    return enabled;
}

bool CW_ShouldStayAwake(void)
{
    int64_t now = esp_timer_get_time();
    taskENTER_CRITICAL(&s_lock);
    bool awake = s_status.enabled || now - s_last_console_activity < 30000000;
    taskEXIT_CRITICAL(&s_lock);
    return awake;
}

static void set_error(esp_err_t err)
{
    taskENTER_CRITICAL(&s_lock);
    s_status.error = err;
    if (err != ESP_OK) s_status.state = CW_FAILED;
    taskEXIT_CRITICAL(&s_lock);
}

static esp_err_t save_credentials(void)
{
    nvs_handle_t handle;
    esp_err_t err = nvs_open("chromatic_wifi", NVS_READWRITE, &handle);
    if (err != ESP_OK) return err;
    err = nvs_set_blob(handle, "config", &s_credentials, sizeof(s_credentials));
    if (err == ESP_OK) err = nvs_commit(handle);
    nvs_close(handle);
    return err;
}

static void on_event(void *arg, esp_event_base_t base, int32_t id, void *data)
{
    (void)arg;
    if (base == WIFI_EVENT && id == WIFI_EVENT_STA_START) {
        taskENTER_CRITICAL(&s_lock);
        bool connect = s_connect_wanted;
        taskEXIT_CRITICAL(&s_lock);
        if (connect) {
            esp_err_t err = esp_wifi_connect();
            if (err != ESP_OK) set_error(err);
        }
    } else if (base == WIFI_EVENT && id == WIFI_EVENT_STA_DISCONNECTED) {
        wifi_event_sta_disconnected_t *event = data;
        taskENTER_CRITICAL(&s_lock);
        bool retry = s_connect_wanted && s_status.enabled && s_status.retries < 5;
        s_status.ip[0] = '\0';
        if (s_connect_wanted && s_status.enabled) {
            s_status.reason = event->reason;
            s_status.state = retry ? CW_CONNECTING : CW_FAILED;
            if (retry) s_status.retries++;
        }
        taskEXIT_CRITICAL(&s_lock);
        if (retry) {
            esp_err_t err = esp_wifi_connect();
            if (err != ESP_OK) set_error(err);
        }
    } else if (base == IP_EVENT && id == IP_EVENT_STA_GOT_IP) {
        ip_event_got_ip_t *event = data;
        wifi_ap_record_t ap;
        if (esp_wifi_sta_get_ap_info(&ap) != ESP_OK) return;
        taskENTER_CRITICAL(&s_lock);
        if (s_connect_wanted && s_status.enabled &&
            strncmp((const char *)ap.ssid, s_status.ssid, 32) == 0) {
            snprintf(s_status.ip, sizeof(s_status.ip), IPSTR, IP2STR(&event->ip_info.ip));
            s_status.state = CW_CONNECTED;
            s_status.error = ESP_OK;
            s_status.reason = 0;
            s_status.retries = 0;
        }
        taskEXIT_CRITICAL(&s_lock);
        CW_Status current;CW_GetStatus(&current);
        if(current.state==CW_CONNECTED)CW_TimeNetworkReady();
    } else if (base == WIFI_EVENT && id == WIFI_EVENT_SCAN_DONE) {
        uint16_t count = CW_MAX_NETWORKS;
        wifi_ap_record_t *records = calloc(count, sizeof(*records));
        esp_err_t err = records ? esp_wifi_scan_get_ap_records(&count, records) : ESP_ERR_NO_MEM;
        if (!records) esp_wifi_clear_ap_list();
        taskENTER_CRITICAL(&s_lock);
        if (s_status.scanning) {
            s_status.scanning = false;
            s_status.count = err == ESP_OK ? count : 0;
            s_status.error = err;
            for (unsigned i = 0; err == ESP_OK && i < count; i++) {
                memcpy(s_status.networks[i].ssid, records[i].ssid, 32);
                s_status.networks[i].ssid[32] = '\0';
                s_status.networks[i].rssi = records[i].rssi;
                s_status.networks[i].channel = records[i].primary;
                s_status.networks[i].auth = records[i].authmode;
            }
        }
        taskEXIT_CRITICAL(&s_lock);
        free(records);
    }
}

static esp_err_t init_radio(void)
{
    esp_err_t err;
    if (!s_network_ready) {
        err = esp_netif_init();
        if (err != ESP_OK) return err;
        err = esp_event_loop_create_default();
        if (err != ESP_OK && err != ESP_ERR_INVALID_STATE) return err;
        s_network_ready = true;
    }
    if (!s_netif) {
        s_netif = esp_netif_create_default_wifi_sta();
        if (!s_netif) return ESP_ERR_NO_MEM;
        esp_netif_set_hostname(s_netif, "chromatic");
    }
    if (!s_driver_ready) {
        wifi_init_config_t config = WIFI_INIT_CONFIG_DEFAULT();
        config.static_rx_buf_num = 6;
        config.dynamic_rx_buf_num = 12;
        config.dynamic_tx_buf_num = 12;
        err = esp_wifi_init(&config);
        if (err != ESP_OK) return err;
        s_driver_ready = true;
    }
    if (!s_wifi_handler) {
        err = esp_event_handler_instance_register(WIFI_EVENT, ESP_EVENT_ANY_ID,
            on_event, NULL, &s_wifi_handler);
        if (err != ESP_OK) return err;
    }
    if (!s_ip_handler) {
        err = esp_event_handler_instance_register(IP_EVENT, IP_EVENT_STA_GOT_IP,
            on_event, NULL, &s_ip_handler);
        if (err != ESP_OK) return err;
    }
    if ((err = esp_wifi_set_storage(WIFI_STORAGE_RAM)) != ESP_OK) return err;
    if ((err = esp_wifi_set_mode(WIFI_MODE_STA)) != ESP_OK) return err;
    // The default regulatory region is Japan. Permit channels 1-14 according to the SDK's JP profile.
    if ((err = esp_wifi_set_country_code("JP", true)) != ESP_OK) return err;
    return ESP_OK;
}

static esp_err_t stop_radio(void)
{
    taskENTER_CRITICAL(&s_lock);
    s_connect_wanted = false;
    s_status.scanning = false;
    taskEXIT_CRITICAL(&s_lock);
    esp_err_t err = s_started ? esp_wifi_stop() : ESP_OK;
    if (err != ESP_OK) return err;
    s_started = false;
    taskENTER_CRITICAL(&s_lock);
    s_status.enabled = false;
    s_status.state = CW_OFF;
    s_status.ip[0] = '\0';
    taskEXIT_CRITICAL(&s_lock);
    return ESP_OK;
}

static esp_err_t connect_radio(void)
{
    if (!s_credentials.ssid[0]) return ESP_ERR_INVALID_STATE;
    esp_err_t err = stop_radio();
    if (err != ESP_OK) return err;
    // Keep the stock idle-sleep task from suspending radio initialization.
    taskENTER_CRITICAL(&s_lock);
    s_status.enabled = true;
    s_status.state = CW_CONNECTING;
    taskEXIT_CRITICAL(&s_lock);
    if ((err = init_radio()) != ESP_OK) goto fail;
    wifi_config_t config = {0};
    memcpy(config.sta.ssid, s_credentials.ssid, strnlen(s_credentials.ssid, 32));
    memcpy(config.sta.password, s_credentials.password, strlen(s_credentials.password));
    config.sta.threshold.authmode = s_credentials.password[0] ? WIFI_AUTH_WPA2_PSK : WIFI_AUTH_OPEN;
    config.sta.sae_pwe_h2e = WPA3_SAE_PWE_BOTH;
    config.sta.pmf_cfg.capable = true;
    config.sta.pmf_cfg.required = false;
    if ((err = esp_wifi_set_config(WIFI_IF_STA, &config)) != ESP_OK) goto fail;
    taskENTER_CRITICAL(&s_lock);
    s_status.retries = 0;
    s_status.reason = 0;
    s_status.error = ESP_OK;
    s_connect_wanted = true;
    taskEXIT_CRITICAL(&s_lock);
    if ((err = esp_wifi_start()) != ESP_OK) goto fail;
    s_started = true;
    // Limit peak demand on the handheld power supply; power saving leaves association active.
    esp_wifi_set_max_tx_power(56);
    esp_wifi_set_ps(WIFI_PS_MIN_MODEM);
    return ESP_OK;
fail:
    stop_radio();
    return err;
}

static esp_err_t scan_radio(void)
{
    CW_Status status;
    CW_GetStatus(&status);
    if (status.state == CW_CONNECTING || status.scanning) return ESP_ERR_WIFI_STATE;
    taskENTER_CRITICAL(&s_lock);
    s_status.enabled = true;
    s_status.scanning = true;
    s_status.error = ESP_OK;
    taskEXIT_CRITICAL(&s_lock);
    esp_err_t err = init_radio();
    if (err != ESP_OK) goto fail;
    if (!s_started) {
        if ((err = esp_wifi_start()) != ESP_OK) goto fail;
        s_started = true;
        esp_wifi_set_max_tx_power(56);
        taskENTER_CRITICAL(&s_lock);
        s_status.state = CW_IDLE;
        taskEXIT_CRITICAL(&s_lock);
    }
    wifi_scan_config_t config = { .show_hidden = false };
    if ((err = esp_wifi_scan_start(&config, false)) != ESP_OK) goto fail;
    return ESP_OK;
fail:
    taskENTER_CRITICAL(&s_lock);
    s_status.scanning = false;
    taskEXIT_CRITICAL(&s_lock);
    if (!s_started) stop_radio();
    return err;
}

static void worker(void *arg)
{
    (void)arg;
    nvs_handle_t handle;
    if (nvs_open("chromatic_wifi", NVS_READONLY, &handle) == ESP_OK) {
        size_t size = sizeof(s_credentials);
        esp_err_t err = nvs_get_blob(handle, "config", &s_credentials, &size);
        nvs_close(handle);
        if (err != ESP_OK || size != sizeof(s_credentials) || s_credentials.version != 1 ||
            !memchr(s_credentials.ssid, '\0', sizeof(s_credentials.ssid)) ||
            !memchr(s_credentials.password, '\0', sizeof(s_credentials.password))) {
            memset(&s_credentials, 0, sizeof(s_credentials));
        }
    }
    taskENTER_CRITICAL(&s_lock);
    s_status.configured = s_credentials.ssid[0] != '\0';
    memcpy(s_status.ssid, s_credentials.ssid, sizeof(s_status.ssid));
    taskEXIT_CRITICAL(&s_lock);
    if (s_credentials.enabled && s_credentials.ssid[0]) set_error(connect_radio());
    Request request;
    while (xQueueReceive(s_queue, &request, portMAX_DELAY) == pdTRUE) {
        esp_err_t err = ESP_OK;
        switch (request.action) {
            case CMD_JOIN:
                s_credentials = request.credentials;
                taskENTER_CRITICAL(&s_lock);
                s_status.configured = true;
                memcpy(s_status.ssid, s_credentials.ssid, sizeof(s_status.ssid));
                taskEXIT_CRITICAL(&s_lock);
                err = connect_radio();
                if (err == ESP_OK) err = save_credentials();
                break;
            case CMD_ON:
                err = connect_radio();
                if (err == ESP_OK) {
                    s_credentials.enabled = true;
                    err = save_credentials();
                }
                break;
            case CMD_OFF:
                err = stop_radio();
                s_credentials.enabled = false;
                if (err == ESP_OK && s_credentials.ssid[0]) err = save_credentials();
                break;
            case CMD_SCAN:
                err = scan_radio();
                break;
            case CMD_FORGET:
                err = stop_radio();
                memset(&s_credentials, 0, sizeof(s_credentials));
                if (nvs_open("chromatic_wifi", NVS_READWRITE, &handle) == ESP_OK) {
                    err = nvs_erase_key(handle, "config");
                    if (err == ESP_ERR_NVS_NOT_FOUND) err = ESP_OK;
                    if (err == ESP_OK) err = nvs_commit(handle);
                    nvs_close(handle);
                }
                taskENTER_CRITICAL(&s_lock);
                s_status.configured = false;
                s_status.ssid[0] = '\0';
                taskEXIT_CRITICAL(&s_lock);
                break;
        }
        memset(&request, 0, sizeof(request));
        if (request.action == CMD_SCAN) {
            taskENTER_CRITICAL(&s_lock);
            s_status.error = err;
            taskEXIT_CRITICAL(&s_lock);
        } else set_error(err);
    }
}

static esp_err_t enqueue(Action action, const Credentials *credentials)
{
    if (!s_queue) return ESP_ERR_INVALID_STATE;
    Request request = { .action = action };
    if (credentials) request.credentials = *credentials;
    BaseType_t result = xQueueSend(s_queue, &request, 0);
    memset(&request, 0, sizeof(request));
    return result == pdTRUE ? ESP_OK : ESP_ERR_TIMEOUT;
}

esp_err_t CW_On(void) { return enqueue(CMD_ON, NULL); }
esp_err_t CW_Off(void) { return enqueue(CMD_OFF, NULL); }
esp_err_t CW_Scan(void) { return enqueue(CMD_SCAN, NULL); }
esp_err_t CW_Join(const char *ssid, const char *password)
{
    size_t a=strlen(ssid), b=strlen(password);
    if(!a||a>32||b>63||(b&&b<8))return ESP_ERR_INVALID_ARG;
    Credentials c={.version=1,.enabled=true};
    memcpy(c.ssid,ssid,a);memcpy(c.password,password,b);
    esp_err_t err=enqueue(CMD_JOIN,&c);memset(&c,0,sizeof(c));return err;
}

static void reply(esp_err_t error)
{
    CW_Status status;
    CW_GetStatus(&status);
    static const char *states[] = { "off", "idle", "connecting", "connected", "failed" };
    cJSON *root = cJSON_CreateObject();
    if (!root) { printf("CWJSON {\"ok\":false,\"error\":\"out_of_memory\"}\n"); return; }
    cJSON_AddBoolToObject(root, "ok", error == ESP_OK);
    cJSON_AddStringToObject(root, "error", esp_err_to_name(error));
    cJSON_AddStringToObject(root, "lastError", esp_err_to_name(status.error));
    cJSON_AddStringToObject(root, "state", states[status.state]);
    cJSON_AddBoolToObject(root, "enabled", status.enabled);
    cJSON_AddBoolToObject(root, "configured", status.configured);
    cJSON_AddBoolToObject(root, "scanning", status.scanning);
    cJSON_AddStringToObject(root, "ssid", status.ssid);
    cJSON_AddStringToObject(root, "ip", status.ip);
    cJSON_AddNumberToObject(root, "reason", status.reason);
    cJSON_AddNumberToObject(root, "retries", status.retries);
    cJSON_AddNumberToObject(root, "freeHeap", esp_get_free_heap_size());
    esp_netif_ip_info_t info;
    if(s_netif&&esp_netif_get_ip_info(s_netif,&info)==ESP_OK){
        char address[16];
        snprintf(address,sizeof(address),IPSTR,IP2STR(&info.gw));
        cJSON_AddStringToObject(root,"gateway",address);
        snprintf(address,sizeof(address),IPSTR,IP2STR(&info.netmask));
        cJSON_AddStringToObject(root,"netmask",address);
    }
    cJSON *networks = cJSON_AddArrayToObject(root, "networks");
    for (unsigned i = 0; i < status.count; i++) {
        cJSON *network = cJSON_CreateObject();
        cJSON_AddStringToObject(network, "ssid", status.networks[i].ssid);
        cJSON_AddNumberToObject(network, "rssi", status.networks[i].rssi);
        cJSON_AddNumberToObject(network, "channel", status.networks[i].channel);
        cJSON_AddNumberToObject(network, "auth", status.networks[i].auth);
        cJSON_AddItemToArray(networks, network);
    }
    char *text = cJSON_PrintUnformatted(root);
    if (text) { printf("CWJSON %s\n", text); free(text); }
    cJSON_Delete(root);
}

static int hex_digit(char c)
{
    if (c >= '0' && c <= '9') return c - '0';
    if (c >= 'a' && c <= 'f') return c - 'a' + 10;
    if (c >= 'A' && c <= 'F') return c - 'A' + 10;
    return -1;
}

static bool decode_hex(const char *text, char *destination, size_t capacity)
{
    size_t length = strlen(text);
    if (length % 2 || length / 2 >= capacity) return false;
    memset(destination, 0, capacity);
    for (size_t i = 0; i < length / 2; i++) {
        int hi = hex_digit(text[i * 2]), lo = hex_digit(text[i * 2 + 1]);
        if (hi < 0 || lo < 0 || (hi == 0 && lo == 0)) return false;
        destination[i] = (char)((hi << 4) | lo);
    }
    return true;
}

static int command(int argc, char **argv)
{
    int64_t now = esp_timer_get_time();
    taskENTER_CRITICAL(&s_lock);
    s_last_console_activity = now;
    taskEXIT_CRITICAL(&s_lock);
    esp_err_t err = ESP_OK;
    if (strcmp(argv[0], "wifi_status") == 0) {
        reply(ESP_OK); return 0;
    } else if (strcmp(argv[0], "wifi_scan") == 0) err = CW_Scan();
    else if (strcmp(argv[0], "wifi_on") == 0) err = CW_On();
    else if (strcmp(argv[0], "wifi_off") == 0) err = CW_Off();
    else if (strcmp(argv[0], "wifi_forget") == 0) err = enqueue(CMD_FORGET, NULL);
    else if (strcmp(argv[0], "wifi_join") == 0) {
        Credentials credentials = { .version = 1, .enabled = true };
        if (argc != 3 || !decode_hex(argv[1], credentials.ssid, sizeof(credentials.ssid)) ||
            !credentials.ssid[0] || (strcmp(argv[2], "-") != 0 &&
            !decode_hex(argv[2], credentials.password, sizeof(credentials.password))) ||
            (credentials.password[0] && strlen(credentials.password) < 8)) {
            err = ESP_ERR_INVALID_ARG;
        } else err = enqueue(CMD_JOIN, &credentials);
        memset(&credentials, 0, sizeof(credentials));
    } else err = ESP_ERR_INVALID_ARG;
    reply(err);
    return err == ESP_OK ? 0 : 1;
}

esp_err_t CW_Init(void)
{
    // Leave a provisioning window after USB-triggered reset, before stock idle-sleep resumes.
    s_last_console_activity = esp_timer_get_time();
    s_queue = xQueueCreate(4, sizeof(Request));
    if (!s_queue) return ESP_ERR_NO_MEM;
    if (xTaskCreatePinnedToCore(worker, "chromatic_wifi", 6144, NULL, 3, NULL, 0) != pdPASS) {
        vQueueDelete(s_queue); s_queue = NULL; return ESP_ERR_NO_MEM;
    }
    static const struct { const char *name; const char *help; } commands[] = {
        { "wifi_status", "Wi-Fi connection status (no passwords)." },
        { "wifi_scan", "Scan nearby 2.4 GHz Wi-Fi networks." },
        { "wifi_join", "Provision from the local Chromatic Wi-Fi setup page." },
        { "wifi_on", "Connect using the saved Wi-Fi settings." },
        { "wifi_off", "Turn the Wi-Fi radio off." },
        { "wifi_forget", "Remove only the saved Wi-Fi credentials." },
    };
    for (unsigned i = 0; i < sizeof(commands) / sizeof(commands[0]); i++) {
        esp_console_cmd_t cmd = { .command = commands[i].name, .help = commands[i].help, .func = command };
        esp_err_t err = esp_console_cmd_register(&cmd);
        if (err != ESP_OK) return err;
    }
    esp_err_t err=CW_TimeInit();
    return err==ESP_OK?CW_HTTPSInit():err;
}
