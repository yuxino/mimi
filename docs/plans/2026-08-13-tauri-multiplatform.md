# mimi Tauri 多平台迁移实施计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 把 Swift/macOS 版 mimi 完全迁移为 Tauri v2 桌面应用（Rust + React/TS），1:1 保留功能与 UI，支持 macOS 与 Windows，验证通过后删除 Swift 源码。

**Architecture:** 见 `docs/plans/2026-08-13-tauri-multiplatform-design.md`。Rust 侧逐文件移植 MimiCore/MimiApp 逻辑（纯逻辑带测试），前端三窗口（Overlay / TrayPanel / Settings）复刻 SwiftUI 视图。前后端通过固定 IPC 契约通信。

**Tech Stack:** Tauri v2、Rust 2024 + tokio、React 19 + TypeScript + Tailwind 3.4 + Vite 7、screencapturekit 1.x（macOS 音频）、cpal 0.16 + rubato（Windows 环回采集）、tokio-tungstenite、reqwest、keyring 3、tauri-plugin-global-shortcut、tauri-plugin-positioner。

**关键约定：**
- 开发期间保留 Swift 源码作为对照参考（git 历史亦有存档）；最终任务在所有验证通过后删除。
- 所有命令、事件、载荷结构以本文档「IPC 契约」一节为准，前后端并行开发不得偏离。
- 遵守 AGENTS.md 约束：只采系统音频；不落盘音频/字幕；凭证只进系统钥匙串（keyring），无明文/环境变量回退；诊断日志只含计时、计数、语言码、状态码、错误标签。
- 每个 Rust 纯逻辑模块 = 先移植原测试（TDD）→ 再移植实现 → 跑测试 → 提交。

---

## IPC 契约（前后端唯一接口基准）

窗口名：`overlay`（浮窗字幕）、`tray-panel`（托盘面板）、`settings`（设置窗）。

### 命令（前端 → Rust，`invoke`）

| 命令 | 参数 | 返回 |
| --- | --- | --- |
| `session_start` | 无（用当前已保存设置） | `void`（抛错则带错误消息） |
| `session_stop` | 无 | `void` |
| `session_toggle_paused` | 无 | `void` |
| `session_clear_subtitles` | 无 | `void` |
| `session_switch_source_language` | `{ language: "ja" \| "en" \| "ko" \| "zh" }` | `void` |
| `session_switch_translation_mode` | `{ mode: "lowLatency" \| "highQuality" \| "turbo" }` | `void` |
| `settings_get` | 无 | `SettingsSnapshot` |
| `settings_save` | `SettingsDraft` | `SettingsSnapshot`（校验失败抛错，错误消息同 Swift 版文案） |
| `overlay_set_collapsed` | `{ collapsed: boolean }` | `void` |
| `overlay_set_locked` | `{ locked: boolean }` | `void` |
| `overlay_show` | 无 | `void` |
| `overlay_set_size` | `{ width: number, height: number }` | `void`（前端自绘缩放手柄用） |
| `tray_panel_hide` | 无 | `void` |
| `app_quit` | 无 | `void` |
| `app_show_settings` | 无 | `void` |

### 事件（Rust → 前端，`listen`）

`session-state`：任何会话状态变化后广播全量快照（对应 AppModel.publishState）：

```ts
interface SessionStateEvent {
  status: { kind: "idle" } | { kind: "connecting" } | { kind: "listening" }
        | { kind: "stopping" } | { kind: "error"; message: string };
  isActive: boolean;
  isPaused: boolean;
  isOverlayCollapsed: boolean;
  subtitles: SubtitleSnapshot;
  detectedLanguage: string | null;          // "zh" | "ja" | "en" | "ko" | ...
  isTranslationPending: boolean;
}
interface SubtitleSnapshot {
  source: { text: string; isFinal: boolean };
  translation: { text: string; isFinal: boolean };
  history: Array<{ source: string; translation: string; createdAt: number /* epoch ms */ }>;
}
```

`settings-changed`：设置变更后广播：

```ts
interface SettingsSnapshot {
  workspaceID: string;
  hasAPIKey: boolean;                 // 永不回传 key 明文
  sourceLanguage: "auto" | "zh" | "en" | "ja" | "ko";
  targetLanguage: "original" | "zh" | "en" | "ja";
  translationMode: "lowLatency" | "highQuality" | "turbo";
  fontSize: number;                   // 14..20
  isOverlayLocked: boolean;
  credentialLoadError: string | null;
}
interface SettingsDraft {
  workspaceID?: string;
  apiKey?: string;                    // 仅在 settings_save 时传输；其余接口不出现
  sourceLanguage?: "auto" | "zh" | "en" | "ja" | "ko";
  targetLanguage?: "original" | "zh" | "en" | "ja";
  translationMode?: "lowLatency" | "highQuality" | "turbo";
  fontSize?: number;
  isOverlayLocked?: boolean;
}
```

窗口可见性规则（Rust 侧负责）：会话进入活动态 → 显示 overlay；非活动态（idle/error）→ 隐藏 overlay；托盘图标点击 → 切换 tray-panel 显隐；`overlay_show` 仅 listening 时有效。

---

## Phase 0：脚手架

### Task 1: 创建 Tauri v2 + React + TS 项目骨架

