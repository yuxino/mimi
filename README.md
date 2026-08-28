<div align="center">
  <img src="src-tauri/icons/128x128@2x.png" width="96" alt="mimi">
  <h1>mimi</h1>
  <p>给系统声音加实时字幕；macOS 开发版已验收，Windows 仍为预览。</p>
  <p>
    <a href="https://github.com/yuxino/mimi/releases/latest"><strong>查看构建</strong></a>
    · <a href="README_EN.md">English</a>
    · <a href="README_JA.md">日本語</a>
  </p>
</div>

`mimi` 取自日语「耳（みみ）」。

把设备上正在播放的中文、日语、英语或韩语实时变成字幕，也可以翻译成简体中文、英语或日语。基于 Tauri v2（Rust + React），同一份代码面向 macOS 与 Windows；Windows 在完成真机验收前仍是预览。

## 功能

- **实时字幕** — 浏览器、播放器、游戏、会议和桌面应用都能用。
- **实时翻译** — 极速、低延迟、高质量三种模式。
- **多服务配置** — 为不同服务商保存多套配置，切换时无需反复填写凭证。
- **字幕浮窗** — 支持移动、缩放、收起和锁定穿透。
- **多语言** — 识别中文、日语、英语和韩语。
- **隐私** — 不使用麦克风，不需要 mimi 账号，不保存音频和字幕历史。
- **全局快捷键** — macOS 按 **⌘ ⇧ Space**、Windows 按 **Ctrl+Shift+Space** 开始或停止监听；按 **⌘ ⇧ M** 或 **Ctrl+Shift+M** 切换沉浸模式。

## 开始使用

1. 先阅读下面的平台状态，再核对对应 [Release](https://github.com/yuxino/mimi/releases/latest) 的说明或从源码构建。公开资产可能落后于当前源码。
2. 打开「服务配置」，选择服务商并保存对应连接凭证。默认仍使用阿里云百炼。
3. 播放内容，点击 **开始**。

每个服务配置的凭证都独立保存在操作系统的安全凭据存储中（macOS 钥匙串 / Windows 凭据管理器），设置页不会把已保存内容读回显示。单 Key 服务只需 API Key；Azure 需要资源端点以及独立的翻译、转写部署名称，腾讯和百度则显示各自官方要求的字段。已有阿里云设置会自动迁移为默认服务配置。服务商调用可能产生费用。

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
| xAI Grok Voice | 自动 | 简体中文、英语、日语 | 极速、回合式 |

Gemini、Azure OpenAI、豆包、腾讯云、百度和 xAI 已通过协议夹具、模拟 WebSocket 与 UI 逻辑测试；各服务付费账号的端到端质量和延迟尚未逐一验收。

### 平台说明

- **Apple 芯片上的 macOS 13 或更高版本**：GitHub Releases 提供可供开发者安装、使用临时签名且未经 Apple 公证的 DMG。如果首次打开被拦截，请在「系统设置 → 隐私与安全性」中选择「仍要打开」。更新后可能需要重新允许「屏幕与系统音频录制」或钥匙串访问。本地权限相关测试使用仓库独立的稳定开发身份。mimi 只采集系统音频，不录制屏幕，也会排除自身声音。
- **Windows 预览**：源码已实现 WASAPI 环回采集，CI 也能生成 x64 MSI 和 NSIS 安装程序；当前安装程序未做 Authenticode 签名。Windows 真机安装、信任提示、凭据存储、系统音频采集、浮窗行为和完整实时字幕流程尚未验收；CI 成功不能代替这些验证。

## 从源码构建

需要 Rust 1.88+，以及 Node.js 20.19.x、22.13+ 或 24+（macOS 上还需 Xcode Command Line Tools）。

```bash
git clone https://github.com/yuxino/mimi.git
cd mimi
npm ci
./scripts/dev-app.sh     # macOS：以稳定应用身份开发运行
npm run tauri:dev        # Windows：使用独立开发配置运行
./scripts/check.sh       # 完整检查（fmt/clippy/测试/前端构建）
./scripts/package-app.sh # 打包（macOS: DMG；Windows: MSI / NSIS EXE）
```

Windows 打包请在 Windows 机器上执行（Rust 依赖的 C 代码无法从 macOS 交叉编译到 MSVC 目标）；CI 会在 macOS 与 Windows 两个平台跑完整的 Rust 测试。

### macOS 开发说明

- macOS 开发请始终通过 `./scripts/dev-app.sh` 启动。它会生成并校验真正的 `.app`，安装到固定位置，并拒绝不稳定的临时签名，使「屏幕与系统音频录制」的应用身份在重新构建后保持稳定。本地证书是没有 Apple Team ID 的自签证书，更新二进制后 macOS 仍可能要求一次钥匙串授权。
- 启动器会通过 `scripts/codesign-identity.sh` 选择稳定的 `mimi Local Development` 身份。不要在 macOS 上使用任何 `tauri dev` 命令或直接运行裸二进制；临时签名每次构建都可能被系统当成新应用，从而重复请求授权。
- 开发版使用独立的应用标识、设置目录和凭证命名空间，不会读取或修改已安装正式版的服务配置与 API Key。

## 测试

```bash
cargo test --manifest-path src-tauri/Cargo.toml   # Rust 单元测试（协议、字幕组装、配置、PCM 等）
npm run test                                      # 前端 vitest
```

macOS 的 UI 冒烟请运行 `./scripts/dev-app.sh --ui-only`；UI 测试模式不会访问真实凭证、服务商网络或系统音频采集。端到端服务验证再使用普通命令和本机安全凭据存储中的凭证。

更多内容见 [CONTRIBUTING.md](CONTRIBUTING.md) 和 [SECURITY.md](SECURITY.md)。

[MIT](LICENSE) © 2026 yuxino
