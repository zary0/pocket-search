idf_component_register(SRCS "pocket_search_ui.c" "pocket_search_net.c" "pocket_search_origin.c" "pocket_typography.c" "pocket_native_image.c" "pocket_extended_image.c" "pocket_store.c" INCLUDE_DIRS "."
    REQUIRES common button menu_mgr tab lvgl chromatic_wifi fonts
    PRIV_REQUIRES console esp_http_client esp-tls mbedtls esp_timer nvs_flash spi_flash esp_partition esp_rom)
if(NOT DEFINED POCKET_RUST_ARCHIVE OR NOT EXISTS "${POCKET_RUST_ARCHIVE}")
    message(FATAL_ERROR "Build chromatic-ffi for Xtensa and set POCKET_RUST_ARCHIVE to its static library")
endif()
add_library(pocket_rust_decoder STATIC IMPORTED)
set_target_properties(pocket_rust_decoder PROPERTIES IMPORTED_LOCATION "${POCKET_RUST_ARCHIVE}")
target_link_libraries(${COMPONENT_LIB} PUBLIC pocket_rust_decoder)
