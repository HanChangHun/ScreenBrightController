/* Non-modal update banner, following claude-usage. No gamma commands here. */
'use strict';
(()=>{
 const node=id=>document.getElementById(id),invoke=command=>window.__TAURI__.core.invoke(command);
 let checking=false,installing=false,release=null;
 async function check(manual=false){
  if(checking||installing)return;checking=true;node('update-check').disabled=true;
  if(manual)node('update-status').textContent='Checking for updates…';
  try{
   const result=await invoke('check_update');release=result.release;
   node('update-banner').hidden=!release;
   if(release){node('update-message').textContent=`v${release.version} available`;node('update-notes').textContent=release.notes||'No release notes.';}
   if(manual)node('update-status').textContent=release?`Current: v${result.current}`:`You are on the latest version (v${result.current}).`;
  }catch(e){if(manual)node('update-status').textContent=`Update check failed: ${e}`;}
  finally{checking=false;node('update-check').disabled=false;}
 }
 node('update-check').addEventListener('click',e=>{if(e.isTrusted)check(true);});
 node('update-dismiss').addEventListener('click',e=>{if(e.isTrusted&&!installing)node('update-banner').hidden=true;});
 node('update-install').addEventListener('click',async e=>{
  if(!e.isTrusted||installing||checking||!release)return;
  installing=true;node('update-install').disabled=node('update-dismiss').disabled=node('update-check').disabled=true;
  node('update-message').textContent='Downloading and verifying…';node('update-install').textContent='Preparing…';
  try{await invoke('install_update');node('update-message').textContent='Restoring brightness, then installing and restarting…';}
  catch(error){installing=false;node('update-message').textContent=`Update failed: ${error}`;node('update-install').textContent='Retry install';node('update-install').disabled=node('update-dismiss').disabled=node('update-check').disabled=false;}
 });
 check();setInterval(()=>check(),6*60*60*1000);
})();
