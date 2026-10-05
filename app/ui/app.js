/* Ephemeral values belong to the Rust backend, not to a particular window. */
'use strict';
const $ = id => document.getElementById(id);
const invoke = (command,args={}) => window.__TAURI__.core.invoke(command,args);
const popup = document.body.dataset.surface === 'popup';
let snapshot=null,busy=false,pending=0,queue=Promise.resolve(),localError='';
const controls=new Map();
function error(e){localError=String(e);if($('notice'))$('notice').textContent=localError;$('result').textContent=`Error: ${localError}\nIf restoration fails, keep the app open and retry Restore.`;}
function effective(id){const master=snapshot.controls.master;return id==='master'||!master.enabled?snapshot.controls[id].dim:master.dim;}
function ready(mode='preview'){return !busy&&!pending&&snapshot&&(snapshot.armed.length===0||(mode==='continuous'&&snapshot.operation==='continuous'))&&snapshot.consent&&snapshot.monitors.some(m=>m.supported&&snapshot.controls[m.id].enabled&&effective(m.id)>0);}
function sync(){
 if(!snapshot)return;
 for(const [id,c] of controls){
  const model=snapshot.controls[id],master=id==='master',monitor=snapshot.monitors.find(m=>m.id===id);
  c.check.checked=model.enabled;c.check.disabled=busy||(!master&&!monitor.supported);
  const inherited=!master&&snapshot.controls.master.enabled;
  c.slider.disabled=c.number.disabled=busy||!model.enabled||(!master&&(!monitor.supported||inherited));
  const value=String(effective(id));
  if(document.activeElement!==c.slider)c.slider.value=value;
  if(document.activeElement!==c.number)c.number.value=value;
  c.note.textContent=master?(model.enabled?(popup?'One value for all included':'One value for included displays'):'Adjust each display separately'):!monitor.supported?'Unavailable':!model.enabled?'Excluded · original':inherited?'Linked':snapshot.armed.includes(id)?'Applied · protected':'Independent';
 }
 $('consent').checked=snapshot.consent;$('consent').disabled=busy||pending>0;
 $('preview').disabled=!ready();$('apply').disabled=!ready('continuous');$('restore').disabled=busy||pending>0;
 $('mode').textContent=snapshot.mode==='demo'?'DEMO · no display writes':'NATIVE · no auto apply';
 $('state').textContent=snapshot.armed.length?(snapshot.operation==='continuous'?'Continuous · protected while app is healthy':'Preview · restoring after 15 seconds'):snapshot.consent?'Ready to apply':'Original · nothing applied';
 $('active').textContent=snapshot.armed.length?snapshot.armed.join(', '):'Editing values never changes your screen.';
 const details=snapshot.outcomes.map(r=>`${r.id}: API ${r.api_success?'accepted':'FAILED'} · readback ${r.readback_matches===null?'FAILED: '+r.readback_error:r.readback_matches?'matches':'MISMATCH (driver may ignore the request)'} · visible effect unverified`).join('\n');
 $('result').textContent=[snapshot.message,details,...snapshot.restore_errors.map(e=>'RESTORE ERROR: '+e),localError?'Error: '+localError:''].filter(Boolean).join('\n');
 if($('notice'))$('notice').textContent=[localError,...snapshot.restore_errors,/Safety stop|error|failed|stopped/i.test(snapshot.message)?snapshot.message:''].filter(Boolean).join(' · ');
 if($('debug-data'))$('debug-data').textContent=JSON.stringify(snapshot,null,2);
}
function edit(command,args){
 localError='';pending++;sync();
 queue=queue.then(()=>invoke(command,args)).catch(error).finally(async()=>{pending--;if(!pending)await refresh();else sync();});
 return queue;
}
function renderMonitors(){
 const root=$('monitors');root.replaceChildren();controls.clear();
 const columns=[{id:'master',label:'Link displays',name:'Linked displays',supported:true},...snapshot.monitors.map(m=>({...m,label:snapshot.mode==='demo'?m.name:m.id}))];
 for(const m of columns){
  const card=document.createElement('section');card.className='display-row'+(m.id==='master'?' linked':'');card.title=m.id==='master'?'Linked control for included displays':`${m.name}\n${m.id}${m.error?'\n'+m.error:''}`;
  const label=document.createElement('label');label.className='enable';
  const check=document.createElement('input');check.type='checkbox';check.dataset.control=m.id;check.setAttribute('aria-label',m.id==='master'?'Link included displays':`Include ${m.name} ${m.id}`);
  const letter=document.createElement('span');letter.textContent=m.label;label.append(check,letter);
  const rail=document.createElement('div');rail.className='rail';
  const high=document.createElement('span');high.className='endpoint';high.textContent='90';
  const slider=document.createElement('input');slider.type='range';slider.min='0';slider.max='90';slider.step='5';slider.value='0';slider.dataset.control=m.id;slider.setAttribute('aria-label',`${m.name} dimming amount`);slider.setAttribute('aria-orientation','horizontal');
  const low=document.createElement('span');low.className='endpoint';low.textContent='0';rail.append(high,slider,low);
  const number=document.createElement('input');number.type='number';number.min='0';number.max='90';number.step='5';number.value='0';number.dataset.control=m.id;number.setAttribute('aria-label',`${m.name} dimming amount, zero to ninety, steps of five`);
  const name=document.createElement('div');name.className='monitor-name';name.textContent=m.id==='master'?'Master':m.id;name.title=m.name;
  const note=document.createElement('div');note.className='control-note';
  const c={check,slider,number,note};controls.set(m.id,c);
  const update=()=>{
   const model=snapshot.controls[m.id];
   return edit('set_control',{id:m.id,dim:model.dim,enabled:model.enabled});
  };
  check.addEventListener('change',()=>{snapshot.controls[m.id].enabled=check.checked;return update();});
  slider.addEventListener('input',()=>{const dim=Number(slider.value);snapshot.controls[m.id].dim=dim;number.value=String(dim);return update();});
  number.addEventListener('change',()=>{
   const dim=Number(number.value);
   if(number.value.trim()===''||!Number.isInteger(dim)||dim<0||dim>90||dim%5!==0){number.value=String(snapshot.controls[m.id].dim);error('Use 0–90 in exact steps of 5. Value was not changed.');return;}
   snapshot.controls[m.id].dim=dim;slider.value=String(dim);return update();
  });
  const identity=document.createElement('div');identity.className='identity';identity.append(label,note);
  card.append(identity,rail,number);root.append(card);
 }
}
async function refresh(){
 try{
  const next=await invoke('status');if(pending)return;
  const rebuild=!snapshot||JSON.stringify(snapshot.monitors)!==JSON.stringify(next.monitors);
  snapshot=next;if(rebuild)renderMonitors();sync();
 }catch(e){snapshot=null;$('state').textContent='Safety lock · status unavailable';$('preview').disabled=true;error(e);}
}
$('consent').addEventListener('change',()=>edit('set_consent',{consent:$('consent').checked}));
$('preview').addEventListener('click',async()=>{
 if(!ready())return;
 if(!window.confirm('Preview enabled targets for 15 seconds? Confirm HDR is off. High dimming may be rejected or ignored by your driver.'))return;
 busy=true;localError='';sync();
 try{await invoke('preview');}catch(e){error(e);}finally{busy=false;await refresh();}
});
$('restore').addEventListener('click',async()=>{
 if(busy||pending)return;busy=true;localError='';sync();
 try{await invoke('restore');}catch(e){error(e);}finally{busy=false;await refresh();}
});
if($('refresh'))$('refresh').addEventListener('click',()=>{if(!busy&&!pending)refresh();});
if(popup){
 $('close-popup').addEventListener('click',()=>invoke('hide_popup'));
 $('open-main').addEventListener('click',()=>invoke('open_main'));
 document.addEventListener('keydown',e=>{if(e.key==='Escape'){e.preventDefault();return invoke('hide_popup');}});
}
 $('apply').addEventListener('click',async()=>{
  if(!ready('continuous'))return;
  if(!window.confirm('Apply continuous dimming? It stays on when this window is hidden. Restore or Quit restores saved original gamma. Confirm HDR is off and other color tools are disabled.'))return;
  busy=true;localError='';sync();
  try{await invoke('apply',{mode:'continuous'});}catch(e){error(e);}finally{busy=false;await refresh();}
 });
if(window.__TAURI__?.event){
 window.__TAURI__.event.listen('gamma-error',event=>error(event.payload));
 window.__TAURI__.event.listen('state-changed',()=>{if(!busy&&!pending)refresh();});
}
refresh();setInterval(()=>{if(!busy&&!pending)refresh();},500);
