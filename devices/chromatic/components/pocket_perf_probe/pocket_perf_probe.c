// SPDX-License-Identifier: GPL-3.0-only
#include "pocket_perf_probe.h"
#include <limits.h>
#include "esp_cpu.h"
#include "esp_heap_caps.h"
#include "esp_private/esp_clk.h"
#include "esp_timer.h"

#if !CONFIG_IDF_TARGET_ESP32
#error "This probe is intended for the inspected Chromatic ESP32 target."
#endif

esp_err_t pocket_perf_measure(pocket_perf_operation_t operation, void *context,
                             uint32_t iterations, pocket_perf_report_t *report) {
    if (!operation || !report || iterations == 0 || iterations > 10000) {
        return ESP_ERR_INVALID_ARG;
    }
    *report = (pocket_perf_report_t){0};
    report->core_id = (uint32_t)esp_cpu_get_core_id();
    report->cpu_hz_before = (uint32_t)esp_clk_cpu_freq();
    report->cycles_min = UINT32_MAX;
    report->free_bytes_before = heap_caps_get_free_size(MALLOC_CAP_8BIT);
    report->largest_block_before = heap_caps_get_largest_free_block(MALLOC_CAP_8BIT);
    int64_t start_us = esp_timer_get_time();
    esp_err_t status = ESP_OK;
    for (uint32_t i = 0; i < iterations; ++i) {
        uint32_t start = esp_cpu_get_cycle_count();
        uint32_t result = operation(context);
        uint32_t cycles = esp_cpu_get_cycle_count() - start;
        report->checksum ^= result + i;
        report->iterations++;
        report->cycles_total += cycles;
        if (cycles < report->cycles_min) report->cycles_min = cycles;
        if (cycles > report->cycles_max) report->cycles_max = cycles;
        if ((uint32_t)esp_cpu_get_core_id() != report->core_id) {
            status = ESP_ERR_INVALID_STATE;
            break;
        }
        if (esp_timer_get_time() - start_us > 25000) {
            status = ESP_ERR_TIMEOUT;
            break;
        }
    }
    report->elapsed_us = esp_timer_get_time() - start_us;
    report->cpu_hz_after = (uint32_t)esp_clk_cpu_freq();
    report->free_bytes_after = heap_caps_get_free_size(MALLOC_CAP_8BIT);
    report->largest_block_after = heap_caps_get_largest_free_block(MALLOC_CAP_8BIT);
    // This is a since-boot low-water mark, not the peak used by this callback.
    report->boot_minimum_free_bytes = heap_caps_get_minimum_free_size(MALLOC_CAP_8BIT);
    if (report->cpu_hz_before != report->cpu_hz_after) status = ESP_ERR_INVALID_STATE;
    return status;
}
