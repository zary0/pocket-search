// SPDX-License-Identifier: GPL-3.0-only
#pragma once
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#define PS_NATIVE_IMAGE_WIRE 24576u
#define PS_NATIVE_IMAGE_WIDTH 160u
#define PS_NATIVE_IMAGE_HEIGHT 80u
#define PS_PNG_MAX_PIXELS 8192u
#define PS_PNG_MAX_FILTERED_BYTES 32768u
#define PS_PROGRESSIVE_MAX_PIXELS 9216u
#define PS_EXTENDED_IMAGE_ALLOCATIONS 65536u
enum {PS_FORMAT_BASELINE=1,PS_FORMAT_PROGRESSIVE,PS_FORMAT_PNG};
typedef struct {uint8_t *pixels;uint16_t width,height;uint8_t format;unsigned decoder_peak_bytes,decoder_allocations;} PS_NativeImage;
typedef bool (*PS_ImageCancelled)(void*);
typedef void (*PS_ImageMemory)(void*);
enum {PS_IMAGE_OK,PS_IMAGE_UNSUPPORTED,PS_IMAGE_MALFORMED,PS_IMAGE_LIMIT,PS_IMAGE_MEMORY,PS_IMAGE_CANCELLED};
int PS_NativeJPEG(const uint8_t *bytes,size_t length,PS_NativeImage *out,PS_ImageCancelled cancelled,PS_ImageMemory observe,void *context);
int PS_ExtendedImage(const uint8_t *bytes,size_t length,PS_NativeImage *out,PS_ImageCancelled cancelled,PS_ImageMemory observe,void *context);
int PS_NativeDecode(const uint8_t *bytes,size_t length,PS_NativeImage *out,PS_ImageCancelled cancelled,PS_ImageMemory observe,void *context);
void PS_NativeImageFree(PS_NativeImage *image);
