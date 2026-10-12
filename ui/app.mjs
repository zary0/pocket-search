// SPDX-License-Identifier: GPL-3.0-only
import {groups,bytes,key,tail,QUERY_BYTES} from './input.mjs';
import {decodePacket} from './pcs1.mjs';
import {decodeImage} from './images.mjs';
import {committedEdit,createInputClick} from './feedback.mjs';
const inputClick=createInputClick();
let compositionBefore=null;
let collectionGeneration='';let sampleCollection=null;
const collectionNotice=()=>sampleCollection===true?'オリジナルのサンプル記事を検索できます':sampleCollection===false?'収録した公開ページを検索できます':'登録したページを検索できます';
const generationParameter=()=>collectionGeneration?`&generation=${encodeURIComponent(collectionGeneration)}`:'';
class CollectionChanged extends Error {}
const $=id=>document.getElementById(id),ctx=$('screen').getContext('2d');
const state={query:'',mode:'home',script:0,group:0,letter:0,choosing:false,results:[],selected:0,offset:0,total:0,packet:null,seq:0,controller:null,busy:false,error:'',document:0,back:[],image:null};
function box(x,y,w,h,r,color,stroke){ctx.beginPath();ctx.roundRect(x,y,w,h,r);ctx.fillStyle=color;ctx.fill();if(stroke){ctx.strokeStyle=stroke;ctx.lineWidth=1;ctx.stroke();}}
function text(value,x,y,color='#202124',size=12,weight='normal'){ctx.font=`${weight} ${size}px Arial, "Hiragino Sans", sans-serif`;ctx.fillStyle=color;ctx.fillText(value,x,y+size);}
function resultLines(value,limit,size=12,weight='normal'){
  ctx.font=`${weight} ${size}px Arial, "Hiragino Sans", sans-serif`;
  const lines=[];let rest=value.replace(/\s+/g,' ').trim();
  while(rest&&lines.length<limit){let line='',end=0,space=0;for(const c of rest){if(ctx.measureText(line+c).width>144)break;line+=c;end+=c.length;if(c===' ')space=end;}
    if(end<rest.length&&space>0){end=space;line=rest.slice(0,end);}lines.push(line.trim());rest=rest.slice(end).trim();}
  if(rest&&lines.length){let line=lines.at(-1);while(line&&ctx.measureText(line+'...').width>144)line=[...line].slice(0,-1).join('');lines[lines.length-1]=line+'...';}
  return lines;
}
function cancel(){state.seq++;state.controller?.abort();state.controller=null;state.busy=false;}
function home(){cancel();state.mode='home';state.packet=null;state.results=[];state.back=[];state.image=null;state.error='';$('notice').textContent=collectionNotice();$('notice').classList.remove('error');render();}
function render(){
  ctx.fillStyle='#fff';ctx.fillRect(0,0,160,144);
  if(state.mode==='home'){
    text(sampleCollection===true?'Samples':sampleCollection===false?'Web':'Pages',112,4,'#70757a',9);text('Pocket Search',14,32,'#1a73e8',20);
    box(8,72,144,28,13,'#fff','#dadce0');text(tail(state.query)||'検索語を入力',18,79,'#5f6368',11);
    box(48,105,64,18,3,'#f1f3f4');text('Search',61,108,'#202124',11);text('A:Type Start:Search',8,129,'#70757a',10);
  }else if(state.mode==='keyboard'){
    text('Search',8,4);box(8,25,144,28,13,'#fff','#dadce0');text(tail(state.query)+'_',18,32,'#202124',11);
    const list=groups(state.script),values=state.choosing?[...list[state.group]]:list;
    values.forEach((value,i)=>{const x=8+i%3*49,y=59+Math.floor(i/3)*17,selected=i===(state.choosing?state.letter:state.group);if(selected)box(x-1,y,46,16,3,'#1a73e8');text(!state.choosing&&state.script===0&&i<10?value[0]+'行':value,x+1,y+1,selected?'#fff':'#202124',10);});
    text('Select:ABC Start:OK',8,129,'#70757a',10);
  }else if(state.busy){text('Pocket Search',8,4);text('Loading...',8,38);text('B:Cancel',8,129,'#70757a',10);}
  else if(state.mode==='results'&&state.packet?.id&&state.results[state.selected]){
    const p=state.packet,hit=state.results[state.selected];box(6,3,108,24,13,'#fff','#dadce0');
    ctx.save();ctx.beginPath();ctx.rect(14,5,92,20);ctx.clip();text(tail(state.query),14,7,'#5f6368',11);ctx.restore();
    text(`${p.page+1}/${p.count}`,118,8,'#70757a',10);
    const title=resultLines(hit.title,2,12,'bold');title.forEach((line,i)=>text(line,8,34+i*14,'#1a0dab',12,'bold'));
    const description=resultLines(hit.snippet,title.length>1?3:4,11);description.forEach((line,i)=>text(line,8,(title.length>1?66:52)+i*13,'#3c4043',11));
    let source='';try{source=new URL(hit.url).hostname.replace(/^www\./,'');}catch{source=hit.url;}
    text(resultLines(source,1,10)[0]||'',8,112,'#70757a',10);text('Up/Down A:Read B:Back',8,130,'#70757a',9);
  }
  else if(state.packet&&['results','reader'].includes(state.mode)){
    const p=state.packet;text(`${p.page+1}/${p.count}`,8,3,'#70757a',10);text(p.title,8,18,'#1a73e8',11);
    if(state.mode==='reader'&&state.image){const image=state.image,off=document.createElement('canvas');off.width=image.width;off.height=image.height;off.getContext('2d').putImageData(new ImageData(image.rgba,image.width,image.height),0,0);ctx.drawImage(off,(160-image.width)/2,34+(80-image.height)/2);text(p.lines[0]||'',8,116,'#202124',10);}
    else p.lines.forEach((line,i)=>text(line,8,35+i*13,p.kind===4||p.kind===5?'#1a73e8':'#202124',11));
    text(state.mode==='reader'?(p.kind===4&&p.target?'A:Open Left/Right B':'Left/Right B:Back'):'Up/Down A:Read B:Back',8,129,'#70757a',9);
  }else{text('Pocket Search',8,4);text(state.error?'Connection error':'Enter a query',8,38);text('B:Search home',8,129,'#70757a',10);}
  $('screen').setAttribute('aria-label',`本体プレビュー。${state.mode}。${state.packet?.title||tail(state.query)}。${state.packet?.lines.join(' ')||''}`);
  $('clear').hidden=!$('query').value;document.body.classList.toggle('has-results',['results','reader','loading'].includes(state.mode));
  $('results').hidden=state.mode!=='results';$('reader').hidden=state.mode!=='reader';$('search-form').setAttribute('aria-busy',String(state.busy));
}
async function fetchBounded(url,limit,signal){
  const response=await fetch(url,{signal,cache:'no-store'});if(response.status===409)throw new CollectionChanged();if(!response.ok)throw Error(`HTTP ${response.status}`);
  const length=Number(response.headers.get('Content-Length'));if(length>limit)throw Error('Response limit');
  if(!response.body)throw Error('Missing response');const reader=response.body.getReader(),parts=[];let total=0;
  try{while(true){const {done,value}=await reader.read();if(done)break;total+=value.length;if(total>limit)throw Error('Response limit');parts.push(value);}}finally{await reader.cancel();}
  const out=new Uint8Array(total);let at=0;for(const part of parts){out.set(part,at);at+=part.length;}return out;
}
async function request(work){cancel();const seq=state.seq,controller=new AbortController();state.controller=controller;state.busy=true;state.error='';$('notice').textContent='';$('notice').classList.remove('error');render();const timer=setTimeout(()=>controller.abort(),12000);
  let completed=false;try{await work(controller.signal,seq);completed=true;}catch(error){if(seq!==state.seq)return;if(error instanceof CollectionChanged){state.packet=null;state.image=null;state.results=[];state.back=[];collectionGeneration='';}state.error=error instanceof CollectionChanged?'収録ページが更新されました。もう一度検索してください。':'検索サービスに接続できませんでした。再試行してください。';$('notice').textContent=state.error;$('notice').classList.add('error');if(!state.results.length)state.mode='notice';}
  finally{clearTimeout(timer);if(seq===state.seq){state.busy=false;state.controller=null;render();}}return completed&&seq===state.seq;
}
async function getPacket(url,signal,expected){const p=decodePacket((await fetchBounded(url,512,signal)).buffer);if((expected.kind===2?![2,3,4,5].includes(p.kind):p.kind!==expected.kind)||p.page!==expected.page||(expected.id!==undefined&&p.id!==expected.id))throw Error('Response mismatch');return p;}
function resultNodes(){const list=$('result-list');list.replaceChildren();state.results.forEach((hit,i)=>{const item=document.createElement('li'),source=document.createElement('p'),button=document.createElement('button'),snippet=document.createElement('p');source.className='result-source';source.textContent=hit.url;button.className='result-title';button.textContent=hit.title;button.onclick=()=>open(i);snippet.className='result-snippet';snippet.textContent=hit.snippet;item.append(source,button,snippet);list.append(item);});}
async function search(offset=0){const q=$('query').value.trim();if(!q||bytes(q)>QUERY_BYTES){cancel();state.mode='notice';state.error='';state.results=[];$('notice').textContent=!q?'検索語を入力してください。':'検索語はUTF-8で96バイト以内にしてください。';$('notice').classList.add('error');render();return;}
  state.query=q;state.selected=0;state.results=[];state.packet=null;state.back=[];state.image=null;state.mode='loading';
  return await request(async(signal,seq)=>{const raw=await fetchBounded(`/v1/search?q=${encodeURIComponent(q)}&offset=${offset}${offset?generationParameter():''}`,16384,signal),data=JSON.parse(new TextDecoder('utf-8',{fatal:true}).decode(raw));
    if(typeof data.sample!=='boolean'||typeof data.generation!=='string'||!/^[-a-zA-Z0-9]{1,64}$/.test(data.generation)||!Array.isArray(data.results)||data.results.length>10||!Number.isInteger(data.total)||data.total<0||data.total>256||data.offset!==offset||!Number.isInteger(data.indexed_pages)||data.indexed_pages<data.total||data.indexed_pages>256||data.results.length!==Math.min(10,Math.max(0,data.total-offset))||data.results.some(h=>!Number.isInteger(h.document_id)||h.document_id<1||h.document_id>256||typeof h.title!=='string'||typeof h.url!=='string'||typeof h.snippet!=='string'))throw Error('Invalid results');
    if(seq!==state.seq)return;collectionGeneration=data.generation;sampleCollection=data.sample;
    const p=await getPacket(`/v1/chromatic/search?q=${encodeURIComponent(q)}&result=${offset}${generationParameter()}`,signal,{kind:1,page:offset,id:data.results[0]?.document_id??0});
    if(p.count!==Math.max(1,data.total))throw Error('Result count mismatch');
    if(seq!==state.seq)return;state.results=data.results;state.offset=offset;state.total=data.total;state.packet=p;state.mode='results';resultNodes();$('result-status').textContent=`${data.sample?'サンプル記事':'収集した公開ページ'} ${data.indexed_pages}ページ中 · ${data.total}件`; $('results-page').textContent=data.total?`${offset+1}–${offset+data.results.length} / ${data.total}`:'0件';$('results-prev').disabled=offset===0;$('results-next').disabled=offset+data.results.length>=data.total;$('empty').hidden=data.results.length!==0;$('notice').textContent=data.sample?'収録したサンプル記事の検索結果':'収集した公開ページの検索結果';$('collection-label').textContent=data.sample?'独自検索 · サンプル記事':'独自検索 · 収集した公開ページ';$('notice').classList.remove('error');
  });
}
async function selectResult(i){if(i<0){if(state.offset&&await search(state.offset-10))await selectResult(state.results.length-1);return;}if(i>=state.results.length){if(state.offset+state.results.length<state.total)await search(state.offset+10);return;}await request(async(signal,seq)=>{const p=await getPacket(`/v1/chromatic/search?q=${encodeURIComponent(state.query)}&result=${state.offset+i}${generationParameter()}`,signal,{kind:1,page:state.offset+i,id:state.results[i].document_id});if(seq!==state.seq)return;state.selected=i;state.packet=p;state.mode='results';});}
async function open(i,page=0){const hit=state.results[i];if(!hit)return;state.selected=i;state.back=[];await showDocument(hit.document_id,page);}
function readingNodes(p,meta,image){
  const area=$('reader-text');area.replaceChildren();
  if(p.kind===3){const figure=document.createElement('figure'),caption=document.createElement('figcaption');caption.textContent=p.lines.join(' ');if(image){const canvas=document.createElement('canvas');canvas.width=image.width;canvas.height=image.height;canvas.setAttribute('role','img');canvas.setAttribute('aria-label',caption.textContent||'Image');canvas.getContext('2d').putImageData(new ImageData(image.rgba,image.width,image.height),0,0);figure.append(canvas);}else{const unavailable=document.createElement('p');unavailable.textContent='画像が登録されていません';figure.append(unavailable);}figure.append(caption);area.append(figure);return;}
  if(p.kind===4){const label=document.createElement('p');label.textContent=p.lines.join(' ');area.append(label);if(p.target){const button=document.createElement('button');button.textContent='リンク先の記事を読む';button.onclick=followLink;area.append(button);}else{const note=document.createElement('p');note.className='source-only';note.textContent='このリンク先は未収録です';area.append(note);}return;}
  const tag=meta.style==='heading'?`h${meta.heading_level}`:meta.style==='pre'?'pre':meta.style==='quote'?'blockquote':'p',element=document.createElement(tag);element.textContent=p.lines.join('\n');
  if(meta.style==='list'){const ul=document.createElement('ul'),li=document.createElement('li');li.textContent=p.lines.join(' ').replace(/^- /,'');ul.append(li);area.append(ul);}else area.append(element);
}
async function showDocument(id,page=0){return request(async(signal,seq)=>{
  const p=await getPacket(`/v1/chromatic/page?id=${id}&page=${page}${generationParameter()}`,signal,{kind:2,page,id});
  const raw=await fetchBounded(`/v1/dom?id=${id}&page=${page}${generationParameter()}`,8192,signal),meta=JSON.parse(new TextDecoder('utf-8',{fatal:true}).decode(raw));
  if(meta.generation!==collectionGeneration||meta.document_id!==id||meta.page_index!==page||meta.page_count!==p.count||meta.kind!==p.kind||typeof meta.title!=='string'||typeof meta.url!=='string'||!['paragraph','heading','pre','quote','list','caption','image','link','missing-image'].includes(meta.style)||meta.link_target!==p.target||(p.kind===5&&(!Number.isInteger(meta.heading_level)||meta.heading_level<1||meta.heading_level>6)))throw Error('DOM mismatch');
  let image=null;if(p.kind===3&&meta.image_width){image=decodeImage(await fetchBounded(`/v1/chromatic/image?id=${id}&page=${page}${generationParameter()}`,20720,signal),id,page);if(image.width!==meta.image_width||image.height!==meta.image_height)throw Error('Image mismatch');}
  if(seq!==state.seq)return;state.document=id;state.packet=p;state.image=image;state.mode='reader';$('reader-title').textContent=meta.title;$('reader-source').textContent=meta.url;readingNodes(p,meta,image);$('reader-back').textContent=state.back.length?'前の記事へ戻る':'検索結果へ戻る';$('reader-page').textContent=`${p.page+1} / ${p.count}`;$('reader-prev').disabled=p.page===0;$('reader-next').disabled=p.page+1===p.count;
});}
async function followLink(){const p=state.packet;if(state.busy||state.mode!=='reader'||!p?.target)return;if(state.back.length>=4){$('notice').textContent='リンクは4段階まで開けます。戻ってから再試行してください。';return;}const previous={id:state.document,page:p.page};if(await showDocument(p.target)){state.back.push(previous);$('reader-back').textContent='前の記事へ戻る';}}
async function back(){if(state.busy){home();return;}if(state.back.length){const previous=state.back.at(-1);if(await showDocument(previous.id,previous.page)){state.back.pop();$('reader-back').textContent=state.back.length?'前の記事へ戻る':'検索結果へ戻る';}}else selectResult(state.selected);}
function press(button){if(state.busy){if(['b','select','start'].includes(button)){home();if(button==='start'){state.mode='keyboard';render();}}return;}
  if(['results','reader'].includes(state.mode)){const p=state.packet;if(button==='select'){home();return;}if(button==='start'){cancel();state.mode='keyboard';state.script=0;state.group=0;state.choosing=false;render();return;}
    if(button==='b'){if(state.mode==='reader')back();else home();return;}
    if(state.mode==='results'){if(button==='up')selectResult(state.selected-1);if(button==='down')selectResult(state.selected+1);if(button==='a')open(state.selected);}else{if(button==='a')followLink();if(button==='left'&&p.page>0)showDocument(state.document,p.page-1);if(button==='right'&&p.page+1<p.count)showDocument(state.document,p.page+1);}return;
  }
  const previous=state.query;const action=key(state,button);if(previous!==state.query)inputClick();$('query').value=state.query;if(action==='search')search();else if(action==='wifi'){$('notice').textContent='Wi-Fi設定は本体のSelectから開けます。';$('notice').classList.remove('error');}render();
}
$('query').addEventListener('compositionstart',()=>{compositionBefore=$('query').value;});
$('query').addEventListener('compositionend',()=>{if(committedEdit(compositionBefore??state.query,$('query').value,'insertCompositionText'))inputClick();state.query=$('query').value;compositionBefore=null;});
$('query').addEventListener('input',event=>{if(committedEdit(state.query,$('query').value,event.inputType,event.isComposing||compositionBefore!==null))inputClick();cancel();state.query=$('query').value;state.mode='home';state.packet=null;state.results=[];$('notice').classList.remove('error');$('notice').textContent=bytes(state.query)>QUERY_BYTES?'検索語はUTF-8で96バイト以内にしてください。':collectionNotice();render();});
$('results-prev').onclick=()=>search(state.offset-10);$('results-next').onclick=()=>search(state.offset+10);
$('search-form').addEventListener('submit',e=>{e.preventDefault();search();});$('clear').onclick=()=>{$('query').value='';state.query='';home();$('query').focus();};
$('preview-open').onclick=()=>{$('preview').showModal();render();};$('preview-close').onclick=()=>{$('preview').close();};$('reader-back').onclick=back;$('reader-prev').onclick=()=>press('left');$('reader-next').onclick=()=>press('right');
document.querySelectorAll('[data-key]').forEach(b=>b.onclick=()=>press(b.dataset.key));$('screen').onkeydown=e=>{const map={ArrowUp:'up',ArrowDown:'down',ArrowLeft:'left',ArrowRight:'right',z:'a',x:'b',Enter:'start',Shift:'select'};if(map[e.key]){e.preventDefault();press(map[e.key]);}};render();
