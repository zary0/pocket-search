// SPDX-License-Identifier: GPL-3.0-only
import test from 'node:test';
import assert from 'node:assert/strict';
import {decodePacket} from '../ui/pcs1.mjs';
// Golden PCS1 response from the Rust service's original wifi fixture.
const hex='50435331010207140200000000000300a20000004368726f6d6174696320576946692067756964651868747470733a2f2f6578616d706c652e696e76616c69642f096368726f6d61746963174f726967696e616c206669787475726520666f722061201868616e6468656c64207265616465722e2053656172636820157465726d7320617265206d6174636865642062792011746865205275737420656e67696e652c2011413a20526561642066756c6c2070616765';
const golden=()=>Uint8Array.from(Buffer.from(hex,'hex'));
function bom(at, lengthOffset){
  const old=golden(),b=new Uint8Array(old.length+3);
  b.set(old.slice(0,at));b.set([0xef,0xbb,0xbf],at);b.set(old.slice(at),at+3);
  b[lengthOffset]+=3;b[16]+=3;return b;
}
test('BOM content is preserved and counts toward Rust-compatible width limits',()=>{
  assert.equal(decodePacket(bom(20,7).buffer).title,'\uFEFFChromatic WiFi guide');
  assert.equal(decodePacket(bom(66,65).buffer).lines[1],'\uFEFFchromatic');
  assert.throws(()=>decodePacket(bom(41,40).buffer)); // 24 ASCII cells plus a two-cell BOM.
});
test('browser decoder accepts a real Rust result packet',()=>{
  const p=decodePacket(golden().buffer);
  assert.deepEqual([p.kind,p.id,p.page,p.count,p.title],[1,2,0,3,'Chromatic WiFi guide']);
  assert.equal(p.lines.length,7);
  assert.equal(p.lines.at(-1),'A: Read full page');
});
test('decoder rejects every truncation and an oversized response',()=>{
  const b=golden();
  for(let n=0;n<b.length;n++)assert.throws(()=>decodePacket(b.slice(0,n).buffer));
  assert.throws(()=>decodePacket(new Uint8Array(513).buffer));
});
test('decoder rejects invalid header, pagination, text and trailing bytes',()=>{
  for(const [at,value] of [[0,0],[4,3],[5,0],[6,8],[7,49],[12,3],[14,0],[16,0],[18,1],[20,0],[20,128],[20,255]]){
    const b=golden();b[at]=value;assert.throws(()=>decodePacket(b.buffer));
  }
  const trailing=new Uint8Array(golden().length+1);trailing.set(golden());trailing[16]++;
  assert.throws(()=>decodePacket(trailing.buffer));
  const invalidReader=golden();invalidReader[4]=2;invalidReader[8]=0;
  assert.throws(()=>decodePacket(invalidReader.buffer));
});
