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

ブラウザ、動画プレイヤー、オンライン会議、オンライン授業、デスクトップアプリで使えます。

## 機能

- **リアルタイム字幕** — ブラウザ・プレイヤー・ゲーム・会議・デスクトップアプリに対応。
- **リアルタイム翻訳** — 極速 / 低遅延 / 高品質の3モード。
- **柔軟な字幕ウィンドウ** — 移動・リサイズ・折りたたみ・クリック透過ロック。
- **多言語** — 中国語・日本語・英語・韓国語の認識。
- **プライバシー** — マイク不使用、アカウント不要、音声や字幕履歴の保存なし。
- **グローバルショートカット** — macOS は **⌘ ⇧ Space**、Windows は **Ctrl+Shift+Space** で開始/停止。

## はじめ方

1. [Releases](https://github.com/yuxino/mimi/releases/latest) からお使いのプラットフォーム版をダウンロードします。
2. アリババクラウド Model Studio の Workspace ID と API キーを設定します。
3. 動画などを再生し、「開始」をクリックします。

API キーは OS のキーチェーン（macOS キーチェーン / Windows 資格情報マネージャー）に保存されます。Workspace ID と API キーは同じ華北2（北京）リージョンのワークスペースのものを使用してください。モデル利用には料金がかかる場合があります。

[API キーの作成](https://help.aliyun.com/zh/model-studio/get-api-key) · [Workspace ID の確認](https://help.aliyun.com/zh/model-studio/obtain-the-app-id-and-workspace-id)

### プラットフォームについて

- **macOS**：初回のリスニング時に「画面収録」の許可が求められます（システム音声の取得のみに使用し、画面は記録しません。mimi 自身の音声も除外されます）。
- **Windows**：WASAPI ループバックで既定の再生デバイスのミックス全体を取得します。許可は不要です。mimi は音を再生しないためエコーもありません。

## ソースからビルド

Rust 1.85+ と Node.js 20+ が必要です（macOS は Xcode Command Line Tools も必要）。

```bash
git clone https://github.com/yuxino/mimi.git
cd mimi
npm install
npm run tauri dev        # 開発実行
./scripts/check.sh       # 全チェック（fmt/clippy/テスト/フロントエンドビルド）
./scripts/package-app.sh # パッケージ化（macOS: .dmg / Windows: .msi/.nsis）
```

Windows 向けパッケージは Windows マシンでビルドしてください（Rust の C 依存は macOS から MSVC ターゲットへクロスコンパイルできません）。CI は macOS と Windows の両方で Rust テスト一式を実行します。

## テスト

```bash
cd src-tauri && cargo test   # Rust ユニットテスト（プロトコル、字幕組み立て、設定、PCM など）
npm run test                 # フロントエンド vitest
```

UI スモークテストは `MIMI_UI_TEST=1`（デモ認証情報）と `MIMI_AUTO_START=1`（起動時に自動セッション開始、エラー経路の確認用）を付けて `npm run tauri dev` を実行します。

詳細は [CONTRIBUTING.md](CONTRIBUTING.md) と [SECURITY.md](SECURITY.md) をご覧ください。

[MIT](LICENSE) © 2026 yuxino
