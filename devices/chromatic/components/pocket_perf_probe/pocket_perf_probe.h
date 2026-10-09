// SPDX-License-Identifier: GPL-3.0-only
#pragma once
#include <stddef.h>
#include <stdint.h>
#include "esp_err.h"

typedef uint32_t (*pocket_perf_operation_t)(void *context);

typedef struct {
    uint32_t iterations;
    uint32_t core_id;
    uint32_t cpu_hz_before;
    uint32_t cpu_hz_after;
    uint32_t cycles_min;
    uint32_t cycles_max;
    uint64_t cycles_total;
    int64_t elapsed_us;
    size_t free_bytes_before;
    size_t free_bytes_after;
    size_t largest_block_before;
    size_t largest_block_after;
    size_t boot_minimum_free_bytes;
    uint32_t checksum;
} pocket_perf_report_t;

// Call from a pinned task at fixed frequency with a bounded, synchronous CPU
// operation (e.g. packet decode). No network, logging, blocking, or allocation
// in the callback. A 25 ms batch budget is checked between calls, not inside one.
// Only ESP_OK reports are complete measurements. Timing includes interruptions.
esp_err_t pocket_perf_measure(pocket_perf_operation_t operation, void *context,
                             uint32_t iterations, pocket_perf_report_t *report);
