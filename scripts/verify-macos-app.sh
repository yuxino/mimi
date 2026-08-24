#!/usr/bin/env bash
set -euo pipefail

usage() {
  echo "Usage: $0 [--github-release] /absolute/path/to/mimi.app" >&2
  exit 2
}

GITHUB_RELEASE=0
if [[ "${1:-}" == "--github-release" ]]; then
  GITHUB_RELEASE=1
  shift
fi
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

if [[ "$GITHUB_RELEASE" == "1" ]]; then
  EXPECTED_CERT_SHA1="${MIMI_EXPECTED_CERT_SHA1:-}"
  [[ "$EXPECTED_CERT_SHA1" =~ ^[0-9A-Fa-f]{40}$ ]] || {
    echo "MIMI_EXPECTED_CERT_SHA1 must be a 40-character certificate fingerprint." >&2
    exit 1
  }
  EXPECTED_CERT_SHA1="$(printf '%s' "$EXPECTED_CERT_SHA1" | tr '[:upper:]' '[:lower:]')"
  SIGNATURE_DETAILS="$(codesign --display --verbose=4 "$APP" 2>&1)"
  REQUIREMENT="$(
    codesign --display --requirements - "$APP" 2>&1 \
      | sed -n 's/^designated => //p'
  )"
  EXPECTED_REQUIREMENT="identifier \"app.yuxino.mimi\" and certificate root = H\"$EXPECTED_CERT_SHA1\""

  grep -Fq "Identifier=app.yuxino.mimi" <<<"$SIGNATURE_DETAILS" || {
    echo "The signing identifier is not app.yuxino.mimi." >&2
    exit 1
  }
  if grep -Fq "Signature=adhoc" <<<"$SIGNATURE_DETAILS"; then
    echo "Ad-hoc signatures are forbidden for GitHub releases." >&2
    exit 1
  fi
  [[ "$REQUIREMENT" == "$EXPECTED_REQUIREMENT" ]] || {
    echo "The app does not use the pinned GitHub release identity." >&2
    exit 1
  }
  codesign --verify --deep --strict -R="$EXPECTED_REQUIREMENT" "$APP"
fi

echo "Verified macOS app: $APP"
