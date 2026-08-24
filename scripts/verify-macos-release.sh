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

"$SCRIPT_DIR/verify-macos-app.sh" --release "$APP"

codesign --verify --strict "$DMG"
DMG_SIGNATURE="$(codesign --display --verbose=4 "$DMG" 2>&1)"
grep -Eq '^Authority=Developer ID Application: ' <<<"$DMG_SIGNATURE" || {
  echo "The DMG is not signed with Developer ID Application." >&2
  exit 1
}
grep -Fq "TeamIdentifier=${MIMI_EXPECTED_TEAM_ID:-missing}" <<<"$DMG_SIGNATURE" || {
  echo "The DMG is not signed by the expected Apple team." >&2
  exit 1
}

xcrun stapler validate "$DMG"
spctl --assess --type open --context context:primary-signature --verbose=4 "$DMG"

MOUNT_DIR="$(mktemp -d "${TMPDIR%/}/mimi-release-dmg.XXXXXX")"
cleanup_mount() {
  hdiutil detach "$MOUNT_DIR" -quiet >/dev/null 2>&1 || true
  rmdir "$MOUNT_DIR" >/dev/null 2>&1 || true
}
trap cleanup_mount EXIT

hdiutil attach "$DMG" -nobrowse -readonly -mountpoint "$MOUNT_DIR" >/dev/null
EMBEDDED_APP="$(find "$MOUNT_DIR" -maxdepth 2 -name '*.app' -type d -print -quit)"
[[ -n "$EMBEDDED_APP" ]] || {
  echo "The DMG does not contain an app bundle." >&2
  exit 1
}
"$SCRIPT_DIR/verify-macos-app.sh" --release "$EMBEDDED_APP"

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

echo "Verified signed and notarized macOS release: $DMG"
