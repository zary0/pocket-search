# SPDX-License-Identifier: GPL-3.0-only
# Include after project() in the inspected ModRetro/ESP-IDF application.
# Leave networking, TLS, assertions, and the measurement probe flags unchanged.
function(pocket_optimize_graphics)
    foreach(component IN ITEMS lvgl fonts pocket_band_display)
        idf_component_get_property(library ${component} COMPONENT_LIB)
        if(NOT TARGET ${library})
            message(FATAL_ERROR "Missing graphics component: ${component}")
        endif()
        target_compile_options(${library} PRIVATE -O2)
    endforeach()
endfunction()
