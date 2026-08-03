# Bounded Draft Preview Design

## Goal

Prevent a growing streaming translation preview from suddenly filling the subtitle overlay with a large block of text.

## Design

Keep the translation pipeline and stored subtitle text unchanged. The overlay continues to segment text at natural sentence, punctuation, and word boundaries, but an unconfirmed translation displays only its two most recent segments. For Simplified Chinese this is roughly the latest 56 characters; Japanese is roughly 64 characters; English is roughly 128 characters because its segment limit is wider.

Confirmed Plus translations remain complete in subtitle history and can still be reviewed by scrolling. This makes the rule presentation-only: no recognized or translated content is discarded, and the final history remains suitable for later reading.

The bounded-window operation belongs in `SubtitleTextSegmenter` so it is deterministic and directly testable. `SubtitleOverlayView` applies it only when `SubtitleLine.isFinal` is false. Final lines and history keep using the existing full segmentation method.

## Alternatives

- Globally keeping only the newest rows would make the overlay calmer but discard visible history.
- Truncating the translated string inside the reducer would lose content before the accurate Plus result arrives.
- Shrinking the overlay would not prevent a long preview from creating many rows and would override the user's chosen window size.

## Verification

Unit tests confirm that a long draft exposes only its final two natural segments while a short draft remains unchanged. The full core suite, warnings-as-errors release build, and UI test build must pass before packaging.
