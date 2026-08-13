# mimi Tauri 多平台迁移设计

## 目标

把 mimi 从 macOS 14+ SwiftUI 原生应用迁移为 **Tauri v2** 桌面应用（Rust 后端 + React/TypeScript 前端），1:1 保留现有功能与视觉：实时系统音频字幕、极速/低延迟/高质量三种翻译模式、可移动/缩放/收起/锁定穿透的浮窗字幕、菜单栏（托盘）控制面板、全局快捷键开始/停止、暂停与自动重连，以及「凭证只进系统钥匙串、不落盘不录屏不回传」的隐私约束。同一份代码同时支持 **macOS** 与 **Windows**。Swift 源码在 Tauri 版本通过全部验证后删除。

## 技术选型

| 领域 | 选择 | 理由 |
| --- | --- | --- |
| 应用框架 | Tauri v2 | 多窗口（设置窗 / 浮窗 / 托盘面板）、托盘、透明无边框置顶窗、全局快捷键插件 |
| 后端语言 | Rust（tokio 异步运行时） | 单一技术栈，跨平台 |
| 前端 | React 19 + TypeScript + Tailwind + Vite | 与仓库现有 mimi-web 技术栈一致 |
| macOS 系统音频 | `screencapturekit` crate（ScreenCaptureKit FFI） | 纯 Rust；voicebox 等 Tauri 项目生产验证；支持排除本进程音频、16kHz 单声道 |
| Windows 系统音频 | `cpal`（WASAPI loopback） | 官方 RustAudio 音频栈，0.16+ 支持环回采集默认播放设备混音 |
| 重采样 | `rubato` | WASAPI 混音格式（48kHz 立体声）→ 16kHz 单声道 PCM16 |
| WebSocket | `tokio-tungstenite` | 极速/低延迟实时翻译、Audio 3.0 识别均为 WSS |
| HTTP/SSE | `reqwest`（rustls） | Qwen-MT chat/completions（含流式） |
| 凭证存储 | `keyring` v3 | macOS Keychain / Windows Credential Manager，无明文回退 |
| 偏好存储 | 应用配置目录下 JSON 文件 | 对应原版 UserDefaults |
| 全局快捷键 | `tauri-plugin-global-shortcut`（CmdOrCtrl+Shift+Space） | 跨平台 |
| 托盘面板定位 | `tauri-plugin-positioner` | 把无边框小窗定位到托盘图标旁（macOS 菜单栏下方 / Windows 通知区旁） |
| 单元测试 | `cargo test` + 前端 `vitest` | 移植原 MimiCoreTests |

不引入新外部服务：ASR/翻译仍使用阿里云百炼（DashScope）同一批端点与模型，协议字段保持不变。

## 架构

```
┌───────────────────────────── Tauri App ─────────────────────────────┐
│  前端 React（设置窗 / 浮窗字幕 / 托盘面板 三个窗口）                     │
│   - 只订阅状态事件、调用命令；不含凭证、不含网络请求                     │
├───────────────────────────────────────────────────────────────────┤
│  commands.rs:  start / stop / pause / resume / clear / 切语言/切模式    │
│  事件: "session-state"（全量快照）"settings-changed"                    │
├────────────────────────── MimiCore (Rust) ─────────────────────────┤
│  session.rs  会话状态机（connecting/listening/stopping/error/暂停/恢复/   │
│              健康检查 10s ping、断线重连 2 次退避）                        │
│  translation_client.rs  ──┬─ low_latency_client（qwen3.5-livetranslate-  │
│                           │   flash-realtime，WSS 实时字幕+翻译）          │
│                           └─ high_quality_client（qwen-audio-3.0-asr-     │
│                              flash-streaming 识别 + qwen-mt-plus/flash    │
│                              翻译；草稿稳定提交/最大等待/预览抢占/译文记忆）  │
│  纯逻辑: subtitle_reducer / segmenter / committer / preview_tracker /     │
│          pcm16 / configuration / diagnostics（全部无 IO，可单测）         │
├──────────────────────────── 平台层 ────────────────────────────────┤
│  audio:  macos.rs(screencapturekit SCStream 16k mono, 排除本应用)        │
│          windows.rs(cpal WASAPI loopback 默认播放设备 → rubato 重采样)     │
│  凭证:   keyring(macOS Keychain / Windows Credential Manager)           │
│  偏好:   app_config_dir/preferences.json                                 │
│  窗口:   overlay.rs（置顶/透明/无边框/点击穿透/收起展开/自定义边缘缩放/     │
│          位置记忆）tray.rs（托盘图标+弹出面板+菜单）hotkey.rs（全局快捷键）  │
└───────────────────────────────────────────────────────────────────┘
```

与 Swift 版的映射关系：

