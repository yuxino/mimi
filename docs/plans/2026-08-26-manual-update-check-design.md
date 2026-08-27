# Manual update check

## Context

mimi publishes installer assets through GitHub Releases, but the release
pipeline does not yet produce Tauri updater metadata or update signatures.
The application also has no updater endpoint or updater public key. Presenting
an in-app install action would therefore imply a trust and rollback path that
does not exist.

## Decision

The General settings page exposes a manual **Check for Updates** action. A
click asks a Rust client to read the latest public release tag from the fixed
`yuxino/mimi` GitHub repository. The current package version and release tag
are compared as SemVer; an update is available only when the release is newer
than the running app. This matters for development builds that can be ahead of
the latest published release.

The check is never run at startup or in the background. If a newer release is
available, the action changes to **View Update** and opens the fixed
`https://github.com/yuxino/mimi/releases/latest` page in the system browser.
Installation remains an explicit user action.

## Security and failure behavior

- The WebView cannot choose the repository, API endpoint, or destination URL.
- The network client uses HTTPS, a short timeout, and GitHub's JSON media type.
- Only the settings window may invoke the two update commands.
- The result contains version metadata only; errors shown to the UI are
  localized and do not include response bodies.
- Network, response, version-parse, and browser-open failures leave the app
  usable and offer a retry.

The official Tauri opener plugin is used only from the trusted Rust command
with a hard-coded URL. Its general frontend URL-opening command is not granted
to any window.

## Verification

Pure Rust tests cover leading `v`, newer/equal/older versions, prerelease
ordering, malformed tags, and GitHub response parsing. Frontend build checks
keep the Chinese, English, and Japanese copy sets aligned. Native UI QA should
cover the General page at its minimum width and the idle, checking, current,
available, and error states.

Full in-app installation remains future work. It requires signed updater
artifacts, a pinned updater public key, release metadata, CI verification, and
an update-in-place permission and Keychain continuity test.
