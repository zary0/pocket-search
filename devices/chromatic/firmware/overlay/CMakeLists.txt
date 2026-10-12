# SPDX-License-Identifier: GPL-3.0-only
cmake_minimum_required(VERSION 3.22)
set(POCKET_DISPLAY_ROWS 48)
add_definitions(-DLV_LVGL_H_INCLUDE_SIMPLE)
include($ENV{IDF_PATH}/tools/cmake/project.cmake)
project(pocket_search)
include(cmake/pocket_graphics.cmake)
pocket_optimize_graphics()
