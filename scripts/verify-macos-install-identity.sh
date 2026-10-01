#!/usr/bin/env bash
set -euo pipefail

# Fail-closed preflight for replacing an installed macOS formal app. A stable
# designated requirement is necessary, but a rebuilt self-signed app can still
# have a different file-based Keychain partition (its CDHash).

usage() {
  echo "Usage: ./scripts/verify-macos-install-identity.sh NEW_APP INSTALLED_APP" >&2
  exit 2
}

[[ $# -eq 2 ]] || usage
[[ "$(uname -s)" == "Darwin" ]] || {
  echo "macOS install identity checks must run on macOS." >&2
  exit 1
}

NEW_APP="$1"
INSTALLED_APP="$2"

case "$NEW_APP" in
  /*.app) ;;
  *) usage ;;
esac
case "$INSTALLED_APP" in
  /*.app) ;;
  *) usage ;;
esac

[[ -d "$NEW_APP" && ! -L "$NEW_APP" ]] || {
  echo "Expected a real new app bundle: $NEW_APP" >&2
  exit 1
}

bundle_identifier() {
  /usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' \
    "$1/Contents/Info.plist" 2>/dev/null
}

designated_requirement() {
  codesign --display --requirements - "$1" 2>&1 \
    | /usr/bin/sed -n 's/^#*[[:space:]]*designated => //p'
}

signing_field() {
  codesign --display --verbose=4 "$1" 2>&1 \
    | /usr/bin/sed -n "s/^$2=//p"
}

has_developer_id() {
  # Verify the Apple certificate chain, not a displayed Authority name or a
  # TeamIdentifier that a self-signed build could set without Apple issuance.
  local requirement='anchor apple generic and certificate 1[field.1.2.840.113635.100.6.2.6] exists and certificate leaf[field.1.2.840.113635.100.6.1.13] exists'
  codesign --verify --strict -R "=$requirement" "$1" >/dev/null 2>&1
}

"$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/verify-macos-app.sh" "$NEW_APP"
NEW_IDENTIFIER="$(bundle_identifier "$NEW_APP")"
[[ "$NEW_IDENTIFIER" == "app.yuxino.mimi" ]] || {
  echo "The new app is not the formal mimi bundle: $NEW_IDENTIFIER" >&2
  exit 1
}
NEW_REQUIREMENT="$(designated_requirement "$NEW_APP")"

if [[ ! -e "$INSTALLED_APP" ]]; then
  echo "No installed formal app exists; identity migration is not applicable."
  echo "New designated requirement: $NEW_REQUIREMENT"
  exit 0
fi
[[ -d "$INSTALLED_APP" && ! -L "$INSTALLED_APP" ]] || {
  echo "Installed app path is not a real app bundle: $INSTALLED_APP" >&2
  exit 1
}
codesign --verify --deep --strict "$INSTALLED_APP"
INSTALLED_IDENTIFIER="$(bundle_identifier "$INSTALLED_APP")"
[[ "$INSTALLED_IDENTIFIER" == "$NEW_IDENTIFIER" ]] || {
  echo "Bundle identifiers differ; refusing to replace the installed app." >&2
  echo "Installed: $INSTALLED_IDENTIFIER" >&2
  echo "New:       $NEW_IDENTIFIER" >&2
  exit 1
}
INSTALLED_REQUIREMENT="$(designated_requirement "$INSTALLED_APP")"

if [[ "$INSTALLED_REQUIREMENT" == "$NEW_REQUIREMENT" ]]; then
  NEW_CDHASH="$(signing_field "$NEW_APP" CDHash)"
  INSTALLED_CDHASH="$(signing_field "$INSTALLED_APP" CDHash)"
  [[ "$NEW_CDHASH" =~ ^[[:xdigit:]]{40}$ && "$INSTALLED_CDHASH" =~ ^[[:xdigit:]]{40}$ ]] || {
    echo "Cannot verify both CodeDirectory hashes; refusing replacement." >&2
    exit 1
  }
  if [[ "$NEW_CDHASH" == "$INSTALLED_CDHASH" ]]; then
    echo "Same signed binary identity; existing OS grants still require native acceptance."
    exit 0
  fi
  NEW_TEAM="$(signing_field "$NEW_APP" TeamIdentifier)"
  INSTALLED_TEAM="$(signing_field "$INSTALLED_APP" TeamIdentifier)"
  if [[ "$NEW_TEAM" =~ ^[A-Z0-9]{10}$ && "$NEW_TEAM" == "$INSTALLED_TEAM" ]] \
    && has_developer_id "$NEW_APP" && has_developer_id "$INSTALLED_APP"; then
    echo "Stable Developer ID team and designated requirement; native upgrade acceptance is still required."
    exit 0
  fi
  cat >&2 <<EOF
error: the designated requirement matches, but the rebuilt binary does not
have a verified stable Developer ID team for Keychain continuity.

Installed CDHash: $INSTALLED_CDHASH
New CDHash:       $NEW_CDHASH

Existing file-based Keychain items can authorize only the old CDHash. Refusing
formal replacement; use the isolated dev app for testing. A deliberate move to
Developer ID needs a reviewed signing policy and user-managed migration. Do not
delete credentials, widen ACLs, or fabricate a Team ID to bypass this check.
EOF
  exit 1
fi

if [[ "${MIMI_ALLOW_IDENTITY_CHANGE:-0}" == "1" ]]; then
  cat >&2 <<EOF
warning: explicitly allowing a macOS code-identity migration.

Installed: ${INSTALLED_REQUIREMENT:-<invalid or unsigned>}
New:       ${NEW_REQUIREMENT:-<invalid or unsigned>}

Screen & System Audio Recording and saved Keychain items may require new
authorization. The number of prompts depends on the existing grants/items;
this does not guarantee future continuity for a self-signed identity.
Do not use this override for routine local testing.
EOF
  exit 0
fi

cat >&2 <<EOF
error: refusing to replace mimi with a different macOS code identity.

Installed: ${INSTALLED_REQUIREMENT:-<invalid or unsigned>}
New:       ${NEW_REQUIREMENT:-<invalid or unsigned>}

The replacement can require new Screen Recording and Keychain authorization.
Use ./scripts/dev-app.sh for pre-push testing. A deliberate
certificate migration requires one explicit run with MIMI_ALLOW_IDENTITY_CHANGE=1.
EOF
exit 1
