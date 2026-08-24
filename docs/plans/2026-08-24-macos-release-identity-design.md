# macOS GitHub release identity and permission continuity

## Status

Accepted.

## Context

Mimi is distributed through GitHub Releases without an Apple Developer Program
identity. The v1.2.0 and v1.3.0 DMGs contained app bundles with ad-hoc
signatures whose designated requirements were tied to different binary
`cdhash` values. macOS therefore could not identify an update as the same app,
so Screen & System Audio Recording grants and Keychain access could be
requested again.

The release must remain downloadable from GitHub without Apple credentials,
keep one stable app identity across future builds, keep private signing
material out of source control, and fail before publication when that identity
changes. A first-launch Gatekeeper override is an accepted limitation of this
distribution model.

## Decision

- GitHub macOS packages use one dedicated Self Signed Root code-signing
  certificate, `mimi GitHub Release`. The same certificate is reused for every
  release.
- The P12 and its password live only in the protected `github-release`
  environment secrets
  `MACOS_RELEASE_CERTIFICATE` and `MACOS_RELEASE_CERTIFICATE_PASSWORD`. CI
  imports them into an ephemeral keychain, trusts the public root only on the
  ephemeral runner, and removes that trust, the keychain, and decoded files
  before any artifact upload.
- The certificate SHA-256 fingerprint is pinned in the workflow. An accidental
  or unauthorized identity replacement stops the release instead of silently
  resetting every user's permissions.
- The app and final DMG are signed before publication. Verification requires
  the production bundle identifier, both privacy usage descriptions, a
  non-ad-hoc signature, and an exact designated requirement containing the
  pinned certificate root. The app copied into the DMG must match the loose
  build's requirement, CDHash, and version.
- The package is intentionally not notarized and is not assessed with
  Gatekeeper in CI. Users may need to choose **Open Anyway** in macOS System
  Settings on first launch; this cannot be removed without Developer ID and
  notarization.
- macOS and Windows installers remain private Actions artifacts until both
  builds pass. Publication creates or reuses a draft, uploads the exact asset
  set, verifies remote asset names and digests, rechecks the tag target, and
  makes the release public in one final step.
- Third-party Actions are pinned to reviewed commit hashes. Each platform
  stages only the exact verified installer set instead of uploading bundle
  directories by wildcard.

## Consequences

### Positive

- GitHub Releases work without an Apple developer account.
- The stable certificate and bundle identifier let later versions satisfy the
  same designated requirement, preserving privacy and Keychain identity.
- Signing material never enters the repository or public artifacts.
- A lost or replaced certificate fails visibly because its public fingerprint
  is part of the reviewed workflow.

### Negative

- The first launch from a browser download is not warning-free; Gatekeeper does
  not trust the self-signed certificate.
- The private key has no Apple revocation path. Release-environment secret
  access must remain limited, and certificate rotation is a security migration.
- GitHub Secrets cannot be exported again. Keep one encrypted offline backup of
  the P12 and password; losing both copies forces an identity migration.
- The first move from v1.3.0's ad-hoc identity to this identity requires one
  final authorization.

## Alternatives considered

- **Developer ID plus notarization:** provides warning-free installation, but
  requires Apple Developer Program credentials outside the current release
  scope. The workflow can be deliberately migrated later.
- **Ad-hoc signing:** needs no secret, but its build-specific `cdhash` repeats
  the permission-loss bug.
- **A fixed custom ad-hoc requirement:** can remain textually stable but has no
  private-key proof, so another app could impersonate an update.
- **Signing releases only on one developer Mac:** preserves identity but makes
  releases non-reproducible and dependent on one machine.

## Upgrade behavior

Users upgrading from v1.3.0 may need to approve Screen & System Audio Recording
and Keychain access one final time. Later releases keep the same certificate,
bundle identifier, and designated requirement. Changing any of them must be an
explicit migration decision.
