#!/usr/bin/env bash
# Prints the code-signing identity for local mimi builds, or "-" for ad-hoc
# signing when no stable identity exists. Selection order:
#   1. MIMI_CODESIGN_IDENTITY (explicit override)
#   2. the self-signed "mimi Local Development" identity in the login keychain
#   3. ad-hoc "-"
#
# Ad-hoc signatures change on every build (the cdhash is derived from the
# binary), which makes macOS forget Screen & System Audio Recording grants and
# keychain access each time the app is rebuilt. A stable identity keeps the
# designated requirement identical across builds so those permissions are
# granted once and keep working. See docs/plans/2026-07-22-stable-local-signing-design.md.

set -euo pipefail

if [[ -n "${MIMI_CODESIGN_IDENTITY:-}" ]]; then
  echo "$MIMI_CODESIGN_IDENTITY"
  exit 0
fi

if security find-identity -v -p codesigning 2>/dev/null | grep -q "mimi Local Development"; then
  echo "mimi Local Development"
  exit 0
fi

echo "-"
