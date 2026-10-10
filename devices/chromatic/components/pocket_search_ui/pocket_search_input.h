// SPDX-License-Identifier: GPL-3.0-only
#pragma once
#include <stdbool.h>
#include <stddef.h>
#include <string.h>
#define PS_QUERY_BYTES 96u
#define PS_INPUT_CAPACITY (PS_QUERY_BYTES + 1u)

static inline size_t ps_length(const char *s, size_t cap) {
    size_t n = 0;
    while (n < cap && s[n]) ++n;
    return n;
}
static inline size_t ps_scalar(const unsigned char *s, size_t left) {
    if (!left) return 0;
    unsigned n = s[0] < 0x80 ? 1 : s[0] >= 0xc2 && s[0] <= 0xdf ? 2 :
                 s[0] >= 0xe0 && s[0] <= 0xef ? 3 : s[0] >= 0xf0 && s[0] <= 0xf4 ? 4 : 0;
    if (!n || n > left || s[0] < 0x20 || s[0] == 0x7f) return 0;
    for (unsigned i = 1; i < n; ++i) if ((s[i] & 0xc0) != 0x80) return 0;
    if ((s[0] == 0xe0 && s[1] < 0xa0) || (s[0] == 0xed && s[1] >= 0xa0) ||
        (s[0] == 0xf0 && s[1] < 0x90) || (s[0] == 0xf4 && s[1] >= 0x90)) return 0;
    return n;
}
static inline bool ps_valid(const char *s, size_t n) {
    for (size_t i = 0; i < n;) {
        size_t width = ps_scalar((const unsigned char *)s + i, n - i);
        if (!width) return false;
        i += width;
    }
    return true;
}
// Append atomically: failure leaves the old UTF-8 query intact.
static inline bool ps_append(char buffer[PS_INPUT_CAPACITY], size_t limit, const char *text) {
    size_t used = ps_length(buffer, PS_INPUT_CAPACITY), n = ps_length(text, PS_INPUT_CAPACITY);
    if (limit > PS_QUERY_BYTES || used > limit || n > limit - used ||
        !ps_valid(buffer, used) || !ps_valid(text, n)) return false;
    memcpy(buffer + used, text, n + 1);
    return true;
}
static inline void ps_erase(char buffer[PS_INPUT_CAPACITY]) {
    size_t n = ps_length(buffer, PS_INPUT_CAPACITY);
    if (!n || n == PS_INPUT_CAPACITY) return;
    --n;
    while (n && ((unsigned char)buffer[n] & 0xc0) == 0x80) --n;
    buffer[n] = 0;
}
