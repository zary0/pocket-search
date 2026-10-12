idf_component_register(SRCS "pocket_search_ui.c" "pocket_search_net.c" "pocket_search_origin.c" "pocket_typography.c" "pocket_native_image.c" "pocket_extended_image.c" "pocket_store.c" INCLUDE_DIRS "."
    REQUIRES common button menu_mgr tab lvgl chromatic_wifi fonts
    PRIV_REQUIRES console esp_http_client esp-tls mbedtls esp_timer nvs_flash spi_flash esp_partition esp_rom)
if(NOT DEFINED POCKET_RUST_ARCHIVE OR NOT EXISTS "${POCKET_RUST_ARCHIVE}")
    message(FATAL_ERROR "Build chromatic-ffi for Xtensa and set POCKET_RUST_ARCHIVE to its static library")
endif()
add_library(pocket_rust_decoder STATIC IMPORTED)
set_target_properties(pocket_rust_decoder PROPERTIES IMPORTED_LOCATION "${POCKET_RUST_ARCHIVE}")
target_link_libraries(${COMPONENT_LIB} PUBLIC pocket_rust_decoder)

set(PS_BROTLI third_party/brotli/c)
set(PS_WOFF2 third_party/woff2)
target_sources(${COMPONENT_LIB} PRIVATE pocket_woff2.cpp
    ${PS_BROTLI}/common/constants.c ${PS_BROTLI}/common/context.c
    ${PS_BROTLI}/common/dictionary.c ${PS_BROTLI}/common/platform.c
    ${PS_BROTLI}/common/shared_dictionary.c ${PS_BROTLI}/common/transform.c
    ${PS_BROTLI}/dec/bit_reader.c ${PS_BROTLI}/dec/decode.c
    ${PS_BROTLI}/dec/huffman.c ${PS_BROTLI}/dec/state.c
    ${PS_BROTLI}/dec/static_init.c ${PS_BROTLI}/dec/prefix.c
    ${PS_WOFF2}/src/woff2_dec.cc ${PS_WOFF2}/src/woff2_common.cc
    ${PS_WOFF2}/src/table_tags.cc ${PS_WOFF2}/src/variable_length.cc
    ${PS_WOFF2}/src/woff2_out.cc)
target_include_directories(${COMPONENT_LIB} PRIVATE ${PS_BROTLI}/include ${PS_WOFF2}/include)
