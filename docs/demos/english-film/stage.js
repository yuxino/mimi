'use strict';
// Documentation-only boundary; never imported by the application.
// The input is the real, attributed film response captured in run 34040075430.
const settings={profiles:[{id:'alibaba-default',name:'Alibaba Cloud',provider:'alibabaCloud',credentialState:'present'}],activeProfileId:'alibaba-default',sourceLanguage:'en',targetLanguage:'zh',translationMode:'lowLatency',fontSize:16,subtitleAlignment:'center',subtitleBlendsWithBackground:false,isOverlayLocked:false,uiLanguage:'zh'};
const session={status:{kind:'idle'},isActive:false,isPaused:false,isOverlayCollapsed:false,subtitles:{source:{text:'',isFinal:false},translation:{text:'',isFinal:false},history:[]},detectedLanguage:null,isTranslationPending:false,isTranslationTimedOut:false};
const S={settings,session,calls:[],timeline:[],index:0,startedAt:null,controlMode:'hidden',pauseAt:null,pausedMillis:0};window.demoState=S;
const video=document.getElementById('movie');
let events=[];const originals=new Map(),previousItems=new Map(),historyIds=[];
window.emit=(name,payload)=>{for(const f of document.querySelectorAll('iframe'))f.contentWindow.__emit?.(name,structuredClone(payload))};
window.pointer=(kind,x,y)=>{const f=document.getElementById(kind),r=f?.getBoundingClientRect();document.getElementById('pointer').style.transform=`translate(${(r?.left||0)+x*1.5}px,${(r?.top||0)+y*1.5}px)`;};
document.addEventListener('mousemove',e=>{document.getElementById('pointer').style.transform=`translate(${e.clientX}px,${e.clientY}px)`;});
window.hideWindow=kind=>document.getElementById(kind)?.remove();
function showWindow(kind){if(document.getElementById(kind))return;const f=document.createElement('iframe');f.id=kind;f.src='/app/?window='+kind;f.title='mimi '+kind;document.body.append(f);return f;}
// Mirror OverlayControlWindowManager::sync_presentation and native click-through.
// Hiding only the subtitle canvas background is not complete Immersive Mode.
function syncPresentation(){
 const immersive=settings.subtitleBlendsWithBackground;
 const visible=session.isActive&&!session.isOverlayCollapsed&&!immersive;
 const next=visible?(S.controlMode==='panel'?'panel':'island'):'hidden';
 const control=document.getElementById('overlay-control');
 if(control){
  control.style.display=visible?'':'none';
  control.style.pointerEvents=visible?'auto':'none';
  if(next==='island'){control.style.height='30px';control.style.top='772px';}
 }
 S.controlMode=next;emit('overlay-control-mode',next);
 const overlay=document.getElementById('overlay');
 if(overlay)overlay.style.pointerEvents=(immersive||settings.isOverlayLocked)?'none':'auto';
}
window.handleShortcut=event=>{
 if(event.code!=='KeyM'||!event.shiftKey||(!event.ctrlKey&&!event.metaKey)||event.altKey)return;
 event.preventDefault();
 if(event.repeat)return;
 const now=performance.now();
 if(now-(S.lastShortcutAt??-Infinity)<500)return;
 S.lastShortcutAt=now;
 S.calls.push({command:'immersive_shortcut',at:elapsed(),key:event.metaKey?'Meta+Shift+M':'Control+Shift+M'});
 updateSettings({subtitleBlendsWithBackground:!settings.subtitleBlendsWithBackground});
};
document.addEventListener('keydown',window.handleShortcut);
function snapshot(){syncPresentation();emit('session-state',session)}
function updateSettings(draft){Object.assign(settings,draft);if(settings.subtitleBlendsWithBackground)session.isOverlayCollapsed=false;S.timeline.push({kind:'settings',at:elapsed(),values:draft});syncPresentation();emit('settings-changed',settings);snapshot()}
function elapsed(){return S.startedAt===null?0:((S.pauseAt||performance.now())-S.startedAt-S.pausedMillis)/1000;}
function apply(e){
 const k=e.type;
 if(k==='conversation.item.created'){if(e.item?.role==='assistant')previousItems.set(e.item.id,e.previous_item_id);return;}
 if(k==='conversation.item.input_audio_transcription.text'){session.subtitles.source={text:(e.text||'')+(e.stash||''),isFinal:false};}
 if(k==='conversation.item.input_audio_transcription.completed'){
   const text=e.transcript||e.text||'';originals.set(e.item_id,text);session.subtitles.source={text,isFinal:true};
   historyIds.forEach((id,i)=>{if(previousItems.get(id)===e.item_id)session.subtitles.history[i].source=text});
 }
 if(k==='response.text.text'){session.subtitles.translation={text:(e.text||'')+(e.stash||''),isFinal:false};session.isTranslationPending=false;}
 if(k==='response.text.done'){
  session.subtitles.translation={text:e.text||'',isFinal:true};session.isTranslationPending=false;
  if(e.text?.trim()&&!historyIds.includes(e.item_id)){historyIds.push(e.item_id);session.subtitles.history.push({source:originals.get(previousItems.get(e.item_id))||session.subtitles.source.text||'',translation:e.text,createdAt:Date.now()});}
 }
 session.detectedLanguage='en';snapshot();
}
setInterval(()=>{
 const now=elapsed();
 if(session.isActive&&!session.isPaused){while(S.index<events.length&&events[S.index].received_at<=now)apply(events[S.index++]);}
 document.getElementById('clock').textContent='00:'+String(Math.floor(video.currentTime)).padStart(2,'0')+' / 00:31';
},20);
window.invoke=async(kind,c,a={})=>{
 S.calls.push({command:c,at:elapsed()});
 if(c==='settings_get')return settings;
 if(c==='session_get_state')return session;
 if(c==='app_is_ui_test')return true;
 if(c==='session_start'){
  if(S.startedAt!==null)throw Error('Only one sample session');
  const data=await fetch('/responses.json').then(r=>r.json());events=data.events;
  session.isActive=true;session.status={kind:'listening'};session.detectedLanguage='en';
  showWindow('overlay');showWindow('overlay-control');
  await video.play();S.startedAt=performance.now();S.playStartedEpoch=(performance.timeOrigin+S.startedAt)/1000;snapshot();return null;
 }
 if(c==='settings_save'){updateSettings(a.draft);return settings}
 if(c==='session_toggle_paused'){session.isPaused=!session.isPaused;snapshot();return null}
 if(c==='session_stop'){session.isActive=false;session.status={kind:'idle'};snapshot();return null}
 if(c==='session_clear_subtitles'){session.subtitles={source:{text:'',isFinal:false},translation:{text:'',isFinal:false},history:[]};historyIds.length=0;snapshot();return null}
 if(c==='overlay_set_collapsed'){session.isOverlayCollapsed=a.collapsed;snapshot();return null}
 if(c==='overlay_set_locked'){updateSettings({isOverlayLocked:a.locked});return null}
 if(c==='overlay_control_state')return S.controlMode;
 if(c==='overlay_popover_toggle'||c==='overlay_popover_hide'){
  if(S.controlMode==='hidden')return null;
  S.controlMode=c==='overlay_popover_hide'?'island':S.controlMode==='island'?'panel':'island';
  document.getElementById('overlay-control').style.height=S.controlMode==='panel'?'315px':'34px';
  document.getElementById('overlay-control').style.top=S.controlMode==='panel'?'270px':'772px';
  emit('overlay-control-mode',S.controlMode);return null;
 }
 if(c==='overlay_control_set_panel_height'){if(S.controlMode==='panel')document.getElementById('overlay-control').style.height=a.height+'px';return null}
 if(c==='app_show_settings'){showWindow('settings');return null}
 if(c==='tray_panel_hide'){hideWindow('tray-panel');return null}
 if(c==='overlay_show'){showWindow('overlay');syncPresentation();return null}
 if(['overlay_move_start','overlay_pointer_state','overlay_show_menu'].includes(c))return null;
 if(c==='plugin:app|version')return '1.3.8';
 throw Error('Unexpected documentation command: '+c);
};
document.getElementById('mimi-menu').onclick=()=>showWindow('tray-panel');
video.addEventListener('pointerdown',()=>hideWindow('tray-panel'));
showWindow('tray-panel');
