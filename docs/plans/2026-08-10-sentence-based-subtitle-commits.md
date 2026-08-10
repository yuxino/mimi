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

## Follow-up: hide flash draft previews in high-quality mode

Users still saw the live subtitle line continuously rewrite during speech: the
flash draft preview streamed a low-quality version of the pending tail, and the
high-quality final then replaced it with different wording ("字幕在变，最后才变
对"). High-quality mode now emits only confirmed finals; source drafts still
update the recognition indicator, and the sentence-based committer confirms each
complete sentence on a steady cadence. The draft-preview machinery remains
available for the original-subtitles path.
