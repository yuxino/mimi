# macOS Keychain upgrade continuity

## Problem and verified scope

The local installation preflight accepted any replacement with the same full
designated requirement (DR). That was insufficient for a rebuilt self-signed
app: a dedicated synthetic Keychain item authorized A's CDHash; an atomically
replaced B had the same certificate/DR and passed dynamic signature validation,
but its read returned errSecAuthFailed. Restoring A succeeded. The item's
partition ACL contained only A's CDHash. The item was deleted afterwards.

This was a synthetic CLI experiment, not an inspection of a user's existing
credential or a complete Mimi upgrade. It establishes a risk in the signing
scheme; it does not establish the root cause of the user's previous dialog.

## Minimal safety change

`verify-macos-install-identity.sh` now rejects a changed CDHash under the same
DR unless both apps have the same Apple-verified Developer ID Application team.
It checks the certificate chain, Developer ID certificate OIDs, and the leaf
certificate's OU matching that Team ID, rather than
trusting an Authority label or a TeamIdentifier string. Reinstalling the same
signed binary remains allowed. Missing/malformed hashes fail closed.

The existing explicit certificate-migration flag remains limited to a different
DR. It does not bypass the same-DR/self-signed upgrade rejection. Migration
messages no longer promise exactly one prompt. Tests exercise the real script
with isolated codesign/security fixtures; no Keychain item/signing key is used.

This is a local replacement safety guard, **not a credential-access repair**.
It neither installs an app nor changes app/backend behavior. The public updater
does not invoke this shell preflight; signing policy and native acceptance must
still gate release separately. The existing release pin is not changed here.

## Smallest actual repair

1. Obtain a valid Developer ID Application identity/private-key pair on the
   signing Mac. The read-only inventory found none; an existing Apple
   Development identity does not establish Developer ID distribution readiness.
   Certificate issuance and account authentication are maintainer prerequisites,
   not actions performed by this patch. Private keys stay on the Mac.
2. Review the explicit migration from the existing local-root policy to an
   Apple-backed stable Team ID. Update the complete release DR, signing identity
   selection and asset/DMG/updater verification together; do not insert a fake
   Team ID or silently replace the certificate pin.
3. Keep existing profile service/account names and update existing items in
   place. Current code already caches reads and skips migration-only slots when
   the profile key exists, so another read-count change cannot repair partition
   identity. No plaintext credential fallback or global ACL changes.
4. Perform the deliberate migration with the user present. An item whose
   partition accepts only the old CDHash cannot silently accept a new Team ID;
   user authorization may be needed for each affected saved item. Recording/TCC
   is a separate grant. Exact prompt counts cannot be promised without testing
   the user's relevant history/grants. No dialog is clicked automatically.
5. On isolated synthetic items first, verify A/B app-bundle upgrades with the
   stable Developer ID team, restart/save/cancel/failure behavior, then complete
   the separately authorized normal provider/audio acceptance. Preserve old
   items on failures; do not delete/recreate them to refresh access rules.

Relevant primary sources:

- [Apple Developer ID certificates](https://developer.apple.com/help/account/certificates/create-developer-id-certificates)
- [Apple Security client partition identification](https://github.com/apple-oss-distributions/Security/blob/3dab46a11f45f2ffdbd70e2127cc5a8ce4a1f222/securityd/src/clientid.cpp)
- [Apple Security partition validation](https://github.com/apple-oss-distributions/Security/blob/3dab46a11f45f2ffdbd70e2127cc5a8ce4a1f222/securityd/src/acls.cpp)

## Other credential holders

A separate immutable credential broker could retain its exact CDHash while the
frontend changes, but migrating existing items still needs authorization. It
also needs authenticated IPC, and updating the broker itself recreates the
self-signed problem. It is larger and harder to maintain than signing the
existing app correctly, so it is not implemented as a workaround.

Moving to another Keychain backend is not a plaintext-free magic migration:
Apple identity/entitlement requirements and access to existing legacy items
still need verification. No alternate credential store is introduced here.

## Release status

Until the signing prerequisites and real upgrade acceptance are complete, #83
remains open and the release cannot promise password-free upgrade continuity.
The guard's tests passing does not change that status. No certificate/account,
credential, permission, installer, or public release is changed by this patch.
