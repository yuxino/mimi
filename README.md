<div align="center">
  <img src="src-tauri/icons/128x128@2x.png" width="96" alt="mimi">
  <h1>mimi</h1>
  <p>Live translated subtitles for anything playing on your Mac or PC.</p>
  <p>
    <a href="https://github.com/yuxino/mimi/releases/latest"><strong>Download mimi</strong></a>
    · <a href="README_ZH.md">简体中文</a>
    · <a href="README_JA.md">日本語</a>
  </p>
</div>

`mimi` comes from the Japanese word 耳（みみ, “ear”）.

Turn Chinese, Japanese, English, or Korean audio playing on your device into live subtitles, with optional translation into Simplified Chinese, English, or Japanese. Built on Tauri v2 (Rust + React); the same codebase targets macOS and Windows.

## Features

- **Live subtitles** — works with browsers, players, games, meetings, and desktop apps.
- **Live translation** — Turbo, Low latency, and High quality modes.
- **Service configurations** — save multiple provider configurations and switch without re-entering credentials.
- **Flexible overlay** — move, resize, collapse, or lock the subtitle panel for click-through.
- **Multiple languages** — recognize Chinese, Japanese, English, and Korean.
- **Privacy** — no microphone, no mimi account, and no saved audio or subtitle history.
- **Global shortcuts** — **⌘ ⇧ Space** on macOS or **Ctrl+Shift+Space** on Windows starts/stops listening; **⌘ ⇧ M** or **Ctrl+Shift+M** toggles Immersive Mode.

## Get started

1. Download the latest version for your platform from [Releases](https://github.com/yuxino/mimi/releases/latest).
2. Open **Service configurations**, choose a provider, and save its connection credentials. Alibaba Cloud remains selected by default.
3. Play something and select **Start**.

Each service configuration's credentials are stored separately in the operating system's secure credential storage (macOS Keychain / Windows Credential Manager) and are never read back into the settings page. Single-key providers need only an API key; Azure asks for the resource endpoint plus separate translation and transcription deployment names, while Tencent and Baidu show their own official fields. Existing Alibaba Cloud settings migrate to the default service configuration automatically. Provider usage may incur charges.

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
| xAI Grok Voice | Auto | Simplified Chinese, English, Japanese | Turbo, turn based |

### Platform notes

- **macOS 13 or later**: if the first launch is blocked, choose **Open Anyway** in **System Settings → Privacy & Security**. From v1.3.1, updates using the same release identity normally keep Screen & System Audio Recording access; upgrading from an older version may require one more approval. mimi captures system audio only, never the screen, and excludes its own sound.
- **Windows**: captures the default playback device's full mix through WASAPI loopback — no permissions needed; mimi plays no audio, so there is no echo.

## Build from source

Requires Rust 1.85+ and Node.js 20+ (macOS also needs the Xcode Command Line Tools).

```bash
git clone https://github.com/yuxino/mimi.git
cd mimi
npm ci
./scripts/dev-app.sh     # develop on macOS with a stable app identity
npm run tauri:dev        # develop on Windows with isolated dev settings
./scripts/check.sh       # full check (fmt/clippy/tests/frontend build)
./scripts/package-app.sh # package (macOS: .dmg; Windows: .msi/.nsis)
```

Build the Windows package on a Windows machine (Rust's C dependencies cannot cross-compile from macOS to the MSVC target); CI runs the full Rust suite on both macOS and Windows.

### macOS dev notes

- Always launch a working macOS build through `./scripts/dev-app.sh`. It packages and verifies a real `.app`, installs it at one canonical path, and refuses an unstable ad-hoc identity. This keeps the Screen & System Audio Recording identity stable across rebuilds; because the local certificate is self-signed and has no Apple Team ID, macOS may still ask once for Keychain access after a binary update.
- The launcher selects the stable `mimi Local Development` identity through `scripts/codesign-identity.sh`. Avoid any `tauri dev` command or bare binary on macOS: each ad-hoc rebuild can look like a different app to macOS and request permission again.
- Development builds use a separate app identifier, settings directory, and credential namespace, so they never read or modify an installed release's service configurations or API keys.

## Tests

```bash
cargo test --manifest-path src-tauri/Cargo.toml   # Rust unit tests (protocols, subtitle assembly, config, PCM…)
npm run test                                      # frontend vitest
```

For macOS UI smoke tests, run `./scripts/dev-app.sh --ui-only`; UI-test mode does not access real credentials, provider networks, or system-audio capture. Use the normal command for an end-to-end provider session with credentials from local secure credential storage.

See [CONTRIBUTING.md](CONTRIBUTING.md) and [SECURITY.md](SECURITY.md).

[MIT](LICENSE) © 2026 yuxino
