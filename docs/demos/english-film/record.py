"""Capture the unchanged Mimi renderer over a licensed film, without any credentials.
Provider event times are replayed exactly from the separately verified experiment.
This is a documentation harness, not a native system-audio acceptance test.
"""
import asyncio
import base64
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import threading
import time
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from playwright.async_api import async_playwright

ROOT = Path(__file__).resolve().parent
PREPARED = Path(os.environ.get('MIMI_PREPARED', 'prepared')).resolve()
INPUT = Path(os.environ.get('MIMI_RESPONSE', 'translation/licensed-sample-response.json')).resolve()
OUT = Path('english-film-review').resolve()
OUT.mkdir(exist_ok=True)
FRAMES = OUT/'frames'
FRAMES.mkdir(exist_ok=True)

def sha(p): return hashlib.sha256(p.read_bytes()).hexdigest()
def ff(args): subprocess.run(['ffmpeg','-v','error','-y',*args],check=True)
def probe(p): return json.loads(subprocess.check_output(['ffprobe','-v','error','-show_format','-show_streams','-of','json',str(p)],text=True))

class Handler(SimpleHTTPRequestHandler):
    def log_message(self,*args): pass
    def translate_path(self,path):
        path=path.split('?')[0]
        if path.startswith('/assets/'): return str(PREPARED/'dist'/path.lstrip('/'))
        if path=='/sample.mp4': return str(PREPARED/'sintel-excerpt.mp4')
        if path=='/responses.json': return str(INPUT)
        return str(ROOT/path.lstrip('/'))
    def do_GET(self):
        if self.path.split('?')[0] in ['/app/','/app/index.html']:
            text=(PREPARED/'dist/index.html').read_text().replace('<head>','<head><script src="/bridge.js"></script>',1)
            data=text.encode();self.send_response(200)
            self.send_header('Content-Type','text/html; charset=utf-8')
            self.send_header('Content-Length',str(len(data)));self.end_headers();self.wfile.write(data);return
        super().do_GET()

