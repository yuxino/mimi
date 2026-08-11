<div align="center">
  <img src="Resources/Assets/mimi-icon.png" width="96" alt="mimi">
  <h1>mimi</h1>
  <p>Mac 上的实时翻译字幕</p>
  <p>
    <a href="https://github.com/yuxino/mimi/releases/latest"><strong>下载 mimi</strong></a>
    · <a href="README_EN.md">English</a>
    · <a href="README_JA.md">日本語</a>
  </p>
</div>

mimi 取自日语「耳（みみ）」。它听取 Mac 正在播放的声音，将中文、日语、英语或韩语实时显示为原文字幕，或翻译成简体中文、英语或日语。

浏览器、播放器、线上会议、网课和桌面应用都能使用。

<table>
  <tr>
    <td width="33.33%"><img src="docs/images/mimi-film-real.jpg" alt="mimi 为日本剧情片显示实时中文字幕"></td>
    <td width="33.33%"><img src="docs/images/mimi-game-real.jpg" alt="mimi 为写实剧情游戏显示实时中文字幕"></td>
    <td width="33.33%"><img src="docs/images/mimi-live-real.jpg" alt="mimi 为日本直播显示实时中文字幕"></td>
  </tr>
  <tr>
    <td align="center">看懂没有字幕的海外影视</td>
    <td align="center">玩懂对白很多的剧情游戏</td>
    <td align="center">听懂直播、播客和旅行分享</td>
  </tr>
  <tr>
    <td width="33.33%"><img src="docs/images/mimi-romance-real.jpg" alt="mimi 为外语短剧和人物对白显示实时中文字幕"></td>
    <td width="33.33%"><img src="docs/images/mimi-meeting-real.jpg" alt="mimi 为跨语言线上会议显示实时中文字幕"></td>
    <td width="33.33%"><img src="docs/images/mimi-course-real.jpg" alt="mimi 为海外网课和公开课显示实时中文字幕"></td>
  </tr>
  <tr>
    <td align="center">跟上外语短剧和人物对白</td>
    <td align="center">参加跨语言线上会议</td>
    <td align="center">听懂海外网课和公开课</td>
  </tr>
</table>

## 主要功能

- 字幕窗可以移动、缩放、锁定穿透，也能收起为小状态栏
- 当前对白保持清楚，历史字幕带时间逐渐淡出
- 支持暂停、清空、快速切换识别语言和断线自动恢复
- **⌘ ⇧ Space** 快速开始或停止实时字幕

## 开始使用

1. 从 [Releases](https://github.com/yuxino/mimi/releases/latest) 下载最新版
2. 填入阿里云百炼的 Workspace ID 和 API Key
3. 播放视频、进入会议或打开网课，点击 **Start Listening**

> mimi 目前尚未经过 Apple 公证。如果首次打开被 macOS 拦截，请前往“系统设置 → 隐私与安全性”，点击“仍要打开”。

## 隐私与配置

mimi 不使用麦克风，不需要注册账号，也不会保存音频和字幕记录。API Key 保存在 macOS 钥匙串中。

[创建 API Key](https://help.aliyun.com/zh/model-studio/get-api-key) · [查找 Workspace ID](https://help.aliyun.com/zh/model-studio/obtain-the-app-id-and-workspace-id)

> Workspace ID 和 API Key 需要来自华北 2（北京）的同一个业务空间。模型调用可能产生费用。

## 使用提示

- 拖动顶部中央的短横移动字幕窗；四条边和四个角都可以自由调整大小
- 双击顶部可以收起或展开字幕窗
- 锁定之后，鼠标可以直接穿过字幕窗操作视频或会议窗口
- 左上角切换识别语言；右上角暂停或清空字幕
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
