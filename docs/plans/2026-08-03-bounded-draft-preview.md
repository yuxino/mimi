# Bounded Draft Preview Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Limit streaming draft subtitles to the two most recent readable segments without truncating final history.

**Architecture:** Extend the pure subtitle segmenter with a suffix-window function, then use it only for non-final current translations in the SwiftUI overlay. All stored state and final translation behavior stay unchanged.

**Tech Stack:** Swift 6.1, SwiftUI, Swift Package Manager.

---

### Task 1: Specify the display window

**Files:**
- Modify: `Tests/MimiCoreTests/SubtitleTextSegmenterTests.swift`
- Modify: `Sources/MimiCore/SubtitleTextSegmenter.swift`

1. Add a failing test that requests the last two segments of a long draft.
2. Add a failing test that keeps a short draft intact.
3. Run `swift run mimi-core-tests` and confirm the API is missing.
4. Implement the suffix-window function and rerun tests.

### Task 2: Apply only to draft translations

**Files:**
- Modify: `Sources/MimiApp/SubtitleOverlayView.swift`

1. Keep full segmentation for final current lines.
2. Use the last two segments for non-final current lines.
3. Leave history segmentation unchanged.

### Task 3: Verify and deliver

**Files:**
- Verify: changed source, tests, and plan files

1. Run the complete core test suite.
2. Run the warnings-as-errors release build.
3. Run the UI test build.
4. Package and sign `dist/mimi.app`.
5. Commit and push the change.
