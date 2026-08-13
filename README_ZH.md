<div align="center">
  <img src="Resources/Assets/mimi-icon.png" width="96" alt="mimi">
  <h1>mimi</h1>
  <p>给 Mac 上正在播放的声音加上实时翻译字幕。</p>
  <p>
    <a href="https://github.com/yuxino/mimi/releases/latest"><strong>下载 mimi</strong></a>
    · <a href="README.md">English</a>
    · <a href="README_JA.md">日本語</a>
  </p>
</div>

`mimi` 取自日语「耳（みみ）」。

把 Mac 正在播放的中文、日语、英语或韩语实时变成字幕，也可以翻译成简体中文、英语或日语。

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
- **全局快捷键** — **⌘ ⇧ Space** 开始或停止监听。

## 开始使用

1. 从 [Releases](https://github.com/yuxino/mimi/releases/latest) 下载最新版。
2. 填入阿里云百炼 Workspace ID 和 API Key。
3. 播放内容，点击 **Start Listening**。

API Key 保存在 macOS 钥匙串中。Workspace ID 和 API Key 需要来自华北 2（北京）的同一个业务空间，模型调用可能产生费用。

[创建 API Key](https://help.aliyun.com/zh/model-studio/get-api-key) · [查找 Workspace ID](https://help.aliyun.com/zh/model-studio/obtain-the-app-id-and-workspace-id)

## 从源码构建

需要 macOS 14+ 和 Swift 6。

```bash
git clone https://github.com/yuxino/mimi.git
cd mimi
swift run mimi-core-tests
swift build -c release -Xswiftc -warnings-as-errors
./scripts/package-app.sh
open dist/mimi.app
```

更多内容见 [CONTRIBUTING.md](CONTRIBUTING.md) 和 [SECURITY.md](SECURITY.md)。

[MIT](LICENSE) © 2026 yuxino
