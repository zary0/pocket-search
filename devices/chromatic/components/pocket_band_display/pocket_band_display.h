// SPDX-License-Identifier: GPL-3.0-only
#pragma once
#include "driver/spi_master.h"
#include "lvgl.h"
#include "pocket_band_plan.h"

typedef void (*pocket_band_observer_t)(const lv_area_t *, const lv_color_t *, bool);
// Configure one display on an already initialized, trusted Chromatic QSPI bus.
// queue_size must be >= POCKET_BAND_MAX_TRANSFERS; no other owner may submit
// transactions to that device. The existing ISR callback must be replaced below.
void pocket_band_display_init(spi_device_handle_t spi, lv_disp_draw_buf_t *buffer,
    lv_disp_drv_t *driver, pocket_band_observer_t observer);
void pocket_band_display_spi_done(spi_transaction_t *transaction);