async def main():
    response=json.loads(INPUT.read_text())
    assert sha(INPUT)=='214f8bf43d0696a4fcf01f35a61e425badd60858803d7d52a875e3b8fb90d861'
    assert sha(PREPARED/'sintel-excerpt.mp4')==response['source']['sha256']
    report={'success':False,'source_commit':(PREPARED/'source-commit.txt').read_text().strip(),
        'native':False,'new_provider_calls_in_capture':0,'response_capture_run':34040075430,
        'response_artifact':9991423013,'response_sha256':sha(INPUT),'single_take':True,
        'dimensions':[1920,1080],'errors':[],'http_errors':[],'checks':{},'scenes':[]}
    server=ThreadingHTTPServer(('127.0.0.1',8819),Handler)
    threading.Thread(target=server.serve_forever,daemon=True).start()
    frames=[]
    async with async_playwright() as pw:
        browser=await pw.chromium.launch(channel='chrome')
        context=await browser.new_context(viewport={'width':1920,'height':1080},locale='zh-CN',color_scheme='dark')
        await context.route('**/*',lambda r:r.continue_() if r.request.url.startswith('http://127.0.0.1:8819/') else r.abort())
        page=await context.new_page();page.set_default_timeout(10000)
        page.on('pageerror',lambda error:report['errors'].append(str(error)))
        page.on('response',lambda r:report['http_errors'].append({'url':r.url,'status':r.status}) if r.status>=400 else None)
        cdp=await context.new_cdp_session(page)
        async def receive(event):
            try:
                name=f'{len(frames):06d}.jpg'
                (FRAMES/name).write_bytes(base64.b64decode(event['data']))
                frames.append({'file':name,'t':event['metadata']['timestamp']})
            finally:
                await cdp.send('Page.screencastFrameAck',{'sessionId':event['sessionId']})
        cdp.on('Page.screencastFrame',receive)
        async def shot(name):
            await page.screenshot(path=str(OUT/(name+'.png')))
            report['scenes'].append({'name':name,'epoch':time.time(),'movie_time':await page.locator('#movie').evaluate('(v)=>v.currentTime')})
        async def click(locator):
            await locator.wait_for(state='visible')
            bounds=await locator.bounding_box()
            await page.mouse.move(bounds['x']+bounds['width']/2,bounds['y']+bounds['height']/2,steps=15)
            await page.wait_for_timeout(180)
            await locator.click()
        try:
            await page.goto('http://127.0.0.1:8819/stage.html')
            await page.locator('#movie').evaluate('(v)=>v.readyState>=2?true:new Promise(r=>v.addEventListener("loadeddata",()=>r(true),{once:true}))')
            tray=page.frame_locator('#tray-panel')
            await tray.get_by_role('button',name='开始',exact=True).wait_for()
            await page.screenshot(path=str(OUT/'initial.png'))
            await cdp.send('Page.startScreencast',{'format':'jpeg','quality':98,'maxWidth':1920,'maxHeight':1080,'everyNthFrame':1})
            await page.wait_for_timeout(900)
            await click(tray.get_by_role('button',name='开始',exact=True))
            await page.wait_for_timeout(500)
            await page.mouse.click(1360,300)
            overlay=page.frame_locator('#overlay')
            await page.wait_for_function('document.getElementById("movie").currentTime>=6')
            assert await overlay.locator('body').inner_text()
            assert await overlay.locator('[data-presentation="background-blend"]').count()==0
            report['checks']['ordinary_overlay']=True
            await shot('01-ordinary-subtitles')
            await page.wait_for_function('document.getElementById("movie").currentTime>=12')
            await overlay.locator('body').hover()
            await click(overlay.get_by_test_id('toggle-immersive-mode'))
            blend=overlay.locator('[data-presentation="background-blend"]')
            await blend.wait_for()
            assert await blend.evaluate('(e)=>getComputedStyle(e).backgroundColor')=='rgba(0, 0, 0, 0)'
            report['checks']['blend_transparent']=True
            await page.mouse.move(1810,980,steps=15)
            await page.wait_for_function('document.getElementById("movie").currentTime>=18.8')
            await shot('02-blended-subtitles')
            await page.wait_for_function('document.getElementById("movie").ended',timeout=22000)
            report['checks']['film_played_to_end']=True
            await page.wait_for_timeout(500)
            await shot('03-blended-result')
            report['fast_start']=time.time()
            control=page.frame_locator('#overlay-control')
            await click(control.get_by_role('button').first)
            await control.get_by_role('dialog').wait_for()
            await page.wait_for_timeout(2000)
            await shot('04-display-controls')
            await click(control.get_by_role('switch',name='沉浸模式',exact=True))
            await blend.wait_for(state='detached')
            await overlay.locator('body').hover()
            await click(overlay.get_by_role('button',name='暂停',exact=True))
            assert await page.evaluate('demoState.session.isPaused')
            await page.wait_for_timeout(1200)
            await click(overlay.get_by_role('button',name='继续',exact=True))
            assert not await page.evaluate('demoState.session.isPaused')
            report['checks']['pause_resume_controls']=True
            await click(overlay.get_by_test_id('toggle-immersive-mode'))
            await blend.wait_for()
            await page.mouse.move(1810,980,steps=15)
            report['fast_end']=time.time()
            await page.wait_for_timeout(3000)
            await shot('05-final-immersive')
            report['checks']['finished_in_immersive_mode']=await page.evaluate('demoState.settings.subtitleBlendsWithBackground')
            report['state']=await page.evaluate('({calls:demoState.calls,timeline:demoState.timeline,playStartedEpoch:demoState.playStartedEpoch,eventCount:demoState.index,historyCount:demoState.session.subtitles.history.length})')
            report['success']=not report['errors'] and not report['http_errors']
        except Exception as error:
            report['failure']=str(error)[:1000]
            await page.screenshot(path=str(OUT/'failure.png'))
            report['ui']=[{'url':f.url,'text':await f.locator('body').inner_text(),
                'controls':await f.evaluate("[...document.querySelectorAll('button')].map(b=>({text:b.innerText,label:b.getAttribute('aria-label'),title:b.title,role:b.getAttribute('role')}))")} for f in page.frames]
        finally:
            await cdp.send('Page.stopScreencast');await page.wait_for_timeout(150)
            report['end_epoch']=time.time()
            await context.close();await browser.close()
    server.shutdown()
    report['frames']=frames
    (OUT/'capture-report.json').write_text(json.dumps(report,ensure_ascii=False,indent=2),encoding='utf-8')
    if not report['success']:raise SystemExit('Capture did not pass; inspect documentation-only diagnostics')
    await asyncio.to_thread(encode,report,response)

