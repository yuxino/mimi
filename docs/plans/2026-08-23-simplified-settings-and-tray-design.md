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
window chrome remains owned by the operating system; the web UI uses system
typography, shared spacing, and a monochrome graphite palette rather than
imitating one platform. Selection and hierarchy come from contrast, borders,
weight, and spacing—not a brand tint. App-owned semantic states use icons, copy,
shape, and border weight instead of hue.

Settings is for configuration. A category rail shows one group at a time so the
window reads like a focused utility instead of one long administration form:

1. **Subtitles** contains recognition language, translation target and mode,
   subtitle size, and overlay position lock.
2. **Translation service** shows the active configuration and credential
   readiness. API key replacement is write-only. Adding, renaming, and deleting
   configurations is available through an initially collapsed management area.
3. **General** contains interface language.

The active category is expressed with a high-contrast monochrome treatment. A
missing or unavailable credential opens the Translation service category by
default; otherwise Subtitles is the default. The status/start surface, repeated
provider descriptions, mode badge, and visible profile editor are removed.
Session control belongs to the tray; multi-profile support remains available
without being the default reading path.

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

The custom tray uses the same monochrome tokens as Settings. Its primary action
is a solid high-contrast black/white button; profile marks, quick-setting icons,
switches, status dots, and focus rings stay grayscale. It has no teal glow,
colored gradient, or decorative color wash. App-owned error and warning states
also stay monochrome, using icons, copy, border weight, and contrast instead of
hue. Operating-system window chrome and forced-colors mode remain under system
control.

## Interaction and safety

- Profile and credential mutations stay disabled while a session is active.
- The tray's **Configure service** action carries an explicit service-category
  navigation intent. An existing hidden Settings window handles it directly;
  a recreated window installs its listener and announces readiness before the
  native shell delivers the one-shot intent.
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

The settings content uses a compact category rail beside one content panel. At
narrow widths the rail becomes a three-item horizontal tab strip, preserving
the same information architecture down to the 520 logical-pixel minimum. The
tray is a fixed-width 320 logical pixel surface with height matched to its
content, avoiding transparent dead areas and clipped shadows.

## Verification

- Frontend typecheck, lint, unit tests, and production build.
- Rust format, Clippy, and tests after window-size changes.
- Canonical repository check.
- UI-only packaged-app inspection of settings and tray at normal and narrow
  sizes, in Chinese plus long English/Japanese copy, without Keychain, network,
  or system-audio access.
- Website and README review for macOS/Windows wording, the API-key-only Alibaba
  setup, and replacement of screenshots that still expose Workspace ID.
