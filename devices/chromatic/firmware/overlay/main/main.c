#include <stdio.h>
#include <stdbool.h>

#include "driver/gpio.h"
#include "driver/spi_master.h"
#include "driver/uart.h"
#include "esp_console.h"
#include "pocket_render_probe.h"
#include "pocket_band_display.h"
#include "esp_err.h"
#include "esp_log.h"
#include "esp_timer.h"
#include "esp_vfs.h"
#include "esp_vfs_fat.h"
#include "freertos/FreeRTOS.h"
#include "freertos/queue.h"
#include "freertos/task.h"
#include "lvgl.h"
#include "gfx.h"
#include "board.h"
#include "brightness.h"
#include "color_correct_lcd.h"
#include "color_correct_usb.h"
#include "dpad_ctl.h"
#include "fpga_rx.h"
#include "fpga_tx.h"
#include "frameblend.h"
#include "fw.h"
#include "gbc_color_temp.h"
#include "osd.h"
#include "osd_default.h"
#include "low_batt_icon_ctl.h"
#include "style.h"
#include "player_num.h"
#include "screen_transit_ctl.h"
#include "mutex.h"
#include "serial_num.h"
#include "settings.h"
#include "silent.h"
#include "pwrmgr.h"
#include "chromatic_wifi.h"
#include "pocket_search_ui.h"
#include "pocket_frame_capture.h"
#include "esp_console.h"
#include "freertos/semphr.h"
#include <stdlib.h>

enum {
    kLVGL_TickPeriod_us = 1000u, // 1 millisecond
};

enum {
    kFPGATxTask_StackDepth = 8*1024,   // [bytes]
    kFPGARxTask_StackDepth = 8*1024,   // [bytes]
    kSleepTask_StackDepth  = 4*1024,   // [bytes]
    kTimerTask_StackDepth  = 8*1024,   // [bytes]

    kFPGATxTask_Priority = configMAX_PRIORITIES - 10,
    kFPGARxTask_Priority = configMAX_PRIORITIES - 10,
    kSleepTask_Priority  = configMAX_PRIORITIES - 9,
};

static const char *TAG = "main";
static volatile uint8_t fpga_hsync = 0;

static void lvgl_tick(void *arg);

static void persist_storage_init(void);
static void lvglTimerTask(void* param);

static spi_device_handle_t spi;
static lv_disp_draw_buf_t disp_buf; // contains internal graphic buffer(s) called draw buffer(s)
static lv_disp_drv_t disp_drv;      // contains callback functions
static lv_disp_t *disp;
// Read-only USB diagnostics: capture a completed render, never a mock screen.
static portMUX_TYPE frame_lock = portMUX_INITIALIZER_UNLOCKED;
static PS_Frame *frame_request;
static PS_Frame *frame_capture;
static bool frame_capture_failed;
static SemaphoreHandle_t frame_ready;
static int pocket_screen_command(int argc, char **argv);
static int pocket_display_command(int argc, char **argv);


// GUI task owns a snapshot claimed before a complete invalidation.
static void capture_band(const lv_area_t *area, const lv_color_t *color, bool last)
{
    if (!frame_capture) return;
    if(area->x1!=0||area->x2!=159||area->y1<0||!PS_FrameWrite(frame_capture,(unsigned)area->y1,(unsigned)(area->y2-area->y1+1),(const uint8_t*)color)){
        frame_capture_failed=true;frame_capture=NULL;xSemaphoreGive(frame_ready);return;
    }
    if (last) {
        frame_capture = NULL;
        xSemaphoreGive(frame_ready);
    }
}

static void lvgl_tick(void *arg)
{
    /* Tell LVGL how many milliseconds has elapsed */
    lv_tick_inc(kLVGL_TickPeriod_us / 1000);
}

static void lvglTimerTask(void* param)
{
    while(1)
    {
        // The task running lv_timer_handler should have lower priority than that running `lv_tick_inc`
        PocketUI_Poll();
        pocket_render_probe_step();
        taskENTER_CRITICAL(&frame_lock);
        bool capture_pending = frame_request != NULL && frame_capture == NULL;
        if (capture_pending) { frame_capture = frame_request; frame_request = NULL; }
        taskEXIT_CRITICAL(&frame_lock);
        if(capture_pending) lv_obj_invalidate(lv_disp_get_scr_act(disp));
        int64_t handler_start = esp_timer_get_time();
        lv_timer_handler();
        pocket_render_probe_handler_time((unsigned)(esp_timer_get_time()-handler_start));
        vTaskDelay(pdMS_TO_TICKS(30));
    }
}

