<div align="center">
  <img src="Resources/Assets/mimi-cat.png" width="112" alt="mimi 猫耳字幕图标">
  <h1>mimi</h1>
  <p><strong>看懂正在播放的每一句话。</strong></p>
  <p>macOS 原生实时翻译字幕 · 英语 / 日语 / 韩语 → 简体中文</p>
  <p>
    <a href="https://github.com/yuxino/mimi/releases/latest"><strong>下载最新版</strong></a>
    · <a href="README_EN.md">English</a>
  </p>
  <p>
    <img alt="macOS 14+" src="https://img.shields.io/badge/macOS-14%2B-111111?logo=apple">
    <img alt="Swift 6" src="https://img.shields.io/badge/Swift-6-F05138?logo=swift&logoColor=white">
    <img alt="MIT License" src="https://img.shields.io/badge/License-MIT-6FE0C1">
  </p>
</div>

![mimi 在视频上显示实时中文字幕](docs/images/mimi-product-hero.png)

## 视频继续放，字幕自己来

追剧、看课程、刷直播，不用暂停，不用复制台词，也不用安装浏览器插件。

mimi 直接听取 Mac 正在播放的声音，把译文放在视频上方。

| ⚡ 边听边出 | 💬 不是机翻腔 | 🪟 不打扰画面 |
| --- | --- | --- |
| 草稿先出现，不必等一句话说完 | 定稿参考前文，保留“嗯、啊、吧、呢”这些语气 | 字幕窗可移动、缩放、锁定并穿透鼠标 |

## 打开就能用

1. **下载** — 从 [Releases](https://github.com/yuxino/mimi/releases/latest) 获取 `mimi-v0.1.0-macos.zip`
2. **填配置** — 粘贴阿里云百炼的 Workspace ID 和 API Key
3. **开始听** — 播放视频，点击 **Start Listening**

就这些。mimi 会自动判断英语、日语或韩语，并持续翻译成简体中文。

> 当前版本尚未经过 Apple 公证。首次打开如果被 macOS 拦截，请前往“系统设置 → 隐私与安全性”，点击“仍要打开”。

## 用起来是什么感觉

### 字幕跟得上

识别和翻译分开工作：正在说的内容快速给出草稿，确认后的句子再换成更自然的定稿。视频节奏不会被翻译拖住。

### 连续对话不再句句失忆

mimi 会参考最近几句已经确认的字幕，补回对话里省略的主语和语气。不是把每个词硬拼成中文。

### 长句、短句都留得住

最近的译文会保留在字幕窗中，不再一闪而过。想重新开始时，点右上角橡皮擦即可清空。

## 适合这些时候

- **追海外剧和动画**：对白不断，字幕始终跟在画面下方
- **看课程与发布会**：长句自动换行，前文还能回看
- **直播和访谈**：语言变化时自动识别，不必来回切换
- **任何播放器**：浏览器、桌面客户端或本地视频都可以

## 配置阿里云百炼

mimi 当前使用阿里云百炼 **华北 2（北京）**地域：

1. 按[官方说明创建 API Key](https://help.aliyun.com/zh/model-studio/get-api-key)
2. 在控制台右上角复制 [Workspace ID](https://help.aliyun.com/zh/model-studio/obtain-the-app-id-and-workspace-id)
3. 确认 Workspace ID 与 API Key 属于同一个业务空间

API Key 保存在 macOS 钥匙串中，不会写入项目或应用包。模型调用可能产生费用，请不要把 Key 发到 Issue、日志或截图里。

## 你的声音去了哪里

- 只采集 Mac 正在播放的**系统音频**，不请求麦克风
- 音频直接发送到你配置的阿里云百炼工作空间
- mimi 没有中转服务器、用户账号或分析统计
- 应用不会在本地保存音频和字幕记录

## 使用小贴士

- 自动识别适合语言会变化的视频；单一语言也可以手动指定
- 拖动字幕背景可移动，拖动边缘可调整大小
- 锁定位置后字幕窗会穿透鼠标，不影响视频操作
- 如果出现第二块“正在翻译”字幕，关闭 Chrome 的“实时字幕 / 实时翻译”

<details>
<summary><strong>从源码构建与开发</strong></summary>

需要 macOS 14+，以及 Xcode 16 或带 Swift 6 的 Xcode Command Line Tools。

```bash
git clone https://github.com/yuxino/mimi.git
cd mimi

swift run mimi-core-tests
swift build -c release -Xswiftc -warnings-as-errors
./scripts/package-app.sh
open dist/mimi.app
```

项目结构：

- `Sources/MimiCore`：实时协议、翻译队列、字幕状态与 PCM 编码
- `Sources/MimiApp`：SwiftUI / AppKit 界面、系统音频、钥匙串和悬浮窗
- `Sources/MimiReplay`：真实音频回放与延迟指标
- `Tests/MimiCoreTests`：确定性核心测试

</details>

## 常见问题

**没有字幕？**

确认已经允许“屏幕与系统音频录制”，并在授权后重启 mimi。

**提示 401 或 403？**

Workspace ID 与 API Key 必须来自华北 2（北京）的同一个业务空间。

**每次构建都重新申请权限？**

没有稳定签名证书时，macOS 可能把新构建识别成另一个应用。下载 Release 版本或配置稳定的本地签名可以减少这种情况。

## 一起让 mimi 更好

欢迎提交 [Issue](https://github.com/yuxino/mimi/issues) 和 Pull Request。开始前请阅读 [CONTRIBUTING.md](CONTRIBUTING.md)；安全问题请按 [SECURITY.md](SECURITY.md) 私下报告。

[MIT](LICENSE) © 2026 yuxino
