#!/usr/bin/env bash
set -euo pipefail

# Dev launcher for the mimi Tauri application on macOS.
#
# Runs the app the way the release does — a real .app bundle with the
# original icon.icns — so macOS renders the Dock icon with the standard
# rounded mask, identical to the release app. Like kiri's build-app.sh, this
# builds a RELEASE binary (the Tauri dev-mode runtime icon override, which
# replaces the Dock icon with an unmasked square in debug builds, does not
# run in release). The wrapped app keeps the "app.yuxino.mimi.dev" bundle id,
# and window titles / the tray tooltip carry a "(dev)" marker (see
# windows::dev_title) so it is easy to tell apart from the release app.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_DIR"

export CARGO_HOME="${CARGO_HOME:-$PROJECT_DIR/.cargo-home}"
export npm_config_cache="${npm_config_cache:-$PROJECT_DIR/.npm-cache}"

# Stop any previous dev instance.
pkill -f "mimi-dev.app/Contents/MacOS/mimi" 2>/dev/null || true
pkill -f "target/debug/mimi" 2>/dev/null || true

# Build the frontend (a release binary loads the bundled dist/, not vite).
npm run build

# Build the release binary the way `tauri build` does. The custom-protocol
# feature is what flips Tauri's `cfg(dev)` off: without it, Tauri treats even
# a release build as dev and replaces the Dock icon at runtime with an
# unmasked square (the launch icon is correct, then it flips square once the
# app runs).
cargo build --release --features tauri/custom-protocol --manifest-path src-tauri/Cargo.toml

# Assemble the .app wrapper around the release binary.
APP="$PROJECT_DIR/src-tauri/target/release/mimi-dev.app"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$PROJECT_DIR/src-tauri/target/release/mimi" "$APP/Contents/MacOS/mimi"
cp "$PROJECT_DIR/src-tauri/icons/icon.icns" "$APP/Contents/Resources/icon.icns"

cat > "$APP/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key>
  <string>mimi dev</string>
  <key>CFBundleIdentifier</key>
  <string>app.yuxino.mimi.dev</string>
  <key>CFBundleInfoDictionaryVersion</key>
  <string>6.0</string>
  <key>LSApplicationCategoryType</key>
  <string>public.app-category.utilities</string>
  <key>CFBundleVersion</key>
  <string>dev</string>
  <key>CFBundleExecutable</key>
  <string>mimi</string>
  <key>CFBundleDisplayName</key>
  <string>mimi dev</string>
  <key>LSRequiresCarbon</key>
  <true/>
  <key>NSHighResolutionCapable</key>
  <true/>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>CFBundleIconFile</key>
  <string>icon.icns</string>
  <key>CSResourcesFileMapped</key>
  <true/>
  <key>LSMinimumSystemVersion</key>
  <string>10.15</string>
  <key>CFBundleDevelopmentRegion</key>
  <string>English</string>
  <key>CFBundleShortVersionString</key>
  <string>1.0.0-dev</string>
</dict>
</plist>
PLIST

# Sign with the stable local identity when available so Screen Recording /
# keychain grants survive rebuilds (see codesign-identity.sh).
IDENTITY="$("$SCRIPT_DIR/codesign-identity.sh")"
codesign --force --sign "$IDENTITY" "$APP" 2>/dev/null || true

echo "launching $APP"
open "$APP"
