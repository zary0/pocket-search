// SPDX-License-Identifier: GPL-3.0-only
#pragma once
#include <vector>
#include <map>
#include <memory>
#include <new>
#include <limits>
#include <brotli/decode.h>
void *PSFontAlloc(size_t);
void PSFontFree(void *);
void PSFontCheckpoint();
BrotliDecoderResult PSBrotliDecode(size_t,const uint8_t *,size_t *,uint8_t *);
template<typename T>struct PSAllocator {
    using value_type=T;
    PSAllocator() noexcept=default;
    template<typename U>PSAllocator(const PSAllocator<U>&) noexcept {}
    T *allocate(size_t n){if(n>std::numeric_limits<size_t>::max()/sizeof(T))throw std::bad_alloc();return static_cast<T*>(PSFontAlloc(n*sizeof(T)));}
    void deallocate(T *p,size_t) noexcept{PSFontFree(p);}
    template<typename U>bool operator==(const PSAllocator<U>&)const noexcept{return true;}
    template<typename U>bool operator!=(const PSAllocator<U>&)const noexcept{return false;}
};
template<typename T>using PSVector=std::vector<T,PSAllocator<T>>;
template<typename K,typename V>using PSMap=std::map<K,V,std::less<K>,PSAllocator<std::pair<const K,V>>>;
template<typename T>struct PSDelete{void operator()(T *p)const noexcept{PSFontFree(p);}};
template<typename T>using PSArray=std::unique_ptr<T[],PSDelete<T>>;
template<typename T>T *PSNewArray(size_t n){return PSAllocator<T>().allocate(n);}
