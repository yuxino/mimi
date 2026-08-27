# Common regressions and local-test rules

Read this before launching, packaging, installing, or debugging macOS system
prompts. The visible app name and version are not enough to establish identity.

## macOS app identities

| Use | Canonical app | Bundle identifier | Signing identity |
| --- | --- | --- | --- |
| Pre-push development and UI checks | `/Applications/mimi-dev.app` | `app.yuxino.mimi.dev` | `mimi Local Development` |
| Local release-shaped bundle | `src-tauri/target/release/bundle/macos/mimi.app` | `app.yuxino.mimi` | `mimi Local Development` |
| Published GitHub release | `/Applications/mimi.app` | `app.yuxino.mimi` | Ad-hoc (build-specific) |

The two release-shaped bundles have the same bundle identifier but different
designated requirements. They are not identity-compatible updates. Replacing
one with the other can make macOS ask for Screen & System Audio Recording and
Keychain authorization again. A later GitHub ad-hoc build may do the same.

Rules:

- Use `./scripts/dev-app.sh` for normal local testing. Do not run `tauri dev`, a
  bare `target/*/mimi` executable, or a copy at a disposable path.
- `./scripts/package-app.sh` creates a local, release-shaped package; it does
  not turn the local certificate into the GitHub release identity.
- Before replacing a formal app, run
  `./scripts/verify-macos-install-identity.sh NEW_APP /Applications/mimi.app`.
  A mismatch fails closed. `MIMI_ALLOW_IDENTITY_CHANGE=1` is reserved for a
  deliberate, one-time certificate migration whose extra prompts are expected.
- Never use ad-hoc signing for permission-sensitive local QA. GitHub Releases
  are the explicit developer-installable exception and may require users to
  approve permissions again. Never use `tccutil reset`, delete Keychain entries,
  or rotate a certificate as a routine fix.
- Branch and pull-request CI compiles macOS with `--no-bundle`; tag CI is the
  only path that creates and publishes the developer-installable `.dmg`.
- Keep only one live mimi copy while testing. Confirm its executable path, not
  just the process name, before diagnosing shortcuts, windows, or permissions.

## Know which prompt appeared

These prompts have different causes and fixes:

- **Screen & System Audio Recording:** TCC compares the bundle identifier and
  designated requirement. A changed certificate requires one new grant. A
  stable identity at a canonical path must not require repeated grants.
- **API-key Keychain access:** the running app is reading a saved provider key.
  A normal startup reads the profile key once and caches the result. Migration
  tombstones and legacy slots are read only when the profile key is missing or
  during an explicit save/delete/migration. Keep the same service/account and
  update its value in place: deleting and recreating it discards accumulated
  access rules and creates a crash window in which the secret can be lost.
- **Code-signing private-key access:** `/usr/bin/codesign` is using the private
  key for `mimi Local Development` while packaging the app and DMG. This is not
  API-key access. Grant persistent access only when the dialog names that exact
  private key and tool; do not automate a login-keychain password or widen the
  whole keychain ACL in build scripts.
- **Gatekeeper / Open Anyway:** the GitHub package is ad-hoc signed and not
  notarized. This is separate from capture and Keychain authorization.

The local development certificate is self-signed and has no Apple Team ID. It
provides a stable requirement for local TCC testing, while GitHub's ad-hoc
signature is build-specific. The file-based Keychain also applies a partition
check that can fall back to the build's CDHash. Therefore:

- eliminating the duplicate migration-item read reduces a normal startup to
  one API-key authorization after an identity migration;
- do not promise that a rebuilt self-signed local binary will never ask for
  Keychain access again, and expect GitHub updates to require approval again;
- do not solve this by deleting/recreating a credential, using an allow-all
  ACL, scripting the login password, or assigning a made-up Team ID. Those
  approaches either lose data or weaken code identity;
- password-free Keychain continuity across binary updates requires an
  Apple-issued signing identity with a stable Team ID. Moving to Developer ID
  is an explicit distribution migration, not a local debugging workaround.

When a system prompt repeats, compare the old and new requirements first:

```bash
codesign --display --requirements - /Applications/mimi.app 2>&1
codesign --display --requirements - /path/to/new/mimi.app 2>&1
```

Do not inspect or reset the TCC database as a first response. System logs may
be used only for content-free labels, timestamps, bundle identifiers, and code
requirements; never log recognized text, translated text, credentials, or
audio.

## Overlay and UI checks

- AppKit window mutations, including window level and collection behavior,
  must run on the macOS main thread.
- Full-screen visibility requires the overlay's all-spaces and full-screen
  auxiliary behavior as well as the intended window level. Test above a real
  full-screen app, not only a maximized window.
- Immersive mode owns its complete state: locked overlay position, hidden
  recognition pill, background treatment, and hidden scrollbar. Toggling it
  must restore the prior normal-mode interaction state.
- Streaming drafts are replaceable previews; finals are durable. If a source
  transcript exists while translation is delayed or absent, the bounded source
  fallback remains visible until the translated final replaces it.
- Keep accessibility state in `aria-*`, but drive changing selected/checked
  visuals through explicit React class names. macOS WKWebView has previously
  left attribute-selector styling stale after the underlying state changed;
  verify the selected class visibly moves in the signed development app.
- Category navigation needs a stronger boundary. Do not depend on a dynamic
  `[hidden]` selector or leave inactive panels mounted: WKWebView has shown
  stale pixels from both the previous panel and the previous selected item.
  Mount only the active panel and remount the compact category navigation when
  the category changes. Verify that the highlighted category and visible
  heading agree after the initial settings snapshot arrives.
- Use `./scripts/dev-app.sh --ui-only` for visual states that do not require a
  provider. UI-only mode must never read Keychain items, open provider sockets,
  or start system-audio capture.

## Before handing off

Run `./scripts/check.sh`. For signing changes, additionally build with
`./scripts/package-app.sh`, verify the bundle, and compare its designated
requirement with any app that would be replaced. Before a commit, inspect the
diff for credentials, recordings, subtitle content, personal paths, build
artifacts, and signing material.
