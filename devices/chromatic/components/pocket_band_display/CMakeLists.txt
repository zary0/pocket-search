# SPDX-License-Identifier: GPL-3.0-only
idf_component_register(SRCS "pocket_band_display.c" INCLUDE_DIRS "."
    REQUIRES lvgl esp_driver_spi esp_hw_support freertos pocket_render_probe)
