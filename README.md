<div align="center">
  <img src="Resources/Assets/mimi-icon.png" width="96" alt="mimi">
  <h1>mimi</h1>
  <p>Live translated subtitles for anything playing on your Mac.</p>
  <p>
    <a href="https://github.com/yuxino/mimi/releases/latest"><strong>Download mimi</strong></a>
    · <a href="README_ZH.md">简体中文</a>
    · <a href="README_JA.md">日本語</a>
  </p>
</div>

`mimi` comes from the Japanese word 耳（みみ, “ear”）.

Turn Chinese, Japanese, English, or Korean audio playing on your Mac into live subtitles, with optional translation into Simplified Chinese, English, or Japanese.

<table>
  <tr>
    <td width="33.33%"><img src="docs/images/mimi-film-real-en.jpg" alt="mimi over a Japanese drama"></td>
    <td width="33.33%"><img src="docs/images/mimi-game-real-en.jpg" alt="mimi over a narrative game"></td>
    <td width="33.33%"><img src="docs/images/mimi-meeting-real-en.jpg" alt="mimi during an online meeting"></td>
  </tr>
  <tr>
    <td align="center">Films & videos</td>
    <td align="center">Games & livestreams</td>
    <td align="center">Meetings & courses</td>
  </tr>
</table>

## Features

- **Live subtitles** — works with browsers, players, games, meetings, and desktop apps.
- **Live translation** — Turbo, Low latency, and High quality modes.
- **Flexible overlay** — move, resize, collapse, or lock the subtitle panel for click-through.
- **Multiple languages** — recognize Chinese, Japanese, English, and Korean.
- **Privacy** — no microphone, no account, and no saved audio or subtitle history.
- **Global shortcut** — press **⌘ ⇧ Space** to start or stop listening.

## Get started

1. Download the latest version from [Releases](https://github.com/yuxino/mimi/releases/latest).
2. Add your Alibaba Cloud Model Studio Workspace ID and API key.
3. Play something and select **Start Listening**.

Your API key is stored in macOS Keychain. The Workspace ID and API key must belong to the same China (Beijing) workspace. Model usage may incur charges.

[Create an API key](https://help.aliyun.com/en/model-studio/get-api-key) · [Find your Workspace ID](https://help.aliyun.com/en/model-studio/obtain-the-app-id-and-workspace-id)

## Build from source

Requires macOS 14+ and Swift 6.

```bash
git clone https://github.com/yuxino/mimi.git
cd mimi
swift run mimi-core-tests
swift build -c release -Xswiftc -warnings-as-errors
./scripts/package-app.sh
open dist/mimi.app
```

See [CONTRIBUTING.md](CONTRIBUTING.md) and [SECURITY.md](SECURITY.md).

[MIT](LICENSE) © 2026 yuxino