static void register_update_callbacks(void)
{
    Brightness_RegisterOnUpdateCb(FPGA_Tx_WriteBrightness);
    PlayerNum_RegisterOnUpdateCb(FPGA_Tx_SendSysCtl);
    SilentMode_RegisterOnUpdateCb(FPGA_Tx_SendSysCtl);
    ColorCorrectLCD_RegisterOnUpdateCb(FPGA_Tx_SendSysCtl);
    ColorCorrectUSB_RegisterOnUpdateCb(FPGA_Tx_SendSysCtl);
    FrameBlend_RegisterOnUpdateCb(FPGA_Tx_SendSysCtl);
    ScreenTransitCtl_RegisterOnUpdateCb(FPGA_Tx_SendSysCtl);
    DPadCtl_RegisterOnUpdateCb(FPGA_Tx_SendSysCtl);
    LowBattIconCtl_RegisterOnUpdateCb(FPGA_Tx_SendSysCtl);
    Button_RegisterOnButtonPokeCb(FPGA_Tx_PokeButtons);
    Style_RegisterOnUpdateCb(FPGA_Tx_WritePaletteStyle);
    GBCColorTemp_RegisterOnUpdateCb(FPGA_Tx_WriteColorTemp);
}

static void persist_storage_init(void)
{
    Settings_Initialize();
    Firmware_Initialize();
    SerialNum_Initialize();
    Button_RegisterCommands();

    const fnSettingApply_t fnApplySetting[kNumSettingKeys] = {
        [kSettingKey_FrameBlend]       = FrameBlend_ApplySetting,
        [kSettingKey_ColorCorrectLCD]  = ColorCorrectLCD_ApplySetting,
        [kSettingKey_ColorCorrectUSB]  = ColorCorrectUSB_ApplySetting,
        [kSettingKey_PlayerNum]        = PlayerNum_ApplySetting,
        [kSettingKey_SilentMode]       = SilentMode_ApplySetting,
        [kSettingKey_Brightness]       = Brightness_ApplySetting,
        [kSettingKey_ScreenTransitCtl] = ScreenTransitCtl_ApplySetting,
        [kSettingKey_DPadCtl]          = DPadCtl_ApplySetting,
        [kSettingKey_LowBattIconCtl]   = LowBattIconCtl_ApplySetting,
        [kSettingKey_PaletteStyleID]   = Style_ApplySetting,
        [kSettingKey_GBCColorTemp]     = GBCColorTemp_ApplySetting,
    };

    OSD_Result_t eResult;
    for(SettingKey_t k = kSettingKey_FirstKey; k < kNumSettingKeys; k++)
    {
        SettingValue_t Value = {
            .eType = kSettingDataType_U32,
            .U32 = 0,
        };

        const char* pSettingName = Settings_KeyToName(k);

        // Set up the data type eType for each key
        if ((eResult = Settings_RetrieveDefault(k, &Value)) != kOSD_Result_Ok)
        {
            ESP_LOGE(TAG, "Failed to setup %s with its default, err = %d", pSettingName, eResult);
            continue;
        }

        if ((eResult = Settings_Retrieve(k, &Value.U32)) != kOSD_Result_Ok)
        {
            ESP_LOGE(TAG, "Failed to load setting %s, err = %d. Using default value.", pSettingName, eResult);

            // Create the entry if not existent, or store the default
            if ((eResult = Settings_Update(k, Value.U32)) != kOSD_Result_Ok)
            {
                ESP_LOGE(TAG, "Failed to update setting %s on init, err = %d.", pSettingName, eResult);
            }
        }
        else
        {
            ESP_LOGI(TAG, "Setting loaded, %s = %lu", pSettingName, Value.U32);
        }

        // Send over the setting to the module
        if (fnApplySetting[k] != NULL)
        {
            if ((eResult = fnApplySetting[k](&Value)) != kOSD_Result_Ok)
            {
                ESP_LOGE(TAG, "Failed to apply setting %s to %lu, err = %d.", pSettingName, Value.U32, eResult);
            }
        }
    }
}

static esp_console_repl_t *pRepl = NULL;
static esp_console_repl_config_t ReplConfig = ESP_CONSOLE_REPL_CONFIG_DEFAULT();
static esp_console_dev_uart_config_t ReplHWConfig = ESP_CONSOLE_DEV_UART_CONFIG_DEFAULT();