**Files:** Create `package.json`、`vite.config.ts`、`tsconfig.json`、`tsconfig.app.json`、`tsconfig.node.json`、`index.html`、`tailwind.config.js`、`postcss.config.js`、`src/main.tsx`、`src/index.css`、`src-tauri/Cargo.toml`、`src-tauri/tauri.conf.json`、`src-tauri/build.rs`、`src-tauri/src/main.rs`、`src-tauri/src/lib.rs`、`src-tauri/capabilities/default.json`、`.gitignore`（更新）。

**Step 1: 初始化**
- `package.json` deps：`@tauri-apps/api@^2`、`@tauri-apps/cli@^2`(dev)、`react@^19`、`react-dom@^19`、`tailwindcss@^3.4`、`vite@^7`、`@vitejs/plugin-react`、`typescript@~5.9`、`zustand@^5`、`vitest`(dev)、`@tauri-apps/plugin-global-shortcut`(js 绑定，仅 types)。
- `tauri.conf.json` 关键内容：
  - `identifier: "app.yuxino.mimi"`，`productName: "mimi"`。
  - `app.windows`: 初始创建 `settings`（width 560, height 570, resizable false, title "mimi 设置"）；`overlay` 与 `tray-panel` 由 Rust 运行时创建（不列入静态 windows，或列入但 `visible:false`）。**采用运行时创建**：`overlay`：`{transparent:true, decorations:false, alwaysOnTop:true, skipTaskbar:true, shadow:false, resizable:true, visible:false}`；`tray-panel`：`{decorations:false, alwaysOnTop:true, skipTaskbar:true, resizable:false, visible:false, width:290}`。
  - `bundle`: `active:true`，icon 数组先指向占位（Task 32 换正式图标），`macOS.minimumSystemVersion: "10.15"`。
- `src-tauri/Cargo.toml` deps：`tauri = { version = "2", features = ["tray-icon", "image-png"] }`、`tauri-build`、`tauri-plugin-global-shortcut = "2"`、`tauri-plugin-positioner = "2"`、`serde`、`serde_json`、`tokio = { version = "1", features = ["full"] }`、`tokio-tungstenite = { version = "0.26", features = ["rustls-tls-native-roots"] }`、`futures-util`、`reqwest = { version = "0.12", default-features = false, features = ["rustls-tls-native-roots", "json", "stream"] }`、`base64`、`uuid = { version = "1", features = ["v4"] }`、`keyring = "3"`、`tracing`、`tracing-subscriber`、`thiserror`、`cpal = "0.16"`、`rubato`、`anyhow`。`[target.'cfg(target_os = "macos")'.dependencies] screencapturekit = "1.5"`。
- `capabilities/default.json`：`core:default` + `core:event:default` + `core:window:allow-start-dragging`、`core:window:allow-set-size`、`core:window:allow-set-position`、`core:window:allow-show/hide/close`、`core:window:allow-set-ignore-cursor-events`、`core:window:allow-set-focus`、`core:window:allow-set-always-on-top`；`global-shortcut:allow-register`、`global-shortcut:allow-unregister`、`global-shortcut:allow-is-registered`；`positioner:default`；窗口列表 `["settings","overlay","tray-panel"]`。

**Step 2: 验证**
Run: `npm install && npm run tauri dev`
Expected: 弹出一个 560×570 的空设置窗口；无编译错误。

**Step 3: Commit** `git add -A && git commit -m "chore: scaffold tauri v2 app"`

---

## Phase 1：Rust 核心纯逻辑（对照 Swift 逐文件移植，TDD）

所有模块放 `src-tauri/src/core/`。每条任务：先写测试（移植对应 Swift 测试），跑红，实现，跑绿，`cargo fmt`，提交。

### Task 2: models.rs

**Files:** Create `src-tauri/src/core/models.rs`、`src-tauri/src/core/mod.rs`；Test: `src-tauri/src/core/models.rs` 内 `#[cfg(test)]`。

**Port:** `Sources/MimiCore/Models.swift` 全量：
- `SourceLanguage`（auto/zh/en/ja/ko；`manualCases` 顺序 ja,en,ko,zh；`from_detected` 归一化 "zh-"/"chinese"/"mandarin" 等；`display_name`；`status_display_name`（自动识别中 / 自动识别（XX）逻辑，注意 targetLanguage==zh 且 detected==zh 时显示"自动识别中"）；`target_language_after_quick_switch`）。
- `DetectedLanguage`（code 归一化取主语言段；display_name 全表，default 大写 code）。
- `TargetLanguage`（original/zh/en/ja；display_name；`translates_audio`）。
- `TranslationMode`（lowLatency/highQuality/turbo；display_name）。
- `SessionStatus`（idle/connecting/listening/stopping/error(String)；`is_active`）。
- `SubtitleLine`、`SubtitlePair`（Eq 忽略 createdAt）、`SubtitleSnapshot`、`SubtitleEvent`。

**Step 1: 测试**（移植 `Tests/MimiCoreTests/ConfigurationTests.swift` 中语言相关部分 + 各 displayName 断言 + isActive 断言）
**Step 2:** `cargo test -p mimi` 红。**Step 3:** 实现。**Step 4:** 绿。**Step 5:** Commit `feat(core): port models from Swift`.

### Task 3: configuration.rs

