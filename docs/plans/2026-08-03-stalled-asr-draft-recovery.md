# Stalled ASR Draft Recovery Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Translate audible speech even when Audio 3 does not promptly mark an interim transcript as a finished sentence.

**Architecture:** Add a pure incremental draft committer to remove already translated prefixes and suppress punctuation-only tails. Drive it from two cancellable timers in `HighQualityTranslationClient`: a short stability debounce and a maximum continuous-speech deadline.

**Tech Stack:** Swift 6.1, actors and structured concurrency, Swift Package Manager, Qwen Audio 3 ASR, Qwen-MT Plus.

---

### Task 1: Specify incremental commit behavior

**Files:**
- Create: `Sources/MimiCore/ASRDraftCommitter.swift`
- Create: `Tests/MimiCoreTests/ASRDraftCommitterTests.swift`
- Modify: `Tests/MimiCoreTests/main.swift`

1. Add failing tests for first commit, exact late-final deduplication, suffix extraction, punctuation-only suppression, and reset.
2. Run `swift run mimi-core-tests` and confirm the new type is missing.
3. Implement the smallest state machine that satisfies those cases.
4. Run the core tests and confirm they pass.

### Task 2: Add timed fallback finalization

**Files:**
- Modify: `Sources/MimiCore/HighQualityTranslationClient.swift`

1. Track the latest draft language and two cancellable tasks.
2. Restart a 1.2-second stability timer on every changed draft.
3. Start one 4.5-second maximum-wait timer for each uncommitted chunk.
4. Queue a local commit through the existing final translation worker when either timer wins.
5. Cancel timers and remove already committed text when a server final arrives.
6. Reset all draft state on connect, finish, and disconnect.

### Task 3: Make future incidents observable

**Files:**
- Modify: `Sources/MimiCore/PipelineDiagnostics.swift`
- Modify: `Sources/MimiCore/HighQualityTranslationClient.swift`

1. Enable metadata-only pipeline diagnostics by default with an environment opt-out.
2. Log boundary reason and text length, never transcript or translation content.
3. Audit every pipeline diagnostic call for content safety.

### Task 4: Verify and deliver

**Files:**
- Verify: all changed source, tests, and plan files

1. Run `swift run mimi-core-tests`.
2. Run `swift build -c release -Xswiftc -warnings-as-errors`.
3. Package and sign `dist/mimi.app`.
4. Restart Mimi and inspect metadata-only logs for audio, ASR draft/final, local fallback, and MT completion.
5. Commit and push the fix.
