#!/usr/bin/env bash
set -euo pipefail

usage() {
  echo "Usage: $0 [--developer-release] /absolute/path/to/mimi.app" >&2
  exit 2
}

DEVELOPER_RELEASE=0
case "${1:-}" in
  --developer-release)
    DEVELOPER_RELEASE=1
    shift
    ;;
esac
[[ $# -eq 1 ]] || usage

APP="$1"
[[ "$(uname -s)" == "Darwin" ]] || {
  echo "macOS bundle verification must run on macOS." >&2
  exit 1
}
[[ "$APP" == /* && -d "$APP" ]] || {
  echo "Expected an existing absolute .app path: $APP" >&2
  exit 1
}

PLIST="$APP/Contents/Info.plist"
[[ -f "$PLIST" ]] || {
  echo "Missing Info.plist: $PLIST" >&2
  exit 1
}

read_plist() {
  /usr/libexec/PlistBuddy -c "Print :$1" "$PLIST" 2>/dev/null
}

IDENTIFIER="$(read_plist CFBundleIdentifier)"
SCREEN_USAGE="$(read_plist NSScreenCaptureUsageDescription)"
AUDIO_USAGE="$(read_plist NSAudioCaptureUsageDescription)"

[[ "$IDENTIFIER" == "app.yuxino.mimi" ]] || {
  echo "Unexpected bundle identifier: $IDENTIFIER" >&2
  exit 1
}
[[ -n "$SCREEN_USAGE" ]] || {
  echo "NSScreenCaptureUsageDescription must not be empty." >&2
  exit 1
}
[[ -n "$AUDIO_USAGE" ]] || {
  echo "NSAudioCaptureUsageDescription must not be empty." >&2
  exit 1
}

codesign --verify --deep --strict "$APP"

SIGNATURE_DETAILS="$(codesign --display --verbose=4 "$APP" 2>&1)"
REQUIREMENT="$(
  codesign --display --requirements - "$APP" 2>&1 \
    | sed -n 's/^#*[[:space:]]*designated => //p'
)"
grep -Fq "Identifier=app.yuxino.mimi" <<<"$SIGNATURE_DETAILS" || {
  echo "The signing identifier is not app.yuxino.mimi." >&2
  exit 1
}
if [[ "$DEVELOPER_RELEASE" != "1" ]] && { \
  grep -Fq "Signature=adhoc" <<<"$SIGNATURE_DETAILS" \
    || [[ -z "$REQUIREMENT" || "$REQUIREMENT" == cdhash\ * ]]; \
}; then
  echo "Ad-hoc or build-specific signatures are forbidden for mimi app bundles." >&2
  exit 1
fi

echo "Verified macOS app: $APP"
