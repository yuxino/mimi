# 参与贡献 / Contributing

感谢你帮助改进 mimi。小而聚焦的 Pull Request 最容易审查和合并。

Thank you for improving mimi. Small, focused pull requests are the easiest to review and merge.

## 开始之前 / Before you start

1. 不要在 Issue、日志、测试或截图中提交真实 Workspace ID 和 API Key。
2. Bug 请附上操作系统（macOS / Windows）与版本、复现步骤、预期行为和实际行为。
3. 较大的功能先创建 Issue，说明使用场景和体验目标。

1. Never commit a real Workspace ID or API key in issues, logs, tests, or screenshots.
2. Bug reports should include the operating system (macOS / Windows) and version, reproduction steps, expected behavior, and actual behavior.
3. Open an issue before a large feature and explain the use case and UX goal.

## 本地验证 / Local verification

```bash
./scripts/check.sh
./scripts/package-app.sh
```

界面改动还需要实际启动应用（`npm run tauri dev`），检查设置窗、托盘面板和字幕浮窗的普通、空白、错误、暂停、收起、翻译中和长字幕状态。涉及延迟或流式管线的改动应使用真实阿里云会话（本地凭证）验证并记录测量结果。

For UI changes, run `npm run tauri dev` and inspect the settings window, tray panel, and overlay in normal, empty, error, paused, collapsed, translating, and long-subtitle states. Latency- or streaming-sensitive changes should be verified against a real Alibaba Cloud session (with local credentials) and include measured results.

## 平台 / Platforms

- macOS 与 Windows 共用一套代码。平台差异集中在 `src-tauri/src/audio/`（macOS 用 ScreenCaptureKit，Windows 用 WASAPI loopback）与凭证存储（macOS 钥匙串 / Windows 凭据管理器）。
- Windows 打包请在 Windows 机器上执行；CI 会在 macOS 与 Windows 两个平台运行完整的 Rust 测试与 clippy。

macOS and Windows share one codebase. Platform differences live in `src-tauri/src/audio/` (ScreenCaptureKit on macOS, WASAPI loopback on Windows) and credential storage (macOS Keychain / Windows Credential Manager). Build the Windows package on a Windows machine; CI runs the full Rust tests and clippy on both platforms.

## Pull Request

- 说明用户可见的变化和原因。
- 列出运行过的测试；界面改动附截图。
- 保持提交信息简洁，避免把无关重构混在一起。
- 确认 `git diff` 中没有凭证、录音、个人路径或构建产物。

- Explain the user-visible change and why it is needed.
- List tests run and include screenshots for UI changes.
- Keep commits focused and avoid unrelated refactors.
- Confirm the diff contains no credentials, recordings, personal paths, or build artifacts.
