/* Browser-only memory bridge: never overrides Tauri and cannot call gamma APIs. */
'use strict';
if (!window.__TAURI__ && new URLSearchParams(window.location.search).has('demo')) {
 const key='screen-bright-browser-demo-v3';
 const initial=()=>({mode:'demo',operation:'idle',monitors:[{id:'demo-screen-1',name:'Demo display 1',supported:true,error:null},{id:'demo-screen-2',name:'Demo display 2',supported:true,error:null}],armed:[],restore_errors:[],preview_seconds:15,controls:{master:{dim:0,enabled:true},'demo-screen-1':{dim:0,enabled:true},'demo-screen-2':{dim:0,enabled:true}},consent:false,outcomes:[],message:'Browser demo · memory state only, no gamma APIs.',deadline:0});
 const read=()=>{try{return JSON.parse(localStorage.getItem(key))||initial();}catch{return initial();}};
 const save=state=>localStorage.setItem(key,JSON.stringify(state));
 window.__TAURI__={core:{invoke:async(command,args={})=>{
  const state=read();
  if(state.operation==='preview'&&state.deadline&&Date.now()>=state.deadline){state.armed=[];state.deadline=0;state.operation='idle';state.message='Browser demo preview expired; memory state restored.';save(state);}
  if(command==='status')return state;
  if(command==='set_control'){
   if(!state.controls[args.id]||!Number.isInteger(args.dim)||args.dim<0||args.dim>90||args.dim%5)throw Error('Use 0–90 in exact steps of 5.');
   if(args.id==='master'&&args.enabled&&args.dim===0)state.armed=[];
   else if(args.id!=='master'&&(!args.enabled||(args.dim===0&&!state.controls.master.enabled)))state.armed=state.armed.filter(id=>id!==args.id);
   state.controls[args.id]={dim:args.dim,enabled:args.enabled};
   if(!state.armed.length)state.operation='idle';
  }else if(command==='set_consent'){state.consent=Boolean(args.consent);}
  else if(command==='preview'||command==='apply'){
   const mode=command==='preview'?'preview':args.mode;
   if(!['preview','continuous'].includes(mode)||!state.consent||(state.armed.length&&(mode==='preview'||state.operation==='preview')))throw Error('Valid mode/consent required; restore preview first.');
   state.armed=state.monitors.filter(m=>state.controls[m.id].enabled&&(state.controls.master.enabled?state.controls.master.dim:state.controls[m.id].dim)>0).map(m=>m.id);
   state.outcomes=state.armed.map(id=>({id,api_success:true,readback_matches:true,readback_error:null,visible_effect_verified:false}));
   state.consent=false;state.operation=state.armed.length?mode:'idle';state.deadline=mode==='preview'?Date.now()+15000:0;state.message='Browser demo: simulated '+mode+' only. Native lease protection is tested separately.';
  }else if(command==='restore'){state.armed=[];state.deadline=0;state.operation='idle';state.consent=false;state.outcomes=[];for(const c of Object.values(state.controls))c.dim=0;state.message='Browser demo: memory originals restored.';}
  else if(command==='hide_popup'){document.body.hidden=true;}
  else if(command==='open_main'){window.location.href='index.html?demo=1';}
  else throw Error('Unknown browser demo command');
  save(state);
 }},event:{listen:()=>Promise.resolve(()=>{})}};
}
