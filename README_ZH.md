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
- **字幕浮窗** — 支持移动、缩放、收起和锁定穿透。
- **多语言** — 识别中文、日语、英语和韩语。
- **隐私** — 不使用麦克风，不需要账号，不保存音频和字幕历史。
- **全局快捷键** — macOS 按 **⌘ ⇧ Space**、Windows 按 **Ctrl+Shift+Space** 开始或停止监听。

## 开始使用

1. 从 [Releases](https://github.com/yuxino/mimi/releases/latest) 下载对应平台版本。
2. 填入阿里云百炼（DashScope）API Key。
3. 播放内容，点击 **开始**。

API Key 保存在系统钥匙串中（macOS 钥匙串 / Windows 凭据管理器）。mimi 使用 DashScope 统一端点，只需 API Key 即可，无需 Workspace ID。模型调用可能产生费用。

[创建 API Key](https://help.aliyun.com/zh/model-studio/get-api-key)

### 平台说明

- **macOS**：首次监听时系统会请求「屏幕录制」权限（用于采集系统音频，mimi 不录制屏幕内容，也排除了自身声音）。
- **Windows**：使用 WASAPI 环回采集默认播放设备的整体混音，无需任何权限授权；mimi 不播放声音，因此没有回声问题。

## 从源码构建

需要 Rust 1.85+ 和 Node.js 20+（macOS 上还需 Xcode Command Line Tools）。

```bash
git clone https://github.com/yuxino/mimi.git
cd mimi
npm install
npm run tauri dev        # 开发运行（裸二进制）
./scripts/check.sh       # 完整检查（fmt/clippy/测试/前端构建）
./scripts/package-app.sh # 打包（macOS: .dmg；Windows: .msi/.nsis）
```

Windows 打包请在 Windows 机器上执行（Rust 依赖的 C 代码无法从 macOS 交叉编译到 MSVC 目标）；CI 会在 macOS 与 Windows 两个平台跑完整的 Rust 测试。

### macOS 开发说明

- `npm run tauri dev` 运行的是裸二进制，没有 `.app` 包——macOS 只从 bundle 读图标，所以 Dock 里只会显示通用图标。要看到正确的应用图标，请改用 `./scripts/dev-app.sh`：它会按 `tauri build` 的方式构建 release 二进制（带 `--features tauri/custom-protocol`，否则 Tauri 会把构建当 dev、运行时把 Dock 图标换成未蒙版的方形），包成真正的 `.app`（原版图标，窗口标题/托盘提示带 "(dev)" 标记）再启动，Dock 图标与正式版完全一致。
- 本机构建优先用稳定的 `mimi Local Development` 签名身份（见 `scripts/codesign-identity.sh`），这样屏幕录制与钥匙串授权在每次重新构建后仍然有效（每个应用身份需各授权一次）。

## 测试

```bash
cd src-tauri && cargo test   # Rust 单元测试（协议、字幕组装、配置、PCM 等）
npm run test                 # 前端 vitest
```

UI 冒烟可用 `MIMI_UI_TEST=1`（注入演示凭证）与 `MIMI_AUTO_START=1`（启动后自动开始会话，用于观察错误路径）配合 `npm run tauri dev`。

更多内容见 [CONTRIBUTING.md](CONTRIBUTING.md) 和 [SECURITY.md](SECURITY.md)。

[MIT](LICENSE) © 2026 yuxino
