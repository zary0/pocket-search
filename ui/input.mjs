// SPDX-License-Identifier: GPL-3.0-only
export const QUERY_BYTES = 96;
const KANA = ['あいうえお','かきくけこ','さしすせそ','たちつてと','なにぬねの','はひふへほ','まみむめも','やゆよ','らりるれろ','わをんー','゛゜小','空白'];
const LOWER = ['abc','def','ghi','jkl','mno','pqrs','tuv','wxyz','01234','56789','.,!?-_/','空白'];
const SYMBOLS = ["0123", "4567", "89", "@#$", "%&*", "()+", "-=:", ";.,", "!?/", "[]{}<>", "`\\|~^'\"", "空白"];
export function groups(script) { return script===0?KANA:script===1?LOWER:script===2?LOWER.map(s=>s.toUpperCase()):SYMBOLS; }
export function bytes(text) { return new TextEncoder().encode(text).length; }
export function append(query, text) { return bytes(query+text)<=QUERY_BYTES?query+text:query; }
export function modifyKana(text) {
  const cycles=['あぁ','いぃ','うぅゔ','えぇ','おぉ','かが','きぎ','くぐ','けげ','こご','さざ','しじ','すず','せぜ','そぞ','ただ','ちぢ','つっづ','てで','とど','はばぱ','ひびぴ','ふぶぷ','へべぺ','ほぼぽ','やゃ','ゆゅ','よょ','わゎ'];
  const chars=[...text],last=chars.pop(); if(!last)return text;
  const cycle=cycles.find(s=>s.includes(last));chars.push(cycle?cycle[(cycle.indexOf(last)+1)%cycle.length]:last);return chars.join('');
}
export function key(state, button) {
  if(button==='start')return 'search';
  if(state.mode==='notice'){if(button==='a'){state.mode='keyboard';state.script=0;state.group=0;state.letter=0;state.choosing=false;}if(button==='b'||button==='select')state.mode='home';return;}
  if(state.mode==='home'){if(button==='a'){state.mode='keyboard';state.script=0;state.group=0;state.letter=0;state.choosing=false;}if(button==='select')return 'wifi';return;}
  const list=groups(state.script),letters=[...list[state.group]],delta=['down','right'].includes(button)?1:['up','left'].includes(button)?-1:0;
  if(button==='select'){state.script=(state.script+1)%4;state.choosing=false;}
  else if(button==='b'){if(state.choosing)state.choosing=false;else if(state.query)state.query=[...state.query].slice(0,-1).join('');else state.mode='home';}
  else if(state.choosing){if(delta)state.letter=(state.letter+delta+letters.length)%letters.length;if(button==='a'){state.query=append(state.query,letters[state.letter]);state.choosing=false;}}
  else{const step=button==='down'?3:button==='up'?-3:delta;if(step)state.group=(state.group+step+12)%12;if(button==='a'){if(state.group===11)state.query=append(state.query,' ');else if(state.script===0&&state.group===10)state.query=modifyKana(state.query);else{state.choosing=true;state.letter=0;}}}
}
