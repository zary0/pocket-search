// SPDX-License-Identifier: GPL-3.0-only
#pragma once
#include <stdbool.h>
#include <stddef.h>
#include "lvgl.h"

// All LVGL calls occur on the existing GUI task, never the console task.
void pocket_render_probe_init(lv_disp_t *display, const lv_font_t *font,
                              const char *mode, size_t draw_buffer_bytes);
void pocket_render_probe_step(void);
void pocket_render_probe_handler_time(unsigned elapsed_us);
void pocket_render_probe_render_start(lv_disp_drv_t *driver);
void pocket_render_probe_flush_start(size_t bytes, bool last);
void pocket_render_probe_flush_end(void);
