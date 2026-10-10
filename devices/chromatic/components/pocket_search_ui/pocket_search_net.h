// SPDX-License-Identifier: GPL-3.0-only
#pragma once
#include <stdbool.h>
#include <stdint.h>
#include <stddef.h>
typedef struct {uint16_t offset,length;} PS_Text;
typedef struct {uint32_t document_id;uint16_t page_index,page_count;uint8_t kind,line_count;uint16_t reserved;PS_Text title,lines[7];} PS_View;
_Static_assert(sizeof(PS_View)==44,"Rust/C packet ABI mismatch");
int32_t ps_packet_decode(const uint8_t *bytes,size_t length,PS_View *out);
typedef struct {unsigned sequence;uint8_t kind;uint32_t document;uint16_t page;char query[97];} PS_Request;
typedef struct {unsigned sequence;int error,http_status;unsigned elapsed_ms;bool tls_verified,clock_synced;size_t length;uint8_t bytes[512];} PS_Reply;
bool PS_NetStart(const PS_Request *request);
bool PS_NetPoll(PS_Reply *reply);
bool PS_NetConfigured(void);
void PS_NetRegister(void);
