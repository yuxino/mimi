# Show the original text until the translation takes over

Date: 2026-08-12

## Goal

While translating (for example Japanese → Simplified Chinese), the live line in
the floating subtitle window should show the original spoken text while it is
being recognized, then become the Chinese translation once the translation is
available — same font size throughout. The viewer reads the Japanese as it
appears, and it naturally turns into Chinese when the translation lands.

## Change

`SubtitleOverlayView` now picks the live current line like this whenever the
target language translates audio (`targetLanguage.translatesAudio`):

- While the source is still a non-final draft, the live line shows the original
  text, updating as recognition streams in.
- Once the source is final and no translation is available yet (translation in
  flight), the live line keeps showing the final original.
- Once the translation arrives, the live line becomes the Chinese translation —
  the same row, the same font size — and the confirmed pair enters history as
  the translation only, exactly as before.
- Original-subtitle mode is unchanged: source and translation are the same text.

An earlier attempt rendered the original as a small dim line above the
translation; that was rejected — the original is a temporary live line that
becomes the translation, not a permanent second line.

## Verification

- The strict release build passes with warnings treated as errors; the change is
  UI-only in `MimiApp`, so the packaged app is manually inspected in translated
  and original modes.
