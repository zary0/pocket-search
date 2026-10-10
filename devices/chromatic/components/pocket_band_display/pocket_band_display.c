// SPDX-License-Identifier: GPL-3.0-only
// QSPI geometry follows ModRetro's GPL-3.0 Chromatic MCU display transport.
#include "pocket_band_display.h"
#include "pocket_render_probe.h"
#include <string.h>
#include "esp_attr.h"
#include "esp_err.h"
#include "freertos/FreeRTOS.h"

_Static_assert(sizeof(lv_color_t) == 2, "This driver requires RGB565");
static DMA_ATTR lv_color_t pixels[POCKET_WIDTH * POCKET_BAND_ROWS];
static spi_transaction_t transactions[POCKET_BAND_MAX_TRANSFERS];
static spi_device_handle_t device;
static lv_disp_drv_t *display_driver;
static pocket_band_observer_t observe;
static portMUX_TYPE completion_lock = portMUX_INITIALIZER_UNLOCKED;
static unsigned expected, completed, queued;

void pocket_band_display_spi_done(spi_transaction_t *transaction) {
    (void)transaction;
    taskENTER_CRITICAL_ISR(&completion_lock);
    bool last = ++completed == expected;
    taskEXIT_CRITICAL_ISR(&completion_lock);
    if (last) {
        pocket_render_probe_flush_end();
        lv_disp_flush_ready(display_driver);
    }
}

static void round_area(lv_disp_drv_t *driver, lv_area_t *area) {
    (void)driver;
    // Full-width bands need no row repacking or second framebuffer.
    area->x1 = 0; area->x2 = POCKET_WIDTH - 1;
    area->y1 = area->y1 / POCKET_BAND_ROWS * POCKET_BAND_ROWS;
    area->y2 = (area->y2 / POCKET_BAND_ROWS + 1) * POCKET_BAND_ROWS - 1;
    if (area->y2 >= POCKET_HEIGHT) area->y2 = POCKET_HEIGHT - 1;
}

static void flush(lv_disp_drv_t *driver, const lv_area_t *area, lv_color_t *color) {
    // Retire every previous descriptor before reuse; the final DMA interrupt
    // may release LVGL just before the SPI result reaches its return queue.
    for (unsigned i = 0; i < queued; ++i) {
        spi_transaction_t *result;
        ESP_ERROR_CHECK(spi_device_get_trans_result(device, &result, portMAX_DELAY));
    }
    queued = 0;
    configASSERT(area->x1 == 0 && area->x2 == POCKET_WIDTH - 1 && area->y1 >= 0);
    pocket_band_transfer_t plan[POCKET_BAND_MAX_TRANSFERS];
    size_t count = pocket_band_plan((unsigned)area->y1,
        (unsigned)(area->y2 - area->y1 + 1), plan, POCKET_BAND_MAX_TRANSFERS);
    configASSERT(count > 0 && color == pixels);
    bool last = lv_disp_flush_is_last(driver);
    if (observe) observe(area, color, last);
    pocket_render_probe_flush_start((size_t)(area->y2-area->y1+1)*POCKET_WIDTH*2, last);
    for (size_t i = 0; i < count; ++i) {
        transactions[i] = (spi_transaction_t){0};
        transactions[i].tx_buffer = (const uint8_t *)color + plan[i].offset;
        transactions[i].length = plan[i].bytes * 8;
        transactions[i].flags = SPI_TRANS_MODE_QIO;
        transactions[i].cmd = 0x400 | 1023;
        transactions[i].addr = plan[i].address;
    }
    taskENTER_CRITICAL(&completion_lock);
    expected = (unsigned)count;
    completed = 0;
    taskEXIT_CRITICAL(&completion_lock);
    for (size_t i = 0; i < count; ++i) {
        ESP_ERROR_CHECK(spi_device_queue_trans(device, &transactions[i], portMAX_DELAY));
        queued++;
    }
}

void pocket_band_display_init(spi_device_handle_t spi, lv_disp_draw_buf_t *buffer,
    lv_disp_drv_t *driver, pocket_band_observer_t observer) {
    configASSERT(spi && buffer && driver && !device);
    device = spi;
    display_driver = driver;
    observe = observer;
    lv_disp_draw_buf_init(buffer, pixels, NULL, POCKET_WIDTH * POCKET_BAND_ROWS);
    driver->draw_buf = buffer;
    driver->direct_mode = 0;
    driver->full_refresh = 0;
    driver->flush_cb = flush;
    driver->rounder_cb = round_area;
}
