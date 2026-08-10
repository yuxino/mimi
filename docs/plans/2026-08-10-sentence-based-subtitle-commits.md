# Sentence-based subtitle commits for high-quality ASR

Date: 2026-08-10

## Problem

With semantic punctuation enabled, the server only finalizes a sentence when it
is semantically complete. mimi's local draft committer force-committed the whole
accumulated draft every ~1.2s, cutting long speech at arbitrary character
positions. Users saw:

- **Chopped sentences**: history entries that are grammatical fragments.
- **Duplicates / jumping**: a late server final that overlapped the committed
  prefix was re-committed, showing the same text twice and shifting rows.
- **Missing words**: fragments translated out of context dropped the feeling of
  complete subtitles.

## Change

`ASRDraftCommitter` now splits the cumulative draft on sentence-ending
punctuation (`。！？.!?\n`) and only commits complete sentences:

- `commitCompleteSentences()` commits complete sentences and leaves the
  incomplete trailing sentence pending as the live draft.
- `commitLatestDraft(commitLongIncomplete:)` additionally commits a long
  pending tail (>= 20 characters) as a single chunk on the 4.5s maximum-wait and
  session-finish paths so subtitles keep flowing during very long uninterrupted
  speech.
- `finishSentence()` drops server finals already covered by committed text and
  strips overlapping suffixes so late finals cannot duplicate content.

`HighQualityTranslationClient` schedules one stable-draft timer instead of
resetting it on every draft, so complete sentences are confirmed on a steady
cadence rather than in 4.5s bursts.

## Verification

- Unit tests cover complete-sentence commits, tail preservation, multiple
  sentences per draft, English/Chinese delimiters, late-final deduplication,
  overlapping suffix stripping, long-incomplete flow, and reset.
- Real-speech run of a five-sentence Japanese phrase produced five clean
  complete-sentence history entries with no fragments, duplicates, or dropped
  words.
