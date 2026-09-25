#!/usr/bin/env bash
# Produce release assets on the maintainer Mac; never export the signing key.
set -euo pipefail
[[ $# -eq 0 && "$(uname -s)" == Darwin ]] || {
  echo "Usage: ./scripts/prepare-macos-release.sh (on the signing Mac)" >&2
  exit 2
}
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd -P)"
cd "$SCRIPT_DIR/.."
[[ -z "$(git status --porcelain)" ]] || {
  echo "Commit the reviewed release source before preparing signed assets." >&2
  exit 1
}
export APPLE_SIGNING_IDENTITY="$(tr -d '[:space:]' < scripts/macos-release-identity.txt)"
[[ "$APPLE_SIGNING_IDENTITY" =~ ^[0-9A-F]{40}$ ]] || exit 1
export MACOSX_DEPLOYMENT_TARGET=13.0
export CARGO_HOME="${CARGO_HOME:-$PWD/.cargo-home}"
export npm_config_cache="${npm_config_cache:-$PWD/.npm-cache}"
[[ "$(uname -m)" == arm64 ]] || {
  echo "The current public macOS release target is Apple silicon." >&2
  exit 1
}
TEMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/mimi-release-config.XXXXXX")"
trap 'rm -rf "$TEMP_DIR"' EXIT
REVISION="$(git rev-parse HEAD)"
python3 - "$TEMP_DIR" "$REVISION" <<'PY'
import json, pathlib, plistlib, sys
folder = pathlib.Path(sys.argv[1])
with open('src-tauri/Info.plist', 'rb') as f:
    info = plistlib.load(f)
info['MimiSourceRevision'] = sys.argv[2]
with (folder / 'Info.plist').open('wb') as f:
    plistlib.dump(info, f)
(folder / 'tauri.conf.json').write_text(json.dumps({
    'bundle': {'createUpdaterArtifacts': False, 'macOS': {'infoPlist': str(folder / 'Info.plist')}}
}))
PY
npm run tauri -- build --config "$TEMP_DIR/tauri.conf.json" -- --locked
VERSION="$(node -p 'require("./package.json").version')"
BUNDLE="$PWD/src-tauri/target/release/bundle"
APP="$BUNDLE/macos/mimi.app"
DMG="$BUNDLE/dmg/mimi_${VERSION}_aarch64.dmg"
./scripts/verify-macos-release.sh "$APP" "$DMG"
[[ "$(/usr/libexec/PlistBuddy -c 'Print :MimiSourceRevision' "$APP/Contents/Info.plist")" == "$REVISION" ]]
# CI applies the existing updater signature only after verifying this app.
# Disable AppleDouble metadata; all required bundle files are ordinary files.
COPYFILE_DISABLE=1 tar -czf "$BUNDLE/macos/mimi.app.tar.gz" -C "$BUNDLE/macos" mimi.app
python3 scripts/extract-macos-updater.py "$BUNDLE/macos/mimi.app.tar.gz" "$TEMP_DIR/extracted"
./scripts/verify-macos-release-source.sh "$TEMP_DIR/extracted/mimi.app" "$REVISION" "$VERSION"
./scripts/verify-macos-release.sh "$TEMP_DIR/extracted/mimi.app" "$DMG"
cat <<END
Verified local release assets for v$VERSION at $REVISION:
  $DMG
  $BUNDLE/macos/mimi.app.tar.gz
Follow docs/development/macos-release-signing.md to stage the draft before
pushing its release tag. This script does not upload, publish, or install.
END
