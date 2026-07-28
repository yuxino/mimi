# 安全策略 / Security Policy

## 报告漏洞 / Reporting a vulnerability

请不要公开提交可能泄露凭证、绕过权限或执行任意代码的安全问题。请通过 GitHub 仓库所有者主页提供的私密联系方式报告，并提供最小复现步骤。收到报告后，维护者会先确认影响范围，再安排修复与披露。

Do not open a public issue for vulnerabilities that may expose credentials, bypass permissions, or execute arbitrary code. Report them privately through the contact method on the repository owner's GitHub profile and include minimal reproduction steps. The maintainer will confirm impact before coordinating a fix and disclosure.

## 凭证安全 / Credential safety

- mimi 的 API Key 存储在 macOS Keychain 服务 `app.yuxino.mimi.credentials.v2` 中。
- 不要在 Issue、Pull Request、日志或截图中包含 Workspace ID 与 API Key。
- 如果 Key 曾公开，请立即在阿里云百炼控制台重置或停用。

- mimi stores the API key in macOS Keychain service `app.yuxino.mimi.credentials.v2`.
- Never include Workspace IDs or API keys in issues, pull requests, logs, or screenshots.
- Reset or disable a key immediately if it was exposed.