| Swift 原文件 | Rust 落点 |
| --- | --- |
| Models.swift | `src-tauri/src/models.rs` |
| LiveTranslationConfiguration.swift | `configuration.rs` |
| SubtitleReducer.swift | `subtitle_reducer.rs` |
| SubtitleTextSegmenter.swift | `segmenter.rs` |
| ASRDraftCommitter.swift | `committer.rs` |
| DraftPreviewTracker.swift | `preview_tracker.rs` |
| PCM16Encoder.swift | `pcm16.rs` |
| PipelineDiagnostics.swift | `diagnostics.rs`（tracing，仅计时/计数/语言码/错误标签） |
| SessionController.swift | `session.rs` |
| RealtimeASRProtocol / LiveTranslateProtocol | `protocols/live_translate.rs` |
| Audio3ASRProtocol / Audio3ASRContext | `protocols/audio3.rs` |
| QwenMTProtocol（含领域提示词、语气词对照表） | `protocols/qwen_mt.rs` |
| LiveTranslateClient | `clients/live_translate_client.rs` |
| Audio3ASRClient | `clients/audio3_client.rs` |
| QwenMTClient | `clients/qwen_mt_client.rs` |
| HighQualityTranslationClient | `clients/high_quality_client.rs` |
| TranslationClient（三模式分派） | `clients/translation_client.rs` |
| SystemAudioCapture.swift | `audio/macos.rs` + `audio/windows.rs` |
| AppModel.swift（含 AudioSendPipeline） | `session_manager.rs` + `audio/send_pipeline.rs` |
| AppSettings.swift / KeychainStore.swift | `settings_store.rs`（keyring + preferences.json） |
| OverlayWindowController / ResizeCursorPanel | `windows/overlay.rs` + 前端缩放手柄 |
| MenuBarView / SettingsView / SubtitleOverlayView | `src/windows/{TrayPanel,Settings,Overlay}/` |
| GlobalHotKeyController | tauri-plugin-global-shortcut |
| MimiReplay | 本期不迁移（用户确认） |

## 平台差异与等价实现

### 系统音频采集

- **macOS**：`screencapturekit` crate 创建 `SCStream`：`capturesAudio=true`、`excludesCurrentProcessAudio=true`、16kHz、单声道；`SCContentFilter` 选主显示器并排除本应用。行为与现版 SystemAudioCapture 一致（音频-only，需要屏幕录制权限，由系统弹窗授权）。采样缓冲 → f32 → PCM16。
- **Windows**：WASAPI loopback 捕获**默认播放设备的整体混音**（Windows 无按应用排除能力；本应用不播放任何声音，故无回声问题），原始格式通常 48kHz 立体声，用 `rubato` 重采样到 16kHz 单声道后转 PCM16。无需任何权限弹窗。

### 浮窗字幕

Tauri 窗口配置：`transparent + decorations:false + alwaysOnTop + skipTaskbar + shadow:false + resizable`。
- 拖动：顶部拖柄区域 `data-tauri-drag-region`（对应 WindowDragArea）。
- 锁定穿透：`set_ignore_cursor_events(true)`（对应 `ignoresMouseEvents`）。
- 收起/展开：把窗口尺寸动画到 280×54 / 记忆尺寸（对应 SubtitleOverlayCollapseLayout + settleFrame）。
- 边缘/四角缩放：Windows 无边框窗口没有原生缩放边框，因此**由前端实现 8 区域命中测试 + 光标 + `setSize` 拖拽缩放**，与现版 OverlayResizeHitTester 行为一致（两端一致体验）。
- 位置记忆：preferences.json 存帧位置与版本号，约束回可见屏幕内。

### 托盘入口

- macOS：状态栏图标（复用现版菜单栏图标逻辑：ear / ear.badge.waveform / pause / exclamation），左键点击在图标下方弹出无边框控制面板小窗（复刻 MenuBarView），右键/菜单含开始/停止、语言、锁定、显示字幕、清空、设置、退出。
- Windows：通知区图标，点击在托盘旁弹出同一控制面板；退出菜单项保留。

### 凭证与偏好

- API Key：`keyring`（macOS Keychain，service=`app.yuxino.mimi.credentials.v2`，account=`dashscope-api-key`；Windows Credential Manager 同名条目）。无明文、无环境变量回退。
- Workspace ID / 语言 / 翻译模式 / 字号 / 锁定 / 帧位置：`app_config_dir()/preferences.json`。

### 权限

- macOS：`capabilities` 仅给需要的插件；屏幕录制权限在首次启动采集时由系统提示（与现版一致）。Info.plist 与 .app 组装交给 `tauri build`。
- Windows：无特殊权限。

## 协议与算法（保持原字段与参数）

