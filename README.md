<div align="center">
  <img src="src-tauri/icons/128x128@2x.png" width="96" alt="mimi">
  <h1>mimi</h1>
  <p>系统音频实时字幕与翻译；支持 Apple 芯片 macOS 13+，Windows 为预览版。</p>
  <p>
    <a href="https://github.com/yuxino/mimi/releases/latest"><strong>下载最新版</strong></a>
    · <a href="README_EN.md">English</a>
    · <a href="README_JA.md">日本語</a>
  </p>
</div>

`mimi` 取自日语「耳（みみ）」。

把设备正在播放的系统音频变成实时字幕，并按服务商能力翻译成简体中文、英语或日语；可用输入包括中文、日语、英语和韩语。

## 功能

- **实时字幕** — 采集设备当前播放的系统输出音频。
- **实时翻译** — 目标语言与极速、低延迟、高质量模式随服务商而异。
- **多服务配置** — 保存并切换多套服务配置，无需反复填写凭证。
- **字幕浮窗** — 支持移动、缩放、收起、暂停、锁定穿透和沉浸模式。
- **多语言** — 可用中文、日语、英语和韩语；具体选项随服务商而异。
- **隐私** — 不使用麦克风或 mimi 账号，不保存音频和字幕；系统音频只发送给当前服务商。
- **全局快捷键** — macOS 按 **⌘ ⇧ Space**、Windows 按 **Ctrl+Shift+Space** 开始或停止监听；按 **⌘ ⇧ M** 或 **Ctrl+Shift+M** 切换沉浸模式。

## 开始使用

1. 从 [Latest Release](https://github.com/yuxino/mimi/releases/latest) 下载 macOS Apple Silicon DMG 或 Windows x64 EXE / MSI；也可以从源码构建。
2. 打开「翻译服务」，选择服务商并保存凭证。
3. 播放内容，从菜单栏/系统托盘的 mimi 图标点击 **开始**；macOS 首次使用时按提示允许「屏幕与系统音频录制」。

每套服务配置的凭证都独立保存在操作系统的安全凭据存储中（macOS 钥匙串 / Windows 凭据管理器）；设置页只显示是否已保存，不会读回凭证。不同服务商要求的字段不同，调用可能产生费用。

[阿里云](https://help.aliyun.com/zh/model-studio/get-api-key) · [OpenAI](https://platform.openai.com/api-keys) · [Google Gemini](https://aistudio.google.com/app/apikey) · [Azure OpenAI](https://learn.microsoft.com/zh-cn/azure/foundry/openai/concepts/gpt-realtime-translate) · [火山引擎](https://docs.volcengine.com/docs/6561/1631605) · [腾讯云](https://cloud.tencent.com/document/api/1093/127565) · [百度翻译](https://cloud.baidu.com/doc/MT/s/Sl9p2h5k9) · [xAI](https://docs.x.ai/developers/model-capabilities/audio/speech-to-speech)

| 服务商 | 音频源语言 | 字幕目标 | 模式 |
| --- | --- | --- | --- |
| 阿里云百炼 | 自动、中文、英语、日语、韩语 | 原文、简体中文、英语、日语 | 极速、低延迟、高质量 |
| OpenAI Realtime | 自动 | 简体中文、英语、日语 | 极速 |
| Google Gemini Live Translate（预览） | 自动 | 简体中文、英语、日语 | 极速 |
| Azure OpenAI Realtime Translate | 自动 | 简体中文、英语、日语 | 极速 |
| 火山引擎豆包同传 2.0 | 中文、英语、日语 | 简体中文、英语、日语 | 极速 |
| 腾讯云实时语音翻译 | 中文、英语、日语、韩语 | 简体中文、英语、日语 | 极速 |
| 百度实时语音翻译 | 中文、英语、日语、韩语 | 简体中文、英语、日语 | 极速 |
| xAI Grok Voice | 自动 | 简体中文、英语、日语 | 极速（回合式） |

Gemini、Azure OpenAI、豆包、腾讯云、百度和 xAI 已通过协议、模拟 WebSocket 与 UI 逻辑测试；付费账号的端到端质量和延迟尚未逐一验收。

### 平台说明

- **Apple 芯片上的 macOS 13 或更高版本**：GitHub Releases 提供使用临时签名且未经 Apple 公证的 DMG。如果首次打开被拦截，请在「系统设置 → 隐私与安全性」中选择「仍要打开」。更新后可能需要重新允许「屏幕与系统音频录制」或钥匙串访问。mimi 只采集系统音频，不录制屏幕，并会排除自身声音。
- **Windows 预览**：GitHub Releases 提供 x64 MSI 和 NSIS 安装程序，但尚未做 Authenticode 签名，Windows Defender SmartScreen 可能显示警告。真机安装、信任提示、凭据存储、系统音频采集、浮窗和完整字幕流程尚未验收。

## 从源码构建

需要 Rust 1.88+，以及 Node.js 20.19.x、22.13+ 或 24+；macOS 还需 Xcode Command Line Tools 和 `mimi Local Development` 身份或显式 `MIMI_CODESIGN_IDENTITY`。

```bash
git clone https://github.com/yuxino/mimi.git
cd mimi
npm ci
./scripts/dev-app.sh     # macOS：以稳定应用身份开发运行
npm run tauri:dev        # Windows：使用独立开发配置运行
./scripts/check.sh       # 完整检查（fmt/clippy/测试/前端构建）
./scripts/package-app.sh # 打包（macOS: DMG；Windows: MSI / NSIS EXE）
```

Windows 安装包必须在 Windows 机器上构建；CI 会在 macOS 与 Windows 上运行完整的 Rust 测试。

### macOS 开发说明

- macOS 开发请始终通过 `./scripts/dev-app.sh` 启动。它会生成并校验固定路径下的稳定签名 `.app`；本地证书没有 Apple Team ID，更新二进制后仍可能要求一次钥匙串授权。
- 启动器通过 `scripts/codesign-identity.sh` 选择 `mimi Local Development` 身份。不要运行 `tauri dev` 或裸二进制，以免系统把重建版本视为新应用并重复请求权限。
- 开发版使用独立的应用标识、设置目录和凭证命名空间，不会读取或修改已安装正式版的服务配置与 API Key。

## 测试

```bash
cargo test --manifest-path src-tauri/Cargo.toml   # Rust 单元测试（协议、字幕组装、配置、PCM 等）
npm run test                                      # 前端 vitest
```

macOS 的 UI 冒烟请运行 `./scripts/dev-app.sh --ui-only`；该模式不访问真实凭证、服务商网络或系统音频。端到端验证再使用普通启动命令和本机安全凭据。

更多内容见 [CONTRIBUTING.md](CONTRIBUTING.md) 和 [SECURITY.md](SECURITY.md)。

[MIT](LICENSE) © 2026 yuxino
