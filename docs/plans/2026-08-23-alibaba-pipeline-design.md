# Alibaba translation pipeline

## Scope

This record describes the Alibaba Cloud backend that ships with mimi. It is a
provider-specific implementation behind the typed provider facade; OpenAI
Realtime has its own protocol and audio-format contract.

All Alibaba clients use the shared DashScope endpoints and one provider/profile
scoped API key. No Workspace ID participates in configuration or IPC.

## Modes

- **Low latency** uses the DashScope LiveTranslate realtime WebSocket. Automatic
  source detection omits the language hint and routes here unless the user has
  explicitly selected Turbo.
- **High quality** uses Audio 3.0 streaming ASR and translates confirmed source
  chunks with `qwen-mt-plus`. It requires an explicit source language.
- **Turbo** uses the same Audio 3.0 recognizer with `qwen-mt-flash`, a 500 ms
  stable-draft delay, a 2 s maximum wait, and a 12-character long-tail
  threshold. Turbo remains Turbo with automatic source detection.
- **Original subtitles** keep the strongest recognition path available for the
  selected source/mode and do not call machine translation.

Provider capabilities are the authority for source languages, target languages,
translation modes, and the 16 kHz mono PCM input format. Unsupported persisted
preferences are normalized before a session starts and are never silently
accepted by the runtime.

## Durable subtitle rules

Audio 3.0 drafts are replaceable previews. `ASRDraftCommitter` commits complete
sentences at punctuation boundaries and retains an incomplete tail. A bounded
maximum-wait path commits long uninterrupted speech so subtitles continue to
flow.

Server finals remain authoritative:

- an exact duplicate is ignored;
- a final that structurally covers the last provisional chunk replaces it;
- overlapping suffixes are stripped before a new chunk is appended; and
- revoke and replacement events are applied together so history does not blink
  or duplicate.

The final-translation queue preserves server finals and session-finish work.
When backlog grows, replaceable stable-draft work may be coalesced or shed, but
confirmed provider output is not discarded. Draft and pending buffers stay
bounded.

## Translation quality

`QwenMTDomainHint::spoken_dialogue` and its filler glossary are wire assets.
They preserve particles, vocalizations, tone, deliberate repetition, and
explicit dialogue, and they require translation-only output. Do not casually
rewrite these strings.

High-quality finals use recent source/translation pairs as bounded translation
memory. For automatic recognition, a detected source language is pinned when
available. Memory is context only: remembered lines must not be repeated in the
new output.

## Lifecycle and recovery

Session state owns an immutable resolved provider configuration. Pause stops
capture and transport work without clearing durable subtitle history; resume
creates a new guarded generation. Manual stop, provider failure, health checks,
and automatic recovery all use lifecycle generations so stale async work cannot
publish into a newer session.

Health checks use bounded ping/pong timeouts rather than audio-volume inference.
Stop drains already queued audio for a finite interval, closes the provider,
and only accepts provider-confirmed atomic tail pairs while stopping.

## Verification

The Rust suite covers protocol payloads, mode dispatch, sentence commits,
deduplication/replacement, queue bounds, retry ownership, stale generations,
pause/resume, recovery, and bounded shutdown. Run `./scripts/check.sh` after any
pipeline change. Real-provider checks use credentials already stored in the OS
credential store and must not log audio or subtitle content.
