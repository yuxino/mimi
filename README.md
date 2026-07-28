<div align="center">
  <img src="Resources/Assets/mimi-cat.png" width="88" alt="mimi">
  <h1>mimi</h1>
  <p>Mac 上的实时翻译字幕</p>
  <p>
    <a href="https://github.com/yuxino/mimi/releases/latest"><strong>下载 mimi</strong></a>
    · <a href="README_EN.md">English</a>
    · <a href="README_JA.md">日本語</a>
  </p>
</div>

<br>

![mimi 为番剧显示实时中文字幕](docs/images/mimi-anime-train.png)

<br>

mimi，日语里“耳朵”的意思。

它听取 Mac 正在播放的声音，把日语、英语或韩语实时翻译成简体中文、英语或日语字幕。浏览器、播放器和桌面应用都可以使用。

字幕窗可以移动、缩放和锁定。当前对白保持清楚，刚刚说过的内容会慢慢淡去。

## 开始使用

1. 从 [Releases](https://github.com/yuxino/mimi/releases/latest) 下载最新版
2. 填入阿里云百炼的 Workspace ID 和 API Key
3. 播放视频，点击 **Start Listening**

mimi 支持英语、日语和韩语。你可以让它自动判断正在播放的语言，再选择显示原文，或翻译成简体中文、英语或日语。

> mimi 目前尚未经过 Apple 公证。如果首次打开被 macOS 拦截，请前往“系统设置 → 隐私与安全性”，点击“仍要打开”。

## 只做字幕，不多打扰

mimi 不使用麦克风，不需要注册账号，也不会保存音频和字幕记录。API Key 保存在 macOS 钥匙串中。

[创建 API Key](https://help.aliyun.com/zh/model-studio/get-api-key) · [查找 Workspace ID](https://help.aliyun.com/zh/model-studio/obtain-the-app-id-and-workspace-id)

> Workspace ID 和 API Key 需要来自华北 2（北京）的同一个业务空间。模型调用可能产生费用。

## 使用提示

- 拖动字幕背景可以移动窗口，拖动边缘可以调整大小
- 锁定之后，鼠标可以直接穿过字幕窗操作视频
- 右上角的橡皮擦会清空当前字幕
- 如果同时出现两块字幕，请关闭 Chrome 的“实时字幕 / 实时翻译”

<details>
<summary>从源码构建</summary>

需要 macOS 14+，以及 Xcode 16 或带 Swift 6 的 Xcode Command Line Tools。

```bash
git clone https://github.com/yuxino/mimi.git
cd mimi
swift run mimi-core-tests
swift build -c release -Xswiftc -warnings-as-errors
./scripts/package-app.sh
open dist/mimi.app
```

</details>

## 参与 mimi

欢迎提交 [Issue](https://github.com/yuxino/mimi/issues) 和 Pull Request。参与开发前请阅读 [CONTRIBUTING.md](CONTRIBUTING.md)；安全问题请按 [SECURITY.md](SECURITY.md) 私下报告。

[MIT](LICENSE) © 2026 yuxino
