# Collapsible subtitle overlay

The subtitle overlay can be collapsed without stopping audio capture, speech
recognition, or translation. The expanded header gains a collapse button, and
double-clicking its drag handle performs the same action. Collapsed mode displays
a compact floating bar with the recognition animation, current pipeline phase,
and an explicit expand button. Its drag handle remains usable.

Collapsing stores the current expanded size, shrinks around the same horizontal
center, and keeps the bottom edge anchored. If the compact bar is moved, expansion
uses its new center and bottom position. The compact frame is excluded from AppKit
frame autosaving, so a fresh app launch always opens the normal expanded overlay.
The expanded frame continues to be remembered.

Resize interactions and resize cursors are disabled while compact. Existing lock
behavior is unchanged. Geometry tests verify anchoring and moved-bar restoration;
the full core test suite and a seeded UI screenshot verify behavior and appearance.
