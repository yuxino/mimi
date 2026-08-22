# Subtitle overlay behavior

## State and content

The overlay is a Tauri window rendered by React and backed by the native session
state. Streaming drafts are replaceable; confirmed pairs form bounded, durable
on-screen history. No audio or subtitle content is persisted.

The timeline uses one scrolling flow for confirmed history plus the active tail.
Long text wraps, the newest content remains visible, and draft presentation is
bounded without truncating confirmed history.

The activity presentation is derived from session state:

- connecting and recovering show a non-destructive progress state;
- listening distinguishes recognition from pending final translation;
- paused is static and keeps existing subtitles visible;
- errors show sanitized status text; and
- reduced-motion mode removes decorative movement without hiding state.

## Controls

Expanded and collapsed layouts expose only actions that can take effect in the
current lifecycle state. Pause/resume, collapse/expand, language/mode selection,
clear, lock, and settings actions share native session guards; controls do not
optimistically claim a mutation that the backend rejected.

Clearing subtitles resets visible drafts, pending pairing state, and confirmed
history without changing listening status. It is available from the overlay and
tray panel and does not require confirmation because it deletes no persisted
content.

The language picker is a separate window anchored to the overlay capsule, so it
never changes overlay geometry. Options come from the active provider's
capabilities; unsupported or transition-state mutations are disabled.

## Geometry and interaction

Native code owns the overlay frame, collapse/expand transitions, resize bounds,
screen clamping, click-through lock state, and popover anchoring. Geometry writes
are versioned and atomic. The overlay stays partially reachable when screens or
work areas change.

The drag handle and resize regions provide visible hover/focus feedback. Locking
enables click-through without hiding subtitles. Animation is brief and uses the
same 180 ms ease-in-out timing in native geometry and React content; reduced
motion replaces it with immediate state changes.

## Verification

Pure tests cover state derivation, segmentation, geometry, resize regions,
popover clamping, and reducer behavior. UI checks cover expanded/collapsed,
empty, listening, translating, paused, error, long-text, locked, reduced-motion,
and keyboard-focus states. Use `./scripts/dev-app.sh --ui-only` for visual checks
that must not access credentials, provider networks, or system-audio capture.
