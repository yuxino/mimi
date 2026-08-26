# Mainstream Realtime Providers and Profile Deletion Design

**Date:** 2026-08-27

## Goal

Make destructive profile actions work reliably inside the packaged Tauri
WebView, and expand Mimi from two hard-coded services to the mainstream public
realtime translation APIs that can produce both source and translated text for
Mimi's Chinese, English, and Japanese subtitle workflow.

## Provider boundary

Built-in providers are typed adapters, not editable “OpenAI-compatible” URLs.
Each adapter must mirror the provider's current official authentication,
session setup, audio framing, transcript events, and graceful shutdown.

The provider registry includes:

- Alibaba Cloud Qwen LiveTranslate (existing dedicated continuous translation)
- OpenAI Realtime Translate (existing dedicated continuous translation)
- Google Gemini Live Translate (dedicated continuous translation, Preview)
- Azure OpenAI Realtime Translate (dedicated continuous translation)
- Volcano Engine Simultaneous Interpretation 2.0 (dedicated continuous translation)
- Tencent Cloud Realtime Speech Translation (dedicated continuous translation)
- Baidu Realtime Speech Translation (dedicated continuous translation)
- xAI Grok Voice (turn-based voice-agent translation, labelled as such)

AWS Nova 2 Sonic is intentionally excluded because its current official
language list does not include Chinese or Japanese. Generic chat-completions
compatibility is not accepted as evidence of Realtime Translation support.

## Credentials

Profiles continue to serialize only an ID, display name, and provider kind.
Credential material remains write-only and profile-scoped in the OS keychain.
The keychain value becomes a provider-specific credential envelope:

- one API key for Alibaba, OpenAI, Google, xAI, and Volcano Engine;
- Azure endpoint, translation deployment, transcription deployment, and API key;
- Tencent AppID, SecretID, and SecretKey;
- Baidu AppID and AppKey.

Legacy single-string Alibaba/OpenAI keychain entries remain readable. No
credential field is returned to the frontend, logged, or serialized to the
profile catalog.

## Deletion interaction

`window.confirm` is removed. The packaged macOS WebView does not provide a
reliable JavaScript confirmation panel, so the first destructive click opens a
small in-app confirmation row beside the affected profile or credential. The
row names the action and offers Cancel and a second explicit destructive
button. Changing profiles or completing an action clears the pending
confirmation. Backend deletion remains transactional and idempotent when a
keychain item is already absent.

## Runtime integration

Provider capabilities determine the permitted source languages, target
languages, translation mode, and capture sample rate. Dedicated clients map
provider-specific events into Mimi's existing bounded provider-event lanes:
source draft/final, translation draft/final, session ready/finished, and
content-free errors. Each client waits for its official setup acknowledgement
before audio, uses the documented frame size, and performs the documented
finish handshake so tail subtitles are not dropped.

xAI is presented distinctly because it uses server-VAD voice-agent turns and a
translation instruction rather than a dedicated continuous-translation model.

## Verification without paid credentials

Every adapter receives protocol fixture tests and a local mock WebSocket
lifecycle test covering authentication shape, setup ordering, audio frames,
transcript mapping, provider errors, and graceful finish. Credential encoding,
redaction, legacy compatibility, provider normalization, and profile deletion
receive unit tests. Frontend tests cover provider capability/credential models,
then the packaged UI-test app is used to create, edit, select, and delete
profiles through the real settings interface.

Public 401/403 handshakes may confirm current hosts but do not substitute for a
real account. The delivery report must explicitly distinguish protocol/mock UI
verification from unperformed paid-provider quality and latency testing.
