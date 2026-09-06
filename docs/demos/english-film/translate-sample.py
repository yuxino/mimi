"""Opt-in documentation experiment, not an application credential path.

Only the attributed public 31-second film excerpt is sent. The provided key is
received encrypted for this single disposable runner, held in process memory,
and never exported. No microphone, system audio, or user file is accessed.
"""
import asyncio
import base64
import hashlib
import json
import logging
import os
from pathlib import Path
import time
import uuid

import aiohttp
from cryptography.hazmat.primitives import hashes, serialization
from cryptography.hazmat.primitives.asymmetric import padding
from websockets.asyncio.client import connect

OUTPUT = Path('film-translation-review')
OUTPUT.mkdir(exist_ok=True)
REPORT = {'success': False, 'api_sessions': 0, 'source_seconds': 31,
          'provider': 'Alibaba Cloud', 'model': 'qwen3.5-livetranslate-flash-realtime',
          'native_audio_capture': False, 'event_counts': {}}

async def receive_key():
    path = Path(os.environ['RUNNER_TEMP']) / 'film-handoff-private.pem'
    private = serialization.load_pem_private_key(path.read_bytes(), password=None)
    path.unlink()
    run = os.environ['GITHUB_RUN_ID']
    url = ('https://api.github.com/repos/yuxino/mimi/contents/'
           f'docs/demos/english-film/envelope-{run}.json?ref=docs/english-video-demo')
    async with aiohttp.ClientSession(timeout=aiohttp.ClientTimeout(total=12)) as client:
        for _ in range(90):
            async with client.get(url, headers={'Authorization': 'Bearer ' + os.environ['GH_TOKEN']},
                                  allow_redirects=False) as response:
                if response.status == 200:
                    document = await response.json()
                    envelope = json.loads(base64.b64decode(document['content']))
                    if str(envelope['run']) != run:
                        raise ValueError('Incorrect handoff run')
                    return private.decrypt(base64.b64decode(envelope['ciphertext']),
                        padding.OAEP(mgf=padding.MGF1(hashes.SHA256()),
                                     algorithm=hashes.SHA256(), label=run.encode())).decode()
                if response.status != 404:
                    raise RuntimeError('Handoff unavailable')
            await asyncio.sleep(3)
    raise TimeoutError('Handoff expired')

async def main():
    prepared = Path('prepared')
    movie = prepared / 'sintel-excerpt.mp4'
    source = json.loads((prepared / 'source.json').read_text())
    if hashlib.sha256(movie.read_bytes()).hexdigest() != source['sha256']:
        raise ValueError('Film source changed')
    pcm = (prepared / 'speech.pcm').read_bytes()
    if len(pcm) != 31 * 16000 * 2:
        raise ValueError('Unexpected sample-audio length')
    REPORT['source_sha256'] = source['sha256']
    REPORT['pcm_sha256'] = hashlib.sha256(pcm).hexdigest()
    key = await receive_key()
    events = []
    configured = asyncio.Event()
    started = None
    counts = REPORT['event_counts']
    try:
        REPORT['api_sessions'] = 1
        endpoint = ('wss://dashscope.aliyuncs.com/api-ws/v1/realtime'
                    '?model=qwen3.5-livetranslate-flash-realtime')
        async with connect(endpoint, additional_headers={'Authorization': 'Bearer ' + key},
                           open_timeout=20, max_size=2_000_000) as socket:
            await socket.send(json.dumps({'event_id': str(uuid.uuid4()), 'type': 'session.update',
                'session': {'modalities': ['text'], 'sample_rate': 16000,
                'input_audio_format': 'pcm', 'translation': {'language': 'zh'},
                'input_audio_transcription': {'model': 'qwen3-asr-flash-realtime', 'language': 'en'}}}))
            async def send_sample():
                nonlocal started
                await asyncio.wait_for(configured.wait(), timeout=20)
                started = time.monotonic()
                for index in range(0, len(pcm), 3200):
                    await socket.send(json.dumps({'event_id': str(uuid.uuid4()),
                        'type': 'input_audio_buffer.append',
                        'audio': base64.b64encode(pcm[index:index+3200]).decode()}))
                    await asyncio.sleep(max(0, started + (index + 3200)/32000 - time.monotonic()))
                await socket.send(json.dumps({'event_id': str(uuid.uuid4()), 'type': 'session.finish'}))
            sender = asyncio.create_task(send_sample())
            try:
                async with asyncio.timeout(90):
                    async for message in socket:
                        data = json.loads(message)
                        kind = data.get('type', 'unknown')
                        counts[kind] = counts.get(kind, 0) + 1
                        if kind == 'error':
                            REPORT['error_code'] = data.get('error', {}).get('code', 'provider_error')
                            raise RuntimeError('Provider rejected sample')
                        if kind == 'session.updated':
                            configured.set()
                        if kind in ['conversation.item.input_audio_transcription.text',
                                    'conversation.item.input_audio_transcription.completed',
                                    'conversation.item.created', 'response.text.text', 'response.text.done']:
                            if started is None:
                                continue
                            allowed = ['type','text','stash','transcript','item_id','response_id','previous_item_id']
                            event = {name:data[name] for name in allowed if name in data}
                            if kind == 'conversation.item.created':
                                item = data.get('item', {})
                                event['item'] = {name:item[name] for name in ['id','role'] if name in item}
                            event['received_at'] = round(time.monotonic()-started, 6)
                            events.append(event)
                        if kind == 'session.finished':
                            REPORT['finished'] = True
                            break
            finally:
                if not sender.done():
                    sender.cancel()
                await asyncio.gather(sender, return_exceptions=True)
        final_texts = [e['text'] for e in events if e['type']=='response.text.done' and e.get('text')]
        if not REPORT.get('finished') or not final_texts:
            raise RuntimeError('No completed sample translation')
        REPORT['final_translations'] = len(final_texts)
        REPORT['translation_characters'] = sum(map(len, final_texts))
        REPORT['first_translation_at'] = next(e['received_at'] for e in events if e['type'].startswith('response.text.'))
        REPORT['last_translation_at'] = max(e['received_at'] for e in events if e['type'].startswith('response.text.'))
        # This is a clearly separated documentation replay input, not app diagnostics.
        payload = json.dumps({'source': source, 'events': events, 'timing': 'original service receive offsets'},
                             ensure_ascii=False, indent=2).encode()
        (OUTPUT/'licensed-sample-response.json').write_bytes(payload)
        REPORT['response_file_sha256'] = hashlib.sha256(payload).hexdigest()
        REPORT['success'] = True
    finally:
        key = None

if __name__ == '__main__':
    logging.disable(logging.CRITICAL)
    try:
        asyncio.run(main())
    except Exception as error:
        REPORT['failure_class'] = type(error).__name__
    (OUTPUT/'report.json').write_text(json.dumps(REPORT, indent=2)+'\n', encoding='utf-8')
    if not REPORT['success']:
        raise SystemExit('Sample translation did not complete; no credential or raw error is logged')
