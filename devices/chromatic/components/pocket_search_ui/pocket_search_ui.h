// SPDX-License-Identifier: GPL-3.0-only
#pragma once
#include "osd_shared.h"
OSD_Result_t PocketUI_Draw(void *arg);
OSD_Result_t PocketUI_Button(Button_t button, ButtonState_t state, void *arg);
OSD_Result_t PocketUI_Transition(void *arg);
OSD_Result_t PocketEntry_Draw(void *screen);
OSD_Result_t PocketEntry_Button(Button_t button, ButtonState_t state, void *arg);
OSD_Result_t PocketEntry_Transition(void *arg);
void PocketUI_RegisterCommands(void);
void PocketUI_Poll(void); // GUI task only; diagnostic input is explicitly synthetic.
