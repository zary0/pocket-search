// SPDX-License-Identifier: GPL-3.0-only
#pragma once
#include <stddef.h>
#include <stdint.h>

enum { POCKET_WIDTH = 160, POCKET_HEIGHT = 144, POCKET_BAND_ROWS = 16,
       POCKET_SPI_CHUNK_BYTES = 1024, POCKET_BAND_MAX_TRANSFERS = 5,
       POCKET_BAND_BUFFER_BYTES = POCKET_WIDTH * POCKET_BAND_ROWS * 2 };
typedef struct { uint32_t address, offset, bytes; } pocket_band_transfer_t;

// The FPGA consumes fixed 1024-byte bursts. Sixteen complete rows are five
// bursts; reject short or unaligned bands instead of sending a partial burst.
static inline size_t pocket_band_plan(unsigned top, unsigned rows,
    pocket_band_transfer_t *transfers, size_t capacity) {
    if (!transfers || top >= POCKET_HEIGHT || top % POCKET_BAND_ROWS ||
        rows != POCKET_BAND_ROWS ||
        rows > POCKET_HEIGHT - top) return 0;
    unsigned bytes = rows * POCKET_WIDTH * 2;
    size_t count = (bytes + POCKET_SPI_CHUNK_BYTES - 1) / POCKET_SPI_CHUNK_BYTES;
    if (capacity < count) return 0;
    for (size_t i = 0; i < count; ++i) {
        unsigned offset = (unsigned)i * POCKET_SPI_CHUNK_BYTES;
        unsigned length = bytes - offset;
        if (length > POCKET_SPI_CHUNK_BYTES) length = POCKET_SPI_CHUNK_BYTES;
        transfers[i] = (pocket_band_transfer_t){top * POCKET_WIDTH * 2 + offset, offset, length};
    }
    return count;
}
