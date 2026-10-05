/* Live intent is emitted only by trusted user events; DOM synchronization is read-only. */
'use strict';
const $=id=>document.getElementById(id),invoke=(c,a={})=>window.__TAURI__.core.invoke(c,a);
const popup=document.body.dataset.surface==='popup',controls=new Map(),intents=new Map();
let snapshot=null,flight=null,timer=null,restoring=false,localError='',serial=0;
const label=id=>id==='master'?'Linked displays':`Display ${snapshot.monitors.findIndex(m=>m.id===id)+1}`;
const readable=text=>{let value=String(text);for(const m of snapshot?.monitors||[])value=value.split(m.id).join(label(m.id));return value;};
function error(e){localError=String(e);if(!snapshot&&$('notice')){$('notice').textContent=localError;$('notice').hidden=false;}sync();}
function model(id){return intents.get(id)?.control||(flight&&flight.generation===snapshot.generation&&flight.id===id?flight.control:snapshot.controls[id]);}
function effective(id){const m=model('master');return id==='master'||!m.enabled?model(id).dim:m.dim;}
function cancel(){intents.clear();clearTimeout(timer);timer=null;}
function accept(next){
 if(snapshot&&(next.revision<snapshot.revision||next.generation<snapshot.generation))return;
 if(snapshot&&next.generation!==snapshot.generation){cancel();for(const c of controls.values())c.number.value=String(next.controls[c.number.dataset.control].dim);}
 const rebuild=!snapshot||JSON.stringify(snapshot.monitors)!==JSON.stringify(next.monitors);
 snapshot=next;if(rebuild)renderMonitors();sync();
}
function sync(){
 if(!snapshot)return;
 for(const [id,c] of controls){const v=model(id),master=id==='master',m=snapshot.monitors.find(m=>m.id===id),inherited=!master&&model('master').enabled;
 c.check.checked=v.enabled;c.check.disabled=restoring||(!master&&!m.supported);
 c.slider.disabled=c.number.disabled=restoring||!v.enabled||(!master&&(!m.supported||inherited));
 const value=String(effective(id));c.slider.value=value;if(document.activeElement!==c.number)c.number.value=value;
 c.note.textContent=master?(v.enabled?'One value for included displays':'Adjust displays separately'):!m.supported?'Unavailable':!v.enabled?'Excluded · original':inherited?'Linked':snapshot.armed.includes(id)?'Applied · protected':'Independent';
 }
 $('restore').disabled=restoring;
 const notice=[localError,...snapshot.restore_errors,/Safety stop|error|failed|stopped/i.test(snapshot.message)?snapshot.message:''].filter(Boolean).map(readable).join(' · ');
 if($('notice')){$('notice').textContent=notice;$('notice').hidden=!notice;}

}
function schedule(){if(!timer&&!restoring)timer=setTimeout(()=>{timer=null;drain();},80);}
function intent(id,control){if(restoring||!snapshot)return;localError='';intents.set(id,{id,control,generation:snapshot.generation,serial:++serial});sync();schedule();}
async function drain(){
 if(flight||restoring||!intents.size)return;
 const item=intents.values().next().value;intents.delete(item.id);flight=item;
 item.done=(async()=>{try{const next=await invoke('live_control',{id:item.id,...item.control,generation:item.generation});if(next)accept(next);}catch(e){cancel();error(e);}finally{if(flight===item)flight=null;await refresh();if(intents.size)schedule();}})();
 await item.done;
}
function renderMonitors(){
 const root=$('monitors');root.replaceChildren();controls.clear();
 for(const m of [{id:'master',name:'Linked displays',supported:true},...snapshot.monitors]){
 const card=document.createElement('section');card.className='display-row'+(m.id==='master'?' linked':'');card.title=m.id==='master'?'Linked control for included displays':`${m.name}\n${m.id}`;
 const identity=document.createElement('div');identity.className='identity';const enable=document.createElement('label');enable.className='enable';
 const check=document.createElement('input');check.type='checkbox';check.dataset.control=m.id;check.setAttribute('aria-label',m.id==='master'?'Link included displays':`Include ${label(m.id)}`);
 const text=document.createElement('span');text.textContent=m.id==='master'?'Link displays':label(m.id);enable.append(check,text);
 const note=document.createElement('div');note.className='control-note';identity.append(enable,note);
 const rail=document.createElement('div');rail.className='rail';const high=document.createElement('span');high.className='endpoint';high.textContent='90';const low=document.createElement('span');low.className='endpoint';low.textContent='0';
 const slider=document.createElement('input'),number=document.createElement('input');slider.type='range';number.type='number';
 for(const input of [slider,number]){input.min='0';input.max='90';input.step='1';input.value='0';input.dataset.control=m.id;input.setAttribute('aria-label',`${label(m.id)} dimming amount`);}
 slider.title='Drag: 1 · Arrow keys / wheel: 5 · Home: 0 · End: 90';slider.setAttribute('aria-orientation','horizontal');rail.append(high,slider,low);controls.set(m.id,{check,slider,number,note});
 const update=dim=>intent(m.id,{dim,enabled:model(m.id).enabled});
 check.addEventListener('change',e=>{if(e.isTrusted)intent(m.id,{dim:model(m.id).dim,enabled:check.checked});});
 slider.addEventListener('input',e=>{if(e.isTrusted){const dim=Number(slider.value);number.value=String(dim);update(dim);}});
 const adjust=value=>{const dim=Math.max(0,Math.min(90,value));slider.value=number.value=String(dim);update(dim);};
 slider.addEventListener('keydown',e=>{if(!e.isTrusted||slider.disabled)return;const d={ArrowLeft:-5,ArrowDown:-5,ArrowRight:5,ArrowUp:5};if(e.key in d||e.key==='Home'||e.key==='End'){e.preventDefault();adjust(e.key==='Home'?0:e.key==='End'?90:Number(slider.value)+d[e.key]);}});
 slider.addEventListener('wheel',e=>{if(!e.isTrusted||slider.disabled||!e.deltaY)return;e.preventDefault();adjust(Number(slider.value)+(e.deltaY<0?5:-5));},{passive:false});
 number.addEventListener('change',e=>{if(!e.isTrusted)return;const dim=Number(number.value);if(number.value.trim()===''||!Number.isInteger(dim)||dim<0||dim>90){number.value=String(effective(m.id));error('Use an integer from 0 to 90.');return;}update(dim);});
 card.append(identity,rail,number);root.append(card);
 }
}
async function refresh(){try{accept(await invoke('status'));}catch(e){error(`Status unavailable: ${e}. Restore remains available.`);}}
$('restore').addEventListener('click',async e=>{if(!e.isTrusted||restoring)return;restoring=true;cancel();localError='';sync();try{if(flight)await flight.done;accept(await invoke('restore'));}catch(e){error(e);}finally{restoring=false;await refresh();}});
if(popup){$('close-popup').addEventListener('click',()=>invoke('hide_popup'));$('open-main').addEventListener('click',()=>invoke('open_main'));document.addEventListener('keydown',e=>{if(e.key==='Escape'){e.preventDefault();invoke('hide_popup');}});}
if(window.__TAURI__?.event){window.__TAURI__.event.listen('gamma-error',e=>error(e.payload));window.__TAURI__.event.listen('state-changed',()=>refresh());}
refresh();setInterval(refresh,500);
