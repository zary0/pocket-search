// SPDX-License-Identifier: GPL-3.0-only
#pragma once
#include <stdbool.h>
#include <stdint.h>
#include "esp_err.h"
typedef struct { bool started,synchronized; unsigned sync_count; int64_t unix_time; esp_err_t error; } CW_TimeStatus;
esp_err_t CW_TimeInit(void);
void CW_TimeNetworkReady(void);
void CW_TimeGetStatus(CW_TimeStatus *status);
esp_err_t CW_TimeWait(unsigned timeout_ms);
