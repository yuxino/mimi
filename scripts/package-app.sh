#!/usr/bin/env bash
set -euo pipefail

# Release packaging for the mimi Tauri application. Produces the native
# bundles for the current platform (macOS: .app + .dmg; Windows: .msi/.nsis)
# under src-tauri/target/release/bundle/. Never commit dist/ or signing
# identities.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_DIR"

export CARGO_HOME="${CARGO_HOME:-$PROJECT_DIR/.cargo-home}"
export npm_config_cache="${npm_config_cache:-$PROJECT_DIR/.npm-cache}"

npm run tauri build

BUNDLE_DIR="$PROJECT_DIR/src-tauri/target/release/bundle"
echo "Bundle produced under: $BUNDLE_DIR"
find "$BUNDLE_DIR" -maxdepth 2 \( -name "*.app" -o -name "*.dmg" -o -name "*.msi" -o -name "*.exe" \) -print 2>/dev/null || true