**Port:** `LiveTranslationConfiguration.swift`：字段 + `effective_translation_mode`（turbo 直接返回；auto 源语言 → lowLatency）+ `validated()`（trim；workspaceID 非空；正则 `^[A-Za-z0-9][A-Za-z0-9-]{1,126}[A-Za-z0-9]$`；apiKey 非空）。错误消息逐字：`"Add your Alibaba Cloud Model Studio Workspace ID in Settings."` / `"The Workspace ID is not valid. Copy it from Alibaba Cloud Model Studio."` / `"Add your Alibaba Cloud Model Studio API key in Settings."`。

**Step 1:** 测试（移植 ConfigurationTests 全量：缺 workspaceID / 非法 workspaceID / 缺 key / auto→lowLatency / turbo 保真 / trim 生效）。
**Step 2:** 红。**Step 3:** 实现。**Step 4:** 绿。**Step 5:** Commit。

### Task 4: subtitle_reducer.rs

**Port:** `SubtitleReducer.swift`（maxHistoryCount=20；sourceDraft/final、translationDraft（空白 draft 不覆盖已确认 final）、translationFinal（pendingFinalSources 队列配对入历史、去重、截断 20）、revokeLastConfirmed、clear；trim 用 Unicode 空白）。

**Step 1:** 测试（移植 SubtitleReducerTests 全量 + 去重/上限用例）。**Step 2–5:** 同上，Commit。

### Task 5: segmenter.rs

**Port:** `SubtitleTextSegmenter.swift`：`segments(in:maximumCharacters:)`（句界集 `。！？!?；;\n`；优先断点 `，、,：:—–- `；minBreak=maxC/2；无句界按优先断点倒扫，空白断点不吞字符；段 trim）与 `visible_draft_segments`（suffix 2）。字符按 Unicode 标量迭代。

**Step 1:** 测试（移植 SubtitleTextSegmenterTests 全量）。**Step 2–5:** Commit。

### Task 6: committer.rs

**Port:** `ASRDraftCommitter.swift` 全量语义（sentenceDelimiters `。！？.!?\n`；updateDraft → pending；commitCompleteSentences 只提交完整句、记录 lastCommittedChunk + provisional 标记；commitLatestDraft(commitLongIncomplete) 长尾兜底提交（阈值 20）；finishSentence 的 none/appended/replaced 三分支含 supersedes 前缀/包含判定、suffixOverlap 倒序匹配；isMeaningful 需含非空白非标量标点）。这是字幕去重正确性的关键，测试必须逐条对应 Swift 用例。

**Step 1:** 测试（移植 ASRDraftCommitterTests 全量）。**Step 2–5:** Commit。

### Task 7: preview_tracker.rs

**Port:** `DraftPreviewTracker.swift`：update 返回 generation（空文本/相同文本返回 None）、accepts 要求 generation==当前且 text==currentText、reset 清空并 generation+1。

**Step 1:** 测试（移植 DraftPreviewTrackerTests）。**Step 2–5:** Commit。

### Task 8: pcm16.rs

**Port:** `PCM16Encoder.swift`：多声道 f32 平均混音 → clamp [-1,1] → i16 LE（正样本 `(x*32767).round()`，负样本 `(x*32768).round()`，与 Swift 浮点取整语义一致：round half away from zero）。

**Step 1:** 测试（移植 PCMConversionTests：全零、正满幅=32767、负满幅=-32768、半幅、多声道混音、空输入）。**Step 2–5:** Commit。

### Task 9: diagnostics.rs

**Port:** `PipelineDiagnostics.swift`：`log!` 宏（tracing info；环境变量 `MIMI_PIPELINE_DIAGNOSTICS=0` 关闭）、`error_label(&dyn Error)`（映射已知错误 → "QwenMTClientError.requestFailed(status=429)" 风格标签）、毫秒计时助手。内容只含计时/计数/语言码/状态码。

**Step 1:** 测试（error_label 各分支）。**Step 2–5:** Commit。

### Task 10: protocols/live_translate.rs

**Port:** `LiveTranslateProtocol.swift` + `RealtimeASRProtocol.swift`：
- `LiveTranslateEndpoint::new(workspaceID)`：`wss://{workspaceID}.cn-beijing.maas.aliyuncs.com/api-ws/v1/realtime?model=qwen3.5-livetranslate-flash-realtime`，workspaceID 正则校验，错误 `invalidWorkspaceID/invalidEndpoint`。
- 编码器：`session_update(source_language, target_language, hotwords)`、`audio_append(base64)`、`finish()`，`event_id` 形如 `event_{32hex}`，JSON 键名 snake_case，序列化时键排序（BTreeMap / 手工构造 serde_json::Value 保证字节级一致可断言）。
- 服务端事件解码 `LiveTranslateServerEvent`（session.created/updated/finished、conversation.item.input_audio_transcription.text（text+stash 合并）、.completed（transcript）、response.text.text / response.audio_transcript.text → translationDraft(combined)、response.text.done / response.audio_transcript.done → translationFinal(trim)、error{code,message}、未知类型 ignored）。

**Step 1:** 测试（移植 LiveTranslateProtocolTests + Audio3ASRProtocolTests 中端点部分：URL 构造、session.update JSON 快照断言、audio_append base64 断言、各事件解码含 text+stash 合并、error 事件、ignored）。
**Step 2–5:** Commit。

### Task 11: protocols/audio3.rs

