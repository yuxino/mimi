<div align="center">
  <img src="Resources/Assets/mimi-cat.png" width="112" alt="mimi">
  <h1>mimi</h1>
  <p><strong>Chinese subtitles for whatever is playing on your Mac.</strong></p>
  <p>Films, livestreams, and courses—ready when you press play.</p>
  <p>
    <a href="https://github.com/yuxino/mimi/releases/latest"><strong>Download mimi</strong></a>
    · <a href="README.md">简体中文</a>
  </p>
</div>

![mimi translating dialogue over a video](docs/images/mimi-product-hero.png)

## Subtitles that belong in the picture

mimi listens to the sound playing on your Mac. Whether it comes from a browser, a media player, or a desktop app, the Chinese subtitles sit quietly over the picture.

It starts before the speaker finishes, then quietly refines the line as more of it becomes clear. Pauses, tone, and recent context remain part of the translation.

The current line stays clear while earlier dialogue gently recedes. Place the subtitle window where it feels right, resize it, and lock it. Once locked, it stays out of the way of your mouse.

## Get started

1. Download the latest version from [Releases](https://github.com/yuxino/mimi/releases/latest)
2. Add your Alibaba Cloud Model Studio Workspace ID and API key
3. Play a video and select **Start Listening**

mimi translates English, Japanese, and Korean into Simplified Chinese. Let it detect the language or choose one yourself.

> mimi is not yet notarized by Apple. If macOS blocks the first launch, open System Settings → Privacy & Security and select **Open Anyway**.

## Your subtitles stay yours

mimi captures system playback only. It never uses your microphone. Audio goes directly to the Alibaba Cloud Model Studio workspace you configure, without passing through a mimi server.

Your API key stays in macOS Keychain. There are no mimi accounts, analytics, saved recordings, or transcript history.

[Create an API key](https://help.aliyun.com/en/model-studio/get-api-key) · [Find your Workspace ID](https://help.aliyun.com/en/model-studio/obtain-the-app-id-and-workspace-id)

> The Workspace ID and API key must belong to the same China (Beijing) workspace. Model usage may incur charges.

## A few useful details

- Drag the panel background to move it; drag an edge to resize it
- Lock the panel to click through it
- Use the eraser in the top-right corner to clear the current subtitles
- If two subtitle panels appear, disable Chrome Live Caption / Live Translate

<details>
<summary>Build from source</summary>

Requires macOS 14+ and Xcode 16 or Xcode Command Line Tools with Swift 6.

```bash
git clone https://github.com/yuxino/mimi.git
cd mimi
swift run mimi-core-tests
swift build -c release -Xswiftc -warnings-as-errors
./scripts/package-app.sh
open dist/mimi.app
```

</details>

## Contributing

Issues and pull requests are welcome. Read [CONTRIBUTING.md](CONTRIBUTING.md) before starting and report vulnerabilities privately according to [SECURITY.md](SECURITY.md).

[MIT](LICENSE) © 2026 yuxino
