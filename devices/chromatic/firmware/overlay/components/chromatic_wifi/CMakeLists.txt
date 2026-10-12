idf_component_register(SRCS "chromatic_wifi.c" "https_probe.c" "network_time.c" INCLUDE_DIRS "."
    REQUIRES esp_wifi esp_event esp_netif nvs_flash console json esp_timer esp_http_client esp-tls mbedtls lwip heap)
