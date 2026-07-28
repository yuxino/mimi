#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
APP_DIR="$PROJECT_DIR/dist/mimi.app"
CONTENTS_DIR="$APP_DIR/Contents"
MACOS_DIR="$CONTENTS_DIR/MacOS"
RESOURCES_DIR="$CONTENTS_DIR/Resources"
LOCAL_SIGNING_IDENTITY="mimi Local Development"

cd "$PROJECT_DIR"
swift build -c release --product mimi

rm -rf "$APP_DIR"
mkdir -p "$MACOS_DIR" "$RESOURCES_DIR"
cp "$PROJECT_DIR/.build/release/mimi" "$MACOS_DIR/mimi"
cp "$PROJECT_DIR/Resources/Info.plist" "$CONTENTS_DIR/Info.plist"
cp "$PROJECT_DIR/Resources/mimi.icns" "$RESOURCES_DIR/mimi.icns"
chmod 755 "$MACOS_DIR/mimi"

plutil -lint "$CONTENTS_DIR/Info.plist" >/dev/null
SIGNING_IDENTITY="${MIMI_CODESIGN_IDENTITY:-}"
if [[ -z "$SIGNING_IDENTITY" ]]; then
    if security find-identity -v -p codesigning | /usr/bin/grep -Fq "\"$LOCAL_SIGNING_IDENTITY\""; then
        SIGNING_IDENTITY="$LOCAL_SIGNING_IDENTITY"
    else
        SIGNING_IDENTITY="-"
    fi
fi

codesign --force --deep --timestamp=none --sign "$SIGNING_IDENTITY" "$APP_DIR"
codesign --verify --deep --strict "$APP_DIR"

echo "Signed with: $SIGNING_IDENTITY"
echo "$APP_DIR"
