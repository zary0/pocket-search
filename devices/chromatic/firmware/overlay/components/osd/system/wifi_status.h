// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include "osd_shared.h"
OSD_Result_t WiFiStatus_Draw(void *screen);
OSD_Result_t WiFiStatus_OnButton(Button_t button, ButtonState_t state, void *arg);
OSD_Result_t WiFiStatus_OnTransition(void *arg);
