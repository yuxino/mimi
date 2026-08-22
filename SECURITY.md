# 安全策略 / Security Policy

## 报告漏洞 / Reporting a vulnerability

请不要公开提交可能泄露凭证、绕过权限或执行任意代码的安全问题。请通过 GitHub 仓库所有者主页提供的私密联系方式报告，并提供最小复现步骤。收到报告后，维护者会先确认影响范围，再安排修复与披露。

Do not open a public issue for vulnerabilities that may expose credentials, bypass permissions, or execute arbitrary code. Report them privately through the contact method on the repository owner's GitHub profile and include minimal reproduction steps. The maintainer will confirm impact before coordinating a fix and disclosure.

## 凭证安全 / Credential safety

- mimi 的 API Key 只保存在系统钥匙串中：macOS Keychain / Windows 凭据管理器。每个服务档案按「档案 ID + 服务商」独立存储，代码中没有明文、仓库或环境变量回退。
- 设置页与 IPC 只返回 `present`、`missing` 或 `unavailable`，不会读回或广播已保存的 Key。
- 不要在 Issue、Pull Request、日志或截图中包含 API Key；如果 Key 曾公开，请立即在对应服务商控制台重置或停用。

- mimi stores API keys only in the OS keychain: macOS Keychain / Windows Credential Manager. Every service profile is isolated by profile ID and provider; there is no plaintext, source-controlled, or environment-variable fallback.
- Settings and IPC expose only `present`, `missing`, or `unavailable`, never the saved key itself.
- Never include API keys in issues, pull requests, logs, or screenshots. Reset or disable a key in its provider console immediately if it was exposed.

## 数据与权限 / Data and permissions

- 音频与字幕只驻留在内存中，不落盘；系统音频只发送到当前明确选中的服务商。
- macOS 仅请求「屏幕与系统音频录制」权限用于系统音频采集，不录制屏幕内容；Windows 使用 WASAPI 环回，无需额外权限。
- 诊断日志只包含计时、计数、语言码、状态码与错误标签，不包含识别或翻译文本。

- Audio and subtitles live only in memory and are never persisted. System audio is sent only to the provider explicitly selected for the active profile.
- macOS requests Screen & System Audio Recording access solely for system-audio capture (no screen content is recorded); Windows uses WASAPI loopback with no extra permissions.
- Diagnostics contain only timings, counts, language codes, status codes, and error labels — never recognized or translated text.
