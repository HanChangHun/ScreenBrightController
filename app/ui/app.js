/* Live intent is emitted only by trusted user events; DOM synchronization is read-only. */
'use strict';
const $=id=>document.getElementById(id),invoke=(c,a={})=>window.__TAURI__.core.invoke(c,a);
const controls=new Map(),intents=new Map();
let snapshot=null,flight=null,timer=null,restoring=false,locallyBlocked=false,statusUnavailable=false,localError='',statusSerial=0,statusSuccess=0;
const label=id=>id==='master'?'Linked displays':`Display ${snapshot.monitors.findIndex(m=>m.id===id)+1}`;
const readable=text=>{let value=String(text);for(const m of snapshot?.monitors||[])value=value.split(m.id).join(label(m.id));return value;};
function error(e){localError=String(e);sync();}
function paused(){return statusUnavailable||locallyBlocked||Boolean(snapshot?.live_blocked)||Boolean(snapshot?.restore_errors?.length);}
function model(id){if(paused())return snapshot.controls[id];return intents.get(id)?.control||(flight&&flight.generation===snapshot.generation&&flight.id===id?flight.control:snapshot.controls[id]);}
function effective(id){const m=model('master');return id==='master'||!m.enabled?model(id).dim:m.dim;}
function cancel(){intents.clear();clearTimeout(timer);timer=null;}
function accept(next){
 if(snapshot&&(next.revision<snapshot.revision||next.generation<snapshot.generation))return false;
 const wasPaused=paused();
 if(snapshot&&next.generation!==snapshot.generation){cancel();for(const c of controls.values())c.number.value=String(next.controls[c.number.dataset.control].dim);}
 const rebuild=!snapshot||JSON.stringify(snapshot.monitors)!==JSON.stringify(next.monitors);
 snapshot=next;if(next.live_blocked||wasPaused){cancel();localError='';}locallyBlocked=false;statusUnavailable=false;
 if(rebuild)renderMonitors();sync();return true;
}
function renderNotice(){
 const needsRestore=paused();
 const recoveryMessage=snapshot?.recovery==='restore'||snapshot?.restore_errors?.length?'Restore failed. Keep the app open and try again.':snapshot?.recovery==='safety'?'Display state changed. Restore to continue.':'Dimming paused. Requested values are unconfirmed. Restore, then try a lower amount.';
 const dimmed=(snapshot?.monitors||[]).filter(m=>m.dimmed).map(m=>label(m.id));
 const baseline=dimmed.length?`${dimmed.join(', ')} started dimmed: the saved gamma is far below normal, probably left by an earlier session that could not restore.`:'';
 const message=statusUnavailable?'Display status unavailable. Restore is still available.':needsRestore?recoveryMessage:localError||baseline;
 const evidence=(snapshot?.outcomes||[]).map(row=>`${label(row.id)} — API: ${row.api_success?'accepted':'rejected'}; readback: ${row.readback_matches===true?'matched':row.readback_matches===false?'not matched':'unavailable'}${row.readback_error?` (${readable(row.readback_error)})`:''}`);
 const detail=needsRestore&&!statusUnavailable?[readable(snapshot?.message||localError),...evidence,...(snapshot?.restore_errors||[]).map(readable)].filter(Boolean).join('\n'):localError;
 if($('notice-message').textContent!==message)$('notice-message').textContent=message;
 $('notice-detail').textContent=detail;$('notice-details').hidden=!detail;
 $('notice').hidden=!message;
 $('reset-baseline').hidden=needsRestore||Boolean(localError)||!baseline;$('reset-baseline').disabled=restoring;
 $('recover').hidden=!needsRestore;$('recover').disabled=restoring;$('recover').textContent=restoring?'Restoring…':'Restore';
}
function sync(){
 $('restore').disabled=restoring;renderNotice();
 if(!snapshot)return;
 for(const [id,c] of controls){const v=model(id),master=id==='master',m=snapshot.monitors.find(m=>m.id===id),inherited=!master&&model('master').enabled;
 c.check.checked=v.enabled;c.check.disabled=restoring||paused()||(!master&&!m.supported);
 c.slider.disabled=c.number.disabled=restoring||paused()||!v.enabled||(!master&&(!m.supported||inherited));
 const value=String(effective(id));c.slider.value=value;if(document.activeElement!==c.number||paused()||restoring)c.number.value=value;
 c.number.setAttribute('aria-invalid',String(paused()));
 c.check.title=!master&&!m.supported?readable(m.error||'Display is unavailable'):'';
 }
}
function schedule(){if(!timer&&!restoring&&!paused())timer=setTimeout(()=>{timer=null;drain();},80);}
function intent(id,control){if(restoring||paused()||!snapshot){sync();return;}localError='';intents.set(id,{id,control,generation:snapshot.generation});sync();schedule();}
async function drain(){
 if(flight||restoring||paused()||!intents.size)return;
 const item=intents.values().next().value;intents.delete(item.id);flight=item;
 item.done=(async()=>{try{const next=await invoke('live_control',{id:item.id,...item.control,generation:item.generation});if(next)accept(next);}catch(e){if(item.generation===snapshot.generation||snapshot.live_blocked){cancel();locallyBlocked=true;error(e);}}finally{if(flight===item)flight=null;await refresh();if(intents.size)schedule();}})();
 await item.done;
}
function renderMonitors(){
 const root=$('monitors');root.replaceChildren();controls.clear();
 for(const m of [{id:'master',name:'Linked displays',supported:true},...snapshot.monitors]){
 const card=document.createElement('section');card.className='display-row'+(m.id==='master'?' linked':'');card.title=m.id==='master'?'Linked control for included displays':`${m.name}\n${m.id}`;
 const identity=document.createElement('div');identity.className='identity';const enable=document.createElement('label');enable.className='enable';
 const check=document.createElement('input');check.type='checkbox';check.dataset.control=m.id;check.setAttribute('aria-label',m.id==='master'?'Link included displays':`Include ${label(m.id)}`);
 const text=document.createElement('span');text.textContent=m.id==='master'?'Link displays':label(m.id);enable.append(check,text);
 identity.append(enable);
 const rail=document.createElement('div');rail.className='rail';const high=document.createElement('span');high.className='endpoint';high.textContent='90';const low=document.createElement('span');low.className='endpoint';low.textContent='0';
 const slider=document.createElement('input'),number=document.createElement('input');slider.type='range';number.type='number';
 for(const input of [slider,number]){input.min='0';input.max='90';input.step='1';input.value='0';input.dataset.control=m.id;input.setAttribute('aria-label',`${label(m.id)} dimming amount`);}
 slider.title='Drag: 1 · Arrow keys / wheel: 5 · Home: 0 · End: 90';slider.setAttribute('aria-orientation','horizontal');rail.append(high,slider,low);controls.set(m.id,{check,slider,number});
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
async function refresh(){const request=++statusSerial,generation=snapshot?.generation,revision=snapshot?.revision;try{if(accept(await invoke('status')))statusSuccess=Math.max(statusSuccess,request);}catch(e){if(statusSuccess>request||generation!==snapshot?.generation||revision!==snapshot?.revision)return;statusUnavailable=true;cancel();error(`Status unavailable: ${e}. Restore remains available.`);}}
async function restore(e){if(!e.isTrusted||restoring)return;restoring=true;cancel();localError='';sync();try{if(flight)await flight.done;accept(await invoke('restore'));}catch(e){locallyBlocked=true;error(e);}finally{restoring=false;await refresh();}}
async function resetBaseline(e){if(!e.isTrusted||restoring)return;try{accept(await invoke('reset_baseline'));}catch(e){error(e);}}
$('reset-baseline').addEventListener('click',resetBaseline);
$('restore').addEventListener('click',restore);$('recover').addEventListener('click',restore);
$('close-popup').addEventListener('click',()=>invoke('hide_popup'));
document.addEventListener('keydown',e=>{if(e.key==='Escape'){e.preventDefault();invoke('hide_popup');}});
if(window.__TAURI__?.event){window.__TAURI__.event.listen('display-error',e=>error(e.payload));window.__TAURI__.event.listen('state-changed',()=>refresh());}
if($('autostart')){
 const check=$('autostart'),message=$('startup-message');let busy=false;
 function displayStartup(result){check.checked=result.enabled===true;check.disabled=result.enabled===null;message.textContent=result.error||'';}
 async function loadStartup(){
  if(busy)return;busy=true;check.disabled=true;
  try{displayStartup(await invoke('get_autostart'));}
  catch(e){displayStartup({enabled:null,error:`Startup state unavailable: ${e}`});}
  finally{busy=false;}
 }
 $('settings-toggle').addEventListener('click',()=>{const panel=$('settings');panel.hidden=!panel.hidden;$('settings-toggle').setAttribute('aria-expanded',String(!panel.hidden));if(!panel.hidden)loadStartup();});
 check.addEventListener('change',async e=>{
  if(!e.isTrusted||busy||check.disabled)return;
  const enabled=check.checked;busy=true;check.disabled=true;message.textContent='Saving…';
  try{displayStartup(await invoke('set_autostart',{enabled}));}
  catch(e){try{const result=await invoke('get_autostart');displayStartup({...result,error:`Startup change failed: ${e}. ${result.error||''}`});}catch(readError){displayStartup({enabled:null,error:`Startup state unavailable: ${readError}`});}}
  finally{busy=false;}
 });
 window.addEventListener?.('focus',loadStartup);
 loadStartup();
}
// The backend emits state-changed after every change and each heartbeat.
refresh();