**Port:** `Audio3ASRProtocol.swift`：
- `Audio3ASREndpoint`：`wss://{workspaceID}.cn-beijing.maas.aliyuncs.com/api-ws/v1/inference`（无 query）。
- `run_task`（header{action:"run-task",task_id,streaming:"duplex"} + payload{task_group:"audio",task:"asr",function:"recognition",model:"qwen-audio-3.0-asr-flash-streaming",parameters{format:"pcm",sample_rate:16000,language_hints?,semantic_punctuation_enabled:true,heartbeat:true},input{context?[user 消息{role,content:[{type:"input_text",text}]}]}}）、`finish_task`。
- 解码：task-started/task-finished/task-failed{error_code,error_message}/result-generated{payload.output.sentence{text,sentence_end,heartbeat}} → subtitle_event(source_language) 映射。
- `Audio3ASRContext::audiovisual_dialogue` 五种语言文案**逐字**移植（含 auto）。

**Step 1:** 测试（移植 Audio3ASRProtocolTests 全量 + context 文案断言）。**Step 2–5:** Commit。

### Task 12: protocols/qwen_mt.rs

**Port:** `QwenMTProtocol.swift` 全量：
- `QwenMTEndpoint`：`https://{workspaceID}.cn-beijing.maas.aliyuncs.com/compatible-mode/v1/chat/completions`。
- `QwenMTModel`（lite/flash/plus）、`QwenMTRequestEncoder::request`（model、messages[{role:"user",content:text}]、stream、translation_options{source_lang:"auto"/"Chinese"/"English"/"Japanese"/"Korean",target_lang:"Chinese"/"English"/"Japanese",domains?,terms?,tm_list?}）。
- `QwenMTDomainHint::spoken_dialogue(source,target)` 与 `fillerTerms` 全部文案与对照表**逐字**移植（这是翻译质量核心资产，禁止改写）。实现为 `match (source, target)` 返回 `&'static str` / 静态表。
- 解码：非流式 choices[0].message.content（trim 后空 → missingTranslation）；SSE 行 `data: ` 前缀 → delta.content 累积，`[DONE]` 结束。
- 错误枚举与消息（missingAPIKey / invalidHTTPResponse / requestTimedOut / requestFailed{status,message} / invalidJSON / missingTranslation / invalidWorkspaceID / invalidEndpoint）+ `is_authentication_failure`（401/403/missingKey）。
- `QwenMTRetryPolicy::delay(error, attempt)`：transient = timeout/invalidHTTPResponse/408/429/>=500；delay = min(8000, 600 << min(attempt-1,4)) ms。

**Step 1:** 测试（移植 QwenMTProtocolTests 全量：请求 JSON 快照、tm_list、流式解码、错误路径、重试策略表）。
**Step 2–5:** Commit。

### Task 13: session.rs

**Port:** `SessionController.swift`：`TranslationSessionState{status,subtitles,detected_language,is_translation_pending}` + 状态迁移（beginConnecting/didConnect/didPause/beginStopping/didStop/didFail/clearSubtitles）+ `handle(event)`（stopping 时丢弃事件；sessionUpdated→didConnect；sourceDraft/Final 更新检测语言 + reducer；translationStarted→pending=true；translationFinal→pending=false；subtitleRevoked→revoke；error→didFail）。结构体纯函数风格，无 IO。

**Step 1:** 测试（移植 SessionControllerTests 全量：连接序列、draft/final 推进、stopping 丢弃、clear、error）。**Step 2–5:** Commit。

---

## Phase 2：网络客户端（tokio）

### Task 14: clients/ws.rs 通用 WSS 传输

**Files:** Create `src-tauri/src/clients/mod.rs`、`ws.rs`。

实现：`connect(endpoint, headers) -> (WsWriter, WsReader)`；writer 支持 text/binary/ping；reader 是 `async_stream`（Message::Text/Binary/Ping/Pong/Close）；断开 → `transport_error` 语义由调用方处理。用 `tokio_tungstenite::connect_async` + `rustls`；`Authorization: Bearer {key}`；连接超时 15s。测试：仅测 URL/请求头构造（网络测试不进单测）。

### Task 15: clients/live_translate_client.rs（低延迟模式）

**Port:** `LiveTranslateClient.swift`：connect 发送 session.update 后启动接收循环；`send_audio`（空数据跳过）；`ping`（4s 超时）；`finish`（发 session.finish，最多等 2s session.finished）；`disconnect`。接收循环把 WSS 消息 decode 成 `LiveTranslateServerEvent` 交给回调；`sessionFinished` 置位；错误 → `transport_error` 事件后终止。回调为 `async Fn`（tokio 任务内发 mpsc）。

### Task 16: clients/audio3_client.rs

**Port:** `Audio3ASRClient.swift`：run-task → 等 task-started（10s）；`send_audio` 发二进制帧；ping 4s；finish（finish-task → 等 task-finished 3s）；task-failed → terminal error；User-Agent `mimi-tauri`；错误映射同 Swift。

### Task 17: clients/qwen_mt_client.rs

**Port:** `QwenMTClient.swift`：`translate`（POST，plus 超时 30s / 其他 10s，非 2xx 解析 error.message）；`translate_streaming`（SSE 解析、整体 8s 超时（turbo 模式 5s）、partial 回调、空结果 → missingTranslation）；重试策略独立函数（Task 12 已测）。

### Task 18: clients/high_quality_client.rs（核心：整条高质量流水线）

