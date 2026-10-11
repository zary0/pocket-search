// SPDX-License-Identifier: GPL-3.0-only
#pragma once
#include <stdbool.h>
#include <stdint.h>
#include "esp_err.h"

#define CW_MAX_NETWORKS 20
typedef enum { CW_OFF, CW_IDLE, CW_CONNECTING, CW_CONNECTED, CW_FAILED } CW_State;
typedef struct {
    char ssid[33];
    int8_t rssi;
    uint8_t channel;
    uint8_t auth;
} CW_Network;
typedef struct {
    CW_State state;
    bool enabled;
    bool configured;
    bool scanning;
    char ssid[33];
    char ip[16];
    uint16_t reason;
    uint8_t retries;
    esp_err_t error;
    uint16_t count;
    CW_Network networks[CW_MAX_NETWORKS];
} CW_Status;

esp_err_t CW_Init(void);
void CW_GetStatus(CW_Status *status);
bool CW_IsEnabled(void);
bool CW_ShouldStayAwake(void);
esp_err_t CW_On(void);
esp_err_t CW_Off(void);
esp_err_t CW_Scan(void);
esp_err_t CW_Join(const char *ssid, const char *password);
