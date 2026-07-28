# Low-Latency Translation Implementation Plan

**Goal:** Make low-latency Japanese-to-Chinese subtitles the default while retaining the current higher-quality end-to-end translation mode.

**Architecture:** Add a mode-selecting client facade. The low-latency backend streams audio to Qwen realtime ASR, translates debounced partial/final transcripts through Qwen-MT-Lite, and maps both backends into the existing subtitle event model.

**Tech Stack:** Swift 6, SwiftUI/AppKit, Foundation WebSocket and URLSession, Alibaba Cloud Model Studio realtime ASR and Qwen-MT APIs, dependency-free Swift test harness.

---

### Task 1: Add and persist translation modes

**Files:**
- Modify: `Sources/MimiCore/Models.swift`
- Modify: `Sources/MimiCore/LiveTranslationConfiguration.swift`
- Modify: `Sources/MimiApp/AppSettings.swift`
- Modify: `Sources/MimiApp/SettingsView.swift`
- Modify: `Sources/MimiApp/MenuBarView.swift`
- Modify: `Tests/MimiCoreTests/ConfigurationTests.swift`

**Steps:**

1. Add `TranslationMode.lowLatency` and `.highQuality`, defaulting configuration to low latency.
2. Persist the mode in `AppSettings` and expose a disabled-while-active picker in Settings and the menu.
3. Add configuration tests for the default and explicit modes.

### Task 2: Implement and test the realtime ASR protocol

**Files:**
- Create: `Sources/MimiCore/RealtimeASRProtocol.swift`
- Create: `Tests/MimiCoreTests/RealtimeASRProtocolTests.swift`
- Modify: `Tests/MimiCoreTests/main.swift`

**Steps:**

1. Encode the dedicated-workspace ASR endpoint, session update, 16 kHz PCM audio append, and finish requests.
2. Configure server VAD with Alibaba Cloud's low-latency preset: threshold 0.0 and 400 ms silence duration.
3. Decode source drafts as confirmed text plus stash, finals, session readiness, and errors into the existing event type.
4. Add deterministic request and event-decoding tests.

### Task 3: Implement and test Qwen-MT-Lite requests

**Files:**
- Create: `Sources/MimiCore/QwenMTProtocol.swift`
- Create: `Sources/MimiCore/QwenMTClient.swift`
- Create: `Tests/MimiCoreTests/QwenMTProtocolTests.swift`
- Modify: `Tests/MimiCoreTests/main.swift`

**Steps:**

1. Build the workspace-specific OpenAI-compatible chat-completions request for `qwen-mt-lite`.
2. Encode source/target language translation options and decode incremental SSE content.
3. Add request URL/body, complete-response, and stream-chunk tests.

### Task 4: Compose the low-latency backend

**Files:**
- Create: `Sources/MimiCore/RealtimeASRClient.swift`
- Create: `Sources/MimiCore/LowLatencyTranslationClient.swift`
- Create: `Sources/MimiCore/TranslationClient.swift`
- Modify: `Sources/MimiApp/AppModel.swift`

**Steps:**

1. Implement the realtime ASR WebSocket with connect, audio, ping, finish, and disconnect operations.
2. Debounce partial transcripts, stream translation drafts, translate finals immediately, cancel superseded requests, and suppress stale responses.
3. Select the low-latency or high-quality backend through `TranslationClient` without changing the reducer or recovery pipeline.
4. Ensure stop/reconnect cancels all socket and translation tasks.

### Task 5: Verify and package

**Files:**
- Modify if needed: `README.md`

**Steps:**

1. Run `swift run mimi-core-tests` and fix every failure.
2. Run `swift build -c release -Xswiftc -warnings-as-errors`.
3. Package `dist/mimi.app`, verify its property list and stable code signature.
4. Launch with the saved Japanese configuration and confirm low-latency partial subtitles appear and update.
5. Commit and push the verified change to `codex/macos-mvp`.