void app_main(void)
{
    persist_storage_init();
    Mutex_Init();

    // Set up the system management UART to/from the FPGA
    ESP_LOGI(TAG, "Initialize FPGA UART");
    const uart_config_t uart_config = {
        .baud_rate = 115200,
        .data_bits = UART_DATA_8_BITS,
        .parity = UART_PARITY_DISABLE,
        .stop_bits = UART_STOP_BITS_1,
        .flow_ctrl = UART_HW_FLOWCTRL_DISABLE,
        .source_clk = UART_SCLK_DEFAULT,
    };

    uart_driver_install(UART_NUM_1, kFPGA_TxConsts_BufferSize, kFPGA_RxConsts_BufferSize, 0, NULL, 0);
    uart_param_config(UART_NUM_1, &uart_config);
    uart_set_pin(UART_NUM_1, PIN_NUM_UART_TO_FPGA, PIN_NUM_UART_FROM_FPGA, -1, -1);

    gpio_sleep_set_direction(PIN_NUM_UART_FROM_FPGA, GPIO_MODE_INPUT);
    gpio_sleep_set_pull_mode(PIN_NUM_UART_FROM_FPGA, GPIO_PULLUP_ONLY);

    // Task is created earlier than the others to apply the settings ASAP
    xTaskCreate(FPGA_TxTask, "fpga_tx_task", kFPGATxTask_StackDepth, NULL, kFPGATxTask_Priority, FPGA_GetTxTaskHandle());

    // Task is created early in order to recieve fast messages (e.g hotkey palette data)
    // These tasks are started in the "paused" state
    xTaskCreate(FPGA_RxTask, "fpga_rx_task", kFPGARxTask_StackDepth, NULL, kFPGARxTask_Priority, FPGA_GetRxTaskHandle());

    // Let the tasks run for a bit to transmit config data to the FPGA core
    const size_t kTransmitCfgCounts = 6;
    for(size_t i = 0; i < kTransmitCfgCounts; i++)
    {
        FPGA_Tx_SendAll();
        vTaskDelay( pdMS_TO_TICKS(10) );
    }

    ESP_LOGI(TAG, "Initialize SPI Master");

    spi_bus_config_t buscfg={
        .miso_io_num=PIN_NUM_QSPI_MISO,
        .mosi_io_num=PIN_NUM_QSPI_MOSI,
        .sclk_io_num=PIN_NUM_QSPI_CLK,
        .quadwp_io_num=PIN_NUM_QSPI_WP,
        .quadhd_io_num=PIN_NUM_QSPI_HD,
        .max_transfer_sz=(1024*8)+46,
        .flags=SPICOMMON_BUSFLAG_QUAD | SPICOMMON_BUSFLAG_MASTER
    };
    spi_device_interface_config_t devcfg={
        .clock_speed_hz=40*1000*1000,           //Clock out at 40 MHz
        .mode=0,                                //SPI mode 0
        .spics_io_num=PIN_NUM_QSPI_CS,
        .queue_size=POCKET_BAND_MAX_TRANSFERS,
        .pre_cb=NULL,
        .flags=SPI_DEVICE_HALFDUPLEX,
        .cs_ena_pretrans=3,
        .cs_ena_posttrans=3,
        .command_bits=11,
        .address_bits=32,
        .dummy_bits=3,
        .post_cb=pocket_band_display_spi_done
    };

    //Initialize the SPI bus
    esp_err_t ret=spi_bus_initialize(VSPI_HOST, &buscfg, SPI_DMA_CH_AUTO);
    ESP_ERROR_CHECK(ret);
    ret=spi_bus_add_device(VSPI_HOST, &devcfg, &spi);
    ESP_ERROR_CHECK(ret);

    ESP_LOGI(TAG, "Initialize LVGL library");
    lv_init();
    ESP_LOGI(TAG, "Register display driver to LVGL");
    lv_disp_drv_init(&disp_drv);
    disp_drv.hor_res = kOSD_Width_px;
    disp_drv.ver_res = kOSD_Height_px;
    disp_drv.render_start_cb = pocket_render_probe_render_start;
    pocket_band_display_init(spi, &disp_buf, &disp_drv, capture_band);
    disp = lv_disp_drv_register(&disp_drv);
    extern const lv_font_t pocket_jp;
    pocket_render_probe_init(disp, &pocket_jp, "stripe48-o2", POCKET_BAND_BUFFER_BYTES);

    //Configuration is completed.

    ESP_LOGI(TAG, "Install LVGL tick timer");
    // Tick interface for LVGL (using esp_timer to generate 2ms periodic event)
    const esp_timer_create_args_t lvgl_tick_timer_args = {
        .callback = &lvgl_tick,
        .name = "lvgl_tick"
    };
    esp_timer_handle_t lvgl_tick_timer = NULL;
    ESP_ERROR_CHECK(esp_timer_create(&lvgl_tick_timer_args, &lvgl_tick_timer));
    ESP_ERROR_CHECK(esp_timer_start_periodic(lvgl_tick_timer, kLVGL_TickPeriod_us));

    ESP_LOGI(TAG, "Display LVGL animation");
    lv_obj_t *scr = lv_disp_get_scr_act(disp);

    OSD_Initialize();
    OSD_Default_Init(scr);
    register_update_callbacks();

    Gfx_Start(scr);

    xTaskCreatePinnedToCore(lvglTimerTask, "lvgl Timer", kTimerTask_StackDepth, NULL, 4, NULL, 1);

    // Prompt to be printed before each line.
    ReplConfig.prompt = "mcu> ";
    ReplConfig.max_cmdline_length = 1024;

    ReplHWConfig.rx_gpio_num = GPIO_NUM_35;
    ReplHWConfig.tx_gpio_num = GPIO_NUM_36;

    ESP_ERROR_CHECK(esp_console_new_repl_uart(&ReplHWConfig, &ReplConfig, &pRepl));
    esp_console_register_help_command();
    // Networking is optional: an initialization error must not prevent games or the OSD from running.
    esp_err_t wifi_result = CW_Init();
    if (wifi_result != ESP_OK)
    {
        printf("Wi-Fi initialization unavailable: %s\n", esp_err_to_name(wifi_result));
    }
    PocketUI_RegisterCommands();
    frame_ready=xSemaphoreCreateBinary();
    if(frame_ready){
        esp_console_cmd_t screen_cmd={.command="pocket_screen",.help="Read the actual LCD render buffer over USB.",.func=pocket_screen_command};
        esp_console_cmd_register(&screen_cmd);
    }
    esp_console_cmd_t display_cmd={.command="pocket_display",.help="Read display and task status (no credentials).",.func=pocket_display_command};
    esp_console_cmd_register(&display_cmd);
    // All commands must be registered prior to starting the REPL
    ESP_ERROR_CHECK(esp_console_start_repl(pRepl));

    // Keep the stock power-manager startup timing; the user opens the menu.
    vTaskDelay( pdMS_TO_TICKS(2000) );
    xTaskCreate(PwrMgr_Task, "pwr_mgr_task", kSleepTask_StackDepth, NULL, kSleepTask_Priority, NULL);
}

