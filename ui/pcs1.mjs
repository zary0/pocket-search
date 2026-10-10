// SPDX-License-Identifier: GPL-3.0-only
export function decodePacket(buffer){
  const b=new Uint8Array(buffer),v=new DataView(b.buffer,b.byteOffset,b.byteLength),decoder=new TextDecoder('utf-8',{fatal:true,ignoreBOM:true});
  if(b.length<20||b.length>512||String.fromCharCode(...b.slice(0,4))!=='PCS1'||b[18]||b[19]||![1,2].includes(b[4])||b[6]>7)throw Error('Invalid packet');
  const kind=b[4],id=v.getUint32(8,true),page=v.getUint16(12,true),count=v.getUint16(14,true);let at=20;
  if(!count||page>=count||(kind===2&&!id)||b[5]!==((page>0?1:0)|(page+1<count?2:0))||v.getUint16(16,true)+20!==b.length)throw Error('Invalid pagination');
  function text(n){if(n>48||at+n>b.length)throw Error('Invalid text');const s=decoder.decode(b.slice(at,at+n));at+=n;let cells=0;for(const c of s){const p=c.codePointAt(0);if(p<32||(p>=127&&p<=159))throw Error('Control text');cells+=p<128?1:2;}if(cells>24)throw Error('Invalid width');return s;}
  const title=text(b[7]),lines=[];for(let i=0;i<b[6];i++){if(at===b.length)throw Error('Truncated line');const n=b[at++];lines.push(text(n));}if(at!==b.length)throw Error('Trailing data');return {kind,id,page,count,title,lines};
}
