// SPDX-License-Identifier: GPL-3.0-only
// Platform test scenes and opt-in actual native-UI rendering metrics; no E2E claim.
#include "pocket_render_probe.h"
#include <inttypes.h>
#include <stdio.h>
#include <string.h>
#include "esp_app_desc.h"
#include "esp_console.h"
#include "esp_heap_caps.h"
#include "esp_private/esp_clk.h"
#include "esp_timer.h"
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"

typedef struct {
    uint32_t handlers, handler_max_us, flushes, frames, frame_max_us;
    uint64_t handler_total_us, bytes, frame_total_us;
} metrics_t;

static portMUX_TYPE lock = portMUX_INITIALIZER_UNLOCKED;
static metrics_t metrics;
static unsigned requested;
static bool measuring, flush_last;
static int64_t render_started, next_update, window_start;
static unsigned active_case, updates, warmup;
static lv_disp_t *display;
static lv_obj_t *scene, *previous_screen, *body, *counter;
static const lv_font_t *probe_font;
static const char *render_mode;
static size_t buffer_bytes;
static char counter_text[16];
static uint32_t cpu_start, boot_minimum_before;

static const char *case_names[] = {"none", "idle", "counter", "page", "full", "restore", "show", "native"};
static const char *pages[] = {
    "01 pocket search\n02 small display\n03 bounded packets\n04 native rendering\n05 button controls\n06 verified bytes\n07 reader page one",
    "01 page two\n02 original fixture\n03 text stays data\n04 no browser script\n05 fixed row budget\n06 same test device\n07 native SPI output",
};

static int command(int argc, char **argv) {
    if (argc != 2) return 1;
    unsigned selected = 0;
    for (unsigned i = 1; i <= 4; ++i) {
        if (!strcmp(argv[1], case_names[i])) selected = i;
    }
    if (!strcmp(argv[1], "restore")) selected = 5;
    if (!strcmp(argv[1], "show")) selected = 6;
    if (!strcmp(argv[1], "native")) selected = 7;
    if (!selected) return 1;
    taskENTER_CRITICAL(&lock);
    bool available = !requested && !active_case;
    if (available) requested = selected;
    taskEXIT_CRITICAL(&lock);
    if (!available) return 1;
    printf("RENDER_ACCEPTED %s\n", argv[1]);
    return 0;
}

void pocket_render_probe_init(lv_disp_t *value, const lv_font_t *font,
                              const char *mode, size_t draw_buffer_bytes) {
    display = value;
    probe_font = font;
    render_mode = mode;
    buffer_bytes = draw_buffer_bytes;
    const esp_console_cmd_t cmd = {.command="render_probe", .help="Physical LVGL probe: idle/counter/page/full/restore/show/native (30-second actual UI window)", .func=command};
    ESP_ERROR_CHECK(esp_console_cmd_register(&cmd));
}

static lv_obj_t *label(int x, int y, const char *text) {
    lv_obj_t *value = lv_label_create(scene);
    lv_obj_set_style_text_font(value, probe_font, 0);
    lv_obj_set_style_text_color(value, lv_color_hex(0xd5edcc), 0);
    lv_obj_set_style_text_line_space(value, 0, 0);
    lv_obj_set_pos(value, x, y);
    lv_obj_set_width(value, 148);
    lv_label_set_long_mode(value, LV_LABEL_LONG_CLIP);
    lv_label_set_text_static(value, text);
    return value;
}

static void begin_scene(void) {
    if (scene) { lv_disp_load_scr(scene); return; }
    previous_screen = lv_disp_get_scr_act(display);
    scene = lv_obj_create(NULL);
    lv_obj_remove_style_all(scene);
    lv_obj_set_style_bg_color(scene, lv_color_hex(0x111b16), 0);
    lv_obj_set_style_bg_opa(scene, LV_OPA_COVER, 0);
    lv_obj_clear_flag(scene, LV_OBJ_FLAG_SCROLLABLE);
    label(6, 5, "POCKET SEARCH");
    body = label(6, 25, pages[0]);
    counter = label(6, 125, "FRAME 00");
    lv_disp_load_scr(scene);
}

void pocket_render_probe_render_start(lv_disp_drv_t *driver) {
    (void)driver;
    taskENTER_CRITICAL(&lock);
    if (measuring) render_started = esp_timer_get_time();
    taskEXIT_CRITICAL(&lock);
}

void pocket_render_probe_flush_start(size_t bytes, bool last) {
    taskENTER_CRITICAL(&lock);
    flush_last = last;
    if (measuring) { metrics.flushes++; metrics.bytes += bytes; }
    taskEXIT_CRITICAL(&lock);
}

// Called only by the existing SPI completion ISR after the final transaction.
void pocket_render_probe_flush_end(void) {
    const int64_t now = esp_timer_get_time();
    taskENTER_CRITICAL_ISR(&lock);
    if (measuring && flush_last && render_started) {
        uint32_t elapsed = (uint32_t)(now - render_started);
        metrics.frames++;
        metrics.frame_total_us += elapsed;
        if (elapsed > metrics.frame_max_us) metrics.frame_max_us = elapsed;
        render_started = 0;
    }
    taskEXIT_CRITICAL_ISR(&lock);
}

void pocket_render_probe_handler_time(unsigned elapsed_us) {
    taskENTER_CRITICAL(&lock);
    if (measuring) {
        metrics.handlers++;
        metrics.handler_total_us += elapsed_us;
        if (elapsed_us > metrics.handler_max_us) metrics.handler_max_us = elapsed_us;
    }
    taskEXIT_CRITICAL(&lock);
}

