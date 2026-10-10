// SPDX-License-Identifier: GPL-3.0-only
export const QUERY_BYTES = 96;
const KANA = ['あいうえお','かきくけこ','さしすせそ','たちつてと','なにぬねの','はひふへほ','まみむめも','やゆよ','らりるれろ','わをんー','゛゜小','空白'];
const LOWER = ['abc','def','ghi','jkl','mno','pqrs','tuv','wxyz','01234','56789','.,!?-_/','空白'];
const SYMBOLS = ["0123", "4567", "89", "@#$", "%&*", "()+", "-=:", ";.,", "!?/", "[]{}<>", "`\\|~^'\"", "空白"];
export function groups(script) { return script===0?KANA:script===1?LOWER:script===2?LOWER.map(s=>s.toUpperCase()):SYMBOLS; }
export function bytes(text) { return new TextEncoder().encode(text).length; }
export function tail(value,limit=19) {
  let start=value.length,width=0;
  while(start){let i=start-1,unit=value.charCodeAt(i);if(unit>=0xdc00&&unit<=0xdfff&&i){const high=value.charCodeAt(i-1);if(high>=0xd800&&high<=0xdbff)i--;}
    const cells=value.codePointAt(i)<128?1:2;if(width+cells>limit)break;width+=cells;start=i;}
  return value.slice(start);
}
export function gridMove(index,count,step){
  if(step===3)return index+3<count?index+3:index%3;
  if(step===-3)return index>=3?index-3:index+Math.floor((count-1-index)/3)*3;
  return (index+step+count)%count;
}
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
  else if(state.choosing){const step=button==='down'?3:button==='up'?-3:delta;if(step)state.letter=gridMove(state.letter,letters.length,step);if(button==='a'){state.query=append(state.query,letters[state.letter]);state.choosing=false;}}
  else{const step=button==='down'?3:button==='up'?-3:delta;if(step)state.group=(state.group+step+12)%12;if(button==='a'){if(state.group===11)state.query=append(state.query,' ');else if(state.script===0&&state.group===10)state.query=modifyKana(state.query);else{state.choosing=true;state.letter=0;}}}
}
