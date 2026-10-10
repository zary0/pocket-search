// SPDX-License-Identifier: GPL-3.0-only
#pragma once
#include <stddef.h>
#include "esp_err.h"
typedef unsigned nvs_handle_t;
typedef enum {NVS_READONLY,NVS_READWRITE} nvs_open_mode_t;
#define ESP_ERR_NVS_NOT_FOUND 0x1102
#define ESP_ERR_NVS_INVALID_LENGTH 0x110c
esp_err_t nvs_open(const char *, nvs_open_mode_t, nvs_handle_t *);
esp_err_t nvs_get_str(nvs_handle_t, const char *, char *, size_t *);
esp_err_t nvs_set_str(nvs_handle_t, const char *, const char *);
esp_err_t nvs_commit(nvs_handle_t);
void nvs_close(nvs_handle_t);
