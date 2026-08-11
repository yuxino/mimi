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

mimi 这个名字来自日语「耳（みみ）」的读音，写成罗马字就是 mimi。

它听取 Mac 正在播放的声音，把中文、日语、英语或韩语实时变成原文字幕，或翻译成简体中文、英语或日语。浏览器、播放器、线上会议和桌面应用都可以使用。

字幕窗可以移动、自由调整宽高，也可以锁定后让鼠标直接穿过。当前对白保持清楚，刚刚说过的内容会带着时间慢慢淡去。人物持续说话时，mimi 会把长段落切成容易阅读的小句，只让最后一小段继续更新。

<table>
  <tr>
    <td width="50%"><img src="docs/images/mimi-film-real.jpg" alt="mimi 为日本剧情片显示实时中文字幕"></td>
    <td width="50%"><img src="docs/images/mimi-game-real.jpg" alt="mimi 为写实剧情游戏显示实时中文字幕"></td>
  </tr>
  <tr>
    <td align="center">看懂没有字幕的海外影视</td>
    <td align="center">玩懂对白很多的剧情游戏</td>
  </tr>
  <tr>
    <td width="50%"><img src="docs/images/mimi-romance-real.jpg" alt="mimi 为成熟向夜间剧情显示实时中文字幕"></td>
    <td width="50%"><img src="docs/images/mimi-live-real.jpg" alt="mimi 为日本直播显示实时中文字幕"></td>
  </tr>
  <tr>
    <td align="center">跟上成熟向影片与夜间剧情</td>
    <td align="center">听懂直播、播客和旅行分享</td>
  </tr>
</table>

### 开会的时候也能用

后来发现 mimi 不只是拿来看视频。

开 Zoom、Meet、飞书这些会议的时候，只要声音是从 Mac 里出来的，它一样能听。碰到英文会议、海外面试，或者对方口音有点重的时候，挂在下面当字幕还挺方便。

它只听系统声音，不碰麦克风，所以也不会把自己说的话录进去。就是一个安安静静挂在旁边的字幕。

开会时也不用一直打开设置：字幕窗上可以直接 **暂停 / 继续**、**收起成一条小状态栏**，点左上角的语言状态还能马上切换识别语言。网络偶尔抖一下时，mimi 也会自己尝试重新连接，并保留已经显示出来的字幕。

## 开始使用

1. 从 [Releases](https://github.com/yuxino/mimi/releases/latest) 下载最新版
2. 填入阿里云百炼的 Workspace ID 和 API Key
3. 播放视频或进入会议，点击 **Start Listening**

当前界面可以手动选择 **中文原文、English、日本語、한국어** 作为识别语言。字幕可以显示原文，或翻译成简体中文、英语或日语；需要翻译时，当前版本使用高质量翻译。

> mimi 目前尚未经过 Apple 公证。如果首次打开被 macOS 拦截，请前往“系统设置 → 隐私与安全性”，点击“仍要打开”。

## 只做字幕，不多打扰

mimi 不使用麦克风，不需要注册账号，也不会保存音频和字幕记录。API Key 保存在 macOS 钥匙串中。

已经确认的小句会留在字幕窗里，刚刚说过的内容逐渐淡下去，正在说的这一句保持最清楚。连续讲话时会按标点和长度拆成容易阅读的小段，减少整段文字来回跳动。

[创建 API Key](https://help.aliyun.com/zh/model-studio/get-api-key) · [查找 Workspace ID](https://help.aliyun.com/zh/model-studio/obtain-the-app-id-and-workspace-id)

> Workspace ID 和 API Key 需要来自华北 2（北京）的同一个业务空间。模型调用可能产生费用。

## 使用提示

- `⌘ ⇧ Space` 可以快速开始或停止实时字幕
- 拖动顶部中央的短横移动字幕窗；四条边和四个角都可以自由调整大小
- 双击顶部可以把字幕收成一条小状态栏，再双击或点展开按钮恢复
- 右上角可以暂停 / 继续；暂停不会清掉已经显示的字幕
- 点左上角的语言状态，可以直接切换识别语言
- 左上角和底部波形会区分正在聆听、识别、翻译和暂停状态
- 淡色时间标记属于已经确认的字幕，当前一句保持醒目
- 锁定之后，鼠标可以直接穿过字幕窗操作视频或会议窗口
- 右上角的橡皮擦会清空当前字幕
- 短暂的网络中断会自动尝试恢复；恢复时不会主动清掉已经显示的字幕
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
