# Settings layout refinement

The settings window uses a quiet, neutral desktop layout. A fixed sidebar owns
navigation and session controls; only the content pane scrolls. This replaces
the large full-width session card, responsive top tabs, repeated page headings,
and stacked outer cards. Keep the existing system typography and monochrome
palette. Use spacing, restrained surfaces, and fine separators for hierarchy.

The four destinations are Subtitles, Translation service, General, and Session
export. Existing hashes and service-navigation IPC keep working. Session export
has its own hash. Session controls remain reachable on every page; close-to-tray
and capture explanations are available in a disclosure below the status.

General contains three appearance previews (Light, Dark, System), interface
language, and software update. Appearance choices retain the existing settings-
only storage and live system preference behavior. Pressed/selected visuals use
explicit classes for WKWebView. Native controls and keyboard focus remain usable.

Session export separates the two opt-in controls, memory-only retention notice,
current buffer summary, and export actions. Detailed retention limits and the
transcript/audio timeline caveats live in a labeled disclosure. The default-off
notice and warning to export before starting another session or quitting stay
visible. Buffer clearing and all session/IPC guards retain their existing logic.

Update and export panels remain mounted while navigating so an in-progress
operation retains its owner and feedback. Explicit inactive classes with display:none isolate their layout and
accessibility; avoid mixing the native hidden attribute with WKWebView panel
transitions. Other panels mount on demand.
The sidebar remains available at the 520 px minimum window width. At narrower
widths, selects stack below labels; switches remain aligned with their labels.

Validate in the signed, credential-free development app: both appearances,
all four destinations, language switching, active/stopped controls, empty and
retained export states, disclosure layout, and minimum-size navigation. No real
provider traffic or system capture is required for this visual change.
