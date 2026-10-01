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
APP="${!#}"
REQUIREMENT="$TEST_REQUIREMENT"
CDHASH="${TEST_NEW_CDHASH:-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa}"
TEAM="${TEST_NEW_TEAM:-not set}"
DEVELOPER_ID="${TEST_NEW_DEVELOPER_ID:-1}"
if [[ "$APP" == */installed.app ]]; then
  REQUIREMENT="${TEST_INSTALLED_REQUIREMENT:-$TEST_REQUIREMENT}"
  CDHASH="${TEST_INSTALLED_CDHASH:-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa}"
  TEAM="${TEST_INSTALLED_TEAM:-not set}"
  DEVELOPER_ID="${TEST_INSTALLED_DEVELOPER_ID:-1}"
fi
case "$*" in
  *--verify*)
    while [[ $# -gt 0 ]]; do
      if [[ "$1" == -R ]]; then
        [[ "${2:-}" == =* ]] || exit 1
        if [[ "${2:-}" == *'anchor apple generic'* ]]; then
          exit "$DEVELOPER_ID"
        fi
      fi
      shift
    done
    exit "${TEST_VERIFY_EXIT:-0}" ;;

  *--requirements*) printf 'designated => %s\n' "$REQUIREMENT" ;;
  *) printf 'Identifier=app.yuxino.mimi\nSignature=%s\nCDHash=%s\nTeamIdentifier=%s\n' "${TEST_SIGNATURE:-signed}" "$CDHASH" "$TEAM" ;;
esac
STUB
cat > "$TEST_ROOT/bin/security" <<'STUB'
#!/usr/bin/env bash
printf '%s\n' "${TEST_IDENTITIES:-}"
STUB
chmod +x "$TEST_ROOT/bin/"*
cat > "$TEST_ROOT/bin/lipo" <<'STUB'
#!/usr/bin/env bash
printf '%s\n' "${TEST_ARCH:-arm64}"
STUB
chmod +x "$TEST_ROOT/bin/lipo"
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
"$SCRIPT_DIR/verify-macos-release-source.sh" "$TEST_ROOT/mimi.app" 1111111111111111111111111111111111111111 1.0.0 arm64 >/dev/null
expect_failure "$SCRIPT_DIR/verify-macos-release-source.sh" "$TEST_ROOT/mimi.app" 2222222222222222222222222222222222222222 1.0.0 arm64
expect_failure "$SCRIPT_DIR/verify-macos-release-source.sh" "$TEST_ROOT/mimi.app" 1111111111111111111111111111111111111111 2.0.0 arm64
expect_failure "$SCRIPT_DIR/verify-macos-release-source.sh" "$TEST_ROOT/mimi.app" 1111111111111111111111111111111111111111 1.0.0 x86_64
TEST_ARCH=x86_64 "$SCRIPT_DIR/verify-macos-release-source.sh" "$TEST_ROOT/mimi.app" 1111111111111111111111111111111111111111 1.0.0 x86_64 >/dev/null

# Same DR is not enough for a rebuilt self-signed app: the file-based Keychain
# partition may still authorize only the old binary's CDHash.
mkdir -p "$TEST_ROOT/installed.app/Contents"
cp "$TEST_ROOT/mimi.app/Contents/Info.plist" "$TEST_ROOT/installed.app/Contents/Info.plist"
INSTALL_CHECK="$SCRIPT_DIR/verify-macos-install-identity.sh"
"$INSTALL_CHECK" "$TEST_ROOT/mimi.app" "$TEST_ROOT/missing.app" >/dev/null
"$INSTALL_CHECK" "$TEST_ROOT/mimi.app" "$TEST_ROOT/installed.app" >/dev/null
CHANGED_HASH=bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb
expect_failure env TEST_NEW_CDHASH="$CHANGED_HASH" "$INSTALL_CHECK" "$TEST_ROOT/mimi.app" "$TEST_ROOT/installed.app"
expect_failure env MIMI_ALLOW_IDENTITY_CHANGE=1 TEST_NEW_CDHASH="$CHANGED_HASH" "$INSTALL_CHECK" "$TEST_ROOT/mimi.app" "$TEST_ROOT/installed.app"
# A made-up TeamIdentifier cannot substitute for an Apple-verified certificate.
expect_failure env TEST_NEW_CDHASH="$CHANGED_HASH" TEST_NEW_TEAM=ABCDEFGHIJ TEST_INSTALLED_TEAM=ABCDEFGHIJ "$INSTALL_CHECK" "$TEST_ROOT/mimi.app" "$TEST_ROOT/installed.app"
env TEST_NEW_CDHASH="$CHANGED_HASH" TEST_NEW_TEAM=ABCDEFGHIJ TEST_INSTALLED_TEAM=ABCDEFGHIJ TEST_NEW_DEVELOPER_ID=0 TEST_INSTALLED_DEVELOPER_ID=0 "$INSTALL_CHECK" "$TEST_ROOT/mimi.app" "$TEST_ROOT/installed.app" >/dev/null
expect_failure env TEST_NEW_CDHASH="$CHANGED_HASH" TEST_NEW_TEAM=ABCDEFGHIJ TEST_INSTALLED_TEAM=KLMNOPQRST TEST_NEW_DEVELOPER_ID=0 TEST_INSTALLED_DEVELOPER_ID=0 "$INSTALL_CHECK" "$TEST_ROOT/mimi.app" "$TEST_ROOT/installed.app"
expect_failure env TEST_NEW_CDHASH="$CHANGED_HASH" TEST_NEW_TEAM=ABCDEFGHIJ TEST_INSTALLED_TEAM=ABCDEFGHIJ TEST_NEW_DEVELOPER_ID=0 "$INSTALL_CHECK" "$TEST_ROOT/mimi.app" "$TEST_ROOT/installed.app"
expect_failure env TEST_NEW_CDHASH=invalid "$INSTALL_CHECK" "$TEST_ROOT/mimi.app" "$TEST_ROOT/installed.app"
OLD_REQUIREMENT='identifier "app.yuxino.mimi" and certificate root = H"0000000000000000000000000000000000000000"'
expect_failure env TEST_INSTALLED_REQUIREMENT="$OLD_REQUIREMENT" "$INSTALL_CHECK" "$TEST_ROOT/mimi.app" "$TEST_ROOT/installed.app"
env MIMI_ALLOW_IDENTITY_CHANGE=1 TEST_INSTALLED_REQUIREMENT="$OLD_REQUIREMENT" "$INSTALL_CHECK" "$TEST_ROOT/mimi.app" "$TEST_ROOT/installed.app" >"$TEST_ROOT/migration-output" 2>&1
grep -Fq 'explicitly allowing' "$TEST_ROOT/migration-output"
echo 'macOS signing safety tests passed.'
