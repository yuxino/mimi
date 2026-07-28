# Natural Live Translation Design

## Goal

Make finalized Chinese subtitles sound like natural dialogue without making live
draft subtitles feel slower.

## Design

The translation pipeline uses two quality tiers:

- Drafts continue to use `qwen-mt-lite` so the first readable translation appears
  quickly.
- Final ASR segments use `qwen-mt-flash`, which provides a better
  quality/latency balance while still supporting streamed responses.

When ASR reports a supported language, translation requests use that explicit
language instead of asking Qwen-MT to detect it again. A user-selected source
language remains authoritative.

Final requests include a short English domain hint asking for concise,
idiomatic Simplified Chinese dialogue. They also include up to three immediately
preceding confirmed source/translation pairs through Qwen-MT's `tm_list`.
Translation memory is isolated by source language, bounded, and cleared with the
session so unrelated content cannot accumulate or bias later translations.

Meaningful interjections, hesitation, and sentence-final tone are retained as
natural Chinese particles. Accidental ASR repetition is collapsed, while
Japanese fillers and sentence-final particles are translated selectively rather
than word for word.

Drafts never wait for final translations. Final requests stay ordered, and only
completed final translations enter translation memory. Existing cancellation,
preemption, and authentication error behavior remains unchanged.

## Verification

- Protocol tests verify model selection, domain hints, translation memory, and
  source-language encoding.
- Core tests verify the existing subtitle and session behavior remains intact.
- A strict release build verifies Swift concurrency safety.
- Replay tests compare translation output and latency on saved Japanese dialogue
  audio before the updated app is packaged.
