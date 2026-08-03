# Pause and Reliable Translation Design

## Goal

Add a one-click pause/resume control to both subtitle layouts and stop transient Qwen-MT Plus failures from silently dropping a confirmed sentence.

## Pause behavior

Pausing is an explicit user state, separate from stopping, reconnecting, or a network error. It keeps the subtitle window and the last visible subtitle on screen, stops system-audio capture, disconnects the active recognition/translation client, cancels queued work, and disables connection recovery. Resuming creates a fresh client and capture stream with the current settings without clearing subtitles. A normal application restart begins unpaused.

The expanded overlay shows a pause/play button in the always-visible top-right controls, immediately before the collapse button. The compact overlay shows the same control before its expand button. While paused, the activity indicator becomes static amber and the status copy reads `已暂停`; the pause button changes to a play symbol with `继续翻译` help and accessibility text. Language switching is disabled while paused to avoid presenting a control that cannot take effect until resume.

## Reliable translation behavior

Authentication and invalid-request failures remain terminal and visible. Transient failures (timeouts, invalid HTTP responses, HTTP 408/429/5xx) use capped exponential backoff and keep the same sentence at the head of the translation worker until it succeeds or the client is disconnected. No transient failure emits an empty final translation, so a sentence cannot disappear silently. Pausing or stopping cancels the active request and clears pending sentences, preventing a backlog from appearing after resume.

## Verification

Unit tests cover pause-state cleanup and retry delay classification. Release builds must pass with warnings as errors. UI verification checks expanded and compact pause/resume controls, the amber paused state, preserved subtitle content, and normal restart behavior.
