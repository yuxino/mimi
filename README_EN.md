<div align="center">
  <img src="src-tauri/icons/128x128@2x.png" width="96" alt="mimi">
  <h1>mimi</h1>
  <p>Live subtitles and translation for system audio on Apple silicon macOS 13+ and Windows x64.</p>
  <p>
    <a href="https://github.com/yuxino/mimi/releases/latest"><strong>Download latest</strong></a>
    · <a href="README.md">简体中文</a>
    · <a href="README_JA.md">日本語</a>
  </p>
</div>

`mimi` comes from the Japanese word 耳（みみ, “ear”）. It turns the system audio playing on your device into live subtitles and provider-dependent translation into Simplified Chinese, English, or Japanese.

## Features

- **Live subtitles and translation** — captures system output audio; source languages, targets, and quality modes vary by provider.
- **Service configurations** — save and switch between services without repeatedly entering credentials.
- **Flexible overlay** — move, resize, collapse, pause, enable click-through, or use Immersive Mode.
- **Signed in-app updates** — manually check, download, and install updates in Settings; every download must pass signature verification.
- **Privacy** — no mimi account, microphone, or screen capture; no saved audio or subtitles; audio goes only to the active provider.
- **Shortcuts** — macOS uses **⌘ ⇧ Space** / **⌘ ⇧ M** and Windows uses **Ctrl+Shift+Space** / **Ctrl+Shift+M** to control listening and Immersive Mode.

## Get started

1. Download the macOS Apple silicon DMG or Windows x64 EXE / MSI from the [latest release](https://github.com/yuxino/mimi/releases/latest), or build from source.
2. Open **Translation Service**, choose a provider, and save its credentials.
3. Play something and select **Start** from the mimi menu bar/system tray icon; on first use, macOS then prompts for **Screen & System Audio Recording**.

**v1.3.8 is the first published in-app updater bootstrap release.** Upgrading from the previous public release, v1.3.6, requires one manual download and installation from GitHub Releases. Later releases can be installed from **Settings → Software Update**. On Windows, installing an update closes Mimi; reopen it manually after the installer finishes.

Credentials are stored per service configuration in macOS Keychain or Windows Credential Manager; Settings shows only whether they are saved. Provider usage may incur charges.

[Alibaba Cloud](https://help.aliyun.com/en/model-studio/get-api-key) · [OpenAI](https://platform.openai.com/api-keys) · [Google Gemini](https://aistudio.google.com/app/apikey) · [Azure OpenAI](https://learn.microsoft.com/en-us/azure/foundry/openai/concepts/gpt-realtime-translate) · [Volcano Engine](https://docs.volcengine.com/docs/6561/1631605) · [Tencent Cloud](https://cloud.tencent.com/document/api/1093/127565) · [Baidu Translate](https://cloud.baidu.com/doc/MT/s/Sl9p2h5k9) · [xAI](https://docs.x.ai/developers/model-capabilities/audio/speech-to-speech)

| Provider | Source audio | Subtitle targets | Modes |
| --- | --- | --- | --- |
| Alibaba Cloud Model Studio | Auto, Chinese, English, Japanese, Korean | Original, Simplified Chinese, English, Japanese | Turbo, Low latency, High quality |
| OpenAI Realtime | Auto | Simplified Chinese, English, Japanese | Turbo |
| Google Gemini Live Translate (Preview) | Auto | Simplified Chinese, English, Japanese | Turbo |
| Azure OpenAI Realtime Translate | Auto | Simplified Chinese, English, Japanese | Turbo |
| Volcano Engine Simultaneous Interpretation 2.0 | Chinese, English, Japanese | Simplified Chinese, English, Japanese | Turbo |
| Tencent Cloud Realtime Speech Translation | Chinese, English, Japanese, Korean | Simplified Chinese, English, Japanese | Turbo |
| Baidu Realtime Speech Translation | Chinese, English, Japanese, Korean | Simplified Chinese, English, Japanese | Turbo |
| xAI Grok Voice | Auto | Simplified Chinese, English, Japanese | Turbo (turn based) |

Gemini, Azure OpenAI, Volcano Engine, Tencent, Baidu, and xAI have protocol, mock-WebSocket, and UI-logic coverage. Paid-account end-to-end quality and latency have not yet been accepted for every service.

### Platform support

- **Apple silicon macOS 13+**: Releases provide an ad-hoc-signed DMG without Apple notarization. If first launch is blocked, choose **Open Anyway** in **System Settings → Privacy & Security**. Updates may trigger recording or Keychain permission prompts again.
- **Windows x64**: System-audio capture, Windows Credential Manager, the system tray, the subtitle overlay, and global shortcuts are implemented. Releases provide unsigned preview MSI and NSIS EXE installers while end-to-end real-device acceptance of the public x64 packages continues; SmartScreen may warn.

## Build from source

Requires Rust 1.88+ and Node.js 20.19.x, 22.13+, or 24+. macOS also needs the Xcode Command Line Tools and a `mimi Local Development` identity or explicit `MIMI_CODESIGN_IDENTITY`.

```bash
git clone https://github.com/yuxino/mimi.git
cd mimi
npm ci
npm run tauri:dev        # develop on Windows with isolated dev settings
./scripts/dev-app.sh     # develop on macOS with a stable app identity
./scripts/check.sh       # full check (fmt/clippy/tests/frontend build)
```

Build Windows installers on Windows with `npm run tauri -- build -- --locked`; use `./scripts/package-app.sh` on macOS. CI tests or smoke-launches macOS, Windows x64, and Windows ARM64 builds.

See [CONTRIBUTING.md](CONTRIBUTING.md) and [SECURITY.md](SECURITY.md).

[MIT](LICENSE) © 2026 yuxino
