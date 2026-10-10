// SPDX-License-Identifier: GPL-3.0-only
#pragma once
#include <stdbool.h>
#include "esp_err.h"
#define PS_ORIGIN_BYTES 192
bool PS_OriginValid(const char *value);
esp_err_t PS_OriginLoad(char value[PS_ORIGIN_BYTES]);
esp_err_t PS_OriginSave(const char *value);
