/* Browser-only memory bridge; never overrides native Tauri or calls gamma APIs. */
'use strict';
if(!window.__TAURI__&&new URLSearchParams(location.search).has('demo')){
 const key='screen-bright-browser-demo-v4',listeners=[];
 const initial=()=>({mode:'demo',operation:'idle',generation:0,revision:0,monitors:[{id:'demo-screen-1',name:'Demo display 1',supported:true},{id:'demo-screen-2',name:'Demo display 2',supported:true}],controls:{master:{dim:0,enabled:true},'demo-screen-1':{dim:0,enabled:true},'demo-screen-2':{dim:0,enabled:true}},armed:[],outcomes:[],restore_errors:[],message:'Browser demo · memory only, no gamma APIs.'});
 const read=()=>JSON.parse(localStorage.getItem(key))||initial();
 const changed=()=>listeners.forEach(f=>f({}));
 window.addEventListener('storage',e=>{if(e.key===key)changed();});
 window.__TAURI__={core:{invoke:async(command,args={})=>{
 const state=read();if(command==='status')return state;
 if(command==='live_control'){
 if(args.generation!==state.generation)throw Error('Live request cancelled.');
 if(!state.controls[args.id]||!Number.isInteger(args.dim)||args.dim<0||args.dim>90)throw Error('Use an integer from 0 to 90.');
 state.controls[args.id]={dim:args.dim,enabled:args.enabled};
 state.armed=state.monitors.filter(m=>state.controls[m.id].enabled&&(state.controls.master.enabled?state.controls.master.dim:state.controls[m.id].dim)>0).map(m=>m.id);
 state.outcomes=state.armed.map(id=>({id,api_success:true,readback_matches:true,readback_error:null,visible_effect_verified:false}));
 state.operation=state.armed.length?'continuous':'idle';state.message='Browser demo: simulated live dimming only.';
 }else if(command==='restore'){state.generation++;state.armed=[];state.operation='idle';state.outcomes=[];for(const c of Object.values(state.controls))c.dim=0;state.message='Browser demo: memory originals restored.';}
 else if(command==='hide_popup'){document.body.hidden=true;return;}
 else if(command==='open_main'){location.href='index.html?demo=1';return;}
 else throw Error('Unknown browser demo command');
 state.revision++;localStorage.setItem(key,JSON.stringify(state));changed();return state;
 }},event:{listen:(name,fn)=>{if(name==='state-changed')listeners.push(fn);return Promise.resolve(()=>{});}}};
}
