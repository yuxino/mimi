<div align="center">
  <img src="src-tauri/icons/128x128@2x.png" width="96" alt="mimi">
  <h1>mimi</h1>
  <p>Live subtitles and translation for system audio. Apple silicon macOS 13+; Windows preview.</p>
  <p>
    <a href="https://github.com/yuxino/mimi/releases/latest"><strong>Download latest</strong></a>
    · <a href="README.md">简体中文</a>
    · <a href="README_JA.md">日本語</a>
  </p>
</div>

`mimi` comes from the Japanese word 耳（みみ, “ear”）.

Turn the system audio playing on your device into live subtitles, with provider-dependent translation into Simplified Chinese, English, or Japanese. Available inputs include Chinese, Japanese, English, and Korean.

## Features

- **Live subtitles** — captures the system output currently playing on your device.
- **Live translation** — targets and Turbo, Low latency, or High quality modes vary by provider.
- **Service configurations** — save and switch between services without repeatedly entering credentials.
- **Flexible overlay** — move, resize, collapse, pause, lock for click-through, or use Immersive Mode.
- **Multiple languages** — Chinese, Japanese, English, and Korean are available; exact options vary by provider.
- **Privacy** — no microphone or mimi account, no saved audio or subtitles, and audio goes only to the active provider.
- **Global shortcuts** — **⌘ ⇧ Space** on macOS or **Ctrl+Shift+Space** on Windows starts/stops listening; **⌘ ⇧ M** or **Ctrl+Shift+M** toggles Immersive Mode.

## Get started

1. Download the macOS Apple silicon DMG or Windows x64 EXE / MSI from the [latest release](https://github.com/yuxino/mimi/releases/latest), or build from source.
2. Open **Translation Service**, choose a provider, and save its credentials.
3. Play something and select **Start** from the mimi menu bar/system tray icon; on first use, macOS then prompts for **Screen & System Audio Recording**.

Each service configuration's credentials are stored separately in the operating system's secure credential storage (macOS Keychain / Windows Credential Manager). Settings show only whether credentials are saved and never read them back. Required fields vary by provider, and usage may incur charges.

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

### Platform notes

- **macOS 13 or later on Apple silicon**: GitHub Releases provide an ad-hoc-signed DMG without Apple notarization. If first launch is blocked, choose **Open Anyway** in **System Settings → Privacy & Security**. Updates may require approving Screen & System Audio Recording or Keychain access again. mimi captures system audio only, never the screen, and excludes its own sound.
- **Windows preview**: GitHub Releases provide x64 MSI and NSIS installers without Authenticode signing, so Windows Defender SmartScreen may warn. Real-device installation, trust prompts, credential storage, system-audio capture, overlay behavior, and the complete subtitle flow have not yet been accepted.

## Build from source

Requires Rust 1.88+ and Node.js 20.19.x, 22.13+, or 24+. macOS also needs the Xcode Command Line Tools and a `mimi Local Development` identity or explicit `MIMI_CODESIGN_IDENTITY`.

```bash
git clone https://github.com/yuxino/mimi.git
cd mimi
npm ci
./scripts/dev-app.sh     # develop on macOS with a stable app identity
npm run tauri:dev        # develop on Windows with isolated dev settings
./scripts/check.sh       # full check (fmt/clippy/tests/frontend build)
./scripts/package-app.sh # package (macOS: DMG; Windows: MSI / NSIS EXE)
```

Build Windows packages on Windows; CI runs the full Rust suite on both macOS and Windows.

### macOS dev notes

- Always launch macOS development builds through `./scripts/dev-app.sh`. It verifies a stable-signed `.app` at one canonical path; because the local certificate has no Apple Team ID, a binary update may still prompt once for Keychain access.
- The launcher selects the `mimi Local Development` identity through `scripts/codesign-identity.sh`. Avoid `tauri dev` or bare binaries on macOS so rebuilds are not treated as new apps that need permission again.
- Development builds use a separate app identifier, settings directory, and credential namespace, so they never read or modify an installed release's service configurations or API keys.

## Tests

```bash
cargo test --manifest-path src-tauri/Cargo.toml   # Rust unit tests (protocols, subtitle assembly, config, PCM…)
npm run test                                      # frontend vitest
```

For macOS UI smoke tests, run `./scripts/dev-app.sh --ui-only`; it does not access real credentials, provider networks, or system audio. Use the normal launcher and local secure credentials for end-to-end checks.

See [CONTRIBUTING.md](CONTRIBUTING.md) and [SECURITY.md](SECURITY.md).

[MIT](LICENSE) © 2026 yuxino