**Port:** `HighQualityTranslationClient.swift` 全部语义：
- 构造参数（finalModel plus/flash、stableDraftDelay 1200ms、maximumWaitDelay 4500ms、longIncompleteCommitThreshold 20/12、streamsFinals = finalModel != plus）。
- handleASREvent：sourceDraft → draftCommitter.updateDraft → 定时器（stable/maximum-wait，不重复创建）→ translatesAudio=false 时 draft 即原文；否则 finalWorker 空闲才显示原文 draft。sourceFinal → 取消定时器与草稿翻译 → finishSentence 三分支 → replaced 时（翻译模式）pendingRevokeCount+=1 或（原文模式）立即 emit subtitleRevoked → enqueueConfirmedSource。
- commitPendingDraft(boundary)（stable-draft / maximum-wait / session-finish）。
- 草稿翻译 worker：DraftPreviewTracker 代际、450ms 抢占旧请求、完成回调 emit sourceDraft+translationDraft（要求 pending==None 且 accepts）。
- final worker：串行队列，emit translationStarted + sourceFinal → translateWithRetry（重试策略、sourceOverride=检测语言、tm 记忆 suffix 6）→ 需要时先 emit subtitleRevoked → translationFinal → remember（12 条上限）→ restoreActiveDraftPreview。
- finish：取消草稿 → asrClient.finish → commitPendingDraft(session-finish) → 等 final worker ≤35s。
- 诊断日志逐条对照 Swift（长度/队列深度/耗时/错误标签，不含文本内容）。
- 所有任务取消语义用 `tokio_util::sync::CancellationToken` 或 `JoinHandle.abort` 对应 Swift Task.cancel。

### Task 19: clients/translation_client.rs

**Port:** `TranslationClient.swift`：按 effectiveTranslationMode 分派 lowLatency / highQuality(plus) / turbo(highQuality with flash + 500ms/2000ms/阈值12)。统一 `connect/send_audio/ping/finish/disconnect` trait（`enum Backend`）。

---

## Phase 3：双平台音频采集

### Task 20: audio/macos.rs（screencapturekit）

**Files:** Create `src-tauri/src/audio/mod.rs`、`macos.rs`、`send_pipeline.rs`。

**Port:** `SystemAudioCapture.swift` + `AudioSendPipeline`：
- `SCShareableContent` 取 displays（主显示器优先）→ `SCContentFilter`（display + 排除本应用 bundleID）→ `SCStreamConfiguration`{capturesAudio:true, excludesCurrentProcessAudio:true, sampleRate:16000, channelCount:1, minimumFrameInterval:1, queueDepth:3} → `addStreamOutput(.audio)` → `startCapture`。
- 停止时序（2026-08-14 修订）：SCStream 与其 output handler 必须保留到 `stopCapture` 的完成回调触发后才能释放；过早释放（调用后立即 drop）会留下仍在投递的采集会话、handler 读已释放内存（use-after-free，症状为"stop 后解码日志继续刷"）。release 全部移入完成回调；主线程派发失败必须打日志（`run_on_main_thread` 的 Err 不得吞掉）。
- 音频回调：`did_output_sample_buffer` → 提取 f32 样本（多声道则混音）→ PCM16 → mpsc 发往 send pipeline；>500ms 间隔记 capture gap。
- `send_pipeline`：`mpsc::channel(20)` + `try_send`，满则丢最新并触发 fell-behind 错误（对应 bufferingNewest(20) 的 dropped 语义）；发送阻塞 >200ms 记日志；每 1/100 帧记 buffers/bytes/peakDbFS。
- 停止：stopCapture → removeStreamOutput。
- `#[cfg(target_os = "macos")]`；Windows 构建下本模块编译为空。

**Step 1:** 验证本机：`cargo check`（无法在单测中跑真实采集；跑一次 `npm run tauri dev` 手动触发授权弹窗）。
**Step 2:** Commit。

### Task 21: audio/windows.rs（cpal WASAPI loopback + rubato）

**Files:** Create `audio/windows.rs`。

实现：枚举输入设备，找到默认输出设备对应的 loopback 输入（WASAPI loopback 设备名带 `[Loopback]` 或通过 `default_output_device` 关联；实现为：取 `host.default_output_device()` 的名称/id，在 input_devices 中匹配同名/loopback 变体）。`build_input_stream` 原始格式（如 f32 48kHz 2ch）→ 回调内先用 ring buffer 攒帧 → `rubato` 重采样 16000Hz + 声道平均 → PCM16 → mpsc。设备切换处理：错误回调上报（同 macOS didStopWithError）。`#[cfg(target_os = "windows")]`。

**Step 1:** `cargo check --target x86_64-pc-windows-msvc`（先 `rustup target add x86_64-pc-windows-msvc`）编译级验证。
**Step 2:** Commit。

---

## Phase 4：应用层（Tauri 集成）

### Task 22: settings_store.rs（偏好 + keyring）

**Files:** Create `src-tauri/src/settings_store.rs`。

