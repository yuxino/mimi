<div align="center">
  <img src="src-tauri/icons/128x128@2x.png" width="96" alt="mimi">
  <h1>mimi</h1>
  <p>Mac / Windows で再生中の音声にリアルタイム翻訳字幕</p>
  <p>
    <a href="https://github.com/yuxino/mimi/releases/latest"><strong>mimiをダウンロード</strong></a>
    · <a href="README_ZH.md">简体中文</a>
    · <a href="README.md">English</a>
  </p>
</div>

mimiは、日本語の「耳（みみ）」に由来する名前です。デバイスで再生中の中国語・日本語・英語・韓国語を原文字幕として表示したり、簡体字中国語・英語・日本語へリアルタイム翻訳したりできます。Tauri v2（Rust + React）製で、同じコードベースが macOS と Windows の両方に対応しています。

<table>
  <tr>
    <td width="33.33%"><img src="docs/images/mimi-film-real-ja.jpg" alt="ドラマに字幕を重ねた mimi"></td>
    <td width="33.33%"><img src="docs/images/mimi-game-real-ja.jpg" alt="ノベルゲームに字幕を重ねた mimi"></td>
    <td width="33.33%"><img src="docs/images/mimi-meeting-real-ja.jpg" alt="オンライン会議に字幕を重ねた mimi"></td>
  </tr>
  <tr>
    <td align="center">映画・動画</td>
    <td align="center">ゲーム・ライブ配信</td>
    <td align="center">会議・オンライン授業</td>
  </tr>
</table>

ブラウザ、動画プレイヤー、オンライン会議、オンライン授業、デスクトップアプリで使えます。

## 機能

- **リアルタイム字幕** — ブラウザ・プレイヤー・ゲーム・会議・デスクトップアプリに対応。
- **リアルタイム翻訳** — 超高速 / 低遅延 / 高品質の3モード。
- **複数のサービス設定** — サービス設定を複数保存し、キーを再入力せずに切り替えられます。
- **柔軟な字幕ウィンドウ** — 移動・リサイズ・折りたたみ・クリック透過ロック。
- **多言語** — 中国語・日本語・英語・韓国語の認識。
- **プライバシー** — マイク不使用、mimi アカウント不要、音声や字幕履歴の保存なし。
- **グローバルショートカット** — macOS は **⌘ ⇧ Space**、Windows は **Ctrl+Shift+Space** で開始/停止。

## はじめ方

1. [Releases](https://github.com/yuxino/mimi/releases/latest) からお使いのプラットフォーム版をダウンロードします。
2. 「サービス設定」を開いて API Key を保存します。既定はアリババクラウドで、OpenAI Realtime も利用できます。
3. 動画などを再生し、「開始」をクリックします。

各サービス設定の API Key は OS の安全な認証情報ストレージ（macOS キーチェーン / Windows 資格情報マネージャー）へ個別に保存され、設定画面へ読み戻されることはありません。アリババクラウドの共有 API は API Key のみで利用でき、Workspace ID は不要です。既存のアリババクラウド設定は既定のサービス設定へ自動移行されます。各サービスの利用には料金がかかる場合があります。

[アリババクラウド API キーを作成](https://help.aliyun.com/zh/model-studio/get-api-key) · [OpenAI API キーを作成](https://platform.openai.com/api-keys)

| プロバイダー | 入力音声 | 字幕の出力先 | モード |
| --- | --- | --- | --- |
| アリババクラウド Model Studio | 自動、中国語、英語、日本語、韓国語 | 原文、簡体中国語、英語、日本語 | 超高速、低遅延、高品質 |
| OpenAI Realtime | 自動 | 簡体中国語、英語、日本語 | 超高速 |

### プラットフォームについて

- **macOS**：初回のリスニング時に「画面とシステムオーディオの収録」へのアクセスが求められます（mimi はシステム音声のみを取得し、画面は記録せず、自身の音声も除外します）。
- **Windows**：WASAPI ループバックで既定の再生デバイスのミックス全体を取得します。許可は不要です。mimi は音を再生しないためエコーもありません。

## ソースからビルド

Rust 1.85+ と Node.js 20+ が必要です（macOS は Xcode Command Line Tools も必要）。

```bash
git clone https://github.com/yuxino/mimi.git
cd mimi
npm ci
./scripts/dev-app.sh     # macOS：安定したアプリ ID で開発実行
npm run tauri:dev        # Windows：分離された開発設定で実行
./scripts/check.sh       # 全チェック（fmt/clippy/テスト/フロントエンドビルド）
./scripts/package-app.sh # パッケージ化（macOS: .dmg / Windows: .msi/.nsis）
```

Windows 向けパッケージは Windows マシンでビルドしてください（Rust の C 依存は macOS から MSVC ターゲットへクロスコンパイルできません）。CI は macOS と Windows の両方で Rust テスト一式を実行します。

### macOS の開発メモ

- macOS での開発実行は必ず `./scripts/dev-app.sh` を使用してください。本物の `.app` を生成・検証して固定パスへインストールし、不安定な一時署名を拒否するため、「画面とシステムオーディオの収録」と安全な認証情報ストレージへのアクセス許可が再ビルド後も同じアプリに紐づきます。
- ランチャーは `scripts/codesign-identity.sh` を通じて安定した `mimi Local Development` ID を選択します。macOS では `tauri dev` コマンドや裸のバイナリを直接実行しないでください。一時署名はビルドごとに別アプリと判断され、許可を繰り返し求められる場合があります。
- 開発版はアプリ ID、設定ディレクトリ、認証情報の名前空間を分離し、インストール済み正式版のサービス設定や API Key を読み書きしません。

## テスト

```bash
cargo test --manifest-path src-tauri/Cargo.toml   # Rust ユニットテスト（プロトコル、字幕組み立て、設定、PCM など）
npm run test                                      # フロントエンド vitest
```

macOS の UI スモークテストは `./scripts/dev-app.sh --ui-only` で実行します。UI テストモードは実際の認証情報、プロバイダーのネットワーク、システム音声キャプチャへアクセスしません。プロバイダーとのエンドツーエンド確認には通常のコマンドとローカルの安全な認証情報ストレージにある認証情報を使用してください。

詳細は [CONTRIBUTING.md](CONTRIBUTING.md) と [SECURITY.md](SECURITY.md) をご覧ください。

[MIT](LICENSE) © 2026 yuxino
