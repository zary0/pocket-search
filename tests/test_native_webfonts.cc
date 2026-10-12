// SPDX-License-Identifier: GPL-3.0-only
#include "pocket_woff2.h"
#include <cassert>
#include <cstdio>
#include <cstring>
#include <fstream>
#include <iterator>
#include <vector>
#include <string>
using Bytes=std::vector<uint8_t>;
Bytes read(const char *name){std::ifstream f(name,std::ios::binary);assert(f.good());return Bytes(std::istreambuf_iterator<char>(f),{});}
uint16_t u16(const uint8_t *p){return uint16_t(p[0])<<8|p[1];}
uint32_t u32(const uint8_t *p){return uint32_t(p[0])<<24|uint32_t(p[1])<<16|uint32_t(p[2])<<8|p[3];}
Bytes table(const uint8_t *p,size_t n,const char *tag){unsigned tables=u16(p+4);for(unsigned i=0;i<tables;++i){auto e=p+12+i*16;if(!memcmp(e,tag,4)){size_t at=u32(e+8),length=u32(e+12);assert(at<=n&&length<=n-at);return Bytes(p+at,p+at+length);}}assert(false);return {};}
struct Checks{unsigned count=0,stop=0,allocations=0,deny=0;uint64_t time=0;};
bool cancel(void *p){auto c=static_cast<Checks*>(p);++c->count;return c->stop&&c->count>=c->stop;}
uint64_t clock_ms(void *p){return static_cast<Checks*>(p)->time;}
size_t heap(void *p){auto c=static_cast<Checks*>(p);return c->deny&&++c->allocations==c->deny?0:1000000;}
int decode(const Bytes &b,Checks &c,unsigned *allocations=nullptr){PS_FontBudget budget{cancel,clock_ms,heap,&c,0,0};uint8_t *out=nullptr;size_t n=0;int e=PS_Woff2Decode(b.data(),b.size(),&out,&n,&budget);assert(budget.peak_bytes<=PS_FONT_PRIVATE);if(allocations)*allocations=budget.allocations;if(e){assert(!out&&!n);}else{assert(n>0&&n<=PS_FONT_SFNT);}PS_Woff2Free(out);return e;}
int main(int argc,char **argv){assert(argc==4);auto b=read(argv[1]),reference=read(argv[2]),large=read(argv[3]);Checks c;PS_FontBudget budget{cancel,clock_ms,heap,&c,0,0};uint8_t *out=nullptr;size_t n=0;assert(PS_Woff2Decode(b.data(),b.size(),&out,&n,&budget)==0);
for(const char *tag:{"cmap","hhea","hmtx","maxp","OS/2"})assert(table(out,n,tag)==table(reference.data(),reference.size(),tag));
printf("WOFF2 reference tables pass; output=%zu private_peak=%u allocations=%u\n",n,budget.peak_bytes,budget.allocations);unsigned count=budget.allocations,checkpoints=c.count;PS_Woff2Free(out);
for(unsigned i=1;i<=count;++i){c=Checks{};c.deny=i;assert(decode(b,c)==PS_FONT_MEMORY);}
for(unsigned i:{1u,5u,20u,50u,checkpoints/2,checkpoints-1}){c=Checks{};c.stop=i;assert(decode(b,c)==PS_FONT_CANCELLED);}
c=Checks{};assert(decode(b,c)==0);c=Checks{};assert(decode(large,c)==PS_FONT_MEMORY);
for(size_t i=0;i<b.size();++i){Bytes truncated(b.begin(),b.begin()+i);c=Checks{};assert(decode(truncated,c)!=0);}
for(size_t at=0;at<b.size();at+=17){Bytes changed=b;changed[at]^=0xff;c=Checks{};decode(changed,c);}
for(auto pair:{std::pair<size_t,uint8_t>(14,1),{4,79},{28,1},{16,1}}){auto changed=b;changed[pair.first]=pair.second;c=Checks{};assert(decode(changed,c)!=0);}
puts("WOFF2 allocation failure, cancellation/retry, truncation, mutation and bounds pass");}