**Port:** `AppSettings.swift` + `KeychainStore.swift`：
- `Preferences{workspace_id, source_language, target_language, translation_mode, font_size, overlay_locked, overlay_frame?, frame_layout_version}` ↔ `app_config_dir()/preferences.json`（serde）。
- API Key 走 `keyring`，主 service `app.yuxino.mimi.credentials.v3`（本应用自建条目，默认 ACL 永不弹授权），account=`dashscope-api-key`；读取顺序 v3 → v2（`app.yuxino.mimi.credentials.v2`，首版 Tauri 复用、带 Swift 遗留 ACL）→ 旧 service `app.yuxino.mimi.translation`，旧条目读成功后一次性迁移写入 v3（对应 Swift 的 legacyService 迁移）。2026-08-14 修订：Swift 原条目带 partition-list ACL 绑定创建二进制，dev 重建 cdhash 变化导致每次重启都弹钥匙串授权、并发 settings_get 各自阻塞饿死 tokio 运行时（全窗口冻结）——故凭证改为写入自建 v3 条目并做内存缓存（并发调用共享单次读取）。错误消息 `Keychain: …` / Windows `Credential Manager: …` 风格透传。
- 默认值：source=auto、target=zh、mode=highQuality（auto 时降为 lowLatency）、fontSize=18、locked=false。
- `prepare_for_listening`（auto→ja；zh 源 → original 目标）、`save()` 校验 + 写 keyring + 写偏好、`reload_api_key`。
- `MIMI_UI_TEST=1` 环境变量注入假凭证（同 Swift UITest 模式，供 UI 冒烟）。

**Step 1:** 测试：偏好 roundtrip、校验错误消息、keyring 用 mock trait（trait `SecretStore`，测试注入内存实现；真实 keyring 实现不进单测）。**Step 2–5:** Commit。

### Task 23: session_manager.rs（AppModel 复刻）

**Files:** Create `src-tauri/src/session_manager.rs`。

**Port:** `AppModel.swift` 全部生命周期（不含 UI seed）：start（save settings → clearSubtitles → beginConnecting → client.connect → audioCapture.start → didConnect → 显示 overlay → 10s 健康检查）；stop（beginStopping → 停采集 → client.finish → didStop）；pause/resume；switchSourceLanguage（quick-switch 目标语言规则、需要时重连、paused 时仅改设置）；switchTranslationMode；setOverlayCollapsed/Locked；receive(event)（transport_error → 重连队列；error → 清理）；handleCaptureFailure；queueRecovery（2 次重试，间隔 0s/1s）；AudioSendPipeline 接线。所有状态变更 → 广播 `session-state` 事件；active 变化驱动 overlay 显隐。并发模型：`Arc<Mutex<SessionManager>>` + tokio 任务（网络/音频任务分离），事件经 `AppHandle::emit` 发送。锁内不 await。

### Task 24: commands.rs + lib.rs 装配

**Files:** Create `src-tauri/src/commands.rs`；Modify `lib.rs`。

按 IPC 契约注册全部命令；`settings_save` 中 `apiKey` 只写 keyring 不回显；错误经 `Result<T, String>` 返回，消息与 Swift LocalizedError 文案一致。`lib.rs`：`tracing_subscriber` 初始化（诊断级别 info）、注册插件（global-shortcut `CmdOrCtrl+Shift+Space` → start/stop 切换；connecting/stopping 时忽略；2s 去抖）、`on_tray_icon_event`（LeftClick → 切换 tray-panel；RightClick → 菜单）、托盘菜单（开始/停止 Live Subtitles、识别语言子菜单、锁定字幕位置、显示字幕窗口、清空字幕、设置、退出）、窗口创建（overlay/tray-panel 运行时创建 + positioner 定位）、`on_window_event`（overlay Moved/Resized → 记忆帧；CloseRequested → 隐藏）。

**Step 1:** 手动验证：`npm run tauri dev` 启动、托盘出现、设置窗打开、快捷键注册日志。

### Task 25: windows/overlay.rs

**Files:** Create `src-tauri/src/windows/overlay.rs`。

**Port:** `OverlayWindowController.swift`：默认 640×136 居中偏下（底部 72px）；恢复记忆帧（版本号 4，超屏重置）；show/hide；`update_locked`（set_ignore_cursor_events + 前端锁态事件）；`set_collapsed`（280×54 与展开尺寸切换、动画由前端 CSS 过渡 + settle 400ms 后校正尺寸，对应 settleFrame）；`set_size`（前端自绘缩放手柄）；位置约束到显示器可见区（tauri 取 monitor）。位置/尺寸限制 360×100 / 1200×600。

### Task 26: windows/tray_panel.rs + positioner

托盘点击 → 若 tray-panel 可见则隐藏，否则用 `tauri-plugin-positioner` 定位（macOS `TrayBottomCenter` / Windows `TrayBottomCenter` 风格）+ 显示 + setFocus。面板失焦（Blur）→ 隐藏。

---

## Phase 5：前端（1:1 UI）

### Task 27: 状态层 + 窗口路由

**Files:** Create `src/lib/store.ts`、`src/lib/ipc.ts`、`src/lib/types.ts`、`src/lib/i18n.ts`；Modify `src/main.tsx`、`src/App.tsx`。

