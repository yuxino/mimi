#!/usr/bin/env bash
set -euo pipefail

# Release packaging for the mimi Tauri application. Produces the native
# bundles for the current platform (macOS: .app + .dmg; Windows: .msi/.nsis)
# under src-tauri/target/release/bundle/. Never commit dist/ or signing
# identities.
#
# On macOS the produced .app is re-signed with the stable local identity
# ("mimi Local Development", see codesign-identity.sh) so Screen Recording and
# keychain grants survive rebuilds; ad-hoc signatures change every build and
# force macOS to re-ask for permissions each time. The .dmg is left as built
# by tauri (distribution with a Developer ID certificate and notarization is
# out of scope for local builds).

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_DIR"

export CARGO_HOME="${CARGO_HOME:-$PROJECT_DIR/.cargo-home}"
export npm_config_cache="${npm_config_cache:-$PROJECT_DIR/.npm-cache}"

npm run tauri build

BUNDLE_DIR="$PROJECT_DIR/src-tauri/target/release/bundle"
echo "Bundle produced under: $BUNDLE_DIR"
find "$BUNDLE_DIR" -maxdepth 2 \( -name "*.app" -o -name "*.dmg" -o -name "*.msi" -o -name "*.exe" \) -print 2>/dev/null || true

if [[ "$(uname -s)" == "Darwin" ]]; then
  IDENTITY="$("$SCRIPT_DIR/codesign-identity.sh")"
  APP="$(find "$BUNDLE_DIR" -maxdepth 3 -name "*.app" -type d | head -1 || true)"
  if [[ -n "$APP" && "$IDENTITY" != "-" ]]; then
    echo "re-signing $APP with identity \"$IDENTITY\""
    codesign --force --sign "$IDENTITY" "$APP"
    codesign --verify --deep --strict "$APP" && echo "signature verified"
  else
    echo "no stable signing identity; .app left ad-hoc signed ($IDENTITY)"
  fi
fi
