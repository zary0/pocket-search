// SPDX-License-Identifier: GPL-3.0-only
// Native UI. Rust produces PCS1 from pages collected and indexed on this console.
#include "pocket_search_ui.h"
#include "pocket_search_input.h"
#include "pocket_search_net.h"
#include "pocket_typography.h"
#include "chromatic_wifi.h"
#include "menu_mgr.h"
#include "tab_shared.h"
#include "esp_console.h"
#include "freertos/FreeRTOS.h"
#include <stdio.h>
#include <string.h>
#include <stdlib.h>
LV_FONT_DECLARE(pocket_jp);
typedef enum { HOME, KEYBOARD, NOTICE, WIFI_LIST, WIFI_WAIT, WIFI_CONFIRM, LOADING, RESULTS, READER, IMAGE_LOADING, IMAGE_READER } Mode;
typedef enum { QUERY, SSID, PASSWORD } Role;
static struct {
    Mode mode; Role role, notice_role; int script, group, letter, selection;
    bool choosing, dirty;
    char query[PS_INPUT_CAPACITY], draft[PS_INPUT_CAPACITY], ssid[33],generation[PS_GENERATION_BYTES];
    bool sample;
    const char *notice; CW_Status wifi;
    unsigned sequence; uint16_t result_index,requested_page;uint32_t requested_id;uint8_t requested_kind;
    uint8_t packet[512];PS_View view;int network_error;bool verified;
    PS_RichView rich_view;PS_FlowView flow_view;bool rich,flow,probe;
    uint32_t probe_document;uint16_t probe_position;
    uint8_t *image_pixels;uint16_t image_width,image_height,image_rows;
    struct {uint32_t document;uint16_t page;} back[4];unsigned back_count;
} u = {.dirty=true,.sample=true};
static lv_obj_t *root, *brand, *field, *input, *heading, *body, *footer, *status, *submit, *cells[12], *entry;
static lv_obj_t *rows[7], *picture, *rich_body;
static lv_img_dsc_t image_descriptor;
static const char *kana[] = {"あいうえお","かきくけこ","さしすせそ","たちつてと","なにぬねの","はひふへほ","まみむめも","やゆよ","らりるれろ","わをんー","゛゜小","空白"};
static const char *lower[] = {"abc","def","ghi","jkl","mno","pqrs","tuv","wxyz","01234","56789",".,!?-_/","空白"};
static const char *upper[] = {"ABC","DEF","GHI","JKL","MNO","PQRS","TUV","WXYZ","01234","56789",".,!?-_/","空白"};
static const char *symbols[] = {"0123","4567","89","@#$","%&*","()+","-=:",";.,","!?/","[]{}<>","`\\|~^'\"","空白"};
static portMUX_TYPE lock = portMUX_INITIALIZER_UNLOCKED;
static int pending_key;
static unsigned observed_mode, observed_bytes, observed_image_bytes, observed_back;
static bool observed_rich,observed_probe;static unsigned observed_large,observed_small,observed_bold,observed_gaps;
static PS_View observed_view;static bool observed_verified;
static bool pending_query_set;static char pending_query[97];
static unsigned pending_probe;
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
static void draw_rich(lv_event_t *event){if(u.flow)PS_FlowDraw(event,u.packet,&u.flow_view);else PS_TypographyDraw(event,u.packet,&u.rich_view);}
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
    submit=label(root,36,105,88); lv_obj_set_style_text_align(submit,LV_TEXT_ALIGN_CENTER,0); lv_obj_set_style_bg_color(submit,lv_color_hex(0xf1f3f4),0);
    lv_obj_set_style_bg_opa(submit,LV_OPA_COVER,0); lv_obj_set_style_radius(submit,3,0); lv_obj_set_height(submit,18); lv_label_set_text_static(submit,"Start:Search");
    for(unsigned i=0;i<12;++i) { cells[i]=label(root,0,0,46); lv_obj_set_height(cells[i],16); lv_obj_set_style_radius(cells[i],3,0); }
    for(unsigned i=0;i<7;++i){rows[i]=label(root,8,34+i*13,144);lv_obj_set_height(rows[i],13);}
    rich_body=lv_obj_create(root);lv_obj_remove_style_all(rich_body);lv_obj_set_pos(rich_body,8,34);lv_obj_set_size(rich_body,144,92);
    lv_obj_clear_flag(rich_body,LV_OBJ_FLAG_SCROLLABLE);lv_obj_add_event_cb(rich_body,draw_rich,LV_EVENT_DRAW_MAIN,NULL);visible(rich_body,false);
    picture=lv_img_create(root);lv_obj_add_flag(picture,LV_OBJ_FLAG_HIDDEN);
    u.dirty=true;
}
static void release_image(void){
    if(picture){visible(picture,false);lv_img_set_src(picture,NULL);}
    lv_img_cache_invalidate_src(&image_descriptor);free(u.image_pixels);u.image_pixels=NULL;u.image_width=u.image_height=u.image_rows=0;
    memset(&image_descriptor,0,sizeof(image_descriptor));
}
static void cancel(void){PS_NetCancel(u.sequence);++u.sequence;release_image();}
static void mode(Mode value) { u.mode=value;if(value!=READER)u.probe=false;u.selection=0; u.notice=NULL; u.dirty=true; }
static void keyboard(Role role) {
    mode(KEYBOARD); u.role=role; u.script=role==QUERY?0:1; u.group=0; u.letter=0; u.choosing=false;
    memset(u.draft,0,sizeof(u.draft)); if(role==QUERY) memcpy(u.draft,u.query,sizeof(u.query));
}
static void message(const char *value,Role destination) { mode(NOTICE); u.notice=value; u.notice_role=destination; }
static void request(uint8_t kind,uint32_t document,uint16_t page){
    u.probe=false;
    PS_NetCancel(u.sequence);release_image();
    PS_Request r={.sequence=++u.sequence,.kind=kind,.document=document,.page=page};if(kind==1)memcpy(r.query,u.query,sizeof(r.query));memcpy(r.generation,u.generation,sizeof(r.generation));
    u.requested_kind=kind;u.requested_id=document;u.requested_page=page;u.network_error=0;u.verified=false;
    if(!PS_NetStart(&r)){message("Previous request\nfinishing. Try again.",QUERY);return;}
    mode(kind==3?IMAGE_LOADING:LOADING);
}
static void submit_query(void) {
    if(u.mode==KEYBOARD && u.role==QUERY) memcpy(u.query,u.draft,sizeof(u.query));
    bool empty=true; for(const char *s=u.query;*s;++s) if(*s!=' ') empty=false;
    if(empty) { message("Enter a query.\n\nA: Type",QUERY); return; }
    u.generation[0]=0;u.result_index=0;u.back_count=0;request(1,0,0);
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
    for(unsigned i=0;i<7;++i)visible(rows[i],false);
    visible(rich_body,false);
    visible(picture,false);
    lv_label_set_text_static(status,u.wifi.state==CW_CONNECTED?"Wi-Fi":"Offline");
    lv_obj_set_pos(field,8,home?72:25); lv_obj_set_size(field,144,28);
    lv_label_set_text_static(heading,"Pocket Search"); lv_label_set_text_static(footer,"A:Type >:Recrawl");
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
    } else if(u.mode==LOADING||u.mode==IMAGE_LOADING){lv_label_set_text_static(body,"On-device search\n\nCollecting if needed\nVerified HTTPS");lv_label_set_text_static(footer,"B:Cancel Start:Type");}
    else if(u.mode==RESULTS||u.mode==READER||u.mode==IMAGE_READER){
        visible(body,false);visible(status,true);lv_obj_set_pos(status,8,3);lv_obj_set_width(status,144);lv_obj_set_style_text_align(status,LV_TEXT_ALIGN_LEFT,0);
        if(u.probe)snprintf(text,sizeof(text),"TEST");
        else if(u.mode==READER)snprintf(text,sizeof(text),"%u%%",u.view.page_count>1?(unsigned)u.view.page_index*100/(u.view.page_count-1):100);
        else snprintf(text,sizeof(text),"%u/%u",u.view.page_index+1,u.view.page_count);
        lv_label_set_text(status,text);lv_obj_set_pos(heading,8,18);
        memcpy(text,u.packet+u.view.title.offset,u.view.title.length);text[u.view.title.length]=0;lv_label_set_text(heading,text);lv_obj_set_style_text_color(heading,lv_color_hex(0x1a73e8),0);
        if(u.mode==IMAGE_READER){
            visible(picture,true);lv_obj_set_pos(picture,(160-u.image_width)/2,34+(80-u.image_height)/2);
            if(u.view.line_count){memcpy(text,u.packet+u.view.lines[0].offset,u.view.lines[0].length);text[u.view.lines[0].length]=0;visible(rows[0],true);lv_obj_set_pos(rows[0],8,116);lv_label_set_text(rows[0],text);}
        }else if(u.rich||u.flow){lv_obj_set_pos(rich_body,u.flow?0:8,34);lv_obj_set_size(rich_body,u.flow?160:144,92);visible(rich_body,true);lv_obj_invalidate(rich_body);}
        else for(unsigned i=0;i<u.view.line_count;++i){
            memcpy(text,u.packet+u.view.lines[i].offset,u.view.lines[i].length);text[u.view.lines[i].length]=0;
            visible(rows[i],true);lv_obj_set_pos(rows[i],8,34+i*13);lv_label_set_text(rows[i],text);
            lv_obj_set_style_text_color(rows[i],lv_color_hex(u.view.kind==5||u.view.kind==4?0x1a73e8:0x202124),0);
        }
        lv_label_set_text_static(footer,u.probe?"Original test. B:Back":u.mode==RESULTS?"Up/Down A:Read B:Back":u.view.kind==4&&u.view.link_target?"A:Open Up/Down B":u.flow&&u.flow_view.limited?"Color limit. Up/Down B":"Up/Down:Scroll B:Back");
    }
    else if(u.mode==NOTICE) { lv_label_set_text_static(body,u.notice?u.notice:""); lv_label_set_text_static(heading,u.notice_role==QUERY?"Pocket Search":"Wi-Fi"); lv_label_set_text_static(footer,u.notice_role==QUERY?"A:Type B:Back":"A:Retype B:Wi-Fi"); }
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
    if(home){lv_obj_set_pos(status,104,5);lv_obj_set_width(status,48);lv_obj_set_style_text_align(status,LV_TEXT_ALIGN_RIGHT,0);}
    if(u.mode!=RESULTS&&u.mode!=READER&&u.mode!=IMAGE_READER){lv_obj_set_pos(heading,8,4);lv_obj_set_style_text_color(heading,lv_color_hex(0x202124),0);}
    unsigned large=0,small=0,bold=0,gaps=0;bool styled=u.mode==READER&&(u.rich||u.flow);
    if(styled){const unsigned sizes[]={12,16,14,10};small=16;unsigned row_count=u.flow?u.flow_view.row_count:u.rich_view.row_count;for(unsigned i=0;i<row_count;++i){unsigned glyphs=u.flow?u.flow_view.rows[i].codes.length:u.rich_view.rows[i].glyphs;if(!glyphs)++gaps;for(unsigned j=0;j<glyphs;++j){unsigned s=u.flow?(u.packet[u.flow_view.rows[i].codes.offset+j]&15):(u.packet[u.rich_view.rows[i].styles.offset+j/2]>>((j%2)*4))&15,n=sizes[s&3];if(n>large)large=n;if(n<small)small=n;if(s&4)++bold;}}}
    taskENTER_CRITICAL(&lock); observed_mode=(unsigned)u.mode; observed_bytes=(unsigned)strlen(u.query);observed_view=u.view;observed_verified=u.verified;observed_image_bytes=(unsigned)u.image_width*u.image_height*2;observed_back=u.back_count;observed_rich=styled;observed_probe=u.probe;observed_large=large;observed_small=small;observed_bold=bold;observed_gaps=gaps; taskEXIT_CRITICAL(&lock);
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
    if(u.probe){
        if(b==kButton_B)request(2,u.probe_document,u.probe_position);
        else if(u.flow&&(b==kButton_Up||b==kButton_Down||b==kButton_Left||b==kButton_Right)){
            unsigned page=u.view.page_index,jump=(b==kButton_Left||b==kButton_Right)?7:1;
            if(b==kButton_Up||b==kButton_Left)page=page>jump?page-jump:0;else page=page+jump<u.view.page_count?page+jump:u.view.page_count-1;
            uint8_t bytes[512];PS_FlowView flow;int n=ps_flow_probe(bytes,page);
            if(n>0&&ps_flow_decode(bytes,(size_t)n,&flow)==0){memcpy(u.packet,bytes,(size_t)n);u.flow_view=flow;u.view.page_index=flow.position;u.dirty=true;}
        }
        return kOSD_Result_Ok;
    }
    if(u.mode==LOADING||u.mode==IMAGE_LOADING||u.mode==RESULTS||u.mode==READER||u.mode==IMAGE_READER){
        bool loading=u.mode==LOADING||u.mode==IMAGE_LOADING;
        if(b==kButton_Select||(b==kButton_B&&(loading||u.mode==RESULTS))){cancel();u.back_count=0;mode(HOME);return kOSD_Result_Ok;}
        if(b==kButton_Start){cancel();u.back_count=0;keyboard(QUERY);return kOSD_Result_Ok;}
        if(loading)return kOSD_Result_Ok;
        unsigned jump=u.mode==READER?7:1;
        if(b==kButton_B){if(u.back_count){--u.back_count;request(2,u.back[u.back_count].document,u.back[u.back_count].page);}else request(1,0,u.result_index);return kOSD_Result_Ok;}
        if(u.mode==RESULTS){
            if(b==kButton_A&&u.view.document_id){u.back_count=0;request(2,u.view.document_id,0);}
            else if(b==kButton_Up&&u.view.page_index)request(1,0,u.view.page_index-1);
            else if(b==kButton_Down&&u.view.page_index+1<u.view.page_count)request(1,0,u.view.page_index+1);
        }else if(b==kButton_A&&u.view.kind==4&&u.view.link_target){
            if(u.back_count==4){message("Link depth limit.\nB: Search home",QUERY);return kOSD_Result_Ok;}
            u.back[u.back_count].document=u.view.document_id;u.back[u.back_count++].page=u.view.page_index;request(2,u.view.link_target,0);
        }else if(b==kButton_Up&&u.view.page_index)request(2,u.view.document_id,u.view.page_index-1);
        else if(b==kButton_Down&&u.view.page_index+1<u.view.page_count)request(2,u.view.document_id,u.view.page_index+1);
        else if(b==kButton_Left&&u.view.page_index)request(2,u.view.document_id,u.view.page_index>jump?u.view.page_index-jump:0);
        else if(b==kButton_Right&&u.view.page_index+1<u.view.page_count)request(2,u.view.document_id,u.view.page_index+jump<u.view.page_count?u.view.page_index+jump:u.view.page_count-1);
        return kOSD_Result_Ok;
    }
    if(u.mode==HOME) {
        if(b==kButton_A) keyboard(QUERY);
        else if(b==kButton_Start) submit_query();
        else if(b==kButton_Right) { if(PS_NetRefresh()) message("Collection cleared.\nEnter query to crawl.",QUERY); else message("Previous request\nfinishing. Try again.",QUERY); }
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
OSD_Result_t PocketUI_Transition(void *arg) { (void)arg; cancel();u.back_count=0;mode(HOME);if(root) lv_obj_del(root);root=NULL;picture=NULL;u.dirty=true; return kOSD_Result_Ok; }
OSD_Result_t PocketEntry_Draw(void *screen) { if(!entry) { entry=label(screen,77,44,72); lv_label_set_text_static(entry,"SEARCH\n\nA: Open"); } return kOSD_Result_Ok; }
OSD_Result_t PocketEntry_Button(Button_t b,ButtonState_t s,void *arg) { (void)arg; if(b==kButton_A && s==kButtonState_Pressed) MenuMgr_SelectTab(kTabID_Search); return kOSD_Result_Ok; }
OSD_Result_t PocketEntry_Transition(void *arg) { (void)arg; if(entry) lv_obj_del(entry); entry=NULL; return kOSD_Result_Ok; }
static int command(int argc,char **argv) {
    if(argc==2&&(!strcmp(argv[1],"typography")||!strcmp(argv[1],"webfont")||!strcmp(argv[1],"layout"))){taskENTER_CRITICAL(&lock);bool available=!pending_key&&!pending_query_set&&!pending_probe;if(!strcmp(argv[1],"webfont")&&!PS_WebFontReady())available=false;if(available)pending_probe=!strcmp(argv[1],"layout")?3:!strcmp(argv[1],"webfont")?2:1;taskEXIT_CRITICAL(&lock);if(available)printf("SEARCH_UI_QUEUED\n");return available?0:1;}
    if(argc==3&&!strcmp(argv[1],"query")){size_t n=strlen(argv[2]);if(n>96||!ps_valid(argv[2],n))return 1;taskENTER_CRITICAL(&lock);bool available=!pending_key&&!pending_query_set&&!pending_probe;if(available){memset(pending_query,0,sizeof(pending_query));memcpy(pending_query,argv[2],n);pending_query_set=true;}taskEXIT_CRITICAL(&lock);if(available)printf("SEARCH_UI_QUEUED\n");return available?0:1;}
    if(argc!=2) return 1;
    if(!strcmp(argv[1],"status")) {
        taskENTER_CRITICAL(&lock);unsigned m=observed_mode,n=observed_bytes;PS_View v=observed_view;bool verified=observed_verified,styled=observed_rich,probe=observed_probe;unsigned image_bytes=observed_image_bytes,back=observed_back,large=observed_large,small=observed_small,bold=observed_bold,gaps=observed_gaps;taskEXIT_CRITICAL(&lock);
        printf("SEARCH_UI {\"mode\":%u,\"query_bytes\":%u,\"tls_verified\":%s,\"rust_decoder\":true,\"document_id\":%lu,\"page_index\":%u,\"page_count\":%u,\"kind\":%u,\"image_bytes\":%u,\"back_depth\":%u,\"styled\":%s,\"typography_probe\":%s,\"largest_font_px\":%u,\"smallest_font_px\":%u,\"bold_glyphs\":%u,\"paragraph_gaps\":%u}\n",m,n,verified?"true":"false",(unsigned long)v.document_id,v.page_index,v.page_count,v.kind,image_bytes,back,styled?"true":"false",probe?"true":"false",large,small,bold,gaps);return 0;
    }
    for(unsigned i=0;i<sizeof(key_names)/sizeof(*key_names);++i) if(!strcmp(argv[1],key_names[i])) {
        taskENTER_CRITICAL(&lock); bool available=!pending_key&&!pending_query_set&&!pending_probe; if(available) pending_key=(int)i+1; taskEXIT_CRITICAL(&lock);
        if(available) { printf("SEARCH_UI_QUEUED\n"); }
        return available?0:1;
    }
    return 1;
}
void PocketUI_RegisterCommands(void) { const esp_console_cmd_t cmd={.command="search_ui",.help="Synthetic GUI input and bounded query; no page data over USB",.func=command}; ESP_ERROR_CHECK(esp_console_cmd_register(&cmd));PS_NetRegister(); }
void PocketUI_Poll(void) {
    PS_Reply reply;
    if(PS_NetPoll(&reply)&&reply.sequence==u.sequence&&(u.mode==LOADING||u.mode==IMAGE_LOADING)){
        bool valid=!reply.error&&reply.tls_verified&&reply.clock_synced;
        if(u.mode==IMAGE_LOADING){
            PS_ImageView image;
            valid=valid&&ps_image_decode(reply.bytes,reply.length,&image)==0;
            if(valid&&(image.document_id!=u.requested_id||image.page_index!=u.requested_page||image.y!=u.image_rows||(u.image_rows&&(image.width!=u.image_width||image.height!=u.image_height))))valid=false;
            if(valid&&!u.image_pixels){u.image_width=image.width;u.image_height=image.height;u.image_pixels=malloc((size_t)image.width*image.height*2);if(!u.image_pixels)valid=false;}
            if(valid){memcpy(u.image_pixels+(size_t)image.y*image.width*2,reply.bytes+image.pixels_offset,image.pixels_length);u.image_rows+=image.rows;}
            if(valid&&reply.finished){
                if(u.image_rows!=u.image_height)valid=false;
                else{image_descriptor.header.cf=LV_IMG_CF_TRUE_COLOR;image_descriptor.header.w=u.image_width;image_descriptor.header.h=u.image_height;image_descriptor.data_size=(uint32_t)u.image_width*u.image_height*2;image_descriptor.data=u.image_pixels;lv_img_set_src(picture,&image_descriptor);u.verified=true;mode(IMAGE_READER);}
            }
        }else if(u.requested_kind==2&&reply.length>=4&&!memcmp(reply.bytes,"PFW1",4)){
            PS_FlowView flow;valid=valid&&reply.finished&&reply.length<=sizeof(u.packet)&&ps_flow_decode(reply.bytes,reply.length,&flow)==0;
            if(valid&&(flow.position!=u.requested_page||flow.document_id!=u.requested_id))valid=false;
            if(valid){memcpy(u.packet,reply.bytes,reply.length);u.flow_view=flow;u.flow=true;u.rich=false;u.verified=true;u.view=(PS_View){.document_id=flow.document_id,.page_index=flow.position,.page_count=flow.positions,.kind=flow.link_target?4:2,.title=flow.title,.link_target=flow.link_target};mode(READER);}
        }else if(u.requested_kind==2&&reply.length>=4&&!memcmp(reply.bytes,"PSR1",4)){
            PS_RichView rich;valid=valid&&reply.finished&&reply.length<=sizeof(u.packet)&&ps_rich_decode(reply.bytes,reply.length,&rich)==0;
            if(valid&&(rich.position!=u.requested_page||rich.document_id!=u.requested_id))valid=false;
            if(valid){memcpy(u.packet,reply.bytes,reply.length);u.rich_view=rich;u.rich=true;u.flow=false;u.verified=true;u.view=(PS_View){.document_id=rich.document_id,.page_index=rich.position,.page_count=rich.positions,.kind=2,.title=rich.title};mode(READER);}
        }else{
            PS_View view;valid=valid&&reply.finished&&reply.length<=sizeof(u.packet)&&ps_packet_decode(reply.bytes,reply.length,&view)==0;
            if(valid&&(view.page_index!=u.requested_page||(u.requested_kind==1?(view.kind!=1||view.page_count>256):(view.kind<2||view.kind>5||view.document_id!=u.requested_id))))valid=false;
            if(valid){memcpy(u.generation,reply.generation,sizeof(u.generation));u.sample=reply.sample;memcpy(u.packet,reply.bytes,reply.length);u.view=view;u.rich=false;u.flow=false;u.verified=true;if(view.kind==1){u.result_index=view.page_index;mode(RESULTS);}else if(view.kind==3)request(3,view.document_id,view.page_index);else mode(READER);}
        }
        if(!valid){release_image();u.network_error=reply.error;u.verified=false;message(reply.error==PS_NET_STALE_COLLECTION?"Collection changed.\nA: Search again":reply.error==1?"Wi-Fi disconnected.\nSelect: Wi-Fi setup":reply.error==2?"Clock sync failed.\nTry again.":reply.error==12?"robots.txt blocks\nthis collection.":reply.error==13?"No usable pages.\nRight on home:\nRetry collection.":reply.error?"Connection failed.\nTry again.":"Invalid response.\nTry again.",QUERY);}
    }
    char query[97];taskENTER_CRITICAL(&lock); int key=pending_key; pending_key=0;bool set_query=pending_query_set;unsigned probe=pending_probe;pending_probe=0;if(set_query){memcpy(query,pending_query,sizeof(query));memset(pending_query,0,sizeof(pending_query));pending_query_set=false;}taskEXIT_CRITICAL(&lock);
    if(probe&&u.mode==READER&&u.verified&&!u.probe){u.probe_document=u.view.document_id;u.probe_position=u.view.page_index;
        if(probe==3){PS_FlowView flow;int n=ps_flow_probe(reply.bytes,0);if(n>0&&ps_flow_decode(reply.bytes,(size_t)n,&flow)==0){memcpy(u.packet,reply.bytes,(size_t)n);u.flow_view=flow;u.flow=true;u.rich=false;u.probe=true;u.view=(PS_View){.document_id=flow.document_id,.page_count=flow.positions,.kind=2,.title=flow.title};u.dirty=true;}return;}
        int n=probe==2?ps_webfont_probe(reply.bytes):ps_rich_probe(reply.bytes);PS_RichView rich;if(n>0&&ps_rich_decode(reply.bytes,(size_t)n,&rich)==0){memcpy(u.packet,reply.bytes,(size_t)n);u.rich_view=rich;u.rich=true;u.flow=false;u.probe=true;u.view=(PS_View){.document_id=rich.document_id,.page_count=rich.positions,.kind=2,.title=rich.title};u.dirty=true;}return;}
    if(set_query){cancel();u.back_count=0;mode(HOME);memcpy(u.query,query,sizeof(u.query));MenuMgr_SelectTab(kTabID_Search);return;}if(!key)return;
    if(key==1) { cancel();u.back_count=0;mode(HOME); MenuMgr_SelectTab(kTabID_Search); }
    else (void)PocketUI_Button(buttons[key-1],kButtonState_Pressed,NULL);
}