void pocket_render_probe_step(void) {
    const int64_t now = esp_timer_get_time();
    taskENTER_CRITICAL(&lock);
    unsigned value = !active_case ? requested : 0;
    if (value) { requested = 0; active_case = value; }
    taskEXIT_CRITICAL(&lock);
    if (value) {
        if (value == 6) {
            begin_scene();
            taskENTER_CRITICAL(&lock);
            active_case = 0;
            taskEXIT_CRITICAL(&lock);
            printf("RENDER_SHOWN\n");
            return;
        }
        if (value == 5) {
            if (scene) {
                lv_disp_load_scr(previous_screen);
                lv_obj_del(scene);
                scene = NULL;
            }
            taskENTER_CRITICAL(&lock);
            active_case = 0;
            taskEXIT_CRITICAL(&lock);
            return;
        }
        if(value==7){
            if(scene&&lv_disp_get_scr_act(display)==scene){taskENTER_CRITICAL(&lock);active_case=0;taskEXIT_CRITICAL(&lock);printf("RENDER_ERROR test_scene_active\n");return;}
            previous_screen=lv_disp_get_scr_act(display);
            boot_minimum_before=heap_caps_get_minimum_free_size(MALLOC_CAP_8BIT);
            ESP_ERROR_CHECK(heap_caps_monitor_local_minimum_free_size_start());
            taskENTER_CRITICAL(&lock);metrics=(metrics_t){0};render_started=0;measuring=true;taskEXIT_CRITICAL(&lock);
            cpu_start=(uint32_t)esp_clk_cpu_freq();window_start=now;warmup=0;updates=0;next_update=now+100000;return;
        }
        begin_scene();
        warmup = 6;
        updates = 0;
        next_update = now + 100000;
    }
    if (!active_case) return;
    if (lv_disp_get_scr_act(display) != (active_case==7?previous_screen:scene)) {
        taskENTER_CRITICAL(&lock);
        bool was_measuring = measuring;
        measuring = false;
        active_case = 0;
        taskEXIT_CRITICAL(&lock);
        if (was_measuring) ESP_ERROR_CHECK(heap_caps_monitor_local_minimum_free_size_stop());
        printf("RENDER_ERROR active_screen_changed\n");
        return;
    }
    if (now < next_update) return;
    next_update = now + 100000;
    if (warmup) {
        warmup--;
        if (!warmup) {
            boot_minimum_before = heap_caps_get_minimum_free_size(MALLOC_CAP_8BIT);
            ESP_ERROR_CHECK(heap_caps_monitor_local_minimum_free_size_start());
            taskENTER_CRITICAL(&lock);
            metrics = (metrics_t){0};
            render_started = 0;
            measuring = true;
            taskEXIT_CRITICAL(&lock);
            cpu_start = (uint32_t)esp_clk_cpu_freq();
            window_start = now;
        }
        return;
    }
    if ((active_case==7&&now-window_start>=30000000)||(active_case!=7&&updates==32)) {
        taskENTER_CRITICAL(&lock);
        measuring = false;
        metrics_t result = metrics;
        taskEXIT_CRITICAL(&lock);
        const esp_app_desc_t *app = esp_app_get_description();
        uint32_t window_minimum = heap_caps_get_minimum_free_size(MALLOC_CAP_8BIT);
        ESP_ERROR_CHECK(heap_caps_monitor_local_minimum_free_size_stop());
        printf("RENDER_REPORT {\"schema_version\":1,\"execution_mode\":\"%s\",\"firmware_version\":\"%s\",\"mode\":\"%s\",\"case\":\"%s\",\"updates\":%u,\"draw_buffer_bytes\":%u,\"cpu_hz_before\":%"PRIu32",\"cpu_hz_after\":%lu,\"core\":%u,\"elapsed_us\":%"PRId64",\"handler_calls\":%"PRIu32",\"handler_total_us\":%"PRIu64",\"handler_max_us\":%"PRIu32",\"flushes\":%"PRIu32",\"transfer_bytes\":%"PRIu64",\"frames\":%"PRIu32",\"frame_total_us\":%"PRIu64",\"frame_max_us\":%"PRIu32",\"free_heap_bytes\":%u,\"largest_free_block_bytes\":%u,\"boot_min_before_window_bytes\":%u,\"window_min_free_bytes\":%u,\"gui_stack_free_bytes\":%u}\n",
            active_case==7?"physical-native-ui-render":"physical-lvgl-platform-probe",app->version, render_mode, case_names[active_case], updates, (unsigned)buffer_bytes,
            cpu_start, (unsigned long)esp_clk_cpu_freq(), (unsigned)xPortGetCoreID(), now-window_start,
            result.handlers, result.handler_total_us, result.handler_max_us, result.flushes,
            result.bytes, result.frames, result.frame_total_us, result.frame_max_us,
            (unsigned)heap_caps_get_free_size(MALLOC_CAP_8BIT),
            (unsigned)heap_caps_get_largest_free_block(MALLOC_CAP_8BIT),
            (unsigned)boot_minimum_before, (unsigned)window_minimum,
            (unsigned)uxTaskGetStackHighWaterMark(NULL));
        taskENTER_CRITICAL(&lock);
        active_case = 0;
        taskEXIT_CRITICAL(&lock);
        return;
    }
    if(active_case==7){++updates;return;}
    if (active_case == 2 || active_case == 4) {
        snprintf(counter_text, sizeof(counter_text), "FRAME %02u", updates);
        lv_label_set_text_static(counter, counter_text);
    }
    if (active_case == 3) lv_label_set_text_static(body, pages[updates & 1]);
    if (active_case == 4) lv_obj_set_style_bg_color(scene, lv_color_hex((updates & 1) ? 0x17271a : 0x111b16), 0);
    updates++;
}
