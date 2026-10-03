# Local recognition and translation: issue #40 research

Status: proposal for discussion, not an accepted design or implemented feature.
Reviewed: 2026-09-27. Scope: Mimi desktop; Android needs separate resource and
lifecycle validation. Refs [#40](https://github.com/yuxino/mimi/issues/40).

## Recommendation

Local subtitles are technically feasible. Ship them only after measuring a
continuous system-audio session, not after a successful audio-file transcription.
Use one opt-in **Local** service profile backed by a supervised, persistent native
worker. Keep recognition (ASR) and translation (MT) separate behind that adapter;
keep the existing cloud paths intact. Ordinary users should not need Python,
Docker, a terminal, Ollama, or an API key.

First benchmark streaming Paraformer through sherpa-onnx as the Chinese/English
engineering baseline, and Qwen3-ASR-0.6B as the multilingual challenger. Compare
Qwen3-ASR-1.7B only where resources permit. Do not select a default from parameter
count or vendor throughput claims. For translation, compare Qwen2.5-7B-Instruct
with Qwen3-4B-Instruct-2507 using the same subtitle corpus, runtime, prompt and
quantization class. Translation quality and concurrent ASR/MT latency decide.

This PR adds research only: no model download, dependency, runtime, network
behavior, setting, or microphone capture. No inference benchmark was run; all
performance gates below are proposed targets, not measured results.

## What the discussion asks for

The issue requests recognition and translation without uploading audio or needing
an API key. The follow-up also asks for VAD tuning and voice separation. Those
are four distinct concerns:

- ASR produces source text; it does not by itself provide arbitrary translation.
- MT translates text; a text-only Qwen model cannot consume PCM audio.
- VAD detects speech and helps decide when an utterance ends; it does not remove
  music, identify the desired speaker, or guarantee transcription accuracy.
- Source separation adds another inference stage; its effect on recognition and
  latency must be measured separately.

“Qwen 0.6B/1.7B” likely means **Qwen3-ASR-0.6B/1.7B**. This is an interpretation
of the comment, not confirmation from its author. Use exact model IDs in tests.

## Current integration points

Source inspected at desktop main `d9e8b14996cf3ad6fa2590ca17ac37536af323ed`
(the proposal is additive):

| Existing boundary | Reuse / necessary change |
| --- | --- |
| `src-tauri/src/core/provider.rs` | Profiles and capabilities already select input sample rate. Add a local capability set derived from the installed model; do not advertise Japanese through a Chinese/English-only model. |
| `src-tauri/src/settings_store.rs`, `core/configuration.rs`, `core/credentials.rs` | Session configuration currently reads Keychain credentials and validates them. Resolve local configuration before any credential access; model readiness replaces key readiness. Never fabricate a dummy API key. |
| `src-tauri/src/clients/translation_client.rs` | Add a local adapter at the facade. Its constructor currently validates credentials before provider dispatch; local dispatch must bypass that deliberately. |
| `src-tauri/src/audio/send_pipeline.rs` | Reuse system capture/resampling and bounded ingress. The current full queue closes that generation; preserve explicit failure instead of silently losing speech. Inference must not run on the audio callback. |
| `src-tauri/src/clients/high_quality_client.rs` | Existing ASR/MT path separates replaceable drafts from a bounded, serial final queue. Reuse these ordering rules; do not repoint the cloud Qwen-MT protocol at a generic local chat API. |
| `src-tauri/src/clients/provider_events.rs` and `core/subtitle_reducer.rs` | Keep latest-only previews and authoritative final publication. Adapt worker output to the established events. |
| `src-tauri/src/session_manager.rs` | Own worker startup, cancellation, pause/resume, generation invalidation and shutdown. Local overload/OOM must not enter endless cloud-style reconnect loops. |
| `src/lib/providerCapabilities.ts`, `src/lib/providerCredentials.ts`, `src/windows/settings/` | Display model readiness rather than credential errors, expose only supported languages, and localize new strings through existing i18n. |

Avoid a rewrite of every cloud adapter into a general plugin framework. Introduce
small ASR/MT contracts inside the new local adapter first. Extract common policy
only when tests demonstrate identical behavior.

## Candidate assessment

Sources below were read on the review date. Runtime support is not proof of Mimi
packaging, language quality, or acceptable latency on a specific device.

| Candidate | Evidence and role | Main limitation to test |
| --- | --- | --- |
| Streaming Paraformer + sherpa-onnx | Upstream documents online Chinese/English and Chinese/Cantonese/English models, including int8 variants [1]. Native runtime is a useful lightweight baseline. | Not a Japanese/Korean solution. Verify model-specific license, endpoint behavior, quantization quality and native build per target. |
| Qwen3-ASR-0.6B / 1.7B | Official family supports multilingual ASR; official Python streaming currently requires vLLM [2]. | Server-oriented reference deployment is not a turnkey consumer desktop runtime. Measure partial stability, silence behavior and simultaneous translation. |
| antirez/qwen-asr | Community C implementation supports both sizes, live PCM stdin and chunked streaming with rollback; documents macOS Accelerate and Linux OpenBLAS [3]. | Not the official Qwen runtime; default streaming chunks are 2 seconds. CPU path and Windows packaging need their own acceptance. Emitted tokens are not automatically sentence-final subtitle events. |
| whisper.cpp | Native alternative with a real-time stream example [4]. Useful multilingual comparison if Qwen desktop packaging fails. | The example repeatedly decodes windows; it is not proof of a native incremental ASR contract. Revisions, overlap and deduplication need an adapter. Do not use its microphone demo as Mimi capture. |
| Qwen2.5-7B-Instruct | Exact model suggested for translation; official card lists Apache-2.0 [5]. Evaluate quantized local text translation. | General instruction following is not a subtitle-quality guarantee; memory, added commentary and MT backlog are risks. |
| Qwen3-4B-Instruct-2507 | Smaller text-model comparison; official card lists Apache-2.0 [6]. | Do not assume newer/smaller means better translations. Judge adequacy, names, negation and omissions blindly. |

FunASR is a toolkit/family, not one interchangeable model. Online Paraformer,
SenseVoice and Fun-ASR-Nano have different languages, decoding paths and runtime
requirements. The FunASR online SDK [7] is evidence for its documented online path,
not evidence that every FunASR checkpoint is a drop-in streaming backend.

For local MT, llama.cpp is the product runtime candidate [8]; a developer-owned
Ollama endpoint can speed experiments [9]. An OpenAI-compatible chat endpoint is
not OpenAI Realtime audio and must not be wired into that client unchanged.

**Memory planning, not device requirements:** nominal 4-bit weights alone cost
approximately parameter-count / 2 bytes: 4B ≈ 2 GB and 7B ≈ 3.5 GB (decimal).
Actual quantized artifacts include metadata/scales and sometimes higher-precision
tensors; peak resident memory also includes ASR, activations, KV cache, runtime,
GPU allocations and the OS. Do not promise 8 GB support or infer total memory
from a GGUF file size. Measure both models resident while video is playing.

## Why a managed native worker

| Approach | Advantage | Cost / decision |
| --- | --- | --- |
| Persistent Mimi-managed worker | No user-managed server; native libraries stay out of the UI process; one lifecycle and IPC contract | Recommended product shape. Requires per-platform packaging, signing, supervision and model management. |
| User-managed loopback services | Fastest way to compare ASR/MT engines in a lab | Research adapter only initially. Server installation, API differences and model availability burden ordinary users; loopback alone does not prove a server never forwards data. |
| Direct Rust/native linking | Fewer processes and potentially lower IPC cost | Possible later optimization; native crashes/GPU failures share the app process, and dependencies complicate every platform build. |

Proposed flow:

```text
existing system-audio capture -> bounded PCM ingress
  -> local worker: endpointing + ASR -> replaceable source draft
                                   -> committed source utterance
  -> bounded serial local MT queue -> final source/translation pair
  -> existing provider events / subtitle reducer / overlay
```

Start with a single worker executable that owns the selected ASR and MT engines
on separate bounded queues. Keep models loaded throughout the listening session.
Never launch a new process or reload weights per utterance. Whether engines need
separate child processes is a later crash-isolation/resource decision, not a
requirement to invent a multi-service architecture now.

Use private inherited pipes (or an equivalent private local IPC transport), a
version handshake, framed PCM and size-bounded messages. Include session
generation, utterance ID, revision, audio sample range, and partial/final status.
Reject old generations and duplicate finals. Worker stdout is protocol data, not
a diagnostic log; stderr/errors must not include recognized text, translations,
raw audio, prompts or filesystem details. Do not expose a public listening port.

Start the worker and validate models before opening system capture. On stop,
flush with a bounded deadline; cancel remaining work and reap the process. Pause
must not replay accumulated audio on resume. Model/language switching invalidates
the old generation before loading the replacement. Crash recovery may retry once
with backoff, then show an actionable error. Never silently switch to cloud.

Bound PCM by audio duration and bytes, utterances by duration and text length,
MT context by tokens, and outputs by tokens/bytes/time. On overload, discard stale
previews first. If confirmed work still exceeds the bounded queue, stop the local
session with a clear “cannot keep up” state; do not silently drop finals or grow
delay indefinitely. Offer original-only mode as an explicit user choice.

For the first MT experiment, translate only committed utterances and show source
drafts immediately. Use a short bounded context of previous committed pairs,
delimit the current utterance, request only its translation, and disable tools.
Treat spoken instructions as text to translate. Do not rewrite already confirmed
history when ASR revises a draft. Preview translation can be added only after
measuring spare capacity; speculative requests must never starve finals.

## Model delivery and user experience

Add a Local profile using the existing settings and overlay. Its setup should
show supported languages, download size, measured hardware guidance when known,
and separate recognition/translation readiness. First offer original subtitles;
require a translation model only when the selected target needs one. Keep the
service opt-in and existing profiles/defaults intact. No hidden automatic choice
of cloud translation when a local model is unavailable.

An explicit model download is a network operation, even if inference is local.
Separate it from listening: show source, exact revision, license and size before
download, then resume safely, verify every artifact hash and activate atomically.
Provide import for an already downloaded approved bundle, retry and delete.
Validate the full manifest, architecture/runtime compatibility, archive paths,
required tokenizer/projector assets and disk space. Never trust an imported hash
manifest as executable authority, execute model-supplied code, or enable arbitrary
remote-code loading. Runtime binaries use the app's signed update path, not the
model downloader. Do not update models silently in an active session.

After provisioning, a fully local session must work with external networking
blocked, without credential reads, content uploads, telemetry or fallback.
Disable runtime auto-download/check-in paths. Keep only bounded working buffers;
recording and transcript retention remain separately opt-in as today. Existing
explicit export remains the only content-saving path. Verify child processes as
well as the parent when testing offline behavior. A process boundary alone is
not a network sandbox.

## VAD and source separation

Start with the chosen ASR runtime's endpointing or evaluate Silero VAD [10]. Keep
speech probability threshold separate from end-of-speech silence and pre-roll:
raising a threshold can remove quiet speech, while more trailing silence delays
final text. “More accurate” is not a single meaningful slider.

Proposed advanced controls are speech sensitivity and sentence-end delay with a
reset to model-tested defaults. Preserve pre-roll and trailing audio, force a
bounded split for uninterrupted speech, and test soft consonants, breaths,
Japanese sentence endings, rapid turn-taking, music and silence. VAD settings
apply only to local sessions; do not imply control over a cloud provider's VAD.

Defer voice separation from the MVP. It may help music-heavy clips but also add
lookahead, CPU/GPU contention and speech distortion. It cannot be assumed to
select the intended speaker. Demucs is a useful offline comparison, but its
original repository is archived [11], not an obvious live desktop dependency.
Evaluate a streaming enhancer/separator only on paired raw/processed audio,
including quiet speech and overlapping voices; ship it default-off only if the
ASR gain survives an end-to-end latency/resource test.

## Reproducible experiment and decision gates

No performance or quality results are available in this PR. Record **not run**
until the following experiment is completed; do not substitute upstream graphs.

1. Use licensed or consented, non-private fixtures with reference transcripts and
   reviewed translations: English -> Chinese, Japanese -> Chinese and Chinese ->
   English, plus code-switching, proper names, music, quiet speech and silence.
   Record fixture hashes and rights; do not commit user captures or subtitles.
2. Pin model revision and file hashes, quantization, runtime commit/build flags,
   prompt hash, VAD settings, OS, CPU/GPU, RAM, power mode and thread count. Use
   identical text inputs to compare MT separately from ASR errors.
3. Warm models, then feed 16 kHz mono PCM16 at wall-clock speed through an isolated
   harness; an unrestricted file decode is a separate throughput experiment.
   Capture event timing and counts without transcript content in diagnostics.
4. Run ASR alone and ASR+MT concurrently while the machine plays video. Measure
   cold/warm load, first partial from speech start, source-final and translation-
   final latency from utterance end, p50/p95, real-time factor, peak process-tree
   memory/VRAM, CPU/GPU use, power/thermal behavior and queue age/depth.
5. Score CER/WER with declared normalization, partial rewrite rate and silence
   hallucinations. Blind-review MT for adequacy, omissions, names, negation,
   punctuation and unwanted explanations. Include quantized-vs-reference output.
6. Run at least 30 minutes continuously plus start/stop, pause/resume, language
   switch, worker kill, missing/corrupt model, full disk, OOM/overload and external
   network blocking. Require bounded memory/queue age and no stale-generation
   subtitle publication or leftover child processes.

Suggested go/no-go targets (to approve before measuring): p95 first source draft
within 1.5 seconds of speech start, p95 translated final within 3 seconds of
utterance end, sustained real-time factor below 0.7 for compute headroom, and no
monotonic backlog over 30 minutes. Judge short and long utterances separately.
If a candidate cannot meet the targets, report the failure; do not quietly turn
a 2-second ASR chunk setting into a claimed 1.5-second observed draft latency.

Use at least 100 reviewed utterances per language pair for the initial quality
screen. Proposed gate: no more than 5 utterances with major meaning errors and
zero critical polarity/number/name reversals in the designated regression set.
Report denominators and error categories; this small screen is not a population
quality estimate. Select the smallest model that passes both quality and latency,
not the model with the best aggregate ASR score alone.

| Target | Required evidence | Current result |
| --- | --- | --- |
| macOS Apple Silicon | 8 GB and 16+ GB devices tested separately; stable-signed development app, system capture and child lifecycle | Not run |
| Windows x64 | CPU-only baseline, optional GPU profile, real desktop loopback and signed/package boundary | Not run |
| Linux x64 | CPU baseline and actual PulseAudio/PipeWire-Pulse monitor session | Not run |
| macOS Intel / Windows ARM64 | Separate runtime build, packaging and long-session acceptance before advertising support | Not run |

Archive numerical results with runtime/model revisions and test commands. CI can
check IPC framing, queue/reducer invariants, cancellation and fake-worker crashes
without models; it cannot establish hardware suitability or translation quality.
The eventual behavior PR must run `./scripts/check.sh` and native app acceptance
on each advertised platform, retaining stable signing and user data boundaries.

## Proposed delivery sequence

1. Review this proposal and build an isolated benchmark harness. Compare native
   ASR candidates before changing the product or distributing model artifacts.
2. Implement Local original subtitles on the first passing desktop target,
   credential-free configuration, model lifecycle and worker failure handling.
   Scope language choices to the selected model. Mark this as ASR-only.
3. Add local MT after paired quality/concurrency tests pass. Only then describe
   the feature as fully local translated subtitles; add other targets as proven.
4. Add advanced VAD controls after presets are measured. Keep separation in a
   separate experiment with a clear measured benefit.

Merging this research document would not complete or close issue #40. Remaining
product decisions are the initial languages/device floor and the measured default
ASR/MT combination. This document recommends an integration shape, not a release
date or a claim that local quality already matches the cloud services.

## Sources

1. [sherpa-onnx online Paraformer models](https://k2-fsa.github.io/sherpa/onnx/pretrained_models/online-paraformer/index.html) and [runtime repository](https://github.com/k2-fsa/sherpa-onnx).
2. [Official Qwen3-ASR README, including streaming backend restrictions](https://github.com/QwenLM/Qwen3-ASR#streaming-inference) and [0.6B model card](https://huggingface.co/Qwen/Qwen3-ASR-0.6B).
3. [antirez/qwen-asr runtime and streaming contract](https://github.com/antirez/qwen-asr).
4. [whisper.cpp stream example](https://github.com/ggml-org/whisper.cpp/tree/master/examples/stream).
5. [Qwen2.5-7B-Instruct model card](https://huggingface.co/Qwen/Qwen2.5-7B-Instruct).
6. [Qwen3-4B-Instruct-2507 model card](https://huggingface.co/Qwen/Qwen3-4B-Instruct-2507).
7. [FunASR online SDK](https://github.com/modelscope/FunASR/blob/main/runtime/docs/SDK_tutorial_online.md).
8. [llama.cpp runtime](https://github.com/ggml-org/llama.cpp).
9. [Ollama OpenAI compatibility](https://docs.ollama.com/api/openai-compatibility).
10. [Silero VAD](https://github.com/snakers4/silero-vad).
11. [Demucs repository and archive status](https://github.com/facebookresearch/demucs).
