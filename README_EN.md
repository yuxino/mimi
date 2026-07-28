<div align="center">
  <img src="Resources/Assets/mimi-cat.png" width="168" alt="mimi cat-eared subtitle icon">
  <h1>mimi</h1>
  <p><strong>Hear it. Translate it. Native live bilingual subtitles for macOS.</strong></p>
  <p>
    <a href="README.md">简体中文</a> ·
    <a href="README_EN.md">English</a>
  </p>
</div>

![mimi live translated subtitles](docs/images/mimi-subtitles.png)

`mimi` listens to system audio playing on your Mac, translates English, Japanese, or Korean speech into Simplified Chinese in real time, and keeps the result in an always-on-top subtitle window. No browser extension or microphone is required. Paste an Alibaba Cloud Model Studio Workspace ID and API key, then start listening.

> mimi is currently an early release and is not Apple-notarized yet. macOS requires a manual confirmation on first launch.

## Why mimi

- **Fast first response** — draft recognition and translation are prioritized.
- **Natural final subtitles** — meaningful speech particles and recent context are preserved.
- **Automatic language detection** — English, Japanese, and Korean can switch within one session.
- **Native overlay** — move, resize, lock, click through, and restore its last position.
- **Readable history** — recent translations remain visible and can be cleared manually.
- **No product tracking** — no mimi account, analytics, recording, or local transcript archive.
- **Local credentials** — the API key stays in macOS Keychain and is never bundled with the app.

## Start in three minutes

### 1. Prepare credentials

- macOS 14 or newer
- A Workspace ID and API key from Alibaba Cloud Model Studio, China (Beijing)

1. Select **China (Beijing)** in the Model Studio console.
2. [Create an API key](https://help.aliyun.com/en/model-studio/get-api-key), preferably in the default workspace.
3. Copy the Workspace ID from the console. See Alibaba Cloud's [Workspace ID guide](https://help.aliyun.com/en/model-studio/obtain-the-app-id-and-workspace-id).

The Workspace ID and API key must belong to the same workspace. Model calls may incur charges. Never paste an API key into an issue, log, or screenshot.

### 2. Download and open

1. Download `mimi-v0.1.0-macos.zip` from the [latest release](https://github.com/yuxino/mimi/releases/latest).
2. Unzip it and move `mimi.app` into Applications.
3. If macOS blocks the first launch, open System Settings → Privacy & Security and click **Open Anyway**.

On first launch:

1. Paste the Workspace ID and DashScope API key into Settings.
2. Click **Save**, then **Start Listening**.
3. Allow Screen & System Audio Recording in macOS.
4. If macOS asks for a restart, quit and reopen mimi.

![mimi settings](docs/images/mimi-settings.png)

<details>
<summary>Build from source</summary>

This requires Xcode 16 or Xcode Command Line Tools with Swift 6.

```bash
git clone https://github.com/yuxino/mimi.git
cd mimi
./scripts/package-app.sh
open dist/mimi.app
```

</details>

## Everyday use

- Automatic detection is the best default for mixed-language videos.
- For single-language content, choose English, 日本語, or 한국어 explicitly.
- Drag the subtitle background to move the window and drag an edge to resize it.
- Locking the position makes the overlay click-through.
- Hover over the top-right corner to clear subtitles or open Settings.
- The menu-bar panel also provides Show, Clear, Lock, and Quit actions.

If a second box labeled “Translating” appears, it is usually Chrome Live Caption. Disable Live Caption and Live Translate under Chrome Settings → Accessibility.

## How it works

Low-latency mode uses two independent translation paths:

1. Qwen Realtime ASR continuously emits source-language drafts.
2. Qwen-MT-Lite translates drafts for responsiveness.
3. Qwen-MT-Flash produces more natural final subtitles.
4. Up to three confirmed pairs are supplied as bounded translation memory for conversational continuity.

Apple ScreenCaptureKit captures system audio locally. Audio is sent directly to the Alibaba Cloud Model Studio workspace configured by the user; mimi operates no relay server.

## Development and testing

```bash
# Dependency-free deterministic core tests
swift run mimi-core-tests

# Strict release build
swift build -c release -Xswiftc -warnings-as-errors

# Package, sign, and verify the app
./scripts/package-app.sh
```

Repository layout:

- `Sources/MimiCore` — realtime protocols, translation queues, subtitle state, and PCM encoding.
- `Sources/MimiApp` — SwiftUI/AppKit UI, system audio, Keychain, and floating overlay.
- `Sources/MimiReplay` — real-audio replay and latency metrics.
- `Tests/MimiCoreTests` — deterministic executable test suite.
- `docs/plans` — validated product designs and implementation notes.

## Troubleshooting

**No video audio is detected.**

Grant Screen & System Audio Recording and restart mimi after the permission change.

**The service returns 401 or 403.**

Make sure the Workspace ID and API key belong to the same China (Beijing) workspace and that the key can call the required models.

**Does mimi upload microphone audio or retain transcripts?**

No. mimi captures system playback only and does not request microphone access. The app does not persist audio or transcript history.

**Why can a rebuild request permission again?**

Without a stable local signing identity, the packaging script uses ad-hoc signing and macOS may treat the rebuild as a different app. See [CONTRIBUTING.md](CONTRIBUTING.md).

## Contributing

Issues and pull requests are welcome. Read [CONTRIBUTING.md](CONTRIBUTING.md) before starting. Report vulnerabilities privately according to [SECURITY.md](SECURITY.md), and never include API keys.

## License

[MIT](LICENSE) © 2026 yuxino
