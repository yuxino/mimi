# Manual Subtitle Clear Design

## Goal

Let the user clear visible subtitles and subtitle history directly from the
overlay without interrupting live listening.

## Interaction

When the unlocked overlay is hovered, its top-right controls show an eraser
button before the existing settings button. The eraser is shown only when
subtitle content exists. Clicking it immediately clears current source text,
current translation, pending pairing state, and confirmed history.

The action does not stop audio capture, reconnect translation, move the window,
or change the listening status. No confirmation is shown because live subtitles
are transient and the action is intentionally lightweight. The existing menu-bar
clear command remains available, including while the overlay is locked.

The button has a tooltip and accessibility label. Verification covers reducer
state, session continuity, strict compilation, and a visual click-through in the
packaged app.
