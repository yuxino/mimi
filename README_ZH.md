<div align="center">
  <img src="src-tauri/icons/128x128@2x.png" width="96" alt="mimi">
  <h1>mimi</h1>
  <p>系统音频实时字幕与翻译，支持 Apple 芯片 macOS 13+ 和 Windows x64。</p>
  <p>
    <a href="https://mimi.yuxino.cn">官网</a>
    · <a href="https://github.com/yuxino/mimi/releases/latest"><strong>下载最新版</strong></a>
    · <a href="README.md">English</a>
  </p>
</div>

`mimi` 在日语中意为“耳朵”。它把设备正在播放的系统音频变成实时字幕，并按服务商能力翻译成简体中文、英语或日语。

<!-- project-demo-v1 -->
## 演示

https://github.com/user-attachments/assets/3f2be409-6f43-407e-9bca-a8e4d3592dc3

<p align="center">播放英文视频，按快捷键切换沉浸模式，让字幕融入画面。</p>
<p align="center"><a href="docs/demos/README.md">视频说明与来源</a></p>
<!-- /project-demo-v1 -->

## 功能

- **实时字幕与翻译** — 采集系统输出音频；输入语言、翻译目标和质量模式随服务商而异。
- **服务配置** — 保存并切换多套服务配置，无需反复填写凭证。
- **字幕浮窗** — 支持移动、缩放、收起、暂停、点击穿透和沉浸模式。
- **应用内更新** — 在设置中检查并安装更新。
- **隐私** — 无需 mimi 账号，不使用麦克风、不录制屏幕，也不保存音频或字幕；系统音频只发送给当前服务商。

## 开始使用

1. 从 [Latest Release](https://github.com/yuxino/mimi/releases/latest) 下载 macOS Apple Silicon DMG 或 Windows x64 EXE / MSI；也可以从源码构建。
2. 打开「翻译服务」，选择服务商并保存凭证。
3. 播放内容，从菜单栏/系统托盘的 mimi 图标点击 **开始**；macOS 首次使用时按提示允许「屏幕与系统音频录制」。

需要自备服务商 API 凭证，调用可能产生费用。凭证保存在系统钥匙串中。

更新时，打开 **设置 → 通用 → 版本更新**。Mimi 会显示下载进度，下载完成后可直接安装。
Windows 安装完成后会重新打开 Mimi；macOS 可点击 **重新启动并完成更新**。
早于 v1.3.8 的旧版本需要先手动安装一次，之后即可在应用内更新。

### 平台支持

- **Apple 芯片 macOS 13+**：提供未经 Apple 公证的临时签名 DMG；若首次打开被拦截，请在「系统设置 → 隐私与安全性」中选择「仍要打开」。系统可能在更新后重新请求录音或钥匙串权限。
- **Windows x64**：提供未签名的预览版 EXE / MSI，SmartScreen 可能显示提示。

## 开发

构建与贡献请参阅 [贡献指南](CONTRIBUTING.md)，安全问题请参阅 [安全政策](SECURITY.md)。

[MIT](LICENSE) © 2026 yuxino
