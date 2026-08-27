# Developer-installable GitHub releases

## Status

Accepted.

## Context

Mimi is an open-source developer release and does not use Apple Developer ID,
notarization, or a dedicated private release certificate. Publication must not
be blocked on credentials that the project does not have. macOS users may need
to approve the app manually in System Settings before first launch.

Stable local signing remains useful for permission-sensitive development QA,
but it is not a requirement for downloadable GitHub artifacts.

## Decision

- Tag builds set `APPLE_SIGNING_IDENTITY=-` and produce an ad-hoc-signed Apple
  silicon app and DMG without release-signing secrets.
- Verification still checks the production bundle identifier, non-empty Screen
  & System Audio Recording usage descriptions, a valid app code signature, one
  real `mimi.app` inside the DMG, matching versions, designated requirements,
  and code-directory hashes.
- macOS and Windows artifacts remain private until both builds pass. The final
  job publishes exactly one DMG, one EXE, and one MSI after checking names,
  remote tag targets, and uploaded SHA-256 digests.
- Release notes state that the macOS package is ad-hoc signed and not notarized,
  may require **Open Anyway**, and may trigger privacy or Keychain approval
  again after an update.
- Local development continues to use `/Applications/mimi-dev.app` and the
  stable `mimi Local Development` identity so routine UI and capture testing do
  not inherit the public package's changing identity.

## Consequences

- Releases require no Apple or private signing credentials and remain
  reproducible on GitHub-hosted runners.
- The package is suitable for developers who can manually approve it, but it is
  not a notarized consumer distribution.
- Ad-hoc signatures are build-specific. macOS may not preserve Screen & System
  Audio Recording or Keychain authorization across GitHub updates.
- Installing a GitHub package over a stable local build, or the reverse, is an
  identity change and may require authorization again.
