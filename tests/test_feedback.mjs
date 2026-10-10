// SPDX-License-Identifier: GPL-3.0-only
import test from 'node:test';
import assert from 'node:assert/strict';
import {committedEdit,createInputClick} from '../ui/feedback.mjs';

test('IME drafts, navigation, rejected edits and paste stay quiet',()=>{
  assert.equal(committedEdit('','あ','insertCompositionText',true),false);
  assert.equal(committedEdit('あ','あ','insertText'),false);
  assert.equal(committedEdit('','pasted text','insertFromPaste'),false);
  assert.equal(committedEdit('','あ','insertCompositionText'),true);
  assert.equal(committedEdit('😀','','deleteContentBackward'),true);
});
test('blocked or missing audio never interrupts typing',()=>{
  assert.doesNotThrow(()=>createInputClick(null)());
  assert.doesNotThrow(()=>createInputClick(class {constructor(){throw Error('blocked');}})());
});
test('suspended audio cannot replay a queued click after resume',async()=>{
  let starts=0;
  class Audio {
    state='suspended';
    resume(){this.state='running';return Promise.resolve();}
    createOscillator(){starts++;throw Error('Unexpected replay');}
  }
  const click=createInputClick(Audio);click();await Promise.resolve();assert.equal(starts,0);
});
test('rapid input keeps playback bounded and releases completed audio nodes',()=>{
  let engine,starts=0,disconnects=0,ended;
  const parameter={setValueAtTime(){},linearRampToValueAtTime(){},exponentialRampToValueAtTime(){}};
  class Audio {
    constructor(){engine=this;}
    state='running';currentTime=0;destination={};
    createGain(){return {gain:parameter,connect(){},disconnect(){disconnects++;}};}
    createOscillator(){const node={frequency:parameter,connect(){},disconnect(){disconnects++;},start(){starts++;},stop(){ended=()=>node.onended();}};return node;}
  }
  const click=createInputClick(Audio);for(let i=0;i<1000;i++)click();assert.equal(starts,1);
  ended();assert.equal(disconnects,2);engine.currentTime=.031;click();assert.equal(starts,2);
  ended();assert.equal(disconnects,4);
});
