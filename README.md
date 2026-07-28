<div align="center">
  <img src="Resources/Assets/mimi-cat.png" width="112" alt="mimi">
  <h1>mimi</h1>
  <p><strong>给 Mac 上正在播放的声音，加上中文字幕。</strong></p>
  <p>电影、直播、课程，打开就能看懂。</p>
  <p>
    <a href="https://github.com/yuxino/mimi/releases/latest"><strong>下载 mimi</strong></a>
    · <a href="README_EN.md">English</a>
  </p>
</div>

![mimi 实时翻译视频中的对话](docs/images/mimi-product-hero.png)

## 字幕留在画面里

mimi 听的是 Mac 正在播放的声音。无论内容来自浏览器、播放器还是桌面应用，中文字幕都会安静地待在画面上。

它不会等整句话结束才开始工作。正在说的内容先出现，听清之后再自然地补完整。人物说话的停顿、语气和前后文，也会尽量留在译文里。

当前一句始终清楚，刚刚说过的话逐渐淡去。字幕窗可以放在顺眼的位置；调好大小，锁住，它就不再挡住鼠标。

## 开始使用

1. 从 [Releases](https://github.com/yuxino/mimi/releases/latest) 下载最新版
2. 填入阿里云百炼的 Workspace ID 和 API Key
3. 播放视频，点击 **Start Listening**

mimi 支持英语、日语和韩语，并将它们翻译成简体中文。你可以让它自动判断，也可以指定一种语言。

> mimi 目前尚未经过 Apple 公证。如果首次打开被 macOS 拦截，请前往“系统设置 → 隐私与安全性”，点击“仍要打开”。

## 这是你的字幕

mimi 只采集系统正在播放的音频，不使用麦克风。音频直接发送到你配置的阿里云百炼工作空间，不经过 mimi 的服务器。

API Key 保存在 macOS 钥匙串中。应用没有账号系统，不做使用统计，也不会保存你的音频和字幕记录。

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
