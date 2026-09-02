<div align="center">
  <img src="src-tauri/icons/128x128@2x.png" width="96" alt="mimi">
  <h1>mimi</h1>
  <p>システム音声のリアルタイム字幕・翻訳。Apple シリコンの macOS 13 以降と Windows x64 に対応します。</p>
  <p>
    <a href="https://github.com/yuxino/mimi/releases/latest"><strong>最新版をダウンロード</strong></a>
    · <a href="README.md">简体中文</a>
    · <a href="README_EN.md">English</a>
  </p>
</div>

mimi は、日本語の「耳（みみ）」に由来する名前です。デバイスで再生中のシステム音声をリアルタイム字幕にし、プロバイダーに応じて簡体字中国語・英語・日本語へ翻訳します。

## 機能

- **リアルタイム字幕・翻訳** — システム出力音声を取得します。入力言語・出力先・品質モードはプロバイダーによって異なります。
- **サービス設定** — 認証情報を何度も入力せずに、複数の設定を保存・切り替えできます。
- **柔軟な字幕ウィンドウ** — 移動・リサイズ・折りたたみ・一時停止・クリック透過・没入モードに対応します。
- **署名付きアプリ内更新** — 設定画面から手動で確認・ダウンロード・インストールできます。ダウンロード後は署名検証が必須です。
- **プライバシー** — mimi アカウントは不要です。マイクや画面を取得せず、音声や字幕も保存しません。音声は選択中のサービスだけへ送信します。
- **ショートカット** — macOS は **⌘ ⇧ Space** / **⌘ ⇧ M**、Windows は **Ctrl+Shift+Space** / **Ctrl+Shift+M** で、リスニングと没入モードを操作します。

## はじめ方

1. [最新リリース](https://github.com/yuxino/mimi/releases/latest) から macOS Apple シリコン用 DMG または Windows x64 用 EXE / MSI をダウンロードするか、ソースからビルドします。
2. 「翻訳サービス」を開き、プロバイダーを選んで認証情報を保存します。
3. 音声を再生し、メニューバー / システムトレイの mimi アイコンで「開始」を選びます。macOS の初回使用時は、続いて「画面とシステムオーディオの収録」を許可します。

**v1.3.7 はアプリ内アップデートの導入版です。** v1.3.6 以前から更新する場合は、GitHub Releases から一度だけ手動でダウンロードしてインストールする必要があります。それ以降は「設定 → ソフトウェアアップデート」から更新できます。Windows では更新のインストール時に Mimi が終了するため、完了後に手動で開いてください。

認証情報はサービス設定ごとに macOS キーチェーンまたは Windows 資格情報マネージャーへ保存され、設定画面には保存状態だけが表示されます。サービスの利用には料金がかかる場合があります。

[Alibaba Cloud](https://help.aliyun.com/zh/model-studio/get-api-key) · [OpenAI](https://platform.openai.com/api-keys) · [Google Gemini](https://aistudio.google.com/app/apikey) · [Azure OpenAI](https://learn.microsoft.com/ja-jp/azure/foundry/openai/concepts/gpt-realtime-translate) · [Volcano Engine](https://docs.volcengine.com/docs/6561/1631605) · [Tencent Cloud](https://cloud.tencent.com/document/api/1093/127565) · [Baidu Translate](https://cloud.baidu.com/doc/MT/s/Sl9p2h5k9) · [xAI](https://docs.x.ai/developers/model-capabilities/audio/speech-to-speech)

| プロバイダー | 入力音声 | 字幕の出力先 | モード |
| --- | --- | --- | --- |
| アリババクラウド Model Studio | 自動、中国語、英語、日本語、韓国語 | 原文、簡体中国語、英語、日本語 | 超高速、低遅延、高品質 |
| OpenAI Realtime | 自動 | 簡体中国語、英語、日本語 | 超高速 |
| Google Gemini Live Translate（プレビュー） | 自動 | 簡体中国語、英語、日本語 | 超高速 |
| Azure OpenAI Realtime Translate | 自動 | 簡体中国語、英語、日本語 | 超高速 |
| Volcano Engine 同時通訳 2.0 | 中国語、英語、日本語 | 簡体中国語、英語、日本語 | 超高速 |
| Tencent Cloud リアルタイム音声翻訳 | 中国語、英語、日本語、韓国語 | 簡体中国語、英語、日本語 | 超高速 |
| Baidu リアルタイム音声翻訳 | 中国語、英語、日本語、韓国語 | 簡体中国語、英語、日本語 | 超高速 |
| xAI Grok Voice | 自動 | 簡体中国語、英語、日本語 | 超高速（ターン制） |

Gemini、Azure OpenAI、Volcano Engine、Tencent、Baidu、xAI は、プロトコル、モック WebSocket、UI ロジックまで検証済みです。有料アカウントでのエンドツーエンド品質と遅延は、全サービスではまだ検証していません。

### 対応プラットフォーム

- **Apple シリコンの macOS 13 以降**：Apple 未公証・アドホック署名の DMG を提供します。初回起動がブロックされた場合は「システム設定 → プライバシーとセキュリティ」で「このまま開く」を選びます。更新後に録音またはキーチェーンの権限を再度求められる場合があります。
- **Windows x64**：システム音声取得、Windows 資格情報マネージャー、システムトレイ、字幕ウィンドウ、グローバルショートカットを実装済みです。公開 x64 パッケージの実機エンドツーエンド検証を継続しながら、未署名のプレビュー版 MSI / NSIS EXE を提供しています。SmartScreen が警告する場合があります。

## ソースからビルド

Rust 1.88+ と Node.js 20.19.x、22.13+、または 24+ が必要です。macOS では Xcode Command Line Tools と `mimi Local Development` ID、または明示的な `MIMI_CODESIGN_IDENTITY` も必要です。

```bash
git clone https://github.com/yuxino/mimi.git
cd mimi
npm ci
npm run tauri:dev        # Windows：分離された開発設定で実行
./scripts/dev-app.sh     # macOS：安定したアプリ ID で開発実行
./scripts/check.sh       # 全チェック（fmt/clippy/テスト/フロントエンドビルド）
```

Windows インストーラーは Windows 上で `npm run tauri -- build -- --locked` を実行してビルドします。macOS では `./scripts/package-app.sh` を使用できます。CI は macOS、Windows x64、Windows ARM64 のビルドをテストまたはスモーク起動します。

詳細は [CONTRIBUTING.md](CONTRIBUTING.md) と [SECURITY.md](SECURITY.md) をご覧ください。

[MIT](LICENSE) © 2026 yuxino
