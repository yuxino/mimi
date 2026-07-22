# Automatic Session Recovery Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Automatically rebuild a stalled live-translation session when a new video produces audible system audio but no subtitle events.

**Architecture:** Add testable PCM activity detection and a timestamp-based recovery monitor to `MimiCore`. Feed throttled audio activity and server-event activity into the monitor from the macOS app, then reconnect in place when the monitor declares a stall.

**Tech Stack:** Swift 6, SwiftUI/AppKit, ScreenCaptureKit, Foundation WebSocket, dependency-free Swift test harness.

---

### Task 1: Test and implement audio activity detection

**Files:**
- Create: `Sources/MimiCore/PCM16AudioActivityDetector.swift`
- Create: `Tests/MimiCoreTests/RecoveryMonitorTests.swift`
- Modify: `Tests/MimiCoreTests/main.swift`

**Steps:**

1. Add failing tests showing empty/silent PCM is inactive, low-amplitude noise is inactive, and normal speech-like PCM is active.
2. Run `swift run mimi-core-tests` and verify the new symbols are missing.
3. Implement RMS detection over little-endian signed 16-bit samples with a configurable threshold.
4. Rerun the test harness and verify the detector tests pass.

### Task 2: Test and implement the recovery monitor

**Files:**
- Create: `Sources/MimiCore/TranslationRecoveryMonitor.swift`
- Modify: `Tests/MimiCoreTests/RecoveryMonitorTests.swift`

**Steps:**

1. Add failing tests for no recovery during silence, no recovery after recent server traffic, recovery after active audio plus an eight-second server stall, and cooldown suppression.
2. Run `swift run mimi-core-tests` and verify the monitor tests fail.
3. Implement timestamp recording, the recent-audio window, stall timeout, and recovery cooldown using deterministic `TimeInterval` inputs.
4. Rerun the harness and verify all monitor tests pass.

### Task 3: Integrate activity and automatic reconnection

**Files:**
- Modify: `Sources/MimiApp/SystemAudioCapture.swift`
- Modify: `Sources/MimiApp/AppModel.swift`

**Steps:**

1. Add an activity callback to `SystemAudioCapture`, using the detector and throttling callbacks to at most twice per second.
2. Start a one-second watchdog when a session connects; record every server event and audio-activity callback.
3. When the monitor requests recovery, stop capture, disconnect, retain current subtitles, reconnect using the active settings, and prevent overlapping recoveries.
4. Cancel the watchdog on manual Stop and surface connection failures through the existing error status.

### Task 4: Verify, package, and publish

**Files:**
- Modify if needed: `README.md`

**Steps:**

1. Run `swift run mimi-core-tests`; expect all tests to pass.
2. Run `swift build -c release -Xswiftc -warnings-as-errors`; expect a clean build.
3. Run `bash scripts/package-app.sh`, `plutil -lint`, and strict `codesign --verify`.
4. Launch mimi with the saved Japanese configuration, verify normal live translation, then simulate a stale session in tests and visually confirm the Connecting/Listening transition.
5. Commit the feature, push `codex/macos-mvp`, and update draft PR #1 with the new behavior and validation.
