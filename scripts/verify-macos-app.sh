#!/usr/bin/env bash
set -euo pipefail

usage() {
  echo "Usage: $0 [--release] /absolute/path/to/mimi.app" >&2
  exit 2
}

RELEASE=0
if [[ "${1:-}" == "--release" ]]; then
  RELEASE=1
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

if [[ "$RELEASE" == "1" ]]; then
  EXPECTED_TEAM_ID="${MIMI_EXPECTED_TEAM_ID:-}"
  [[ -n "$EXPECTED_TEAM_ID" ]] || {
    echo "MIMI_EXPECTED_TEAM_ID is required for formal release verification." >&2
    exit 1
  }
  SIGNATURE_DETAILS="$(codesign --display --verbose=4 "$APP" 2>&1)"
  REQUIREMENT="$(codesign --display --requirements - "$APP" 2>&1)"

  grep -Fq "Identifier=app.yuxino.mimi" <<<"$SIGNATURE_DETAILS" || {
    echo "The signing identifier is not app.yuxino.mimi." >&2
    exit 1
  }
  grep -Eq '^Authority=Developer ID Application: ' <<<"$SIGNATURE_DETAILS" || {
    echo "A Developer ID Application signature is required." >&2
    exit 1
  }
  grep -Fq "TeamIdentifier=$EXPECTED_TEAM_ID" <<<"$SIGNATURE_DETAILS" || {
    echo "The app is not signed by the expected Apple team." >&2
    exit 1
  }
  if grep -Fq "Signature=adhoc" <<<"$SIGNATURE_DETAILS"; then
    echo "Ad-hoc signatures are forbidden for formal macOS releases." >&2
    exit 1
  fi
  if grep -Fq "designated => cdhash" <<<"$REQUIREMENT"; then
    echo "The designated requirement is tied to one binary build." >&2
    exit 1
  fi
  grep -Fq 'identifier "app.yuxino.mimi"' <<<"$REQUIREMENT" || {
    echo "The designated requirement does not bind the bundle identifier." >&2
    exit 1
  }
  grep -Fq 'anchor apple generic' <<<"$REQUIREMENT" || {
    echo "The designated requirement is not anchored to Apple." >&2
    exit 1
  }
  grep -Eq "certificate leaf\\[subject\\.OU\\] = \"?${EXPECTED_TEAM_ID}\"?" <<<"$REQUIREMENT" || {
    echo "The designated requirement does not bind the expected Apple team." >&2
    exit 1
  }

  xcrun stapler validate "$APP"
  spctl --assess --type execute --verbose=4 "$APP"
fi

echo "Verified macOS app: $APP"