- `types.ts`：契约中的 TS 类型（含 `SessionStatus` 判别联合、`OverlayActivityPhase`、语言枚举与 displayName 表）。
- `ipc.ts`：`invoke` 包装 + `listen("session-state"/"settings-changed")` 订阅。
- `store.ts`：zustand store：`session: SessionStateEvent`、`settings: SettingsSnapshot`、actions（start/stop/togglePaused/clear/switchLanguage/switchMode/saveSettings/setCollapsed/setLocked）。
- 窗口路由：`getCurrentWindow().label` 决定渲染 Overlay / TrayPanel / Settings 组件。
- `i18n.ts`：全部中文文案常量（空态、状态、帮助文案、错误文案），与 Swift 逐条一致。
- **Step:** `vitest` 冒烟（store action 转发 invoke 的 mock 测试）。

### Task 28: Overlay 窗口 UI

**Files:** Create `src/windows/overlay/OverlayWindow.tsx`、`Timeline.tsx`、`WaveformIndicator.tsx`、`RecognitionActivityIndicator.tsx`、`ControlButton.tsx`、`LanguagePickerPopover.tsx`、`ResizeHandles.tsx`、`DragHandle.tsx`。

逐项复刻 `SubtitleOverlayView.swift`：
- 画布：圆角 16 黑 62% + 顶部 3.5% 白渐变；描边 hover 未锁 → #7AA8FF 34% 1px，否则白 12% 0.75px；外 padding 6。
- 顶部拖柄：宽 120 高 18（活动态行高 38 / 空闲 24），hover 出现胶囊（#7AA8FF 78% / 白 28%），`data-tauri-drag-region` 拖拽，双击收起/展开。
- 空态：活动态波形 56px + 文案（fontSize*0.68，白 50%，错误红 90%）。
- 时间线：行 = 历史（分段后逐段；首段带 HH:mm 时间戳 9pt mono #7AA8FF 46%/28%）+ 当前行（final 全分段 / draft 末 2 段）；行字号：末行 = 设置字号，其余 82%（最小 12）；透明度 1/0.58/0.34；自动滚底。
- 语言胶囊（左上，活动态显示）：相位点 + （已暂停/翻译中）· 源语言 #7AA8FF → 目标语言；翻译模式徽章 + sparkles；点击弹 popover（识别语言 5 项：自动识别/ja/en/ko/zh + 翻译模式 3 项，checkmark 高亮，选中即切换并关闭；原文模式下模式槽位显示"原文"占位）；仅 listening 且未暂停可用。
- 自动识别（2026-08-14 修订）：`auto` 作为正式可选源语言进入所有选择器（popover/托盘/设置）。选中 auto 时翻译模式强制切为 lowLatency（live-translate 流按句检测语言，对应原版 auto→lowLatency）；会话构造时引擎语言解析为日语（原版 prepare 的 auto→ja 行为移到客户端构造点，`TranslationClient::new` 映射），但偏好文件中的 auto 不再被改写，重启后仍保持自动识别。
- 右上控制组（未锁时）：暂停/播放、收起、hover 出现清空（有内容时）+ 设置。
- 底部活动行：compact 波形 + 相位文案（有字幕且活动态）。
- 收起态：280×54 圆角 14 黑 68%；拖柄 42×30 + 相位点 + 文案 + 暂停 + 展开按钮。
- 波形/指示器动画：正弦胶囊（9 根，24fps；相位色与 speed/amplitude 参数照抄 Swift 表格），尊重 `prefers-reduced-motion`。
- 自绘缩放手柄：8 区域命中（边 6px / 角 14px 内缩），cursor 样式 `nwse-resize` 等，pointer 拖拽 → `overlay_set_size`（最小 360×100 / 最大 1200×600）；锁定或收起时禁用。
- resize 拖拽 IPC 契约（2026-08-14 修订）：`resize_start{region,x,y}` / `resize_move{x,y}` / `resize_end`；前端只转发指针位置，坐标一律用 `screenX/screenY`（屏幕 CSS 像素，随窗口移动不变）。严禁 `clientX/Y`——窗口相对坐标会在后端移动窗口后反馈进差值计算，导致拖角时窗口在两个帧之间来回振荡。前端在 pointerdown 时 `setPointerCapture` 并维护本地 active 标志（只在拖动中转发 move），另挂 window 级 pointerup/pointercancel/blur 兜底（防 capture 丢失后后端拖动态悬挂、后续 hover 触发误 resize），结束时必发 `resize_end`（后端 commit 帧）。
- 窗体几何状态机（2026-08-14 重构，取代全局 `POPOVER_RESIZING` 补丁）：Rust 侧 `OverlayState` 持有唯一 `user_frame`（唯一被持久化的帧，只由 resize 拖动与窗口移动防抖 350ms 后写入）+ `OverlayMode`（Expanded / Collapsed）。`OverlayWindowManager::apply` 是唯一写 OS 窗口几何的入口，按 `(mode, user_frame)` 推导 size/position/min/max。收起/展开动画的中间尺寸一律不持久化；收起态拖动只更新位置不污染记忆尺寸。
- 语言/模式弹层（2026-08-14 二次修订，彻底移除"弹层临时撑高窗口"机制）：弹层改为**独立窗口** `language-popover`（200×266 透明无边框置顶，对应原版 NSPopover），字幕窗口的高度/位置在任何情况下都不受弹层影响。契约：`overlay_popover_toggle{anchorX,anchorY}`（锚点 = 胶囊左下角屏幕逻辑坐标，下方放不下时翻转到上方；可见时点击 = 关闭，靠 180ms 延迟隐藏 + 代数计数解决"点击胶囊先夺焦点再 toggle"的竞态）/ `overlay_popover_hide`（选中条目后自关；收起/停止时后端兜底关闭）。弹层失焦延迟隐藏（点外部关闭）。弹层窗口经 `session_get_state` 拉取当前会话快照 + 订阅事件同步。
- 语言分段长度表：zh 28 / en 64 / ja 32 / original 按检测语言。

