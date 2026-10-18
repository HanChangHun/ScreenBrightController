// Actual settings controller with injected IPC; never native startup/gamma.
const fs=require('node:fs'),path=require('node:path'),vm=require('node:vm'),assert=require('node:assert/strict');
let assertions=0;const eq=(a,b)=>{assert.deepEqual(JSON.parse(JSON.stringify(a)),b);assertions++;};
const source=fs.readFileSync(path.join(__dirname,'../app/ui/app.js'),'utf8');
function fixture(){
 const nodes=Object.fromEntries(['settings-toggle','settings','autostart','startup-message'].map(id=>[id,{hidden:true,checked:false,disabled:true,events:{},textContent:'',addEventListener(k,f){this.events[k]=f;},setAttribute(k,v){this[k]=v;}}]));
 const calls=[];let reply={enabled:true,error:null},pending;
 const document={body:{dataset:{surface:'popup'}},getElementById:id=>nodes[id]||{addEventListener(){}},addEventListener(){}};
 // Remove only the existing unrelated dimming initialization; settings code is unchanged.
 const code=source.replace(/\nrefresh\(\);\s*$/,'');
 vm.runInNewContext(code,{document,window:{__TAURI__:{core:{invoke:async(c,a)=>{calls.push({c,a});if(pending)return pending;return reply;}}}},setTimeout,clearTimeout});
 return{nodes,calls,setReply:r=>reply=r,setPending:p=>pending=p};
}
const settle=()=>new Promise(r=>setTimeout(r,0));
(async()=>{
 const f=fixture();await settle();eq(f.nodes.autostart.checked,true);eq(f.calls.map(x=>x.c),['get_autostart']);eq(f.nodes.autostart.disabled,false);
 f.nodes['settings-toggle'].events.click({isTrusted:true});eq(f.nodes.settings.hidden,false);await settle();
 let release;f.setPending(new Promise(r=>release=r));f.nodes.autostart.checked=false;
 f.nodes.autostart.events.change({isTrusted:true});eq(f.nodes.autostart.disabled,true);
 eq(f.calls.at(-1),{c:'set_autostart',a:{enabled:false}});
 release({enabled:false,error:null});await settle();eq(f.nodes.autostart.checked,false);eq(f.nodes.autostart.disabled,false);
 f.setPending(null);f.setReply({enabled:false,error:'Startup change did not take effect'});f.nodes.autostart.checked=true;f.nodes.autostart.events.change({isTrusted:true});await settle();eq(f.nodes.autostart.checked,false);eq(f.nodes['startup-message'].textContent,'Startup change did not take effect');
 f.setReply({enabled:null,error:'Readback unavailable'});f.nodes.autostart.checked=true;f.nodes.autostart.events.change({isTrusted:true});await settle();eq(f.nodes.autostart.checked,false);eq(f.nodes.autostart.disabled,true);
 const before=f.calls.length;f.nodes.autostart.events.change({isTrusted:false});await settle();eq(f.calls.length,before);
 f.setReply({enabled:null,error:'Get failed'});f.nodes['settings-toggle'].events.click({isTrusted:true});f.nodes['settings-toggle'].events.click({isTrusted:true});await settle();eq(f.nodes.autostart.disabled,true);eq(f.nodes['startup-message'].textContent,'Get failed');
 f.setPending(Promise.reject(Error('IPC get failed')));f.nodes['settings-toggle'].events.click({isTrusted:true});f.nodes['settings-toggle'].events.click({isTrusted:true});await settle();eq(f.nodes.autostart.disabled,true);eq(f.nodes.autostart.checked,false);eq(f.nodes['startup-message'].textContent.includes('Startup state unavailable'),true);
 const store=new Map();const demoWindow={addEventListener(){}};vm.runInNewContext(fs.readFileSync(path.join(__dirname,'../app/demo/demo.js'),'utf8'),{window:demoWindow,location:{search:'?demo=1'},URLSearchParams,localStorage:{getItem:k=>store.get(k)||null,setItem:(k,v)=>store.set(k,v)}});eq((await demoWindow.__TAURI__.core.invoke('get_autostart')).enabled,false);eq((await demoWindow.__TAURI__.core.invoke('set_autostart',{enabled:true})).enabled,true);eq((await demoWindow.__TAURI__.core.invoke('status')).armed,[]);
 // The update controller uses memory IPC only: checks never install; only a trusted click does.
 const updateNodes=Object.fromEntries(['update-banner','update-message','update-notes','update-install','update-dismiss','update-check','update-status'].map(id=>[id,{hidden:true,disabled:false,textContent:'',events:{},addEventListener(k,f){this.events[k]=f;}}]));
 const updateCalls=[];let updateReply={current:'0.6.2',release:{version:'0.6.3',notes:'<b>plain notes</b>'}},installReject=false,updateTimer;
 vm.runInNewContext(fs.readFileSync(path.join(__dirname,'../app/ui/update.js'),'utf8'),{document:{getElementById:id=>updateNodes[id]},window:{__TAURI__:{core:{invoke:async c=>{updateCalls.push(c);if(c==='install_update'&&installReject)throw Error('checksum mismatch');return updateReply;}}}},setInterval:f=>{updateTimer=f;},console});
 await settle();eq(updateCalls,['check_update']);eq(updateNodes['update-banner'].hidden,false);eq(updateNodes['update-notes'].textContent,'<b>plain notes</b>');
 updateNodes['update-install'].events.click({isTrusted:false});await settle();eq(updateCalls.includes('install_update'),false);
 updateNodes['update-dismiss'].events.click({isTrusted:true});eq(updateNodes['update-banner'].hidden,true);
 updateNodes['update-check'].events.click({isTrusted:false});await settle();eq(updateCalls.length,1);
 updateNodes['update-check'].events.click({isTrusted:true});await settle();eq(updateNodes['update-banner'].hidden,false);
 installReject=true;updateNodes['update-install'].events.click({isTrusted:true});eq(updateNodes['update-install'].disabled,true);await settle();eq(updateNodes['update-install'].disabled,false);eq(updateNodes['update-message'].textContent.includes('checksum mismatch'),true);
 updateReply={current:'0.6.2',release:null};await updateTimer();await settle();eq(updateNodes['update-banner'].hidden,true);
 updateNodes['update-check'].events.click({isTrusted:true});await settle();eq(updateNodes['update-status'].textContent.includes('latest'),true);
 console.log(`Settings UI PASS: ${assertions} assertions`);
})().catch(e=>{console.error(e);process.exitCode=1;});
