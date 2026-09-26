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

Mimi shows live subtitles in a floating window for films, live streams, lessons, and games playing on your computer. Your chosen cloud service transcribes the system audio or translates it into Simplified Chinese, English, or Japanese; available languages and modes depend on the service. The name `mimi` means “ear” in Japanese.

<!-- project-demo-v1 -->
## Demo

https://github.com/user-attachments/assets/5acd46bb-e6b5-4bb5-b280-d70d4e0cdbb4

<p align="center">A 4K / 60 fps tour of service setup, subtitle controls, and Immersive Mode in the macOS app, with English narration, captions, and original film audio.</p>
<p align="center"><a href="https://mimi.yuxino.cn/en/?lang=en#demo">Watch in English</a> · <a href="https://mimi.yuxino.cn/?lang=zh#demo">观看中文版</a> · <a href="docs/demos/full-tour-4k.md">Video details and credits</a></p>
<!-- /project-demo-v1 -->

## Features

- **Live subtitles and translation** — captures system output audio; source languages, targets, and quality modes vary by provider.
- **Service configurations** — save and switch between services without repeatedly entering credentials.
- **Subtitle overlay** — move, resize, collapse, pause, enable click-through, or use Immersive Mode.
- **In-app updates** — check and install updates in Settings.
- **Session export** — opt in under Settings → Session export to retain timestamped transcripts or record system audio, then stop and export TXT / WAV. Both switches are off by default.
- **Settings appearance** — light, dark, or follow the system.
- **Privacy** — no mimi account, microphone, or screen capture; audio goes only to the active provider. Session content stays in memory until you explicitly export it. Turning an option off clears its buffer; starting a new session or quitting clears both. Export before doing so.

Transcript retention is limited to 10,000 confirmed pairs / 2 MiB of text and audio to 64 MiB; reaching a limit stops retention and shows a notice. Transcript timestamps mark final confirmation time, not media playback time. Pauses and reconnect gaps are omitted from WAV audio, so it is not synchronized to transcript timestamps.

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

- **Apple silicon macOS 13+**: DMG installers are not Apple-notarized. If first launch is blocked, choose **Open Anyway** in **System Settings → Privacy & Security**. See the permission notes below when upgrading from an older build.
- **Windows x64**: Unsigned preview EXE / MSI installers are available; SmartScreen may warn. For an extract-and-run copy, download `mimi_<version>_x64-portable.zip`, extract it, and launch `mimi.exe`. WebView2 must already be installed (it is normally present on Windows 11). The ZIP does not move settings, service credentials, or exported files into its folder; those remain in their existing user-selected or OS-managed locations. Update this copy by quitting Mimi and replacing it with a new ZIP from Releases. The portable build does not run the in-app installer updater.

### macOS permissions after an update

The release pipeline now requires one fixed self-signed certificate across macOS builds. Earlier releases through v1.4.1 used ad-hoc signatures that changed with each build; installing the first release with the fixed identity may require one new recording grant. This source change does not alter an already installed app. Fixed signing prevents build-specific identity changes; it does not promise that macOS will never request consent again. Deleting a local signing certificate does not change the installed app's identity or fix this. Keep the release app at `/Applications/mimi.app`; use the separate `mimi-dev.app` for development.

**Recording is enabled, but Mimi still reports permission denied:** first quit and reopen Mimi and follow the normal permission prompt. If capture is still denied:

1. Quit Mimi. In **System Settings → Privacy & Security → Screen & System Audio Recording** (the label varies by macOS version), select and remove only the old **mimi** entry.
2. Use **+** to add `/Applications/mimi.app`, enable its permission, and complete any system authentication yourself. Leave `mimi-dev` and other apps alone.
3. Reopen that same app, start a session, and play audio containing speech to check that subtitles actually appear. An enabled switch alone does not confirm recovery.

This removes an old recording authorization, not a certificate or API key. If one attempt does not help, stop repeating the reset and [report the error](https://github.com/yuxino/mimi/issues), including your macOS and Mimi versions and installation source. Do not include API keys or subtitle content.

**Keychain asks for your password or access to a saved API key:** confirm that the request comes from the Mimi copy you intended to open. If the dialog offers **Always Allow**, it can retain access for that build, but cannot guarantee access after a self-signed binary changes. Do not delete saved credentials, certificates, or the login keychain, allow all apps to access a key, or run a system-wide permission reset. A `codesign` request for a development signing private key is a separate build-time prompt.

My wallet is still a little empty, and I’m saving up for Apple Developer membership (๑•̀ㅂ•́)و✧ Thank you for understanding! Stable self-signing does not need a paid membership; Apple notarization is a separate step. The manual recovery above is for a stale recording grant, not something you should have to repeat after every update.

## Development

See the [contributing guide](CONTRIBUTING.md) for building and contributing, and the [security policy](SECURITY.md) for reporting vulnerabilities.

## Community links

[LINUX DO](https://linux.do/)

[MIT](LICENSE) © 2026 yuxino
