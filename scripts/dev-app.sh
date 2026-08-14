#!/usr/bin/env bash
set -euo pipefail

# Dev launcher for the mimi Tauri application on macOS.
#
# `npm run tauri dev` runs the bare debug binary, which has no .app bundle:
# macOS then cannot render its Dock icon with the standard rounded mask, and
# runtime icon overrides (setApplicationIconImage) are not reflected by the
# Dock either. This script instead wraps the freshly built debug binary in a
# real .app bundle (with the original icon.icns) and launches that, so the
# dev app's Dock icon is masked and rendered exactly like the release app.
# Window titles and the tray tooltip additionally carry a "(dev)" marker
# (see windows::dev_title) so the two mimi apps are easy to tell apart.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_DIR"

export CARGO_HOME="${CARGO_HOME:-$PROJECT_DIR/.cargo-home}"
export npm_config_cache="${npm_config_cache:-$PROJECT_DIR/.npm-cache}"

# Stop any previous dev instance (bare binary or wrapped app).
pkill -f "mimi-dev.app/Contents/MacOS/mimi" 2>/dev/null || true
pkill -f "target/debug/mimi" 2>/dev/null || true

# Start the vite dev server (serves the frontend at the tauri devUrl) unless
# something already listens on it.
if ! curl -sf http://localhost:1420/ >/dev/null 2>&1; then
  echo "starting vite dev server..."
  npm run dev > /tmp/mimi-vite.log 2>&1 &
  VITE_PID=$!
  for _ in $(seq 1 30); do
    if curl -sf http://localhost:1420/ >/dev/null 2>&1; then
      break
    fi
    sleep 1
  done
  curl -sf http://localhost:1420/ >/dev/null 2>&1 || {
    echo "vite dev server did not come up; see /tmp/mimi-vite.log" >&2
    kill "$VITE_PID" 2>/dev/null || true
    exit 1
  }
  echo "vite dev server ready (pid $VITE_PID)"
fi

# Build the debug binary.
cargo build --manifest-path src-tauri/Cargo.toml

# Assemble the .app wrapper around the debug binary.
APP="$PROJECT_DIR/src-tauri/target/debug/mimi-dev.app"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$PROJECT_DIR/src-tauri/target/debug/mimi" "$APP/Contents/MacOS/mimi"
cp "$PROJECT_DIR/src-tauri/icons/icon.icns" "$APP/Contents/Resources/icon.icns"

cat > "$APP/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key>
  <string>mimi dev</string>
  <key>CFBundleDisplayName</key>
  <string>mimi dev</string>
  <key>CFBundleIdentifier</key>
  <string>app.yuxino.mimi.dev</string>
  <key>CFBundleExecutable</key>
  <string>mimi</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>CFBundleIconFile</key>
  <string>icon.icns</string>
  <key>CFBundleShortVersionString</key>
  <string>1.0.0-dev</string>
  <key>CFBundleVersion</key>
  <string>dev</string>
  <key>LSMinimumSystemVersion</key>
  <string>10.15</string>
  <key>NSHighResolutionCapable</key>
  <true/>
</dict>
</plist>
PLIST

# Ad-hoc sign so the system accepts the wrapper (arm64 binaries need a
# signature to run; the debug binary itself is already ad-hoc signed).
codesign --force --sign - "$APP" 2>/dev/null || true

echo "launching $APP"
open "$APP"
