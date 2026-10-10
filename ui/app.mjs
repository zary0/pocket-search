// SPDX-License-Identifier: GPL-3.0-only
import {groups,bytes,key,QUERY_BYTES} from './input.mjs';
const $=id=>document.getElementById(id),ctx=$('screen').getContext('2d');
const state={query:'',mode:'home',script:0,group:0,letter:0,choosing:false};
function box(x,y,w,h,r,color,stroke){ctx.beginPath();ctx.roundRect(x,y,w,h,r);ctx.fillStyle=color;ctx.fill();if(stroke){ctx.strokeStyle=stroke;ctx.lineWidth=1;ctx.stroke();}}
function text(value,x,y,color='#202124',size=12){ctx.font=`${size}px Arial, "Hiragino Sans", sans-serif`;ctx.fillStyle=color;ctx.fillText(value,x,y+size);}
function tail(value,limit=19){const chars=[...value];while(chars.reduce((n,c)=>n+(c.codePointAt(0)<128?1:2),0)>limit)chars.shift();return chars.join('');}
function render(){
  ctx.fillStyle='#fff';ctx.fillRect(0,0,160,144);
  if(state.mode==='home'){
    text('Preview',112,4,'#70757a',9);text('Pocket Search',14,32,'#1a73e8',20);
    box(8,72,144,28,13,'#fff','#dadce0');text(tail(state.query)||'検索語を入力',18,79,'#5f6368',11);
    box(48,105,64,18,3,'#f1f3f4');text('Search',61,108,'#202124',11);text('A:Type Start:Search',8,129,'#70757a',10);
  }else if(state.mode==='keyboard'){
    text('Search',8,4);box(8,25,144,28,13,'#fff','#dadce0');text(tail(state.query)+'_',18,32,'#202124',11);
    const list=groups(state.script),values=state.choosing?[...list[state.group]]:list;
    values.forEach((value,i)=>{const x=8+i%3*49,y=59+Math.floor(i/3)*17,selected=i===(state.choosing?state.letter:state.group);if(selected)box(x-1,y,46,16,3,'#1a73e8');text(!state.choosing&&state.script===0&&i<10?value[0]+'行':value,x+1,y+1,selected?'#fff':'#202124',10);});
    text('Select:ABC Start:OK',8,129,'#70757a',10);
  }else{ text('Pocket Search',8,4);text('Search service',8,32);text('not connected yet.',8,49);text('B: Search home',8,81,'#70757a');text('A:Type B:Back',8,129,'#70757a',10); }
  $('screen').setAttribute('aria-label',`本体レイアウト案。${{home:'検索ホーム',keyboard:'文字入力',notice:'サービス未接続'}[state.mode]}。${state.query}`);$('clear').hidden=!$('query').value;
}
function search(){state.query=$('query').value.trim();const notice=$('notice');notice.classList.add('error');if(!state.query)notice.textContent='検索語を入力してください。';else if(bytes(state.query)>QUERY_BYTES)notice.textContent='検索語はUTF-8で96バイト以内にしてください。';else{notice.textContent='検索サービスはまだ接続されていません。検索結果の表示は次の実装です。';state.mode='notice';}render();}
function press(button){const action=key(state,button);$('query').value=state.query;if(action==='search')search();else if(action==='wifi'){$('notice').textContent='Wi-Fi設定は本体のSelectから開けます。';$('notice').classList.remove('error');}render();}
$('query').addEventListener('input',()=>{state.query=$('query').value;state.mode='home';$('notice').classList.remove('error');$('notice').textContent=bytes(state.query)>QUERY_BYTES?'検索語はUTF-8で96バイト以内にしてください。':'検索サービス接続前のUI試作';render();});
$('search-form').addEventListener('submit',e=>{e.preventDefault();search();});$('clear').onclick=()=>{$('query').value='';state.query='';state.mode='home';$('notice').textContent='検索サービス接続前のUI試作';$('notice').classList.remove('error');$('query').focus();render();};
$('preview-open').onclick=()=>{$('preview').showModal();render();};$('preview-close').onclick=()=>{$('preview').close();};
document.querySelectorAll('[data-key]').forEach(b=>b.onclick=()=>press(b.dataset.key));
$('screen').onkeydown=e=>{const map={ArrowUp:'up',ArrowDown:'down',ArrowLeft:'left',ArrowRight:'right',z:'a',x:'b',Enter:'start',Shift:'select'};if(map[e.key]){e.preventDefault();press(map[e.key]);}};render();