### Task 29: TrayPanel 窗口 UI

**Files:** Create `src/windows/tray-panel/TrayPanel.tsx`。

复刻 `MenuBarView.swift`：290 宽；头部图标 + "mimi" + 状态行（Setup required / Ready / Connecting… / Listening and translating / Stopping… / Paused / 错误消息；色绿/红/橙/次）；Live Subtitles 开关（⌘⇧Space 提示文案）；识别语言 Picker（ja/en/ko/zh，中文显示"中文原文"；connecting/stopping/暂停时禁用）；翻译模式 Label；Lock Subtitle Position 开关；Show Subtitle Window（仅 listening 可用）；Clear Subtitles；Settings…；Quit mimi。onOpen 时 prepareForListening（auto→ja；zh→original）。

### Task 30: Settings 窗口 UI

**Files:** Create `src/windows/settings/SettingsView.tsx`、`components/`。

复刻 `SettingsView.swift`：560×570；三卡片（圆角 14、次色 7.5% 底 + 4.5% 描边）：会话卡（42px 圆图标 captions.bubble + 状态点脉冲动画 + 开始/停止红色按钮）、字幕卡（识别语言 4 按钮组（选中 #346FE2 12% 底 34% 描边 + checkmark）、翻译成 Picker（原版/简体中文/English/日本語；活动或中文源时禁用）、翻译模式 Picker + 三种模式帮助文案、字号 Slider 14–20 + 数值、锁定开关 + 拖动说明文案）、服务设置折叠卡（Workspace ID 输入、API Key 密码输入、保存凭证按钮、成功/失败反馈、keychain 读取错误提示；初始缺凭证时自动展开）。顶部 onOpen 时 prepareForListening。全部文案与 Swift 一致。

### Task 31: 样式与动效收尾

Tailwind 配置主题色（accent #7AA8FF / settings #3478F0）；系统字体栈 `-apple-system, "PingFang SC", "Microsoft YaHei", "Segoe UI", sans-serif`；收起/展开 180ms easeInOut 过渡；hover 160ms；panel 淡入淡出。全局 CSS 无滚动条默认隐藏（hover 显示，overlay 时间线）。

**Step:** `npm run tauri dev` 逐窗口对照 Swift 截图（Resources/mimi-web/public 中的 overlay-current.png / settings.png）目检。

---

## Phase 6：打包、验证与收尾

### Task 32: 图标与元数据

用 `Resources/Assets/mimi-icon.png`（如有 1024 源图则用之，否则用 `mimi-web/public/mimi/mimi-icon-512.jpg` 转 PNG）跑 `npm run tauri icon`，产出 `src-tauri/icons/`（.icns/.ico/png）。tauri.conf.json 填 icon 数组；bundle 目标 macOS `.app` + `.dmg`、Windows `.msi` + `.nsis`；productName/描述/版本 1.0.0。

### Task 33: scripts/check.sh + 前端检查

**Files:** Create `scripts/check.sh`、`scripts/package-app.sh`。

- check.sh：`cargo fmt --check` → `cargo clippy --all-targets -- -D warnings` → `cargo test` → `cd src 前端: tsc -b && eslint && vitest run && vite build` → `cargo check --target x86_64-pc-windows-msvc`（若目标已安装）。
- package-app.sh：`npm run tauri build`，输出 `src-tauri/target/release/bundle/`（macOS dmg / Windows msi）；不提交 dist/。

### Task 34: 端到端手动冒烟（macOS 本机）

`npm run tauri dev`：首次启动设置窗 → 授权屏幕录制 → 输入真实凭证（用户提供）→ 播放日剧/视频 → 三种模式各验证字幕与翻译、拖动/缩放/收起/锁定穿透、暂停恢复、清空、快捷键、断网重连、错误态、长字幕分段。记录真实延迟测量（对照原版 mimi-replay 方法论：连接耗时、首字幕延迟、final 延迟）。

### Task 35: 文档与 Swift 源码删除

- 更新 `README.md` / `README_ZH.md` / `README_JA.md`（Tauri 构建方式、双平台说明、Windows 说明、凭证存储描述）；`AGENTS.md` 改为 Rust/TS 仓库地图；`CONTRIBUTING.md`/`SECURITY.md` 相应更新。
- **最终步骤**：`rm -rf Sources Tests Package.swift Resources` 并提交（git 历史可恢复）。删除后重跑 `scripts/check.sh` 确认无引用残留。
- 保留 `mimi-web/`（产品官网，非应用本体）与 `docs/`。

---

## 验收清单

- [ ] `./scripts/check.sh` 全绿（macOS 本机）。
- [ ] `cargo check --target x86_64-pc-windows-msvc` 通过（Windows 编译级验证）。
- [ ] macOS 打包产物可打开并完成一次真实会话（三种模式）。
- [ ] Swift 源码已删除，AGENTS.md 与新仓库结构一致。
- [ ] 凭证仅存系统钥匙串；仓库无明文 key；诊断日志无字幕内容。
- [ ] Windows 运行冒烟清单写入 README（需 Windows 机器执行）。
