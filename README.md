<div align="center">
  <img src="Resources/Assets/mimi-cat.png" width="112" alt="mimi">
  <h1>mimi</h1>
  <p><strong>想看的内容，不必再等字幕。</strong></p>
  <p>mimi 为 Mac 上正在播放的外语视频，实时加上中文字幕。</p>
  <p>
    <a href="https://github.com/yuxino/mimi/releases/latest"><strong>下载 mimi</strong></a>
    · <a href="README_EN.md">English</a>
  </p>
</div>

![mimi 实时翻译视频中的对话](docs/images/mimi-product-hero.png)

## 打开视频，就有字幕

不用找字幕文件，不用切到翻译软件，也不用为了看懂一句话反复暂停。

mimi 跟着对白往前走。正在说的内容先出现，听清之后再补完整；人物的停顿和语气，也留在字幕里。

当前一句始终清楚，刚刚说过的话慢慢淡去。把字幕放到顺眼的位置，锁住，然后继续看。

## 下一集，现在就看

<table>
  <tr>
    <td width="50%"><img src="docs/images/mimi-anime-city.png" alt="mimi 翻译都市日常番"></td>
    <td width="50%"><img src="docs/images/mimi-anime-fantasy.png" alt="mimi 翻译幻想动画"></td>
  </tr>
</table>

日语番剧、动画电影、海外访谈，浏览器和播放器里的内容都能看。

![mimi 翻译电影访谈](docs/images/mimi-interview.png)

## 开始使用

1. 从 [Releases](https://github.com/yuxino/mimi/releases/latest) 下载最新版
2. 填入阿里云百炼的 Workspace ID 和 API Key
3. 播放视频，点击 **Start Listening**

mimi 支持英语、日语和韩语，并将它们翻译成简体中文。你可以让它自动判断，也可以指定一种语言。

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
