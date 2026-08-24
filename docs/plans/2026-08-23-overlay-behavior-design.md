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

The existing language-picker WebView is an owned/child `overlay-control` window
with two modes. Island is a compact always-available status/language capsule;
Panel expands in place for provider-supported language/mode choices, subtitle
background visibility, and the settings entry. It never changes subtitle-canvas
geometry and does not add another WebView. Paused sessions may change language
or mode; connecting and stopping mutations remain disabled. Language and mode
buttons form two-column, one-click grids; an unpaired final option spans the
full row so the panel stays compact without hiding choices in submenus. If the
available work area is shorter than the panel, the control content scrolls
inside the clamped native window instead of making its lower actions unreachable.

## Geometry and interaction

Native code owns the overlay frame, control-window mode, collapse/expand
transitions, resize bounds, work-area clamping, click-through lock state, and
control anchoring. Geometry writes are versioned and atomic. Layout uses each
monitor's work-area origin as well as its size, including left/above monitors
with negative coordinates.

The native window remains above ordinary windows, joins every desktop space,
and is an auxiliary full-screen window on macOS so subtitles remain visible
over full-screen media. These behaviors are reasserted when an existing overlay
is shown.

The drag handle and resize regions provide visible hover/focus feedback. Locking
enables click-through without hiding subtitles. Animation is brief and uses the
same 180 ms ease-in-out timing in native geometry and React content; reduced
motion replaces it with immediate state changes.

## Presentation

Subtitle alignment is a persisted visual preference with left, center, and
right options. It affects text layout only and remains safe to change while a
translation session is active. The settings window exposes the complete
preference and the tray panel provides the same three-way quick control.

Background-blending presentation is also a persisted, runtime-safe visual
preference. In this mode native state forces the overlay expanded and renders
subtitle text directly on the transparent window with a restrained readability
shadow. The subtitle canvas removes its card, status chrome, timestamps,
buttons, drag affordance, and resize handles and becomes click-through. The
separate control island stays interactive and exposes an explicit “show subtitle
background” action, so the user always has an on-subtitle escape route in
addition to the tray panel and settings window. No subtitle text or audio is
persisted by this presentation mode.

## Verification

Pure tests cover state derivation, segmentation, geometry, resize regions,
popover clamping, and reducer behavior. UI checks cover expanded/collapsed,
empty, listening, translating, paused, error, long-text, locked, reduced-motion,
and keyboard-focus states. Use `./scripts/dev-app.sh --ui-only` for visual checks
that must not access credentials, provider networks, or system-audio capture.
