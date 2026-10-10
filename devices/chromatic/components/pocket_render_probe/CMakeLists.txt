# SPDX-License-Identifier: GPL-3.0-only
idf_component_register(SRCS "pocket_render_probe.c" INCLUDE_DIRS "."
    REQUIRES lvgl console esp_timer esp_system esp_app_format esp_hw_support heap freertos)
