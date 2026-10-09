# SPDX-License-Identifier: GPL-3.0-only
idf_component_register(SRCS "pocket_perf_probe.c" INCLUDE_DIRS "."
                       PRIV_REQUIRES esp_hw_support esp_timer heap)
