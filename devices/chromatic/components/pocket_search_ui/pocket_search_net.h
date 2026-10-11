// SPDX-License-Identifier: GPL-3.0-only
#pragma once
#include <stdbool.h>
#include <stdint.h>
#include <stddef.h>
#include "pocket_search_collection.h"
#define PS_IMAGE_PACKET_BYTES 2072
#define PS_IMAGE_CACHE_BYTES 20480
#define PS_IMAGE_WIRE_BYTES 20720
enum { PS_NET_STALE_COLLECTION = 11 };

typedef struct {uint16_t offset,length;} PS_Text;
typedef struct {uint32_t document_id;uint16_t page_index,page_count;uint8_t kind,line_count;uint16_t reserved;PS_Text title,lines[7];uint32_t link_target;} PS_View;
_Static_assert(sizeof(PS_View)==48,"Rust/C text ABI mismatch");
typedef struct {uint32_t document_id;uint16_t page_index,width,height,y,rows,pixels_offset,pixels_length,reserved;} PS_ImageView;
_Static_assert(sizeof(PS_ImageView)==20,"Rust/C image ABI mismatch");
int32_t ps_packet_decode(const uint8_t *bytes,size_t length,PS_View *out);
typedef struct {PS_Text text,styles;uint8_t y,height,glyphs,reserved;} PS_RichRow;
typedef struct {uint32_t document_id;uint16_t position,positions;PS_Text title;uint8_t row_count,reserved[3];PS_RichRow rows[8];} PS_RichView;
_Static_assert(sizeof(PS_RichView)==112,"Rust/C rich ABI mismatch");
int32_t ps_rich_decode(const uint8_t*,size_t,PS_RichView*);
int32_t ps_rich_probe(uint8_t*);
int32_t ps_webfont_probe(uint8_t*);
int32_t ps_webfont(const uint8_t*,size_t,uint8_t*);
int32_t ps_image_decode(const uint8_t *bytes,size_t length,PS_ImageView *out);
typedef struct {unsigned sequence;uint8_t kind;uint32_t document;uint16_t page;char query[97],generation[PS_GENERATION_BYTES];} PS_Request;
typedef struct {unsigned sequence;int error,http_status;unsigned elapsed_ms,wire_bytes,heap_before,minimum_free_heap_bytes,largest_free_block_bytes,stack_hwm,compute_us;bool tls_verified,clock_synced,finished,sample,local;char generation[PS_GENERATION_BYTES];size_t length;uint8_t bytes[PS_IMAGE_PACKET_BYTES];} PS_Reply;
bool PS_NetStart(const PS_Request *request);
bool PS_NetPoll(PS_Reply *reply);
bool PS_NetConfigured(void);
void PS_NetCancel(unsigned sequence);
void PS_NetRegister(void);
bool PS_NetRefresh(void);
