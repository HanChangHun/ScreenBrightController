// Real UI in two fake DOMs sharing one IPC backend; never calls display APIs.
const {readFileSync}=require('node:fs');
const {resolve}=require('node:path');
const vm=require('node:vm');
const assert=require('node:assert/strict');
let assertions=0;
const equal=(a,b)=>{assert.deepEqual(a,b);assertions++;};
const ok=a=>{assert.ok(a);assertions++;};
class Element {
 constructor(tag='div'){this.tag=tag;this.children=[];this.events={};this.dataset={};this.checked=false;this.value='0';this.disabled=false;this.textContent='';this.attributes={};this.hidden=false;}
 addEventListener(type,fn){this.events[type]=fn;}
 setAttribute(k,v){this.attributes[k]=v;}
 append(...children){this.children.push(...children);}
 replaceChildren(...children){this.children=children;}
 focus(){this.focused=true;}
}
const state={mode:'demo',monitors:[{id:'mock-1',name:'Demo screen 1',supported:true,error:null},{id:'mock-2',name:'Demo screen 2',supported:true,error:null}],armed:[],restore_errors:[],preview_seconds:15,controls:{master:{dim:0,enabled:true},'mock-1':{dim:0,enabled:true},'mock-2':{dim:0,enabled:true}},consent:false,outcomes:[],message:'No changes applied.'};
const calls=[];
const listeners=[];
const invoke=async(cmd,args={})=>{
 calls.push({cmd,args});
 if(cmd==='status')return structuredClone(state);
 if(cmd==='set_control'){
  if(!Number.isInteger(args.dim)||args.dim<0||args.dim>90||args.dim%5)throw Error('invalid dim');
  state.controls[args.id]={dim:args.dim,enabled:args.enabled};
 } else if(cmd==='set_consent')state.consent=args.consent;
 else if(cmd==='preview'||cmd==='apply'){
  if(cmd==='apply')assert.equal(args.mode,'continuous');
  if(!state.consent||state.armed.length)throw Error('consent/rearm');
  state.armed=state.monitors.filter(m=>m.supported&&state.controls[m.id].enabled&&(state.controls.master.enabled?state.controls.master.dim:state.controls[m.id].dim)>0).map(m=>m.id);
  state.outcomes=state.armed.map(id=>({id,api_success:true,readback_matches:false,readback_error:null,visible_effect_verified:false}));
  state.consent=false;state.message='Preview attempted.';
 } else if(cmd==='restore'){state.armed=[];state.consent=false;state.message='Restored originals.';}
 else if(!['hide_popup','open_main'].includes(cmd))throw Error('unexpected command '+cmd);
};
const traverse=e=>[e,...e.children.flatMap(traverse)];
function surface(popup){
 const ids=['state','mode','active','monitors','result','consent','preview','apply','restore','refresh','debug-data','close-popup','open-main'];
 const nodes=Object.fromEntries(ids.map(id=>[id,new Element()]));
 const tabs=[];
 const document={body:new Element('body'),activeElement:null,events:{},getElementById:id=>nodes[id],createElement:tag=>new Element(tag),addEventListener(type,fn){this.events[type]=fn;},querySelectorAll:selector=>selector==='[role="tab"]'?tabs:selector==='[role="tabpanel"]'?['screens','options','about','debug'].map(id=>nodes[id]):[]};
 document.body.dataset.surface=popup?'popup':'main';
 let interval;
 const context={document,window:{confirm:()=>true,__TAURI__:{core:{invoke},event:{listen:(name,fn)=>{listeners.push({name,fn});return Promise.resolve(()=>{});}}}},setInterval:fn=>{interval=fn;},console};
 vm.runInNewContext(readFileSync(resolve(__dirname,'../app/ui/app.js'),'utf8'),context);
 return {nodes,tabs,document,interval:()=>interval(),find:(id,type)=>traverse(nodes.monitors).find(e=>e.dataset.control===id&&e.type===type)};
}
const tick=async()=>{for(let i=0;i<8;i++)await new Promise(r=>setImmediate(r));};
(async()=>{
 const main=surface(false),popup=surface(true);await tick();
 equal(calls.map(c=>c.cmd),['status','status']);
 for(const ui of [main,popup]){
  equal(ui.find('master','range').min,'0');equal(ui.find('master','range').max,'90');equal(ui.find('master','range').step,'5');equal(ui.find('master','range').value,'0');
  equal(ui.nodes.preview.disabled,true);equal(ui.find('mock-1','number').disabled,true);
 }
 const slider=main.find('master','range');slider.value='90';await slider.events.input();await tick();
 equal(calls.filter(c=>c.cmd==='preview').length,0);equal(state.controls.master.dim,90);
 popup.interval();await tick();equal(popup.find('master','number').value,'90');
 const number=main.find('master','number');number.value='89';await number.events.change();await tick();equal(state.controls.master.dim,90);equal(number.value,'90');
 number.value='40';await number.events.change();await tick();equal(state.controls.master.dim,40);
 const masterCheck=main.find('master','checkbox');masterCheck.checked=false;await masterCheck.events.change();await tick();equal(state.controls.master.enabled,false);equal(main.find('mock-1','range').disabled,false);
 const monitor=main.find('mock-1','number');monitor.value='90';await monitor.events.change();await tick();equal(state.controls['mock-1'].dim,90);
 const enabled=main.find('mock-2','checkbox');enabled.checked=false;await enabled.events.change();await tick();equal(state.controls['mock-2'].enabled,false);
 popup.interval();await tick();equal(popup.find('mock-1','number').value,'90');equal(popup.find('mock-2','checkbox').checked,false);
 main.nodes.consent.checked=true;await main.nodes.consent.events.change();await tick();popup.interval();await tick();equal(popup.nodes.consent.checked,true);equal(popup.nodes.preview.disabled,false);
 ok(typeof popup.nodes.apply.events.click==='function');
 await popup.nodes.apply.events.click();await tick();equal(state.armed,['mock-1']);equal(state.consent,false);equal(calls.filter(c=>c.cmd==='apply').length,1);
 main.interval();await tick();equal(main.nodes.preview.disabled,true);ok(main.nodes.result.textContent.includes('MISMATCH'));ok(main.nodes.result.textContent.includes('unverified'));
 await main.nodes.restore.events.click();await tick();equal(state.armed,[]);popup.interval();await tick();ok(popup.nodes.result.textContent.includes('Restored'));
 await popup.document.events.keydown({key:'Escape',preventDefault(){}});equal(calls.at(-1).cmd,'hide_popup');
 await popup.nodes['open-main'].events.click();equal(calls.at(-1).cmd,'open_main');
 for(const ui of [main,popup])equal(ui.find('mock-1','range').attributes['aria-orientation'],'horizontal');
 ok(main.nodes['debug-data'].textContent.includes('mock-1'));
 console.log(`UI smoke PASS: ${assertions} assertions; shared main+popup state, horizontal ranges, exact steps, consent, explicit continuous Apply, restore, mismatch, Escape/open.`);
})().catch(e=>{console.error(e);process.exitCode=1;});
