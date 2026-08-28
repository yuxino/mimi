<div align="center">
  <img src="src-tauri/icons/128x128@2x.png" width="96" alt="mimi">
  <h1>mimi</h1>
  <p>システム音声のリアルタイム字幕・翻訳。Apple シリコンの macOS 13 以降に対応し、Windows はプレビューです。</p>
  <p>
    <a href="https://github.com/yuxino/mimi/releases/latest"><strong>最新版をダウンロード</strong></a>
    · <a href="README.md">简体中文</a>
    · <a href="README_EN.md">English</a>
  </p>
</div>

mimi は、日本語の「耳（みみ）」に由来する名前です。

デバイスで再生中のシステム音声をリアルタイム字幕にし、プロバイダーに応じて簡体字中国語・英語・日本語へ翻訳します。入力は中国語・日本語・英語・韓国語に対応します。

## 機能

- **リアルタイム字幕** — デバイスで再生中のシステム出力音声を取得します。
- **リアルタイム翻訳** — 出力先と超高速 / 低遅延 / 高品質モードはプロバイダーによって異なります。
- **複数のサービス設定** — 認証情報を何度も入力せずに、複数の設定を保存・切り替えできます。
- **柔軟な字幕ウィンドウ** — 移動・リサイズ・折りたたみ・一時停止・クリック透過ロック・没入モード。
- **多言語** — 中国語・日本語・英語・韓国語に対応し、選択肢はプロバイダーによって異なります。
- **プライバシー** — マイクと mimi アカウントは不要で、音声や字幕を保存せず、音声は選択中のサービスだけへ送信します。
- **グローバルショートカット** — macOS は **⌘ ⇧ Space**、Windows は **Ctrl+Shift+Space** で開始/停止。**⌘ ⇧ M** または **Ctrl+Shift+M** で没入モードを切り替え。

## はじめ方

1. [最新リリース](https://github.com/yuxino/mimi/releases/latest) から macOS Apple シリコン用 DMG または Windows x64 用 EXE / MSI をダウンロードするか、ソースからビルドします。
2. 「翻訳サービス」を開き、プロバイダーを選んで認証情報を保存します。
3. 音声を再生し、メニューバー / システムトレイの mimi アイコンで「開始」を選びます。macOS の初回使用時は、続いて「画面とシステムオーディオの収録」を許可します。

各サービス設定の認証情報は OS の安全なストレージ（macOS キーチェーン / Windows 資格情報マネージャー）へ個別保存されます。設定画面は保存済みかどうかだけを表示し、認証情報を読み戻しません。必要な項目はサービスごとに異なり、利用には料金がかかる場合があります。

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

### プラットフォームについて

- **Apple シリコン搭載の macOS 13 以降**：GitHub Releases では、アドホック署名済み・未公証の DMG を提供します。初回起動がブロックされた場合は「システム設定 → プライバシーとセキュリティ」で「このまま開く」を選びます。更新後に「画面とシステムオーディオの収録」またはキーチェーンへのアクセスを再度許可する場合があります。mimi はシステム音声だけを取得し、画面を収録せず、自身の音声も除外します。
- **Windows プレビュー**：GitHub Releases では、Authenticode 未署名の x64 MSI と NSIS インストーラーを提供します。実機でのインストール、信頼警告、認証情報ストレージ、システム音声取得、字幕ウィンドウ、字幕フロー全体はまだ検証していません。

## ソースからビルド

Rust 1.88+ と Node.js 20.19.x、22.13+、または 24+ が必要です。macOS では Xcode Command Line Tools と `mimi Local Development` ID、または明示的な `MIMI_CODESIGN_IDENTITY` も必要です。

```bash
git clone https://github.com/yuxino/mimi.git
cd mimi
npm ci
./scripts/dev-app.sh     # macOS：安定したアプリ ID で開発実行
npm run tauri:dev        # Windows：分離された開発設定で実行
./scripts/check.sh       # 全チェック（fmt/clippy/テスト/フロントエンドビルド）
./scripts/package-app.sh # パッケージ化（macOS: DMG / Windows: MSI / NSIS EXE）
```

Windows 向けパッケージは Windows でビルドしてください。CI は macOS と Windows の両方で Rust テスト一式を実行します。

### macOS の開発メモ

- macOS の開発実行は必ず `./scripts/dev-app.sh` を使用してください。固定パスの安定署名済み `.app` を検証しますが、ローカル証明書には Apple Team ID がないため、更新後にキーチェーンアクセスを一度確認する場合があります。
- ランチャーは `scripts/codesign-identity.sh` で `mimi Local Development` ID を選択します。`tauri dev` や裸のバイナリは、再ビルド後に別アプリと判断されて権限を再要求するため使用しません。
- 開発版はアプリ ID、設定ディレクトリ、認証情報の名前空間を分離し、インストール済み正式版のサービス設定や API Key を読み書きしません。

## テスト

```bash
cargo test --manifest-path src-tauri/Cargo.toml   # Rust ユニットテスト（プロトコル、字幕組み立て、設定、PCM など）
npm run test                                      # フロントエンド vitest
```

macOS の UI スモークテストは `./scripts/dev-app.sh --ui-only` で実行します。このモードは実際の認証情報、プロバイダーのネットワーク、システム音声へアクセスしません。エンドツーエンド確認には通常の起動方法とローカルの安全な認証情報を使用してください。

詳細は [CONTRIBUTING.md](CONTRIBUTING.md) と [SECURITY.md](SECURITY.md) をご覧ください。

[MIT](LICENSE) © 2026 yuxino
