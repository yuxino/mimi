#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 2 || "$1" != /* || "$2" != /* ]]; then
  echo "Usage: $0 /absolute/path/to/mimi.app /absolute/path/to/mimi.dmg" >&2
  exit 2
fi

APP="$1"
DMG="$2"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

[[ -f "$DMG" ]] || {
  echo "Missing DMG: $DMG" >&2
  exit 1
}

EXPECTED_CERT_SHA1="${MIMI_EXPECTED_CERT_SHA1:-}"
[[ "$EXPECTED_CERT_SHA1" =~ ^[0-9A-Fa-f]{40}$ ]] || {
  echo "MIMI_EXPECTED_CERT_SHA1 must be a 40-character certificate fingerprint." >&2
  exit 1
}
EXPECTED_CERT_SHA1="$(printf '%s' "$EXPECTED_CERT_SHA1" | tr '[:upper:]' '[:lower:]')"

"$SCRIPT_DIR/verify-macos-app.sh" --github-release "$APP"

codesign --verify --strict "$DMG"
DMG_SIGNATURE="$(codesign --display --verbose=4 "$DMG" 2>&1)"
if grep -Fq "Signature=adhoc" <<<"$DMG_SIGNATURE"; then
  echo "The DMG has an ad-hoc signature." >&2
  exit 1
fi
DMG_REQUIREMENT="$(
  codesign --display --requirements - "$DMG" 2>&1 \
    | sed -n 's/^designated => //p'
)"
[[ "$DMG_REQUIREMENT" == *" and certificate root = H\"$EXPECTED_CERT_SHA1\"" ]] || {
  echo "The DMG does not use the pinned GitHub release identity." >&2
  exit 1
}
codesign --verify --strict -R="certificate root = H\"$EXPECTED_CERT_SHA1\"" "$DMG"

MOUNT_DIR="$(mktemp -d "${TMPDIR%/}/mimi-release-dmg.XXXXXX")"
cleanup_mount() {
  hdiutil detach "$MOUNT_DIR" -quiet >/dev/null 2>&1 || true
  rmdir "$MOUNT_DIR" >/dev/null 2>&1 || true
}
trap cleanup_mount EXIT

hdiutil attach "$DMG" -nobrowse -readonly -mountpoint "$MOUNT_DIR" >/dev/null
EMBEDDED_APPS=()
while IFS= read -r -d '' candidate; do
  EMBEDDED_APPS+=("$candidate")
done < <(find "$MOUNT_DIR" -maxdepth 2 -name '*.app' -type d -print0)
[[ "${#EMBEDDED_APPS[@]}" == "1" ]] || {
  echo "The DMG must contain exactly one app bundle." >&2
  exit 1
}
EMBEDDED_APP="${EMBEDDED_APPS[0]}"
[[ "$(basename "$EMBEDDED_APP")" == "mimi.app" && -d "$EMBEDDED_APP" && ! -L "$EMBEDDED_APP" ]] || {
  echo "The DMG contains an unexpected app bundle." >&2
  exit 1
}
"$SCRIPT_DIR/verify-macos-app.sh" --github-release "$EMBEDDED_APP"

designated_requirement() {
  codesign --display --requirements - "$1" 2>&1 \
    | sed -n 's/^designated => //p'
}

LOOSE_REQUIREMENT="$(designated_requirement "$APP")"
EMBEDDED_REQUIREMENT="$(designated_requirement "$EMBEDDED_APP")"
[[ -n "$LOOSE_REQUIREMENT" && "$LOOSE_REQUIREMENT" == "$EMBEDDED_REQUIREMENT" ]] || {
  echo "The DMG contains an app with a different designated requirement." >&2
  exit 1
}

code_directory_hash() {
  codesign --display --verbose=4 "$1" 2>&1 \
    | sed -n 's/^CDHash=//p' \
    | head -1
}

LOOSE_CDHASH="$(code_directory_hash "$APP")"
EMBEDDED_CDHASH="$(code_directory_hash "$EMBEDDED_APP")"
[[ -n "$LOOSE_CDHASH" && "$LOOSE_CDHASH" == "$EMBEDDED_CDHASH" ]] || {
  echo "The DMG does not contain the app from this build." >&2
  exit 1
}

LOOSE_VERSION="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$APP/Contents/Info.plist")"
EMBEDDED_VERSION="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$EMBEDDED_APP/Contents/Info.plist")"
[[ -n "$LOOSE_VERSION" && "$LOOSE_VERSION" == "$EMBEDDED_VERSION" ]] || {
  echo "The DMG contains an app with a different version." >&2
  exit 1
}

cleanup_mount
trap - EXIT

echo "Verified stable-identity macOS GitHub release: $DMG"