- **低延迟**（`lowLatency` / 自动识别时）：`wss://{workspace}.cn-beijing.maas.aliyuncs.com/api-ws/v1/realtime?model=qwen3.5-livetranslate-flash-realtime`，`session.update`（modalities=[text]、16k pcm、转写模型 qwen3-asr-flash-realtime、translation{language, corpus}）→ Base64 PCM `input_audio_buffer.append` → `session.finish`。服务端事件解码、`text+stash` 合并草稿、`response.text.done` 定稿等逻辑 1:1 移植。
- **高质量/极速**：Audio 3.0 识别 `wss://…/api-ws/v1/inference`，`run-task`（qwen-audio-3.0-asr-flash-streaming，language_hints、semantic_punctuation、heartbeat、影视对白 context）→ 二进制 PCM 帧 → `result-generated` 句子。翻译走 HTTPS `POST /compatible-mode/v1/chat/completions`（model=qwen-mt-plus / qwen-mt-flash，`translation_options`{source_lang,target_lang,domains,terms,tm_list}，stream 可选）。**领域提示词与语气词对照表逐字移植**（这部分是翻译质量的核心资产）。
- **字幕组装**：SubtitleReducer（draft/final 分离、revokeLastConfirmed、20 条历史上限）、ASRDraftCommitter（句界提交、local final 替换逻辑）、DraftPreviewTracker（代际拒绝）、SubtitleTextSegmenter（按句界/逗号切分、末 2 段草稿展示）全部按字符级语义移植，并用原测试用例做验收。
- **会话生命周期**：开始/停止/暂停/恢复、10s ping 健康检查、`transport_error` 触发 2 次退避重连、停止时 `session.finish` 等待、`stopping` 期间丢弃合成尾部事件 —— 与 AppModel 一致。

## UI 规格（1:1）

- 浮窗：圆角 16 黑底 62% 透明 + 顶部高光渐变；主行字号 = 设置字号（14–20，默认 18），历史行 82% 且按新旧降透明度 1 → 0.58 → 0.34；时间戳 9pt 等宽 #7AA8FF 系；语言胶囊（源语言 · 目标语言、翻译模式徽章、sparkles 图标）；右上角暂停/收起/清空/设置按钮；顶部拖动柄 hover 高亮（#7AA8FF 透明度 0.34 描边）；活动波形（9/5 根胶囊，正弦动画）与相位色（聆听 #7AA8FF、翻译 #B894FF、暂停 #FFB852、连接白 50%）；空态文案（正在聆听，译文会保留在这里 / 正在连接 / 正在翻译 / 错误消息红字）；收起态 280×54（状态点 + 文案 + 暂停 + 展开）。参考尺寸 640×136，最小 360×100，最大 1200×600。
- 托盘面板：290 宽，ear.badge.waveform 图标 + 标题 + 状态行、Live Subtitles 开关、识别语言 Picker、翻译模式 Label、锁定开关、Show Subtitle Window、Clear Subtitles、Settings…、Quit mimi。
- 设置窗：560×570，三张卡片（实时字幕会话卡：状态点+开始/停止；字幕卡：语言按钮组、翻译成、翻译模式、字号滑杆 14–20、锁定开关；服务设置折叠卡：Workspace ID、API Key、保存凭证及反馈文案），帮助文案逐条移植。
- 字体：系统字体栈（macOS PingFang SC / Windows Microsoft YaHei），monospace 数字用于时间戳。

## 测试与验证

- `cargo test`：移植 MimiCoreTests 全部纯逻辑用例（协议编解码、reducer、committer、segmenter、tracker、配置校验、PCM、重试策略、会话状态机、缩放命中测试）。
- `cargo clippy -- -D warnings`、`cargo fmt --check`、前端 `tsc` + `vitest` + `vite build`。
- `scripts/check.sh` 串联上述全部（对应原 check.sh）。
- `npm run tauri build`（macOS 本机产出 .app/.dmg；Windows 用 `cargo check --target x86_64-pc-windows-msvc` 做编译级验证，运行时验证需 Windows 机器）。
- 手动冒烟：首次启动引导设置、授权、开始监听、真实阿里云会话（需用户本地凭证）、拖动/缩放/收起/锁定穿透、暂停/恢复、断网重连、错误态。

## 迁移顺序

1. 脚手架（Tauri v2 + React + Tailwind）→ 2. Rust 核心纯逻辑（含测试，对照 Swift 逐文件移植）→ 3. 网络客户端 → 4. 双平台音频采集 → 5. 会话管理器 + 命令/事件 → 6. 窗口/托盘/快捷键 → 7. 前端三窗口 UI → 8. 打包脚本与图标 → 9. 文档更新 → 10. 全部验证通过后删除 Swift 源码（Sources/、Tests/、Package.swift、Resources），更新 AGENTS.md。

风险与回退：`screencapturekit` crate 若在音频输出回调上遇到兼容问题，回退方案为内置 Swift 采集助手子进程（用户已认可）；Windows 运行时行为以编译级验证 + 文档标注，交付后可在 Windows 机器上冒烟。
