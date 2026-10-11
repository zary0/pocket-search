// SPDX-License-Identifier: GPL-3.0-only
#pragma once
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#define PS_NATIVE_IMAGE_WIRE 24576u
#define PS_NATIVE_IMAGE_WIDTH 160u
#define PS_NATIVE_IMAGE_HEIGHT 80u
typedef struct {uint8_t *pixels;uint16_t width,height;} PS_NativeImage;
typedef bool (*PS_ImageCancelled)(void*);
typedef void (*PS_ImageMemory)(void*);
enum {PS_IMAGE_OK,PS_IMAGE_UNSUPPORTED,PS_IMAGE_MALFORMED,PS_IMAGE_LIMIT,PS_IMAGE_MEMORY,PS_IMAGE_CANCELLED};
int PS_NativeJPEG(const uint8_t *bytes,size_t length,PS_NativeImage *out,PS_ImageCancelled cancelled,PS_ImageMemory observe,void *context);
void PS_NativeImageFree(PS_NativeImage *image);
