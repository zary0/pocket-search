idf_component_register(SRCS "pocket_search_ui.c" "pocket_search_net.c" INCLUDE_DIRS "."
    REQUIRES common button menu_mgr tab lvgl chromatic_wifi fonts
    PRIV_REQUIRES console esp_http_client esp-tls mbedtls esp_timer)
if(NOT DEFINED POCKET_RUST_ARCHIVE OR NOT EXISTS "${POCKET_RUST_ARCHIVE}")
    message(FATAL_ERROR "Build chromatic-ffi for Xtensa and set POCKET_RUST_ARCHIVE to its static library")
endif()
add_library(pocket_rust_decoder STATIC IMPORTED)
set_target_properties(pocket_rust_decoder PROPERTIES IMPORTED_LOCATION "${POCKET_RUST_ARCHIVE}")
target_link_libraries(${COMPONENT_LIB} PUBLIC pocket_rust_decoder)