static int pocket_display_command(int argc,char **argv)
{
    (void)argc;(void)argv;
    printf("POCKETDISPLAY {\"visible\":%s,\"heap\":%lu,\"gui\":%s}\n",OSD_IsVisible()?"true":"false",(unsigned long)esp_get_free_heap_size(),disp?"true":"false");
    return 0;
}
static int pocket_screen_command(int argc,char **argv)
{
    (void)argc;(void)argv;
    PS_Frame storage={0};PS_Frame *snapshot=&storage;
    if(!PS_FrameInit(snapshot)){printf("POCKETFRAME_ERROR no_memory\n");return 1;}
    // The periodic battery/status redraw triggers a completed flush within this interval.
    xSemaphoreTake(frame_ready,0);
    taskENTER_CRITICAL(&frame_lock);
    frame_capture_failed=false;frame_request=snapshot;
    taskEXIT_CRITICAL(&frame_lock);
    if(xSemaphoreTake(frame_ready,pdMS_TO_TICKS(2000))!=pdTRUE){
        taskENTER_CRITICAL(&frame_lock);
        bool still_pending=frame_request==snapshot;
        if(still_pending)frame_request=NULL;
        taskEXIT_CRITICAL(&frame_lock);
        // If a flush took ownership at the timeout boundary, wait for its completion.
        if(!still_pending)xSemaphoreTake(frame_ready,portMAX_DELAY);
        else{PS_FrameFree(snapshot);printf("POCKETFRAME_ERROR no_redraw\n");return 1;}
    }
    if(frame_capture_failed){PS_FrameFree(snapshot);printf("POCKETFRAME_ERROR invalid_rows\n");return 1;}
    printf("POCKETFRAME 160 144 RGB565\n");
    for(unsigned y=0;y<144;y++){
        for(unsigned x=0;x<320;x++)printf("%02x",PS_FrameRow(snapshot,y)[x]);
        printf("\n");
    }
    printf("POCKETFRAME_END\n");
    PS_FrameFree(snapshot);return 0;
}
