// SPDX-License-Identifier: GPL-3.0-only
import {groups,bytes,key,tail,QUERY_BYTES} from './input.mjs';
import {decodePacket} from './pcs1.mjs';
const $=id=>document.getElementById(id),ctx=$('screen').getContext('2d');
const state={query:'',mode:'home',script:0,group:0,letter:0,choosing:false,results:[],selected:0,packet:null,seq:0,controller:null,busy:false,error:''};
function box(x,y,w,h,r,color,stroke){ctx.beginPath();ctx.roundRect(x,y,w,h,r);ctx.fillStyle=color;ctx.fill();if(stroke){ctx.strokeStyle=stroke;ctx.lineWidth=1;ctx.stroke();}}
function text(value,x,y,color='#202124',size=12){ctx.font=`${size}px Arial, "Hiragino Sans", sans-serif`;ctx.fillStyle=color;ctx.fillText(value,x,y+size);}
function cancel(){state.seq++;state.controller?.abort();state.controller=null;state.busy=false;}
function home(){cancel();state.mode='home';state.packet=null;state.results=[];state.error='';$('notice').textContent='オリジナルのサンプル記事を検索できます';$('notice').classList.remove('error');render();}
function render(){
  ctx.fillStyle='#fff';ctx.fillRect(0,0,160,144);
  if(state.mode==='home'){
    text('Samples',112,4,'#70757a',9);text('Pocket Search',14,32,'#1a73e8',20);
    box(8,72,144,28,13,'#fff','#dadce0');text(tail(state.query)||'検索語を入力',18,79,'#5f6368',11);
    box(48,105,64,18,3,'#f1f3f4');text('Search',61,108,'#202124',11);text('A:Type Start:Search',8,129,'#70757a',10);
  }else if(state.mode==='keyboard'){
    text('Search',8,4);box(8,25,144,28,13,'#fff','#dadce0');text(tail(state.query)+'_',18,32,'#202124',11);
    const list=groups(state.script),values=state.choosing?[...list[state.group]]:list;
    values.forEach((value,i)=>{const x=8+i%3*49,y=59+Math.floor(i/3)*17,selected=i===(state.choosing?state.letter:state.group);if(selected)box(x-1,y,46,16,3,'#1a73e8');text(!state.choosing&&state.script===0&&i<10?value[0]+'行':value,x+1,y+1,selected?'#fff':'#202124',10);});
    text('Select:ABC Start:OK',8,129,'#70757a',10);
  }else if(state.busy){text('Pocket Search',8,4);text('Loading...',8,38);text('B:Cancel',8,129,'#70757a',10);}
  else if(state.packet&&['results','reader'].includes(state.mode)){
    const p=state.packet;text(`Samples ${p.page+1}/${p.count}`,8,3,'#70757a',10);text(p.title,8,18,'#1a73e8',11);
    p.lines.forEach((line,i)=>text(line,8,35+i*13,'#202124',11));text(state.mode==='reader'?'Left/Right B:Results':'Up/Down A:Read B:Back',8,129,'#70757a',9);
  }else{text('Pocket Search',8,4);text(state.error?'Connection error':'Enter a query',8,38);text('B:Search home',8,129,'#70757a',10);}
  $('screen').setAttribute('aria-label',`本体プレビュー。${state.mode}。${state.packet?.title||tail(state.query)}。${state.packet?.lines.join(' ')||''}`);
  $('clear').hidden=!$('query').value;document.body.classList.toggle('has-results',['results','reader','loading'].includes(state.mode));
  $('results').hidden=state.mode!=='results';$('reader').hidden=state.mode!=='reader';$('search-form').setAttribute('aria-busy',String(state.busy));
}
async function fetchBounded(url,limit,signal){
  const response=await fetch(url,{signal,cache:'no-store'});if(!response.ok)throw Error(`HTTP ${response.status}`);
  const length=Number(response.headers.get('Content-Length'));if(length>limit)throw Error('Response limit');
  if(!response.body)throw Error('Missing response');const reader=response.body.getReader(),parts=[];let total=0;
  try{while(true){const {done,value}=await reader.read();if(done)break;total+=value.length;if(total>limit)throw Error('Response limit');parts.push(value);}}finally{await reader.cancel();}
  const out=new Uint8Array(total);let at=0;for(const part of parts){out.set(part,at);at+=part.length;}return out;
}
async function request(work){cancel();const seq=state.seq,controller=new AbortController();state.controller=controller;state.busy=true;state.error='';render();const timer=setTimeout(()=>controller.abort(),12000);
  try{await work(controller.signal,seq);}catch(error){if(seq!==state.seq)return;state.error='検索サービスに接続できませんでした。再試行してください。';$('notice').textContent=state.error;$('notice').classList.add('error');if(!state.results.length)state.mode='notice';}
  finally{clearTimeout(timer);if(seq===state.seq){state.busy=false;state.controller=null;render();}}
}
async function getPacket(url,signal,expected){const p=decodePacket((await fetchBounded(url,512,signal)).buffer);if(p.kind!==expected.kind||p.page!==expected.page||(expected.id!==undefined&&p.id!==expected.id))throw Error('Response mismatch');return p;}
function resultNodes(){const list=$('result-list');list.replaceChildren();state.results.forEach((hit,i)=>{const item=document.createElement('li'),source=document.createElement('p'),button=document.createElement('button'),snippet=document.createElement('p');source.className='result-source';source.textContent=hit.url;button.className='result-title';button.textContent=hit.title;button.onclick=()=>open(i);snippet.className='result-snippet';snippet.textContent=hit.snippet;item.append(source,button,snippet);list.append(item);});}
async function search(){const q=$('query').value.trim();if(!q||bytes(q)>QUERY_BYTES){cancel();state.mode='notice';state.error='';state.results=[];$('notice').textContent=!q?'検索語を入力してください。':'検索語はUTF-8で96バイト以内にしてください。';$('notice').classList.add('error');render();return;}
  state.query=q;state.selected=0;state.results=[];state.packet=null;state.mode='loading';
  await request(async(signal,seq)=>{const raw=await fetchBounded(`/v1/search?q=${encodeURIComponent(q)}`,16384,signal),data=JSON.parse(new TextDecoder('utf-8',{fatal:true}).decode(raw));
    if(data.sample!==true||!Array.isArray(data.results)||data.results.length>10||data.results.some(h=>!Number.isInteger(h.document_id)||h.document_id<1||h.document_id>256||typeof h.title!=='string'||typeof h.url!=='string'||typeof h.snippet!=='string'))throw Error('Invalid results');
    const p=await getPacket(`/v1/chromatic/search?q=${encodeURIComponent(q)}&result=0`,signal,{kind:1,page:0,id:data.results[0]?.document_id??0});
    if(p.count!==Math.max(1,data.results.length))throw Error('Result count mismatch');
    if(seq!==state.seq)return;state.results=data.results;state.packet=p;state.mode='results';resultNodes();$('result-status').textContent=`サンプル記事 · ${data.results.length}件`;$('empty').hidden=data.results.length!==0;$('notice').textContent='収録したサンプル記事の検索結果';$('notice').classList.remove('error');
  });
}
async function selectResult(i){if(i<0||i>=state.results.length)return;await request(async(signal,seq)=>{const p=await getPacket(`/v1/chromatic/search?q=${encodeURIComponent(state.query)}&result=${i}`,signal,{kind:1,page:i,id:state.results[i].document_id});if(seq!==state.seq)return;state.selected=i;state.packet=p;state.mode='results';});}
async function open(i,page=0){const hit=state.results[i];if(!hit)return;await request(async(signal,seq)=>{const p=await getPacket(`/v1/chromatic/page?id=${hit.document_id}&page=${page}`,signal,{kind:2,page,id:hit.document_id});if(seq!==state.seq)return;state.selected=i;state.packet=p;state.mode='reader';$('reader-title').textContent=hit.title;$('reader-source').textContent=hit.url;$('reader-text').textContent=p.lines.join('\n');$('reader-page').textContent=`${p.page+1} / ${p.count}`;$('reader-prev').disabled=p.page===0;$('reader-next').disabled=p.page+1===p.count;});}
function press(button){if(state.busy){if(['b','select','start'].includes(button)){home();if(button==='start'){state.mode='keyboard';render();}}return;}
  if(['results','reader'].includes(state.mode)){const p=state.packet;if(button==='select'){home();return;}if(button==='start'){cancel();state.mode='keyboard';state.script=0;state.group=0;state.choosing=false;render();return;}
    if(button==='b'){if(state.mode==='reader')selectResult(state.selected);else home();return;}
    if(state.mode==='results'){if(button==='up')selectResult(state.selected-1);if(button==='down')selectResult(state.selected+1);if(button==='a')open(state.selected);}else{if(button==='left'&&p.page>0)open(state.selected,p.page-1);if(button==='right'&&p.page+1<p.count)open(state.selected,p.page+1);}return;
  }
  const action=key(state,button);$('query').value=state.query;if(action==='search')search();else if(action==='wifi'){$('notice').textContent='Wi-Fi設定は本体のSelectから開けます。';$('notice').classList.remove('error');}render();
}
$('query').addEventListener('input',()=>{cancel();state.query=$('query').value;state.mode='home';state.packet=null;state.results=[];$('notice').classList.remove('error');$('notice').textContent=bytes(state.query)>QUERY_BYTES?'検索語はUTF-8で96バイト以内にしてください。':'オリジナルのサンプル記事を検索できます';render();});
$('search-form').addEventListener('submit',e=>{e.preventDefault();search();});$('clear').onclick=()=>{$('query').value='';state.query='';home();$('query').focus();};
$('preview-open').onclick=()=>{$('preview').showModal();render();};$('preview-close').onclick=()=>{$('preview').close();};$('reader-back').onclick=()=>selectResult(state.selected);$('reader-prev').onclick=()=>press('left');$('reader-next').onclick=()=>press('right');
document.querySelectorAll('[data-key]').forEach(b=>b.onclick=()=>press(b.dataset.key));$('screen').onkeydown=e=>{const map={ArrowUp:'up',ArrowDown:'down',ArrowLeft:'left',ArrowRight:'right',z:'a',x:'b',Enter:'start',Shift:'select'};if(map[e.key]){e.preventDefault();press(map[e.key]);}};render();
