# Stop duplicate subtitles from local commits racing server finals

Date: 2026-08-11

## Problem

Live pipeline logs from a real session (Chinese/English audio, high-quality
mode) showed the stable-draft local committer racing the server's semantic
finals:

- A 15-character sentence was committed locally at 22:43:43.059, and the server
  final for the same speech arrived 28 ms later at 22:43:43.087 as a
  23-character result. The overlap was committed as a separate "tail", so one
  spoken sentence produced two history entries.
- A sentence committed locally as 11 characters was followed 0.5 s later by a
  25-character revision from ASR, then further fragments. The provisional lines
  stayed in history while the server's corrected final was committed again.

Users see the same speech twice (or chopped into fragments): the local commit is
based on an in-flight partial that the server is still revising, so history gets
the provisional wording and later the authoritative wording as separate rows.

## Change

Local commits are treated as replaceable previews; server finals remain the
single source of durable history.

1. **Supersede and revoke** (`ASRDraftCommitter.finishSentence` now returns
   `FinishOutcome`): when a server final structurally covers the last local
   commit (extends it, or contains it with additional leading words), the
   committer reports `.replaced(fullFinal)` instead of appending a tail.
2. **Revoke plumbing**: a new client-internal `LiveTranslateServerEvent`
   (`subtitleRevoked`) maps to `SubtitleEvent.revokeLastConfirmed`, which
   removes only the last history pair. The high-quality client defers the
   revoke until the replacement translation is ready, so the provisional entry
   has already entered history and the replacement lands immediately after —
   the line updates in place instead of blinking away. Exact duplicates remain
   deduplicated without a revoke, so confirmed lines do not flicker.

An earlier draft of this change added a "stability gate" that only committed a
complete sentence after it appeared in two consecutive drafts. Live testing
showed it added seconds of latency for utterances that arrive as a single
partial, so it was removed. Local commits keep the normal 1.2 s cadence; the
supersede-and-revoke path fixes the duplicate after the fact, and the 4.5 s
maximum-wait and session-finish paths remain the last-resort fallback for
stalled ASR.

## Verification

- Unit tests cover exact server-final dedup, extension supersede, leading-word
  supersede, revised wording falling back to append, and revoke semantics in
  the reducer and session controller.
- The full core suite and release build with warnings treated as errors pass via
  `./scripts/check.sh`.

Residual case: a server final that rewrites a stable local commit with
completely different wording cannot be detected structurally and is appended
like new content. The stability gate makes that rare by keeping still-revising
sentences out of history in the first place.
