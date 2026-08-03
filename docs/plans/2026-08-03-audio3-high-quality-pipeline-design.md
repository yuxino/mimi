# Audio 3.0 high-quality subtitle pipeline

Mimi's high-quality mode uses Alibaba Cloud's dedicated
`qwen-audio-3.0-asr-flash-streaming` recognizer over the `/api-ws/v1/inference`
WebSocket endpoint. The client sends a `run-task` command, waits for
`task-started`, streams 16 kHz mono PCM as binary frames, and converts
`result-generated` sentences into Mimi's existing draft/final subtitle events.
It sends `finish-task` during shutdown and keeps the existing ping-based recovery.

The recognition request explicitly hints the selected language, enables semantic
punctuation for more accurate sentence boundaries, keeps the connection alive
during silence, and supplies short audiovisual-dialogue context. Mimi does not
enable the service's sensitive-word filter, so explicit dialogue and vocalizations
are not intentionally removed.

Only final ASR sentences are translated. The high-quality translator calls
`qwen-mt-plus`, preserving final-sentence order and using the existing spoken
dialogue domain prompt and recent translation memory. Original-language mode
uses the same Audio 3.0 recognizer but echoes the recognized sentence instead of
calling Qwen-MT. The legacy low-latency pipeline remains available internally,
but the visible high-quality workflow no longer depends on LiveTranslate.

Protocol tests cover endpoint construction, task commands, binary-result event
decoding, language hints, lifecycle failures, and the Plus model request. The
required gate remains `swift run mimi-core-tests`, followed by a release build and
app packaging before Mimi is restarted.
