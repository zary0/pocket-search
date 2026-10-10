# SPDX-License-Identifier: GPL-3.0-only
idf_component_register(SRCS "pocket_band_display.c" INCLUDE_DIRS "."
    REQUIRES lvgl esp_driver_spi esp_hw_support freertos pocket_render_probe)
# Override with -DPOCKET_DISPLAY_ROWS=48 for the measured throughput candidate.
if(NOT DEFINED POCKET_DISPLAY_ROWS)
    set(POCKET_DISPLAY_ROWS 16)
endif()
if(NOT POCKET_DISPLAY_ROWS STREQUAL "16" AND NOT POCKET_DISPLAY_ROWS STREQUAL "48")
    message(FATAL_ERROR "POCKET_DISPLAY_ROWS must be 16 or 48")
endif()
target_compile_definitions(${COMPONENT_LIB} PUBLIC POCKET_BAND_ROWS=${POCKET_DISPLAY_ROWS})
