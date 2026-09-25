#!/usr/bin/env bash
# Regression coverage without accessing any Keychain item or signing key.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd -P)"
TEST_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/mimi-signing-test.XXXXXX")"
trap 'rm -rf "$TEST_ROOT"' EXIT
mkdir -p "$TEST_ROOT/bin" "$TEST_ROOT/mimi.app/Contents"
cat > "$TEST_ROOT/mimi.app/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0"><dict>
<key>CFBundleIdentifier</key><string>app.yuxino.mimi</string>
<key>NSScreenCaptureUsageDescription</key><string>System audio</string>
<key>NSAudioCaptureUsageDescription</key><string>System audio</string>
<key>CFBundleShortVersionString</key><string>1.0.0</string>
<key>MimiSourceRevision</key><string>1111111111111111111111111111111111111111</string>
</dict></plist>
PLIST
cat > "$TEST_ROOT/bin/codesign" <<'STUB'
#!/usr/bin/env bash
case "$*" in
  *--verify*)
    while [[ $# -gt 0 ]]; do
      if [[ "$1" == -R ]]; then
        [[ "${2:-}" == =* ]] || exit 1
      fi
      shift
    done
    exit "${TEST_VERIFY_EXIT:-0}" ;;

  *--requirements*) printf 'designated => %s\n' "$TEST_REQUIREMENT" ;;
  *) printf 'Identifier=app.yuxino.mimi\nSignature=%s\n' "${TEST_SIGNATURE:-signed}" ;;
esac
STUB
cat > "$TEST_ROOT/bin/security" <<'STUB'
#!/usr/bin/env bash
printf '%s\n' "${TEST_IDENTITIES:-}"
STUB
chmod +x "$TEST_ROOT/bin/"*
export PATH="$TEST_ROOT/bin:$PATH"
PIN="$(tr -d '[:space:]' < "$SCRIPT_DIR/macos-release-identity.txt")"
export TEST_REQUIREMENT="identifier \"app.yuxino.mimi\" and certificate root = H\"${PIN}\""
expect_failure() {
  if "$@" >"$TEST_ROOT/output" 2>&1; then
    echo "Expected rejection: $*" >&2
    exit 1
  fi
}
"$SCRIPT_DIR/verify-macos-app.sh" --release "$TEST_ROOT/mimi.app" >/dev/null
expect_failure env TEST_VERIFY_EXIT=1 "$SCRIPT_DIR/verify-macos-app.sh" --release "$TEST_ROOT/mimi.app"
expect_failure env TEST_SIGNATURE=adhoc "$SCRIPT_DIR/verify-macos-app.sh" --release "$TEST_ROOT/mimi.app"
expect_failure env TEST_REQUIREMENT='cdhash H"123"' "$SCRIPT_DIR/verify-macos-app.sh" --release "$TEST_ROOT/mimi.app"
expect_failure env TEST_REQUIREMENT='identifier "app.yuxino.mimi" and certificate root = H"0000000000000000000000000000000000000000"' "$SCRIPT_DIR/verify-macos-app.sh" --release "$TEST_ROOT/mimi.app"
expect_failure env TEST_REQUIREMENT="$TEST_REQUIREMENT and cdhash H\"123\"" "$SCRIPT_DIR/verify-macos-app.sh" --release "$TEST_ROOT/mimi.app"
expect_failure env MIMI_CODESIGN_IDENTITY=- "$SCRIPT_DIR/codesign-identity.sh"
expect_failure env MIMI_CODESIGN_IDENTITY= TEST_IDENTITIES= "$SCRIPT_DIR/codesign-identity.sh"
export TEST_IDENTITIES="  1) $PIN \"mimi Local Development\""
[[ "$(MIMI_CODESIGN_IDENTITY= "$SCRIPT_DIR/codesign-identity.sh")" == "$PIN" ]]
expect_failure env MIMI_CODESIGN_IDENTITY= TEST_IDENTITIES="$TEST_IDENTITIES
  2) $PIN \"mimi Local Development\"" "$SCRIPT_DIR/codesign-identity.sh"
"$SCRIPT_DIR/verify-macos-release-source.sh" "$TEST_ROOT/mimi.app" 1111111111111111111111111111111111111111 1.0.0 >/dev/null
expect_failure "$SCRIPT_DIR/verify-macos-release-source.sh" "$TEST_ROOT/mimi.app" 2222222222222222222222222222222222222222 1.0.0
expect_failure "$SCRIPT_DIR/verify-macos-release-source.sh" "$TEST_ROOT/mimi.app" 1111111111111111111111111111111111111111 2.0.0
echo 'macOS signing safety tests passed.'
