# Simplified settings and tray design

## Context

The desktop app already uses Alibaba Cloud's shared DashScope endpoints. The
runtime, profile model, persistence format, and IPC require only an API key;
there is no Workspace ID setting. The shared endpoint remains supported, while
Alibaba's workspace-specific domains are an optional future migration rather
than something every Mimi user should configure.

The first multi-provider settings UI exposed every profile-management action
at once and also duplicated session controls from the tray. This made a small
desktop utility feel like an administration console. The tray had the opposite
problem: it showed low-frequency interface-language settings but omitted the
primary start, pause, and stop actions.

## Product direction

Use one quiet, platform-neutral visual language on macOS and Windows. Native
window chrome remains owned by the operating system; the web UI uses neutral
controls, system typography, shared spacing, and the Mimi teal accent rather
than imitating one platform.

Settings is for configuration:

1. **Subtitles** contains recognition language, translation target and mode,
   subtitle size, and overlay position lock.
2. **Translation service** shows the active configuration and credential
   readiness. API key replacement is write-only. Adding, renaming, and deleting
   configurations is available through an initially collapsed management area.
3. **General** contains interface language.

The status/start surface, repeated provider descriptions, mode badge, and
visible profile editor are removed. Session control belongs to the tray;
multi-profile support remains available without being the default reading path.

The tray is for frequent actions:

1. Status and active service are visible at a glance.
2. The primary controls start, pause/resume, or stop the live session. Missing
   credentials lead directly to settings.
3. Recognition language and overlay lock remain available as quick settings.
4. Show subtitles and clear subtitles are equal secondary actions, shown only
   when the current session or retained subtitle content makes them useful.
5. Settings and Quit live in a quiet footer. Interface language is removed from
   the tray because it is not a session control.

The native right-click menu is deliberately smaller than the custom panel: it
contains only Start/Stop, Settings, and Quit (plus DevTools in development).
Its labels follow the saved language override or the operating-system locale.
macOS reads `NSLocale`; Windows uses the platform globalization API through a
target-only `windows-sys` binding, which adds no runtime service or new package
outside the Windows build.

## Interaction and safety

- Profile and credential mutations stay disabled while a session is active.
- Saved API keys are never read back into frontend state. A replacement draft
  is cleared before the write-only IPC request begins, including on failure.
- Missing and unavailable credential states remain distinct and visible.
- Provider capabilities continue to define available languages and modes; the
  UI never presents an unsupported choice.
- Errors from tray actions remain visible in the tray instead of being silently
  discarded.
- Keyboard focus, `aria-live`, reduced-motion, and forced-colors behavior are
  preserved or improved.

## Responsive behavior

The settings content is a single column capped around 720 logical pixels. The
window remains resizable and usable down to 520 logical pixels without a
second layout model. The tray is a fixed-width 320 logical pixel surface with
height matched to its content, avoiding transparent dead areas and clipped
shadows.

## Verification

- Frontend typecheck, lint, unit tests, and production build.
- Rust format, Clippy, and tests after window-size changes.
- Canonical repository check.
- UI-only packaged-app inspection of settings and tray at normal and narrow
  sizes, in Chinese plus long English/Japanese copy, without Keychain, network,
  or system-audio access.
- Website and README review for macOS/Windows wording, the API-key-only Alibaba
  setup, and replacement of screenshots that still expose Workspace ID.
