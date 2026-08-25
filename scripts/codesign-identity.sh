#!/usr/bin/env bash
# Prints the unique code-signing identity for local mimi builds, or "-" when
# no stable identity exists. Selection order:
#   1. MIMI_CODESIGN_IDENTITY (explicit override)
#   2. the SHA-1 fingerprint of the one self-signed
#      "mimi Local Development" identity in the login keychain
#   3. ad-hoc "-"
#
# Ad-hoc signatures change on every build (the cdhash is derived from the
# binary), which makes macOS forget Screen & System Audio Recording grants.
# A stable identity keeps the designated requirement identical across builds
# so TCC can track the app. File-based Keychain access has an additional
# partition check; a self-signed identity without an Apple Team ID cannot
# promise password-free access across every rebuilt binary. Selecting by
# fingerprint also prevents two same-named certificates from being chosen
# nondeterministically. See
# docs/plans/2026-07-22-stable-local-signing-design.md.

set -euo pipefail

if [[ -n "${MIMI_CODESIGN_IDENTITY:-}" ]]; then
  echo "$MIMI_CODESIGN_IDENTITY"
  exit 0
fi

IDENTITY_LIST="$(security find-identity -v -p codesigning 2>/dev/null || true)"
MATCHING_IDENTITIES="$({
  printf '%s\n' "$IDENTITY_LIST" \
    | /usr/bin/sed -n \
      's/^[[:space:]]*[0-9][0-9]*) \([0-9A-Fa-f][0-9A-Fa-f]*\) "mimi Local Development"$/\1/p'
} || true)"
MATCHING_COUNT="$(
  printf '%s\n' "$MATCHING_IDENTITIES" \
    | /usr/bin/awk 'NF { count += 1 } END { print count + 0 }'
)"

case "$MATCHING_COUNT" in
  0)
    echo "-"
    ;;
  1)
    printf '%s\n' "$MATCHING_IDENTITIES" | /usr/bin/tr '[:lower:]' '[:upper:]'
    ;;
  *)
    echo "error: multiple valid code-signing identities are named mimi Local Development." >&2
    echo "Remove the duplicate identity or set MIMI_CODESIGN_IDENTITY to one fingerprint." >&2
    exit 1
    ;;
esac
