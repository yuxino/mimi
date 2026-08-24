# macOS formal release identity and permission continuity

## Problem

The v1.2.0 and v1.3.0 GitHub DMGs contained app bundles with linker-generated
ad-hoc signatures. Their designated requirements were tied to different
binary `cdhash` values, the bundles had no Apple Team ID, and strict bundle
verification failed. macOS therefore could not identify an update as the same
trusted application. Screen & System Audio Recording grants and Keychain ACLs
could be requested again even when the user had already approved mimi.

The formal bundle also omitted `NSScreenCaptureUsageDescription` and
`NSAudioCaptureUsageDescription`. The development launcher happened to add a
screen-capture description manually, so development and formal packages did
not have the same privacy metadata.

## Decision

A formal macOS release is valid only when all of the following are true:

- The app is signed with a fixed `Developer ID Application` certificate for
  bundle identifier `app.yuxino.mimi`; ad-hoc and local self-signed identities
  are forbidden.
- The app is notarized and stapled before it is copied into the DMG. The final
  DMG is then signed with the same Developer ID identity, notarized, and
  stapled as the exact artifact that will be published.
- The merged app `Info.plist` contains non-empty screen-capture and system-audio
  usage descriptions.
- Strict code-sign verification passes, the signing identifier is the bundle
  identifier, a Team ID is present, the authority is Developer ID Application,
  and the designated requirement is not a build-specific `cdhash` requirement.
- Gatekeeper assessment succeeds for both the app and the final disk image.

Tag builds are split into build and publication phases. macOS and Windows
installers first become private GitHub Actions artifacts. The macOS build fails
closed when any signing/notarization secret or verification condition is
missing. A separate job with the only `contents: write` token publishes both
platforms only after every build and verification succeeds. This prevents a
partially signed or ad-hoc app from becoming a formal release.
Publication stays draft-only while assets upload, verifies that the remote
asset set exactly matches the staged outputs, and becomes public in one final
transition. A retry refuses to mutate an already-published release.

CI uses repository secrets `APPLE_CERTIFICATE`,
`APPLE_CERTIFICATE_PASSWORD`, `APPLE_ID`, `APPLE_PASSWORD` (an app-specific
password), and `APPLE_TEAM_ID`. The temporary build keychain gets a fresh
random password on every run. Certificates and passwords never enter source
control or release artifacts.

## Local packaging

`scripts/package-app.sh` now selects the local signing identity before Tauri
builds the app and DMG. Signing a loose `.app` after the DMG has already been
created is invalid because the disk image still contains the earlier identity.
Local packaging may fall back to ad-hoc signing for diagnostics, but prints a
warning and is never used by the formal publication workflow.

## Upgrade behavior

The first correctly signed release cannot satisfy the build-specific
designated requirement of the old ad-hoc release. Users upgrading from v1.3.0
may therefore need to approve Screen & System Audio Recording and Keychain
access one final time. Subsequent releases keep the same Team ID, Developer ID
certificate lineage, and bundle identifier, allowing macOS to preserve those
grants across updates.

Changing the Team ID, bundle identifier, or signing lineage is now a security
and migration decision, not a routine packaging change.
