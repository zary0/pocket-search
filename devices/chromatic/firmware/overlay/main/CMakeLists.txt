cmake_minimum_required(VERSION 3.22)

idf_component_register(
    SRCS
        "main.c" "gfx.c" "fpga_tx.c" "fpga_rx.c" "fpga_common.c" "pwrmgr.c"
    INCLUDE_DIRS "."
    REQUIRES
        images esp_driver_uart esp_driver_spi esp_driver_gpio nvs_flash fatfs
        common battery button osd menu_mgr settings mutex
        console crc chromatic_wifi pocket_search_ui pocket_render_probe pocket_band_display
)
