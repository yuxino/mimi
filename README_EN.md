<div align="center">
  <img src="Resources/Assets/mimi-cat.png" width="112" alt="mimi cat-eared subtitle icon">
  <h1>mimi</h1>
  <p><strong>Understand every line playing on your Mac.</strong></p>
  <p>Native live translated subtitles for macOS · English / Japanese / Korean → Simplified Chinese</p>
  <p>
    <a href="https://github.com/yuxino/mimi/releases/latest"><strong>Download latest</strong></a>
    · <a href="README.md">简体中文</a>
  </p>
  <p>
    <img alt="macOS 14+" src="https://img.shields.io/badge/macOS-14%2B-111111?logo=apple">
    <img alt="Swift 6" src="https://img.shields.io/badge/Swift-6-F05138?logo=swift&logoColor=white">
    <img alt="MIT License" src="https://img.shields.io/badge/License-MIT-6FE0C1">
  </p>
</div>

![mimi showing live Chinese subtitles over a video](docs/images/mimi-product-hero.png)

## Keep watching. Let the subtitles catch up.

Drama, courses, livestreams, interviews—no pausing, copying dialogue, or browser extension.

mimi listens to system audio and keeps translated subtitles right over the video.

| ⚡ Appears while they speak | 💬 Sounds like dialogue | 🪟 Stays out of the way |
| --- | --- | --- |
| Fast drafts arrive before the sentence ends | Final lines use recent context and preserve conversational tone | Move, resize, lock, and click through the native overlay |

## Three steps to start

1. **Download** — get `mimi-v0.1.0-macos.zip` from [Releases](https://github.com/yuxino/mimi/releases/latest)
2. **Paste credentials** — add an Alibaba Cloud Model Studio Workspace ID and API key
3. **Listen** — play a video and click **Start Listening**

That is it. mimi detects English, Japanese, or Korean and continuously translates it into Simplified Chinese.

> This early release is not Apple-notarized. If macOS blocks the first launch, open System Settings → Privacy & Security and choose **Open Anyway**.

## What it feels like

### Subtitles that keep pace

Recognition and translation run independently. The current speech appears as a fast draft, then becomes a more natural final line without holding up the video.

### A conversation, not isolated sentences

mimi references a few recently confirmed subtitle pairs. Omitted subjects and conversational tone survive instead of every word being assembled mechanically.

### Short lines do not disappear

Recent translations stay in the overlay. When you want a fresh start, clear everything with the eraser in the top-right corner.

## Made for

- **International shows and animation** — dialogue keeps moving while subtitles stay below it
- **Courses and keynotes** — long sentences wrap cleanly and recent context remains readable
- **Livestreams and interviews** — automatic detection handles language changes
- **Any player** — browsers, desktop clients, and local video all work

## Alibaba Cloud Model Studio setup

mimi currently uses the **China (Beijing)** region:

1. [Create an API key](https://help.aliyun.com/en/model-studio/get-api-key)
2. Copy the [Workspace ID](https://help.aliyun.com/en/model-studio/obtain-the-app-id-and-workspace-id) from the console
3. Make sure the Workspace ID and API key belong to the same workspace

The API key stays in macOS Keychain and is never bundled with the app. Model calls may incur charges. Never post a key in an issue, log, or screenshot.

## Where your audio goes

- mimi captures **system playback only** and never requests microphone access
- audio goes directly to the Alibaba Cloud Model Studio workspace you configure
- mimi operates no relay server, user account, or product analytics
- the app does not persist audio or transcript history

## Quick tips

- Automatic detection works best for mixed-language videos; single-language hints are also available
- Drag the panel background to move it and drag an edge to resize it
- Locking the overlay makes it click-through
- If a second “Translating” box appears, disable Chrome Live Caption / Live Translate

<details>
<summary><strong>Build from source and develop</strong></summary>

Requires macOS 14+ and Xcode 16 or Xcode Command Line Tools with Swift 6.

```bash
git clone https://github.com/yuxino/mimi.git
cd mimi

swift run mimi-core-tests
swift build -c release -Xswiftc -warnings-as-errors
./scripts/package-app.sh
open dist/mimi.app
```

Repository layout:

- `Sources/MimiCore` — realtime protocols, translation queues, subtitle state, and PCM encoding
- `Sources/MimiApp` — SwiftUI / AppKit UI, system audio, Keychain, and floating overlay
- `Sources/MimiReplay` — real-audio replay and latency metrics
- `Tests/MimiCoreTests` — deterministic core tests

</details>

## Troubleshooting

**No subtitles?**

Grant Screen & System Audio Recording and restart mimi after changing the permission.

**401 or 403?**

The Workspace ID and API key must belong to the same China (Beijing) workspace.

**Permission requested after every rebuild?**

Without a stable signing identity, macOS may treat each build as a different app. Use the Release build or configure a stable local identity.

## Make mimi better

Issues and pull requests are welcome. Read [CONTRIBUTING.md](CONTRIBUTING.md) before starting and report vulnerabilities privately according to [SECURITY.md](SECURITY.md).

[MIT](LICENSE) © 2026 yuxino
