(() => {
  if(window===parent||!location.pathname.startsWith('/app/'))return;
  localStorage.setItem('mimi-ui-language','zh');
  const kind=new URLSearchParams(location.search).get('window')||'tray-panel';
  const callbacks=new Map(),listeners=new Map();let serial=0;
  window.__emit=(event,payload)=>{for(const [id,handler] of listeners.get(event)||[])callbacks.get(handler)?.({event,id,payload});};
  window.__TAURI_INTERNALS__={metadata:{currentWindow:{label:kind},currentWebview:{label:kind,windowLabel:kind}},
    transformCallback:(fn,once=false)=>{let id=++serial;callbacks.set(id,once?(v)=>{callbacks.delete(id);fn(v)}:fn);return id},
    unregisterCallback:id=>callbacks.delete(id),runCallback:(id,d)=>callbacks.get(id)?.(d),convertFileSrc:p=>p,
    invoke:async(c,a={})=>{
      if(c==='plugin:event|listen'){const id=++serial,list=listeners.get(a.event)||[];list.push([id,a.handler]);listeners.set(a.event,list);return id}
      if(c==='plugin:event|unlisten'){listeners.set(a.event,(listeners.get(a.event)||[]).filter(x=>x[0]!==a.eventId));return null}
      if(c.startsWith('plugin:event|emit')){parent.emit(a.event,a.payload);return null}
      if(c.startsWith('plugin:window|')){
        let op=c.split('|')[1];
        if(op==='scale_factor')return 1;
        if(op==='inner_size')return {width:innerWidth,height:innerHeight};
        if(op==='outer_position'||op==='inner_position')return {x:0,y:0};
        if(op==='close'||op==='hide'){parent.hideWindow(kind);return null}
        if(op.startsWith('is_'))return false;
        return null;
      }
      if(c.startsWith('plugin:webview|'))return null;
      return parent.invoke(kind,c,a);
    }};
  window.__TAURI_EVENT_PLUGIN_INTERNALS__={unregisterListener:()=>{}};
  document.addEventListener('mousemove',e=>parent.pointer(kind,e.clientX,e.clientY));
  document.addEventListener('keydown',e=>parent.handleShortcut(e));
})();
