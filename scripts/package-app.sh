#!/usr/bin/env bash
set -euo pipefail

# Release packaging for the mimi Tauri application. Produces the native
# bundles for the current platform (macOS: .app + .dmg; Windows: .msi/.nsis)
# under src-tauri/target/release/bundle/. Never commit dist/ or signing
# identities.
#
# On macOS the signing identity is selected before Tauri creates either the
# .app or .dmg. This is essential: re-signing only the loose .app afterwards
# leaves the copy already embedded in the DMG with its original identity.
# Local builds prefer the stable "mimi Local Development" identity. Public
# GitHub releases use a separate persistent self-signed identity in CI.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_DIR"

export CARGO_HOME="${CARGO_HOME:-$PROJECT_DIR/.cargo-home}"
export npm_config_cache="${npm_config_cache:-$PROJECT_DIR/.npm-cache}"
if [[ "$(uname -s)" == "Darwin" ]]; then
  export MACOSX_DEPLOYMENT_TARGET="13.0"
  APPLE_SIGNING_IDENTITY="$("$SCRIPT_DIR/codesign-identity.sh")"
  export APPLE_SIGNING_IDENTITY
  if [[ "$APPLE_SIGNING_IDENTITY" == "-" ]]; then
    echo "warning: no stable local signing identity; privacy grants will not survive rebuilds" >&2
  else
    echo "using macOS signing identity: $APPLE_SIGNING_IDENTITY"
  fi
fi

npm run tauri build

BUNDLE_DIR="$PROJECT_DIR/src-tauri/target/release/bundle"
echo "Bundle produced under: $BUNDLE_DIR"
find "$BUNDLE_DIR" -maxdepth 2 \( -name "*.app" -o -name "*.dmg" -o -name "*.msi" -o -name "*.exe" \) -print 2>/dev/null || true

if [[ "$(uname -s)" == "Darwin" ]]; then
  APP="$(find "$BUNDLE_DIR" -maxdepth 3 -name "*.app" -type d | head -1 || true)"
  [[ -n "$APP" ]] || {
    echo "macOS app bundle was not produced." >&2
    exit 1
  }
  "$SCRIPT_DIR/verify-macos-app.sh" "$APP"
fi
