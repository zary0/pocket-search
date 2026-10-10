// SPDX-License-Identifier: GPL-3.0-only
// Native search/input UI prototype. PCS1 networking and reader are not connected.
#include "pocket_search_ui.h"
#include "pocket_search_input.h"
#include "chromatic_wifi.h"
#include "menu_mgr.h"
#include "tab_shared.h"
#include "esp_console.h"
#include "freertos/FreeRTOS.h"
#include <stdio.h>
#include <string.h>
LV_FONT_DECLARE(pocket_jp);
typedef enum { HOME, KEYBOARD, NOTICE, WIFI_LIST, WIFI_WAIT, WIFI_CONFIRM } Mode;
typedef enum { QUERY, SSID, PASSWORD } Role;
static struct {
    Mode mode; Role role, notice_role; int script, group, letter, selection;
    bool choosing, dirty;
    char query[PS_INPUT_CAPACITY], draft[PS_INPUT_CAPACITY], ssid[33];
    const char *notice; CW_Status wifi;
} u = {.dirty=true};
static lv_obj_t *root, *brand, *field, *input, *heading, *body, *footer, *status, *submit, *cells[12], *entry;
static const char *kana[] = {"あいうえお","かきくけこ","さしすせそ","たちつてと","なにぬねの","はひふへほ","まみむめも","やゆよ","らりるれろ","わをんー","゛゜小","空白"};
static const char *lower[] = {"abc","def","ghi","jkl","mno","pqrs","tuv","wxyz","01234","56789",".,!?-_/","空白"};
static const char *upper[] = {"ABC","DEF","GHI","JKL","MNO","PQRS","TUV","WXYZ","01234","56789",".,!?-_/","空白"};
static const char *symbols[] = {"0123","4567","89","@#$","%&*","()+","-=:",";.,","!?/","[]{}<>","`\\|~^'\"","空白"};
static portMUX_TYPE lock = portMUX_INITIALIZER_UNLOCKED;
static int pending_key;
static unsigned observed_mode, observed_bytes;
static const char *key_names[] = {"home","up","down","left","right","a","b","start","select"};
static const Button_t buttons[] = {0,kButton_Up,kButton_Down,kButton_Left,kButton_Right,kButton_A,kButton_B,kButton_Start,kButton_Select};
static const char **groups(void) { return u.script==0?kana:u.script==1?lower:u.script==2?upper:symbols; }
static unsigned count(const char *text) { unsigned n=0; for(;*text;++text) if(((unsigned char)*text&0xc0)!=0x80) ++n; return n; }
static void scalar_at(const char *text, unsigned index, char out[5]) {
    const unsigned char *s=(const unsigned char *)text;
    while(index-- && *s) { ++s; while((*s&0xc0)==0x80) ++s; }
    size_t n=ps_scalar(s,strlen((const char *)s)); memset(out,0,5); if(n) memcpy(out,s,n);
}
static const char *tail(const char *text, unsigned cells_limit) {
    unsigned cells_count=0; for(const unsigned char *s=(const unsigned char *)text;*s;++s) if((*s&0xc0)!=0x80) cells_count+=*s<128?1:2;
    while(cells_count>cells_limit && *text) { unsigned width=(unsigned char)*text<128?1:2; ++text; while(((unsigned char)*text&0xc0)==0x80) ++text; cells_count-=width; }
    return text;
}
static lv_obj_t *label(lv_obj_t *parent,int x,int y,int w) {
    lv_obj_t *o=lv_label_create(parent); lv_obj_remove_style_all(o);
    lv_obj_set_style_text_font(o,&pocket_jp,0); lv_obj_set_style_text_color(o,lv_color_hex(0x202124),0);
    lv_obj_set_style_text_line_space(o,1,0); lv_obj_set_pos(o,x,y); lv_obj_set_width(o,w);
    lv_label_set_long_mode(o,LV_LABEL_LONG_CLIP); return o;
}
static void visible(lv_obj_t *o, bool show) { if(show) lv_obj_clear_flag(o,LV_OBJ_FLAG_HIDDEN); else lv_obj_add_flag(o,LV_OBJ_FLAG_HIDDEN); }
static void create(lv_obj_t *screen) {
    root=lv_obj_create(screen); lv_obj_remove_style_all(root); lv_obj_set_pos(root,0,0); lv_obj_set_size(root,160,144);
    lv_obj_set_style_bg_color(root,lv_color_hex(0xffffff),0); lv_obj_set_style_bg_opa(root,LV_OPA_COVER,0); lv_obj_clear_flag(root,LV_OBJ_FLAG_SCROLLABLE);
    brand=label(root,5,34,150); lv_obj_set_style_text_color(brand,lv_color_hex(0x1a73e8),0); lv_obj_set_style_text_align(brand,LV_TEXT_ALIGN_CENTER,0);
#if LV_FONT_MONTSERRAT_20
    lv_obj_set_style_text_font(brand,&lv_font_montserrat_20,0);
#else
    lv_obj_set_style_text_font(brand,LV_FONT_DEFAULT,0);
#endif
    lv_label_set_text_static(brand,"Pocket Search");
    field=lv_obj_create(root); lv_obj_remove_style_all(field); lv_obj_set_style_radius(field,13,0); lv_obj_set_style_border_width(field,1,0);
    lv_obj_set_style_border_color(field,lv_color_hex(0xdadce0),0); lv_obj_clear_flag(field,LV_OBJ_FLAG_SCROLLABLE);
    input=label(field,10,6,124); lv_obj_set_style_text_color(input,lv_color_hex(0x5f6368),0);
    heading=label(root,8,4,144); body=label(root,8,28,144); lv_obj_set_height(body,90);
    footer=label(root,8,129,144); lv_obj_set_style_text_color(footer,lv_color_hex(0x70757a),0);
    status=label(root,104,5,48); lv_obj_set_style_text_align(status,LV_TEXT_ALIGN_RIGHT,0); lv_obj_set_style_text_color(status,lv_color_hex(0x70757a),0);
    submit=label(root,48,105,64); lv_obj_set_style_text_align(submit,LV_TEXT_ALIGN_CENTER,0); lv_obj_set_style_bg_color(submit,lv_color_hex(0xf1f3f4),0);
    lv_obj_set_style_bg_opa(submit,LV_OPA_COVER,0); lv_obj_set_style_radius(submit,3,0); lv_obj_set_height(submit,18); lv_label_set_text_static(submit,"Search");
    for(unsigned i=0;i<12;++i) { cells[i]=label(root,0,0,46); lv_obj_set_height(cells[i],16); lv_obj_set_style_radius(cells[i],3,0); }
    u.dirty=true;
}
static void mode(Mode value) { u.mode=value; u.selection=0; u.notice=NULL; u.dirty=true; }
static void keyboard(Role role) {
    mode(KEYBOARD); u.role=role; u.script=role==QUERY?0:1; u.group=0; u.letter=0; u.choosing=false;
    memset(u.draft,0,sizeof(u.draft)); if(role==QUERY) memcpy(u.draft,u.query,sizeof(u.query));
}
static void message(const char *value,Role destination) { mode(NOTICE); u.notice=value; u.notice_role=destination; }
static void submit_query(void) {
    if(u.mode==KEYBOARD && u.role==QUERY) memcpy(u.query,u.draft,sizeof(u.query));
    bool empty=true; for(const char *s=u.query;*s;++s) if(*s!=' ') empty=false;
    if(empty) { message("Enter a query.\n\nA: Type",QUERY); return; }
    // Do not open Google, contact Codex, or invent search results.
    message("Search service\nnot connected yet.\n\nQuery is ready.\nB: Search home",QUERY);
}
static void modify_kana(void) {
    static const char *cycles[]={"あぁ","いぃ","うぅゔ","えぇ","おぉ","かが","きぎ","くぐ","けげ","こご","さざ","しじ","すず","せぜ","そぞ","ただ","ちぢ","つっづ","てで","とど","はばぱ","ひびぴ","ふぶぷ","へべぺ","ほぼぽ","やゃ","ゆゅ","よょ","わゎ"};
    unsigned n=count(u.draft); if(!n) return; char last[5],next[5]; scalar_at(u.draft,n-1,last);
    for(unsigned i=0;i<sizeof(cycles)/sizeof(*cycles);++i) for(unsigned j=0;j<count(cycles[i]);++j) {
        scalar_at(cycles[i],j,next); if(strcmp(last,next)) continue;
        scalar_at(cycles[i],(j+1)%count(cycles[i]),next); ps_erase(u.draft); (void)ps_append(u.draft,PS_QUERY_BYTES,next); return;
    }
}
static void render(void) {
    char text[128]; bool home=u.mode==HOME, typing=u.mode==KEYBOARD;
    visible(brand,home); visible(submit,home); visible(status,home); visible(field,home||typing); visible(heading,!home); visible(body,!home&&!typing);
    for(unsigned i=0;i<12;++i) visible(cells[i],false);
    lv_label_set_text_static(status,u.wifi.state==CW_CONNECTED?"Wi-Fi":"Offline");
    lv_obj_set_pos(field,8,home?72:25); lv_obj_set_size(field,144,28);
    lv_label_set_text_static(heading,"Pocket Search"); lv_label_set_text_static(footer,"A:Type Start:Search");
    if(home) { lv_label_set_text(input,u.query[0]?tail(u.query,20):"検索語を入力"); }
    else if(typing) {
        lv_label_set_text_static(heading,u.role==QUERY?"Search":u.role==SSID?"Wi-Fi name":"Wi-Fi password");
        if(u.role==PASSWORD) { unsigned n=count(u.draft); if(n>20) n=20; memset(text,'*',n); text[n]=0; }
        else snprintf(text,sizeof(text),"%s_",tail(u.draft,19));
        lv_label_set_text(input,text); const char **g=groups(); unsigned total=u.choosing?count(g[u.group]):12;
        for(unsigned i=0;i<total;++i) {
            if(u.choosing) scalar_at(g[u.group],i,text);
            else if(u.script==0 && i<10) { scalar_at(g[i],0,text); strcat(text,"行"); }
            else snprintf(text,sizeof(text),"%s",g[i]);
            lv_obj_t *o=cells[i]; visible(o,true); lv_obj_set_pos(o,8+(i%3)*49,59+(i/3)*17);
            bool selected=i==(unsigned)(u.choosing?u.letter:u.group); lv_label_set_text(o,text);
            lv_obj_set_style_bg_color(o,lv_color_hex(0x1a73e8),0); lv_obj_set_style_bg_opa(o,selected?LV_OPA_COVER:LV_OPA_TRANSP,0);
            lv_obj_set_style_text_color(o,lv_color_hex(selected?0xffffff:0x202124),0);
        }
        lv_label_set_text_static(footer,u.notice?u.notice:"Select:ABC Start:OK");
    } else if(u.mode==NOTICE) { lv_label_set_text_static(body,u.notice?u.notice:""); lv_label_set_text_static(heading,u.notice_role==QUERY?"Pocket Search":"Wi-Fi"); lv_label_set_text_static(footer,u.notice_role==QUERY?"A:Type B:Back":"A:Retype B:Wi-Fi"); }
    else if(u.mode==WIFI_LIST) {
        lv_label_set_text_static(heading,"Wi-Fi"); unsigned first=u.selection>3?(unsigned)u.selection-3:0;
        for(unsigned i=0;i<4 && first+i<(unsigned)u.wifi.count+2;++i) {
            unsigned item=first+i; const char *name=item==0?"Scan again":item==1?"Enter name":u.wifi.networks[item-2].ssid;
            snprintf(text,sizeof(text),"%s%s",item==(unsigned)u.selection?"> ":"  ",name);
            visible(cells[i],true); lv_obj_set_pos(cells[i],8,29+i*22); lv_obj_set_width(cells[i],144); lv_label_set_text(cells[i],text);
            lv_obj_set_style_bg_opa(cells[i],LV_OPA_TRANSP,0); lv_obj_set_style_text_color(cells[i],lv_color_hex(0x202124),0);
        }
        lv_label_set_text_static(footer,u.wifi.scanning?"Scanning... B:Back":"A:Connect B:Back"); lv_label_set_text_static(body,"");
    } else if(u.mode==WIFI_WAIT) {
        lv_label_set_text_static(heading,"Wi-Fi"); lv_label_set_text_static(body,u.wifi.state==CW_CONNECTED?"Connected.\n\nA: Search home":u.wifi.state==CW_FAILED?"Connection failed.\n\nA: Try again":"Connecting..."); lv_label_set_text_static(footer,"A:Continue B:Back");
    } else if(u.mode==WIFI_CONFIRM) {
        lv_label_set_text_static(heading,"Connect Wi-Fi?"); snprintf(text,sizeof(text),"%s\n\nPassword: %u chars",u.ssid,count(u.draft)); lv_label_set_text(body,text); lv_label_set_text_static(footer,"A:Connect B:Edit");
    }
    for(unsigned i=0;i<12;++i) lv_obj_set_width(cells[i],u.mode==WIFI_LIST?144:46);
    lv_obj_move_foreground(root); u.dirty=false;
    taskENTER_CRITICAL(&lock); observed_mode=(unsigned)u.mode; observed_bytes=(unsigned)strlen(u.query); taskEXIT_CRITICAL(&lock);
}
OSD_Result_t PocketUI_Draw(void *arg) {
    Tab_DrawCtx_t *ctx=arg; if(!ctx || !ctx->pScreen) return kOSD_Result_Err_NullDataPtr;
    if(!root) create(ctx->pScreen);
    CW_State old=u.wifi.state; unsigned count_before=u.wifi.count; bool scanning=u.wifi.scanning;
    CW_GetStatus(&u.wifi); if(old!=u.wifi.state||count_before!=u.wifi.count||scanning!=u.wifi.scanning) u.dirty=true;
    if(u.dirty) { render(); }
    lv_obj_move_foreground(root); return kOSD_Result_Ok;
}
OSD_Result_t PocketUI_Button(Button_t b,ButtonState_t state,void *arg) {
    (void)arg; if(state!=kButtonState_Pressed) return kOSD_Result_Ok; u.dirty=true;
    int delta=(b==kButton_Down||b==kButton_Right)?1:(b==kButton_Up||b==kButton_Left)?-1:0;
    if(u.mode==HOME) {
        if(b==kButton_A) keyboard(QUERY);
        else if(b==kButton_Start) submit_query();
        else if(b==kButton_Select) { mode(WIFI_LIST); if(!u.wifi.scanning) (void)CW_Scan(); }
        else if(b==kButton_B) MenuMgr_SelectTab(kTabID_System);
    } else if(u.mode==KEYBOARD) {
        const char **g=groups();
        if(b==kButton_Select) { u.script=(u.script+1)%4; u.choosing=false; u.notice=NULL; }
        else if(b==kButton_B) { if(u.choosing) u.choosing=false; else if(u.draft[0]) ps_erase(u.draft); else mode(u.role==QUERY?HOME:WIFI_LIST); }
        else if(u.choosing) {
            unsigned n=count(g[u.group]); int step=b==kButton_Down?3:b==kButton_Up?-3:delta; if(step) u.letter=(int)ps_grid_move((unsigned)u.letter,n,step);
            if(b==kButton_A) { char scalar[5]; scalar_at(g[u.group],u.letter,scalar); size_t limit=u.role==QUERY?96:u.role==SSID?32:63;
                if(!ps_append(u.draft,limit,scalar)) u.notice="Input limit reached"; else u.notice=NULL; u.choosing=false; }
        } else {
            int step=b==kButton_Down?3:b==kButton_Up?-3:delta; if(step) u.group=(u.group+step+12)%12;
            if(b==kButton_A) { if(u.group==11) { size_t limit=u.role==QUERY?96:u.role==SSID?32:63; if(!ps_append(u.draft,limit," ")) u.notice="Input limit reached"; }
                else if(u.script==0&&u.group==10) modify_kana(); else { u.choosing=true; u.letter=0; } }
        }
        if(b==kButton_Start) {
            if(u.role==QUERY) submit_query();
            else if(u.role==SSID && u.draft[0]) { memcpy(u.ssid,u.draft,33); keyboard(PASSWORD); }
            else if(u.role==PASSWORD) mode(WIFI_CONFIRM);
        }
    } else if(u.mode==NOTICE) { if(b==kButton_A) keyboard(u.notice_role); else if(b==kButton_B||b==kButton_Select) mode(u.notice_role==QUERY?HOME:WIFI_LIST); }
    else if(u.mode==WIFI_LIST) {
        int n=(int)u.wifi.count+2; if(delta) u.selection=(u.selection+delta+n)%n;
        if(b==kButton_B) mode(HOME);
        if(b==kButton_A) { if(u.selection==0) { if(!u.wifi.scanning) (void)CW_Scan(); }
            else if(u.selection==1) keyboard(SSID);
            else if(u.selection-2<(int)u.wifi.count) { memcpy(u.ssid,u.wifi.networks[u.selection-2].ssid,33); keyboard(PASSWORD); } }
    } else if(u.mode==WIFI_CONFIRM) {
        if(b==kButton_B) { u.mode=KEYBOARD; u.choosing=false; }
        if(b==kButton_A) { esp_err_t e=CW_Join(u.ssid,u.draft); memset(u.draft,0,sizeof(u.draft)); if(e==ESP_OK) mode(WIFI_WAIT); else message("Invalid Wi-Fi password.\nUse 8-63 bytes.",PASSWORD); }
    } else if(u.mode==WIFI_WAIT) {
        if(b==kButton_B) mode(HOME);
        if(b==kButton_A) { if(u.wifi.state==CW_CONNECTED) mode(HOME); else if(u.wifi.state==CW_FAILED) keyboard(PASSWORD); }
    }
    return kOSD_Result_Ok;
}
OSD_Result_t PocketUI_Transition(void *arg) { (void)arg; if(root) lv_obj_del(root); root=NULL; u.dirty=true; return kOSD_Result_Ok; }
OSD_Result_t PocketEntry_Draw(void *screen) { if(!entry) { entry=label(screen,77,44,72); lv_label_set_text_static(entry,"SEARCH\n\nA: Open"); } return kOSD_Result_Ok; }
OSD_Result_t PocketEntry_Button(Button_t b,ButtonState_t s,void *arg) { (void)arg; if(b==kButton_A && s==kButtonState_Pressed) MenuMgr_SelectTab(kTabID_Search); return kOSD_Result_Ok; }
OSD_Result_t PocketEntry_Transition(void *arg) { (void)arg; if(entry) lv_obj_del(entry); entry=NULL; return kOSD_Result_Ok; }
static int command(int argc,char **argv) {
    if(argc!=2) return 1;
    if(!strcmp(argv[1],"status")) { taskENTER_CRITICAL(&lock); unsigned m=observed_mode,n=observed_bytes; taskEXIT_CRITICAL(&lock); printf("SEARCH_UI {\"mode\":%u,\"query_bytes\":%u,\"transport_connected\":false}\n",m,n); return 0; }
    for(unsigned i=0;i<sizeof(key_names)/sizeof(*key_names);++i) if(!strcmp(argv[1],key_names[i])) {
        taskENTER_CRITICAL(&lock); bool available=!pending_key; if(available) pending_key=(int)i+1; taskEXIT_CRITICAL(&lock);
        if(available) { printf("SEARCH_UI_QUEUED\n"); }
        return available?0:1;
    }
    return 1;
}
void PocketUI_RegisterCommands(void) { const esp_console_cmd_t cmd={.command="search_ui",.help="Synthetic GUI input: home/up/down/left/right/a/b/start/select/status",.func=command}; ESP_ERROR_CHECK(esp_console_cmd_register(&cmd)); }
void PocketUI_Poll(void) {
    taskENTER_CRITICAL(&lock); int key=pending_key; pending_key=0; taskEXIT_CRITICAL(&lock); if(!key) return;
    if(key==1) { mode(HOME); MenuMgr_SelectTab(kTabID_Search); }
    else (void)PocketUI_Button(buttons[key-1],kButtonState_Pressed,NULL);
}
