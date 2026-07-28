# Low-Latency Translation Design

## Goal and mode selection

mimi should show useful Japanese-to-Chinese subtitles sooner and keep updating while a sentence is still being spoken. The default **Low Latency** mode separates speech recognition from translation: Alibaba Cloud Qwen realtime ASR produces partial Japanese text, then Qwen-MT-Lite translates the newest partial text. The existing Qwen LiveTranslate connection remains available as **High Quality** mode for users who prefer its end-to-end sentence translation.

The selected mode is stored in app settings. It is chosen before a session starts and is disabled while listening so a running WebSocket is never silently replaced underneath the audio capture pipeline.

## Low-latency data flow

System audio remains 16 kHz mono PCM. In low-latency mode it is sent to `qwen3-asr-flash-realtime` with Alibaba Cloud's low-latency VAD preset: threshold 0.0 and a 400 ms silence boundary. Interim ASR events combine confirmed `text` with tentative `stash`, so the source subtitle moves continuously instead of waiting for a complete sentence.

Translation requests are debounced briefly to avoid sending one request for every character, then Qwen-MT-Lite's incremental stream is rendered as it arrives. A generation number makes stale responses harmless: only the response belonging to the newest recognized text can update the overlay. Final ASR text is translated immediately and becomes the final subtitle. Recognition keeps running if an individual draft translation request fails.

Both implementations emit the existing `LiveTranslateServerEvent` values through a shared `TranslationClient` facade. This keeps the subtitle reducer, reconnect watchdog, audio capture, and overlay behavior unchanged.

## Reliability and verification

The realtime ASR socket retains the existing ping and reconnect health checks. Stopping cancels pending translation work before closing the socket. Tests cover the ASR endpoint and session request, partial/final event decoding, Qwen-MT request/response JSON, and translation-mode defaults. Verification includes the complete core test suite, a warnings-as-errors release build, stable-signed app packaging, and a live Japanese-video check. The 1–2 second first-subtitle latency is a target to validate, not a guaranteed service SLA.
