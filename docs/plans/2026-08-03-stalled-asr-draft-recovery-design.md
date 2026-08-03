# Stalled ASR Draft Recovery Design

## Goal

Prevent audible speech from remaining untranslated when Audio 3 keeps producing interim recognition text but does not emit a sentence-ending result.

## Approach

Keep server sentence boundaries as the primary path. For every non-empty interim result, retain only the portion that has not already been locally committed. If that portion stops changing for 1.2 seconds, treat it as a confirmed chunk. If speech continues without a stable pause, commit the latest uncommitted portion after 4.5 seconds so continuous dialogue still advances.

Local commits use the existing final Qwen-MT Plus queue, preserving ordering, retry behavior, translation memory, and subtitle history. The client remembers the exact interim prefix that it committed. When Audio 3 eventually emits its real final result, the already committed prefix is removed and only a meaningful remainder is queued. Exact late finals and punctuation-only tails are ignored, preventing duplicate translations.

The incremental text calculation lives in a small deterministic core type. Timing stays in `HighQualityTranslationClient`, where connect, disconnect, pause, and language-switch cancellation already belong. Pipeline diagnostics remain content-free and record whether a chunk came from a stable-draft, maximum-wait, or server-final boundary.

## Alternatives considered

- Translating every interim result gives the fastest feedback but spends many requests and frequently pairs stale translations with newer source text.
- Adding an undocumented server silence parameter is smaller, but an unsupported parameter could reject the entire ASR task and still would not protect continuous speech.
- Switching to the newer Qwen realtime protocol is a larger migration and does not belong in an isolated reliability fix.

## Verification

Unit tests cover exact-prefix subtraction, late-final deduplication, suffix preservation, punctuation-only suppression, and reset behavior. The complete core test suite and release build run with warnings treated as errors. The packaged app is restarted with content-free pipeline diagnostics enabled to verify audio buffers, interim/final boundaries, queueing, and translation completion without logging subtitle text.
