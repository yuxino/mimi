<div align="center">
  <img src="src-tauri/icons/128x128@2x.png" width="96" alt="mimi">
  <h1>mimi</h1>
  <p>给 Mac 或 Windows 上正在播放的声音加上实时翻译字幕。</p>
  <p>
    <a href="https://github.com/yuxino/mimi/releases/latest"><strong>下载 mimi</strong></a>
    · <a href="README.md">English</a>
    · <a href="README_JA.md">日本語</a>
  </p>
</div>

`mimi` 取自日语「耳（みみ）」。

把设备上正在播放的中文、日语、英语或韩语实时变成字幕，也可以翻译成简体中文、英语或日语。基于 Tauri v2（Rust + React），同一份代码同时支持 macOS 与 Windows。

<table>
  <tr>
    <td width="33.33%"><img src="docs/images/mimi-film-real.jpg" alt="mimi 为影视显示实时字幕"></td>
    <td width="33.33%"><img src="docs/images/mimi-game-real.jpg" alt="mimi 为游戏显示实时字幕"></td>
    <td width="33.33%"><img src="docs/images/mimi-meeting-real.jpg" alt="mimi 为线上会议显示实时字幕"></td>
  </tr>
  <tr>
    <td align="center">影视与视频</td>
    <td align="center">游戏与直播</td>
    <td align="center">会议与网课</td>
  </tr>
</table>

## 功能

- **实时字幕** — 浏览器、播放器、游戏、会议和桌面应用都能用。
- **实时翻译** — 极速、低延迟、高质量三种模式。
- **多服务配置** — 为不同服务商保存多套配置，切换时无需反复填写凭证。
- **字幕浮窗** — 支持移动、缩放、收起和锁定穿透。
- **多语言** — 识别中文、日语、英语和韩语。
- **隐私** — 不使用麦克风，不需要 mimi 账号，不保存音频和字幕历史。
- **全局快捷键** — macOS 按 **⌘ ⇧ Space**、Windows 按 **Ctrl+Shift+Space** 开始或停止监听。

## 开始使用

1. 从 [Releases](https://github.com/yuxino/mimi/releases/latest) 下载对应平台版本。
2. 打开「服务配置」并保存 API Key。默认仍使用阿里云百炼，也支持 OpenAI Realtime。
3. 播放内容，点击 **开始**。

每个服务配置的 API Key 都独立保存在操作系统的安全凭据存储中（macOS 钥匙串 / Windows 凭据管理器），设置页不会把已保存的 Key 读回显示。阿里云百炼共享 API 只需 API Key，无需 Workspace ID。已有阿里云设置会自动迁移为默认服务配置。服务商调用可能产生费用。

[创建阿里云 API Key](https://help.aliyun.com/zh/model-studio/get-api-key) · [创建 OpenAI API Key](https://platform.openai.com/api-keys)

| 服务商 | 音频源语言 | 字幕目标 | 模式 |
| --- | --- | --- | --- |
| 阿里云百炼 | 自动、中文、英语、日语、韩语 | 原文、简体中文、英语、日语 | 极速、低延迟、高质量 |
| OpenAI Realtime | 自动 | 简体中文、英语、日语 | 极速 |

### 平台说明

- **macOS 13 或更高版本**：首次监听时系统会请求「屏幕与系统音频录制」权限（mimi 只采集系统音频，不录制屏幕内容，也排除了自身声音）。
- **Windows**：使用 WASAPI 环回采集默认播放设备的整体混音，无需任何权限授权；mimi 不播放声音，因此没有回声问题。

## 从源码构建

需要 Rust 1.85+ 和 Node.js 20+（macOS 上还需 Xcode Command Line Tools）。

```bash
git clone https://github.com/yuxino/mimi.git
cd mimi
npm ci
./scripts/dev-app.sh     # macOS：以稳定应用身份开发运行
npm run tauri:dev        # Windows：使用独立开发配置运行
./scripts/check.sh       # 完整检查（fmt/clippy/测试/前端构建）
./scripts/package-app.sh # 打包（macOS: .dmg；Windows: .msi/.nsis）
```

Windows 打包请在 Windows 机器上执行（Rust 依赖的 C 代码无法从 macOS 交叉编译到 MSVC 目标）；CI 会在 macOS 与 Windows 两个平台跑完整的 Rust 测试。

### macOS 开发说明

- macOS 开发请始终通过 `./scripts/dev-app.sh` 启动。它会生成并校验真正的 `.app`，安装到固定位置，并拒绝不稳定的临时签名，让「屏幕与系统音频录制」和安全凭据存储权限在重新构建后仍归属于同一个应用。
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
