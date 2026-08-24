# mimi 会话启动主线程阻塞设计记录

> 2026-08-15。动画 60fps 优化后仍有「一开始有点卡」，用户直觉「还有别的阻塞事件」——记录排查结论与修复，避免回退。

## 结论（一句话）

**Tauri 的同步命令和 ScreenCaptureKit 的启动等待都跑在 macOS 主线程上**；启动阶段把钥匙串 I/O、状态克隆、SCStream 初始化同步等在主线程完成，就会在会话刚开始时冻结所有窗口的渲染（包括 overlay 动画）。

## 根因机制（源码链）

1. **同步命令在主线程执行** — wry `src/wkwebview/class/wry_web_view_delegate.rs`：
   ```rust
   #[thread_kind = MainThreadOnly]
   unsafe impl WKScriptMessageHandler for WryWebViewDelegate { ... }
   ```
   WKWebView 的 IPC 消息处理（`userContentController:didReceiveScriptMessage:`）被 objc2 标记为 **MainThreadOnly**，Tauri 的 invoke handler 在这个回调里内联执行。因此 `#[tauri::command]` 的**同步**命令直接跑在主线程上，期间事件循环停摆。

2. **被阻塞的同步命令**（启动路径上）：
   - `settings_get` — 首次调用读钥匙串（`keyring` → Security framework，可能触发 ACL 求值）；
   - `settings_save` — 写钥匙串 + `std::fs::write` 持久化 preferences；
   - `session_get_state` — 克隆完整 controller 状态（字幕历史）。
   四个窗口（overlay/tray-panel/settings/language-popover）启动时都并发调用 `settingsGet()` + `sessionGetState()`，第一个 `settings_get` 在主线程上完成钥匙串读取。

3. **ScreenCaptureKit 启动等待** — `src-tauri/src/audio/macos.rs` `start_capture_on_main`：
   ```rust
   stream.start_capture(|error| { tx.send(...) });
   rx.recv_timeout(Duration::from_secs(15))  // ← 在主线程同步等待
   ```
   该函数本身被 dispatch 到主线程执行（SCStream 是 main-thread-only），`recv_timeout` 又让主线程**同步阻塞**直到 ScreenCaptureKit 首次建立 capture session（枚举窗口/应用，可达数百毫秒甚至更久）。这正是「会话一开始卡」的峰值来源：用户点开始 → `establish_session` → `capture.start()` → 主线程冻结。

4. **孤儿启动诊断** — `lib.rs` 曾有一个 3 秒后对全部 4 个窗口 `window.eval` 的 UI probe（前端处理早已删除，纯垃圾主线程 eval 工作）。

## 修复

1. **删除孤儿 UI probe**（lib.rs 的 3s eval 块 + `ui_probe_report` 命令 + invoke_handler 注册）。
2. **重 I/O 命令转 async**：`settings_get` / `settings_save` / `session_get_state` 改为 `pub async fn`。Tauri 对 async 命令走 `respond_async` → `async_runtime::spawn`，在 tokio 线程池执行，不再占用主线程。`AppState` 全为 `Arc`（Send+Sync），安全。
3. **capture 启动异步等待真实 completion**：主线程只创建并发起
   `SCStream::start_capture`，completion 通过 oneshot 回到 Tokio；
   `capture.start()` 异步等待该结果后才让会话进入 Listening。这样既不
   阻塞主线程，也不会在 ScreenCaptureKit 实际失败前错误宣称已经开始
   监听。等待有有限超时，并受当前会话 generation 约束。

## 保持不变的边界

- `SCStream` 及其 delegate 仍是 main-thread-only，`MainThreadDispatcher` 机制不动；只移除了「主线程同步等待启动完成」这一段。
- capture 启动失败在连接阶段直接返回 typed error；运行中的 native stop
  继续走容量为 1 的 typed failure 通道，并按 retryability 决定恢复或终止。
- 窗口操作类同步命令（`overlay_show`、`resize_*`、`overlay_set_collapsed` 等）保留 sync——它们只是轻量窗口调用，且天然需要主线程。
