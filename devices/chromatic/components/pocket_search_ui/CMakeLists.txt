idf_component_register(SRCS "pocket_search_ui.c" INCLUDE_DIRS "."
    REQUIRES common button menu_mgr tab lvgl chromatic_wifi fonts
    PRIV_REQUIRES console)
