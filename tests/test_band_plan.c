// SPDX-License-Identifier: GPL-3.0-only
#include <assert.h>
#include "../devices/chromatic/components/pocket_band_display/pocket_band_plan.h"

int main(void) {
    pocket_band_transfer_t transfers[POCKET_BAND_MAX_TRANSFERS];
    for (unsigned top = 0; top < POCKET_HEIGHT; ++top) {
        for (unsigned rows = 1; rows <= POCKET_BAND_ROWS && top + rows <= POCKET_HEIGHT; ++rows) {
            size_t count = pocket_band_plan(top, rows, transfers, POCKET_BAND_MAX_TRANSFERS);
            if (top % POCKET_BAND_ROWS || rows != POCKET_BAND_ROWS) {
                assert(count == 0);
                continue;
            }
            assert(count == POCKET_BAND_MAX_TRANSFERS);
            unsigned offset = 0;
            for (size_t i = 0; i < count; ++i) {
                assert(transfers[i].offset == offset);
                assert(transfers[i].address == top * POCKET_WIDTH * 2 + offset);
                assert(transfers[i].bytes == POCKET_SPI_CHUNK_BYTES);
                assert(transfers[i].bytes % 2 == 0);
                assert(transfers[i].address + transfers[i].bytes <= POCKET_WIDTH * POCKET_HEIGHT * 2);
                offset += transfers[i].bytes;
            }
            assert(offset == rows * POCKET_WIDTH * 2);
            assert(pocket_band_plan(top, rows, transfers, count - 1) == 0);
        }
    }
    assert(pocket_band_plan(0, 0, transfers, 5) == 0);
    assert(pocket_band_plan(144, 1, transfers, 5) == 0);
    assert(pocket_band_plan(143, 2, transfers, 5) == 0);
    assert(pocket_band_plan(0, POCKET_BAND_ROWS + 1, transfers, POCKET_BAND_MAX_TRANSFERS) == 0);
    assert(pocket_band_plan(0, 1, NULL, 5) == 0);
}
