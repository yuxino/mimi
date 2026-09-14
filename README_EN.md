<div align="center">
  <img src="src-tauri/icons/128x128@2x.png" width="96" alt="mimi">
  <h1>mimi</h1>
  <p>Live subtitles and translation for system audio on Apple silicon macOS 13+ and Windows x64.</p>
  <p>
    <a href="https://mimi.yuxino.cn">Website</a>
    · <a href="https://github.com/yuxino/mimi/releases/latest"><strong>Download latest</strong></a>
    · <a href="README_ZH.md">简体中文</a>
  </p>
</div>

`mimi` means “ear” in Japanese. It turns the system audio playing on your device into live subtitles and provider-dependent translation into Simplified Chinese, English, or Japanese.

<!-- project-demo-v1 -->
## Demo

https://github.com/user-attachments/assets/1cfde3d3-a732-4192-a496-e1d59d4f88d5

<p align="center">A complete 4K / 60 fps tour with English female narration, captions, and original film audio.</p>
<p align="center"><a href="https://mimi.yuxino.cn/en/#demo">Watch in English</a> · <a href="https://mimi.yuxino.cn/#demo">观看中文版</a> · <a href="docs/demos/full-tour-4k.md">Video details and credits</a></p>
<!-- /project-demo-v1 -->

## Features

- **Live subtitles and translation** — captures system output audio; source languages, targets, and quality modes vary by provider.
- **Service configurations** — save and switch between services without repeatedly entering credentials.
- **Flexible overlay** — move, resize, collapse, pause, enable click-through, or use Immersive Mode.
- **In-app updates** — check and install updates in Settings.
- **Privacy** — no mimi account, microphone, or screen capture; no saved audio or subtitles; audio goes only to the active provider.

## Get started

1. Download the macOS Apple silicon DMG or Windows x64 EXE / MSI from the [latest release](https://github.com/yuxino/mimi/releases/latest), or build from source.
2. Open **Translation Service**, choose a provider, and save its credentials.
3. Play something and select **Start** from the mimi menu bar/system tray icon; on first use, macOS then prompts for **Screen & System Audio Recording**.

Bring your own provider API credentials; usage charges may apply. Credentials are stored in the OS credential store.

To update, open **Settings → General → Software Update**. Mimi downloads the
update with progress, then lets you install it. Windows reopens Mimi after
installation; macOS offers a separate **Restart and Finish Update** action.
Versions older than v1.3.8 need one manual installation to enable in-app updates.

### Platform support

- **Apple silicon macOS 13+**: Releases provide an ad-hoc-signed DMG without Apple notarization. If first launch is blocked, choose **Open Anyway** in **System Settings → Privacy & Security**. Updates may trigger recording or Keychain permission prompts again.
- **Windows x64**: Unsigned preview EXE / MSI installers are available; SmartScreen may warn.

## Development

See the [contributing guide](CONTRIBUTING.md) for building and contributing, and the [security policy](SECURITY.md) for reporting vulnerabilities.

[MIT](LICENSE) © 2026 yuxino
