# Pause and Reliable Translation Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add fast pause/resume controls and ensure transient Plus failures never silently drop a confirmed subtitle.

**Architecture:** `AppModel` owns the user pause lifecycle because it already coordinates capture, client, recovery, and overlay visibility. `TranslationSessionController` clears translation activity without discarding subtitle content. `HighQualityTranslationClient` uses a pure retry policy plus cancellable capped backoff while preserving strict sentence order.

**Tech Stack:** Swift 6.1, SwiftUI, AppKit, Swift concurrency, Swift Package Manager.

---

### Task 1: Specify pause and retry behavior with tests

**Files:**
- Modify: `Tests/MimiCoreTests/SessionControllerTests.swift`
- Modify: `Tests/MimiCoreTests/QwenMTProtocolTests.swift`

**Step 1:** Add a session-controller test asserting that pausing clears `isTranslationPending` while preserving status and subtitles.

**Step 2:** Add retry-policy tests for timeouts, 429/5xx, authentication, and ordinary 4xx responses.

**Step 3:** Run `swift run mimi-core-tests` and verify the new tests fail because the pause hook and retry policy do not exist.

### Task 2: Implement the pause lifecycle

**Files:**
- Modify: `Sources/MimiCore/SessionController.swift`
- Modify: `Sources/MimiApp/AppModel.swift`

**Step 1:** Add `didPause()` to clear only the pending translation activity.

**Step 2:** Add published `isPaused`, `pause()`, `resume(using:)`, and `togglePaused(using:)` behavior to `AppModel`.

**Step 3:** On pause, stop health/recovery work, stop capture and audio sending, disconnect the client, preserve settings/subtitles, and keep the overlay visible.

**Step 4:** On resume, reconnect with `clearSubtitles: false`; only clear `isPaused` once resume begins, and restore it if reconnecting fails.

**Step 5:** Run the core tests and a warnings-as-errors Release build.

### Task 3: Add accessible expanded and compact controls

**Files:**
- Modify: `Sources/MimiApp/SubtitleOverlayView.swift`

**Step 1:** Add a paused activity phase with static amber styling and `已暂停` status text.

**Step 2:** Add pause/play controls to the expanded top-right control group and compact bar.

**Step 3:** Disable language switching while paused and provide `暂停翻译` / `继续翻译` help and accessibility labels.

**Step 4:** Build with warnings as errors.

### Task 4: Keep transient translation failures in-order

**Files:**
- Modify: `Sources/MimiCore/HighQualityTranslationClient.swift`
- Modify: `Sources/MimiCore/QwenMTClient.swift`
- Modify: `Tests/MimiCoreTests/QwenMTProtocolTests.swift`

**Step 1:** Add a pure public retry-delay policy that classifies Qwen-MT client errors and returns capped exponential backoff delays only for transient failures.

**Step 2:** Replace one-retry-plus-empty-final behavior with cancellable retry-until-success behavior for transient failures.

**Step 3:** Treat non-retryable failures as visible terminal translation errors; never emit an empty final.

**Step 4:** Run all core tests and the warnings-as-errors Release build.

### Task 5: Package and verify the real app

**Files:**
- Verify: `dist/mimi.app`

**Step 1:** Run `./scripts/package-app.sh` and strict code-signature verification.

**Step 2:** Launch an isolated UI-test instance and verify expanded pause, compact pause, resume, preserved subtitle content, reduced-motion-friendly state, and accessibility labels.

**Step 3:** Restart the normal app, verify the expected executable path and that it starts unpaused.

**Step 4:** Review the complete diff, commit the implementation, push `codex/show-auto-detected-language`, and report the exact test and runtime results.
