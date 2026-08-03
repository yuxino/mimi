# Fast High-Quality Preview Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Show a streaming Flash translation preview quickly, then replace it with the accurate Plus final translation.

**Architecture:** Track draft generations in a small deterministic core type, run one latest-wins Flash preview worker in `HighQualityTranslationClient`, and leave the existing ordered Plus final worker unchanged. Preview events remain drafts so only Plus results enter subtitle history.

**Tech Stack:** Swift 6.1 actors and tasks, URLSession streaming, Qwen-MT Flash and Plus, Swift Package Manager.

---

### Task 1: Specify latest-draft behavior

**Files:**
- Create: `Sources/MimiCore/DraftPreviewTracker.swift`
- Create: `Tests/MimiCoreTests/DraftPreviewTrackerTests.swift`
- Modify: `Tests/MimiCoreTests/main.swift`

1. Test that repeated text does not start another request.
2. Test that a newer generation rejects an older result.
3. Test that reset invalidates in-flight work.
4. Run `swift run mimi-core-tests` and confirm the new type is missing.
5. Implement the tracker and rerun the suite.

### Task 2: Add streaming preview translation

**Files:**
- Modify: `Sources/MimiCore/HighQualityTranslationClient.swift`

1. Add a Qwen-MT Flash client with a five-second streaming timeout.
2. Queue only the newest uncommitted ASR draft.
3. Stream partial translations as `translationDraft` events.
4. Preempt a stale request after 450 milliseconds when newer text is pending.
5. Cancel preview work before local or server finalization.
6. Restore a valid next-sentence preview after an older Plus final completes.

### Task 3: Verify preview/final presentation

**Files:**
- Modify: `Tests/MimiCoreTests/SubtitleReducerTests.swift`

1. Confirm preview translations do not enter history.
2. Confirm the Plus final replaces the preview and creates exactly one history pair.
3. Run the full core suite.

### Task 4: Build and deliver

**Files:**
- Verify: all changed source, test, and plan files

1. Run `swift build -c release -Xswiftc -warnings-as-errors`.
2. Package and sign `dist/mimi.app`.
3. Launch Mimi and compare ASR draft, Flash first-token, and Plus completion timestamps in metadata-only diagnostics.
4. Commit and push the change.
