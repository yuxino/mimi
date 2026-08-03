# 参与贡献 / Contributing

感谢你帮助改进 mimi。小而聚焦的 Pull Request 最容易审查和合并。

Thank you for improving mimi. Small, focused pull requests are the easiest to review and merge.

## 开始之前 / Before you start

1. 不要在 Issue、日志、测试或截图中提交真实 Workspace ID 和 API Key。
2. Bug 请附上 macOS 版本、复现步骤、预期行为和实际行为。
3. 较大的功能先创建 Issue，说明使用场景和体验目标。

1. Never commit a real Workspace ID or API key in issues, logs, tests, or screenshots.
2. Bug reports should include the macOS version, reproduction steps, expected behavior, and actual behavior.
3. Open an issue before a large feature and explain the use case and UX goal.

## 本地验证 / Local verification

```bash
./scripts/check.sh
./scripts/package-app.sh
```

界面改动还需要实际启动 `dist/mimi.app`，检查普通、空白、错误和长字幕状态。涉及延迟的改动应使用 `mimi-replay` 回放真实音频并记录结果。

For UI changes, launch `dist/mimi.app` and inspect normal, empty, error, and long-subtitle states. Latency-sensitive changes should replay real audio with `mimi-replay` and include measured results.

## 本地稳定签名 / Stable local signing

`scripts/package-app.sh` 优先使用登录钥匙串中的 `mimi Local Development` 证书，也可以通过环境变量指定：

The packaging script prefers a `mimi Local Development` identity in the login Keychain. You may override it:

```bash
MIMI_CODESIGN_IDENTITY="Apple Development: Your Name" ./scripts/package-app.sh
```

如果没有可用证书，脚本会使用临时签名。这样仍可运行，但 macOS 可能在每次重新构建后重新请求屏幕录制权限。

Without a usable identity, the script falls back to ad-hoc signing. The app still runs, but macOS may request Screen Recording permission after rebuilds.

## Pull Request

- 说明用户可见的变化和原因。
- 列出运行过的测试；界面改动附截图。
- 保持提交信息简洁，避免把无关重构混在一起。
- 确认 `git diff` 中没有凭证、录音、个人路径或构建产物。

- Explain the user-visible change and why it is needed.
- List tests run and include screenshots for UI changes.
- Keep commits focused and avoid unrelated refactors.
- Confirm the diff contains no credentials, recordings, personal paths, or build artifacts.
