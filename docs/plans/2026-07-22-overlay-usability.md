# Overlay Usability Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Make mimi's subtitle overlay remember its frame, resize cleanly, remove demo subtitles, stay accessible without the menu-bar icon, and avoid false reconnect loops.

**Architecture:** Let AppKit own frame autosaving and window resizing, while SwiftUI supplies lightweight overlay controls. Expose a Dock fallback through the app delegate. Replace audio-amplitude stall inference with an explicit WebSocket ping health check.

**Tech Stack:** Swift 6.1, SwiftUI, AppKit, Foundation URLSessionWebSocketTask, Swift Package Manager.

---

### Task 1: Define testable health-check behavior

**Files:**
- Modify: `Sources/MimiCore/LiveTranslateClient.swift`
- Modify: `Tests/MimiCoreTests/RecoveryMonitorTests.swift`

1. Add a ping timeout error and a single-completion helper.
2. Add a test that verifies the new error has a useful description.
3. Run `swift run mimi-core-tests` and confirm the suite passes.

### Task 2: Replace false-positive recovery detection

**Files:**
- Modify: `Sources/MimiApp/AppModel.swift`
- Modify: `Sources/MimiApp/SystemAudioCapture.swift`

1. Replace audio activity recovery probes with periodic `LiveTranslateClient.ping()` calls.
2. Preserve bounded reconnect attempts on ping failure.
3. Build with `swift build -Xswiftc -warnings-as-errors`.

### Task 3: Persist and resize the subtitle window

**Files:**
- Modify: `Sources/MimiApp/OverlayWindowController.swift`
- Modify: `Sources/MimiApp/SubtitleOverlayView.swift`
- Modify: `Sources/MimiApp/AppModel.swift`
- Modify: `Sources/MimiApp/SettingsView.swift`

1. Enable the AppKit resizable style and set minimum/default sizes.
2. Restore and autosave the window frame under a stable name.
3. Add a settings button and resize affordance that disappear while locked.
4. Build and visually verify drag, resize, and settings access.

### Task 4: Remove demo content and add Dock access

**Files:**
- Modify: `Sources/MimiApp/AppModel.swift`
- Create: `Sources/MimiApp/AppDelegate.swift`
- Modify: `Sources/MimiApp/MimiApp.swift`
- Modify: `Resources/Info.plist`

1. Remove the UI-test subtitle injection.
2. Make mimi a regular Dock app while retaining `MenuBarExtra`.
3. Open Settings when the Dock icon is clicked with no open window.
4. Package and inspect the app bundle metadata.

### Task 5: End-to-end verification

**Files:**
- Modify: `README.md`

1. Document dragging, resizing, locking, frame restoration, and Dock access.
2. Run all tests and the release build.
3. Package and verify `plutil` and strict `codesign` checks.
4. Launch the packaged app, verify the UI flow, and restart while listening in Japanese-to-Chinese mode.
