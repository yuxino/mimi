# Scrolling Subtitle Layout Design

## Goal

Long realtime transcripts must remain readable instead of ending in a SwiftUI truncation ellipsis. The overlay should preserve its remembered, user-resizable frame while keeping the newest spoken words visible.

## Layout

The source and translated subtitle each receive an independent vertical viewport. Text wraps naturally at the current panel width with no `lineLimit`. When streaming text grows beyond its viewport, the view scrolls to a bottom anchor without animation, so the newest lines remain visible and older lines move upward. The source receives roughly 38 percent of the available content height and the larger Chinese translation receives the remainder. If no source is present, translation/status content uses the full height.

The surrounding panel, drag behavior, resize behavior, saved frame, typography, colors, lock mode, and settings button remain unchanged. The minimum height increases to 190 points so an old, very short saved frame cannot collapse both viewports to one line; users can still resize the panel larger. While recognition has started but translation has not yet produced text, the source remains visible without replacing the Chinese line with a misleading truncation marker.

## Verification

Verification requires a warnings-as-errors release build, the complete core suite, stable-signed packaging, and visual inspection with a long Japanese utterance. The screenshot must show wrapped text without a trailing `…`, and successive samples must keep the newest lines visible as the transcript grows.
