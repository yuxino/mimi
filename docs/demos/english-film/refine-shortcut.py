"""Documentation-only correction: faithfully mirror native immersive presentation."""
from pathlib import Path
import os

root = Path(os.environ.get('MIMI_CAPTURE_ROOT', 'docs/demos/english-film'))

def patch(name, changes):
    path = root / name
    text = path.read_text(encoding='utf-8')
    for old, new in changes:
        if text.count(old) != 1:
            raise RuntimeError(f'Review required: {name}, target count {text.count(old)}: {old[:70]}')
        text = text.replace(old, new)
    path.write_text(text, encoding='utf-8')

presentation = '''// Mirror OverlayControlWindowManager::sync_presentation and native click-through.
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
'''
patch('stage.js', [
    ('fontSize:22', 'fontSize:16'),
    ("controlMode:'island'", "controlMode:'hidden'"),
    ("function snapshot(){emit('session-state',session)}", presentation+"function snapshot(){syncPresentation();emit('session-state',session)}"),
    ("function updateSettings(draft){Object.assign(settings,draft);S.timeline.push({kind:'settings',at:elapsed(),values:draft});emit('settings-changed',settings)}", "function updateSettings(draft){Object.assign(settings,draft);if(settings.subtitleBlendsWithBackground)session.isOverlayCollapsed=false;S.timeline.push({kind:'settings',at:elapsed(),values:draft});syncPresentation();emit('settings-changed',settings);snapshot()}"),
    ("  S.controlMode=c==='overlay_popover_hide'?'island':S.controlMode==='island'?'panel':'island';", "  if(S.controlMode==='hidden')return null;\n  S.controlMode=c==='overlay_popover_hide'?'island':S.controlMode==='island'?'panel':'island';"),
    ("?'220px':'720px'", "?'270px':'772px'"),
    ("if(c==='overlay_show'){showWindow('overlay');return null}", "if(c==='overlay_show'){showWindow('overlay');syncPresentation();return null}"),
])
patch('bridge.js', [
    ("  document.addEventListener('mousemove',e=>parent.pointer(kind,e.clientX,e.clientY));", "  document.addEventListener('mousemove',e=>parent.pointer(kind,e.clientX,e.clientY));\n  document.addEventListener('keydown',e=>parent.handleShortcut(e));"),
])
patch('stage.html', [
    ("#overlay{left:480px;top:706px;width:640px;height:136px;z-index:3}", "#overlay{left:480px;top:748px;width:640px;height:124px;z-index:3}"),
    ("#overlay-control{left:492px;top:720px;width:272px;height:34px;z-index:6}", "#overlay-control{left:507px;top:772px;width:236px;height:30px;z-index:6}"),
    ('<video id="movie"', '<video tabindex="0" id="movie"'),
])
patch('record.py', [
    ("browser=await pw.chromium.launch(channel='chrome')", "browser=await pw.chromium.launch(**({'executable_path':os.environ['CHROME_BIN']} if os.environ.get('CHROME_BIN') else {'channel':'chrome'}))"),
    ("            await page.wait_for_function('document.getElementById(\"movie\").currentTime>=6')", "            await page.wait_for_function('document.getElementById(\"movie\").currentTime>=1.2')"),
    ("            assert await overlay.locator('body').inner_text()", "            assert await page.evaluate('demoState.settings.fontSize===16')\n            await page.frame_locator('#overlay-control').get_by_role('button').first.wait_for()\n            report['checks']['subtitle_font_size']=16"),
    ("            await page.wait_for_function('document.getElementById(\"movie\").currentTime>=12')\n            await overlay.locator('body').hover()\n            await click(overlay.get_by_test_id('toggle-immersive-mode'))", "            await page.locator('#movie').focus()\n            await page.keyboard.press('Control+Shift+M')"),
    ("            report['checks']['blend_transparent']=True", """            report['checks']['blend_transparent']=True
            await page.locator('#overlay-control').wait_for(state='hidden')
            assert await page.frame_locator('#overlay-control').locator('body').inner_text()==''
            assert await overlay.get_by_role('button').count()==0
            assert await page.locator('#overlay').evaluate('(e)=>getComputedStyle(e).pointerEvents')=='none'
            # A delayed dismissal/broadcast must not resurrect the status island.
            await page.evaluate('invoke("overlay-control","overlay_popover_hide")')
            await page.wait_for_timeout(350)
            assert await page.evaluate('demoState.controlMode==="hidden"')
            assert await page.locator('#overlay-control').is_hidden()
            report['checks']['immersive_controls_hidden']=True
            report['checks']['immersive_canvas_click_through']=True
            report['checks']['late_dismiss_keeps_hidden']=True
            report['immersive_started_epoch']=time.time()"""),
    ("            await shot('02-blended-subtitles')", "            await shot('02-blended-subtitles')\n            assert await page.locator('#overlay-control').is_hidden()\n            assert '16px' in await blend.locator('span').evaluate_all('(nodes)=>nodes.map(n=>getComputedStyle(n).fontSize)')"),
    ("            control=page.frame_locator('#overlay-control')\n            await click(control.get_by_role('button').first)\n            await control.get_by_role('dialog').wait_for()\n            await page.wait_for_timeout(2000)\n            await shot('04-display-controls')\n            await click(control.get_by_role('switch',name='沉浸模式',exact=True))", "            await page.locator('#movie').focus()\n            await page.keyboard.press('Control+Shift+M')\n            await page.locator('#overlay-control').wait_for(state='visible')\n            await page.wait_for_timeout(800)\n            await shot('04-shortcut-restore-controls')"),
    ("            await click(overlay.get_by_test_id('toggle-immersive-mode'))", "            await page.locator('#movie').focus()\n            await page.keyboard.press('Control+Shift+M')"),
    ("            await page.wait_for_timeout(3000)", "            await page.wait_for_timeout(4000)"),
    ("            report['checks']['finished_in_immersive_mode']=await page.evaluate('demoState.settings.subtitleBlendsWithBackground')", """            report['checks']['finished_in_immersive_mode']=await page.evaluate('demoState.settings.subtitleBlendsWithBackground')
            assert await page.locator('#overlay-control').is_hidden()
            assert await overlay.get_by_role('button').count()==0
            shortcuts=await page.evaluate('demoState.calls.filter(c=>c.command==="immersive_shortcut")')
            assert len(shortcuts)==3 and all(x['key']=='Control+Shift+M' for x in shortcuts)
            report['checks']['keyboard_immersive_toggles']=3
            report['checks']['finished_with_subtitles_only']=True"""),
    ("    preview_start=audio_delay/1000+10.5", "    preview_start=audio_delay/1000+15.0"),
    ("    poster_time=at(next(s['epoch'] for s in r['scenes'] if s['name']=='03-blended-result'))", "    poster_time=at(next(s['epoch'] for s in r['scenes'] if s['name']=='02-blended-subtitles'))"),
    ("'scene_cuts':0,'scenes'", "'scene_cuts':0,'subtitle_font_size':16,'shortcut':'Control+Shift+M','immersive_started_at':at(r['immersive_started_epoch']),'scenes'"),
    ("The recorder substitutes native IPC and does not test OS audio capture or keychain integration.", "The recorder substitutes native IPC and the OS shortcut boundary; it does not test OS audio capture, global shortcut registration or keychain integration. Its presentation mirror now hides the complete control window and makes the subtitle canvas click-through when immersive, matching the native window manager. The real keyboard sequence changes that state; the production components render transparent text at font size 16."),
])
compile((root/'record.py').read_text(), str(root/'record.py'), 'exec')
