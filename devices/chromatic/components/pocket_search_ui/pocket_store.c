// SPDX-License-Identifier: GPL-3.0-only
// Two commit-last banks in a verified, owned, previously erased flash region.
#include "pocket_store.h"
#include "pocket_store_format.h"
#include "esp_partition.h"
#include "esp_flash.h"
#include <stdlib.h>
#include <stdatomic.h>
static const esp_partition_t *partition;
static _Atomic bool ready;
static bool owned;
static unsigned active=2;
static _Atomic uint32_t generation;
int32_t ps_local_snapshot(uint8_t*,size_t,uint8_t*,size_t);
int32_t ps_local_restore_snapshot(uint8_t*,size_t,const uint8_t*,size_t);
int32_t ps_local_set_profile(uint8_t*,size_t,uint32_t);
static size_t bank(unsigned value){return 4096u+value*PS_STORE_BANK;}
static bool read_header(unsigned value,uint8_t h[32]){return esp_partition_read(partition,bank(value),h,32)==ESP_OK&&PS_StoreValid(h);}
void PS_StoreInit(void){
    uint8_t bytes[512];
    if(esp_partition_register_external(esp_flash_default_chip,PS_STORE_OFFSET,PS_STORE_REGION,"pocket_corpus",ESP_PARTITION_TYPE_DATA,0x40,&partition)!=ESP_OK)return;
    if(esp_partition_read(partition,0,bytes,32)!=ESP_OK)return;
    if(PS_StoreOwned(bytes)){owned=ready=true;return;}
    // Never erase or reinterpret another application's data or a torn owner record.
    for(size_t at=0;at<PS_STORE_REGION;at+=sizeof(bytes)){
        if(esp_partition_read(partition,at,bytes,sizeof(bytes))!=ESP_OK)return;
        for(unsigned i=0;i<sizeof(bytes);++i)if(bytes[i]!=0xff)return;
    }
    ready=true;
}
bool PS_StoreReady(void){return ready;}
unsigned PS_StoreGeneration(void){return generation;}
bool PS_StoreLoad(uint8_t *state,size_t size,unsigned profile){
    if(!ready||!owned)return false;
    uint8_t headers[2][32];bool valid[2]={read_header(0,headers[0]),read_header(1,headers[1])};
    unsigned first=valid[1]&&(!valid[0]||PS_StoreNewer(PS_StoreGet(headers[1]+8),PS_StoreGet(headers[0]+8)))?1:0;
    uint8_t *bytes=malloc(PS_STORE_BYTES);if(!bytes)return false;
    bool loaded=false;
    for(unsigned attempt=0;attempt<2;++attempt){unsigned value=first^attempt;if(!valid[value])continue;
        size_t length=PS_StoreGet(headers[value]+12);
        if(esp_partition_read(partition,bank(value)+32,bytes,length)!=ESP_OK||PS_StoreCRC(bytes,length)!=PS_StoreGet(headers[value]+16)||length<9||bytes[5]!=profile)continue;
        if(ps_local_restore_snapshot(state,size,bytes,length)==0){active=value;generation=PS_StoreGet(headers[value]+8);loaded=true;break;}
        (void)ps_local_set_profile(state,size,profile);
    }
    free(bytes);return loaded;
}
bool PS_StoreSave(uint8_t *state,size_t size){
    if(!ready)return false;
    uint8_t *bytes=malloc(PS_STORE_BYTES);if(!bytes)return false;
    int length=ps_local_snapshot(state,size,bytes,PS_STORE_BYTES);if(length<=0){free(bytes);return false;}
    if(!owned){uint8_t owner[32];PS_StoreOwner(owner);
        if(esp_partition_write(partition,0,owner,sizeof(owner))!=ESP_OK){ready=false;free(bytes);return false;}
        uint8_t check[32];owned=esp_partition_read(partition,0,check,sizeof(check))==ESP_OK&&PS_StoreOwned(check);
        if(!owned){ready=false;free(bytes);return false;}
    }
    unsigned value=active==0?1:0;size_t offset=bank(value);uint32_t crc=PS_StoreCRC(bytes,(size_t)length),next=generation+1;uint8_t header[32];PS_StoreHeader(header,next,(uint32_t)length,crc);
    bool success=esp_partition_erase_range(partition,offset,PS_STORE_BANK)==ESP_OK&&esp_partition_write(partition,offset+32,bytes,(size_t)length)==ESP_OK;
    if(success)success=esp_partition_read(partition,offset+32,bytes,(size_t)length)==ESP_OK&&PS_StoreCRC(bytes,(size_t)length)==crc;
    // Publish only after verified payload. The previous bank remains untouched.
    if(success)success=esp_partition_write(partition,offset,header,sizeof(header))==ESP_OK;
    if(success){uint8_t check[32];success=read_header(value,check)&&!memcmp(header,check,32);}
    if(success){active=value;generation=next;}
    free(bytes);return success;
}