def encode(r,response):
    frames=r['frames'];assert len(frames)>300
    assert all(b['t']>a['t'] for a,b in zip(frames,frames[1:]))
    fast=[r['fast_start'],r['fast_end']]
    def dt(a,b):return (b-a)-max(0,min(b,fast[1])-max(a,fast[0]))*.9
    def at(t):return dt(frames[0]['t'],t)
    lines=['ffconcat version 1.0']
    for index,frame in enumerate(frames):
        end=frames[index+1]['t'] if index+1<len(frames) else r['end_epoch']
        length=dt(frame['t'],end);assert length>0
        lines.extend(["file '"+str(FRAMES/frame['file'])+"'",'option framerate 1000',f'duration {length:.9f}'])
    lines.append("file '"+str(FRAMES/frames[-1]['file'])+"'")
    concat=OUT/'timeline.ffconcat';concat.write_text('\n'.join(lines)+'\n')
    delivery=Path('english-film-delivery').resolve();delivery.mkdir(exist_ok=True)
    raw=OUT/'silent.mp4'
    ff(['-f','concat','-safe','0','-i',str(concat),'-vf','fps=30,setsar=1','-an','-c:v','libx264','-preset','fast','-crf','17','-pix_fmt','yuv420p','-threads','2',str(raw)])
    duration=float(probe(raw)['format']['duration'])
    audio_delay=round(at(r['state']['playStartedEpoch'])*1000)
    assert 0<=audio_delay<5000 and 32<duration<60
    ff(['-i',str(raw),'-i',str(PREPARED/'sintel-excerpt.mp4'),'-map','0:v:0','-map','1:a:0','-c:v','copy','-af',f'adelay={audio_delay}:all=1,apad','-c:a','aac','-b:a','192k','-t',str(duration),'-movflags','+faststart',str(delivery/'demo.mp4')])
    ff(['-i',str(delivery/'demo.mp4'),'-filter_complex','fps=10,scale=1440:810:flags=lanczos,split[a][b];[a]palettegen=max_colors=160[p];[b][p]paletteuse=dither=bayer:bayer_scale=4:diff_mode=rectangle','-loop','0',str(delivery/'preview.gif')])
    poster_time=at(next(s['epoch'] for s in r['scenes'] if s['name']=='02-blended-subtitles'))
    ff(['-ss',str(poster_time),'-i',str(delivery/'demo.mp4'),'-frames:v','1',str(delivery/'poster.png')])
    for name in ['demo.mp4','preview.gif']:ff(['-i',str(delivery/name),'-f','null','-'])
    info=probe(delivery/'demo.mp4')
    video=next(s for s in info['streams'] if s['codec_type']=='video')
    assert [video['width'],video['height']]==[1920,1080]
    assert any(s['codec_type']=='audio' for s in info['streams'])
    assert abs(float(probe(delivery/'preview.gif')['format']['duration'])-duration)<.2
    p={key:r[key] for key in ['source_commit','native','new_provider_calls_in_capture','response_capture_run','response_artifact','response_sha256','single_take','dimensions','checks']}
    p.update({'duration':duration,'sha256':sha(delivery/'demo.mp4'),'capture_run':os.environ.get('GITHUB_RUN_ID'),
        'source':response['source'],'dialogue_speed':1,'control_actions_speed':10,'audio_delay_ms':audio_delay,
        'scene_cuts':0,'scenes':[{'name':s['name'],'at':round(at(s['epoch']),3)} for s in r['scenes']],
        'response_events_consumed':r['state']['eventCount'],'history_count':r['state']['historyCount'],
        'disclosure':'Actual unchanged Mimi frontend over a licensed Sintel excerpt. Real provider responses from run 34040075430 are replayed with original receive timing. No subtitle correction or latency shortening. The recorder substitutes native IPC and does not test OS audio capture or keychain integration. Original English soundtrack is synchronized to playback; settings actions after the film ends play at 10x. No API credential is present during recording.'})
    p['media']={name:{'bytes':(delivery/name).stat().st_size,'sha256':sha(delivery/name)} for name in ['demo.mp4','preview.gif','poster.png']}
    assert all(meta['bytes']<60_000_000 for meta in p['media'].values())
    (delivery/'provenance.json').write_text(json.dumps(p,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
    print(json.dumps({'duration':duration,'media':p['media'],'checks':p['checks']},ensure_ascii=False))

if __name__=='__main__':asyncio.run(main())
