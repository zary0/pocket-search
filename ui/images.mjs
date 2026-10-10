// SPDX-License-Identifier: GPL-3.0-only
export function crc32(bytes){let crc=0xffffffff;for(const byte of bytes){crc^=byte;for(let i=0;i<8;i++)crc=(crc>>>1)^((crc&1)?0xedb88320:0);}return (crc^0xffffffff)>>>0;}
export function decodeImage(bytes,id,page){
  if(!(bytes instanceof Uint8Array)||bytes.length<26||bytes.length>20720)throw Error('Image size');
  const v=new DataView(bytes.buffer,bytes.byteOffset,bytes.byteLength);let at=0,y=0,width=0,height=0,rgba;
  while(at<bytes.length){
    if(at+24>bytes.length||String.fromCharCode(...bytes.subarray(at,at+4))!=='PCI1'||bytes[at+4]!==1||bytes[at+5]!==1)throw Error('Image header');
    const w=bytes[at+6],rows=bytes[at+7],h=v.getUint16(at+16,true),n=v.getUint16(at+18,true);
    if(!w||w>128||!h||h>80||v.getUint32(at+8,true)!==id||!id||v.getUint16(at+12,true)!==page||v.getUint16(at+14,true)!==y||y>=h||y%8||rows!==Math.min(8,h-y)||n!==w*rows*2||at+24+n>bytes.length||(height&&(w!==width||h!==height)))throw Error('Image stripe');
    if(!rgba){width=w;height=h;rgba=new Uint8ClampedArray(w*h*4);}
    const pixels=bytes.subarray(at+24,at+24+n);if(crc32(pixels)!==v.getUint32(at+20,true))throw Error('Image CRC');
    for(let i=0;i<n/2;i++){const color=pixels[i*2]|pixels[i*2+1]<<8,r=color>>>11,g=(color>>>5)&63,b=color&31,offset=(y*w+i)*4;rgba[offset]=(r<<3)|(r>>>2);rgba[offset+1]=(g<<2)|(g>>>4);rgba[offset+2]=(b<<3)|(b>>>2);rgba[offset+3]=255;}
    y+=rows;at+=24+n;
  }
  if(y!==height)throw Error('Incomplete image');return {width,height,rgba};
}
