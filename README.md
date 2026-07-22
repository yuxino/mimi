# mimi

Live translated subtitles for macOS.

`mimi` listens to audio playing on your Mac, sends 16 kHz PCM audio to Alibaba Cloud Model Studio's realtime LiveTranslate API, and displays the original speech plus Simplified Chinese in a floating subtitle panel. It does not require a browser extension and does not capture the microphone.

## Current MVP

- Native SwiftUI menu-bar app with a Dock fallback
- ScreenCaptureKit system-audio capture
- English, Japanese, or Korean to Simplified Chinese
- Draft and confirmed bilingual subtitles
- WebSocket health checks with bounded automatic reconnects
- Movable, resizable, click-through floating subtitle window with remembered placement
- API key stored in macOS Keychain
- No analytics, accounts, recording, or cloud transcript storage in the app

## Requirements

- macOS 14 or newer
- Swift 6 command-line toolchain or Xcode
- Alibaba Cloud Model Studio Workspace ID and DashScope API key for the China (Beijing) region
- Screen & System Audio Recording permission

## Build and test

```bash
swift run mimi-core-tests
swift build
./scripts/package-app.sh
open dist/mimi.app
```

The executable test suite is intentionally dependency-free so it also works with Apple's standalone Command Line Tools installation, where XCTest may be unavailable.

The packaging script uses the `mimi Local Development` identity when it exists in the login Keychain. You can override it with `MIMI_CODESIGN_IDENTITY`; when no stable identity is available, the script falls back to ad-hoc signing. macOS may request Screen & System Audio Recording permission again after each ad-hoc rebuild because its code identity changes.

## Configure

1. Launch `mimi.app`, then use either its Dock icon or the ear icon in the menu bar.
2. Open **Settings…**.
3. Enter the Workspace ID and DashScope API key from Alibaba Cloud Model Studio.
4. Select the video's source language and save.
5. Click **Start Listening** and approve Screen & System Audio Recording access.
6. If macOS asks you to restart the app after granting permission, quit and reopen mimi.

While the subtitle position is unlocked, drag the panel background to move it and drag any edge to resize it. mimi restores the last position and size after restart. The gear button inside the subtitle panel opens Settings without requiring the menu-bar icon. Locking the subtitle position makes the panel click-through and hides its controls.

The API key is stored as a generic password in macOS Keychain under service `app.yuxino.mimi.translation`. It is never committed to the repository or included in the app bundle.

## Architecture

- `Sources/MimiCore`: provider protocol, subtitle reducer, configuration validation, PCM encoding, and session state
- `Sources/MimiApp`: ScreenCaptureKit, Keychain, menu bar, settings, and overlay window
- `Tests/MimiCoreTests`: executable unit-test harness and deterministic core tests
- `docs/plans`: validated product design and implementation plan

The realtime API integration follows Alibaba Cloud's [Qwen LiveTranslate documentation](https://help.aliyun.com/en/model-studio/qwen3-5-livetranslate-flash-realtime). System audio capture follows Apple's [ScreenCaptureKit guidance](https://developer.apple.com/documentation/screencapturekit/capturing-screen-content-in-macos).
