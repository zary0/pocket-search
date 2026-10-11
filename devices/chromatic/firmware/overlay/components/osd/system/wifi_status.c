// SPDX-License-Identifier: GPL-3.0-or-later
#include "wifi_status.h"
#include "chromatic_wifi.h"
#include "lvgl.h"
#include <stdio.h>
#include <string.h>

static lv_obj_t *s_label;
static char s_previous[180];

OSD_Result_t WiFiStatus_Draw(void *screen)
{
    if (!screen) return kOSD_Result_Err_NullDataPtr;
    CW_Status status;
    CW_GetStatus(&status);
    const char *state = "OFF";
    switch (status.state) {
        case CW_IDLE: state = "READY"; break;
        case CW_CONNECTING: state = "CONNECTING"; break;
        case CW_CONNECTED: state = "CONNECTED"; break;
        case CW_FAILED: state = "FAILED"; break;
        default: break;
    }
    char text[180];
    snprintf(text, sizeof(text), "2.4GHZ WI-FI\n\n%s\n%s\n\n%s\n\nA:ON B:OFF",
        status.scanning ? "SCANNING" : state,
        status.configured ? "SSID SAVED" : "SETUP ON MAC",
        status.ip[0] ? status.ip : "USB FOR SETUP");
    if (!s_label) {
        s_label = lv_label_create(screen);
        lv_obj_add_style(s_label, OSD_GetStyleTextWhite(), 0);
        lv_obj_set_pos(s_label, 77, 43);
        lv_obj_set_width(s_label, 76);
        lv_label_set_long_mode(s_label, LV_LABEL_LONG_WRAP);
        s_previous[0] = '\0';
    }
    if (strcmp(text, s_previous) != 0) {
        lv_label_set_text(s_label, text);
        memcpy(s_previous, text, strlen(text) + 1);
    }
    lv_obj_move_foreground(s_label);
    return kOSD_Result_Ok;
}

OSD_Result_t WiFiStatus_OnButton(Button_t button, ButtonState_t state, void *arg)
{
    (void)arg;
    if (state != kButtonState_Pressed) return kOSD_Result_Ok;
    if (button == kButton_A) {
        CW_Status status;
        CW_GetStatus(&status);
        if (status.configured) CW_On();
        else CW_Scan();
    } else if (button == kButton_B) CW_Off();
    return kOSD_Result_Ok;
}

OSD_Result_t WiFiStatus_OnTransition(void *arg)
{
    (void)arg;
    if (s_label) { lv_obj_del(s_label); s_label = NULL; }
    s_previous[0] = '\0';
    return kOSD_Result_Ok;
}
