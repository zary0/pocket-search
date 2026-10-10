// SPDX-License-Identifier: GPL-3.0-only
import test from 'node:test';
import assert from 'node:assert/strict';
import {append,bytes,key,modifyKana,gridMove,tail} from '../ui/input.mjs';
test('UTF-8 query limit preserves the old value on overflow',()=>{const q='あ'.repeat(32);assert.equal(bytes(q),96);assert.equal(append(q,'a'),q);assert.equal(bytes(append('x'.repeat(92),'😀')),96);});
test('keyboard selects characters and deletes complete code points',()=>{const s={query:'',mode:'home',script:0,group:0,letter:0,choosing:false};key(s,'a');key(s,'a');key(s,'a');assert.equal(s.query,'あ');key(s,'select');key(s,'a');key(s,'a');assert.equal(s.query,'あa');key(s,'b');assert.equal(s.query,'あ');key(s,'b');assert.equal(s.query,'');key(s,'b');assert.equal(s.mode,'home');});
test('submit requests search without an account or approval action',()=>{const s={query:'rust',mode:'home'};assert.equal(key(s,'start'),'search');s.mode='notice';key(s,'b');assert.equal(s.mode,'home');assert.equal(s.query,'rust');});
test('kana modifiers preserve the byte limit',()=>{assert.equal(modifyKana('は'),'ば');assert.equal(modifyKana('ば'),'ぱ');assert.equal(modifyKana('ぱ'),'は');assert.equal(bytes(modifyKana('あ'.repeat(32))),96);});
test('vertical picker navigation keeps the column through partial rows',()=>{const s={query:'',mode:'home',script:0,group:0,letter:0,choosing:false};key(s,'a');key(s,'a');key(s,'down');key(s,'a');assert.equal(s.query,'え');assert.equal(gridMove(3,5,3),0);assert.equal(gridMove(0,5,-3),3);assert.equal(gridMove(2,5,3),2);assert.equal(gridMove(2,5,-3),2);});
test('visible suffix keeps complete Unicode scalars with bounded scanning',()=>{assert.equal(tail('x'.repeat(1000000)+'あ😀',5),'xあ😀');assert.equal(tail('😀',1),'');assert.equal(tail('あい',2),'い');assert.equal(tail('a😀',2),'😀');});
