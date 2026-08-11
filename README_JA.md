<div align="center">
  <img src="Resources/Assets/mimi-icon.png" width="96" alt="mimi">
  <h1>mimi</h1>
  <p>Macのリアルタイム翻訳字幕</p>
  <p>
    <a href="https://github.com/yuxino/mimi/releases/latest"><strong>mimiをダウンロード</strong></a>
    · <a href="README.md">简体中文</a>
    · <a href="README_EN.md">English</a>
  </p>
</div>

mimiは、日本語の「耳（みみ）」に由来する名前です。Macで再生中の中国語・日本語・英語・韓国語を原文字幕として表示したり、簡体字中国語・英語・日本語へリアルタイム翻訳したりできます。

ブラウザ、動画プレイヤー、オンライン会議、オンライン授業、デスクトップアプリで使えます。

<table>
  <tr>
    <td width="33.33%"><img src="docs/images/mimi-film-real-ja.jpg" alt="海外ドラマにリアルタイム日本語字幕を表示するmimi"></td>
    <td width="33.33%"><img src="docs/images/mimi-game-real-ja.jpg" alt="写実的なストーリーゲームにリアルタイム日本語字幕を表示するmimi"></td>
    <td width="33.33%"><img src="docs/images/mimi-live-real-ja.jpg" alt="旅行ライブ配信にリアルタイム日本語字幕を表示するmimi"></td>
  </tr>
  <tr>
    <td align="center">字幕のない海外映画を楽しむ</td>
    <td align="center">台詞の多いストーリーゲームを楽しむ</td>
    <td align="center">ライブ、ポッドキャスト、旅動画を理解する</td>
  </tr>
  <tr>
    <td width="33.33%"><img src="docs/images/mimi-romance-real-ja.jpg" alt="海外のショートドラマにリアルタイム日本語字幕を表示するmimi"></td>
    <td width="33.33%"><img src="docs/images/mimi-meeting-real-ja.jpg" alt="多言語のオンライン会議にリアルタイム日本語字幕を表示するmimi"></td>
    <td width="33.33%"><img src="docs/images/mimi-course-real-ja.jpg" alt="海外のオンライン授業にリアルタイム日本語字幕を表示するmimi"></td>
  </tr>
  <tr>
    <td align="center">海外のショートドラマや会話を追いかける</td>
    <td align="center">多言語のオンライン会議に参加する</td>
    <td align="center">海外のオンライン授業や講義を理解する</td>
  </tr>
</table>

## 主な機能

- 字幕ウィンドウの移動、サイズ変更、固定、コンパクト表示
- 現在の台詞を強調し、確定済み字幕を時刻付きで表示
- 一時停止、消去、認識言語の切り替え、短い通信エラーからの自動復旧
- **⌘ ⇧ Space** でリアルタイム字幕を開始 / 停止

## はじめる

1. [Releases](https://github.com/yuxino/mimi/releases/latest)から最新版をダウンロード
2. Alibaba Cloud Model StudioのWorkspace IDとAPI Keyを入力
3. 動画を再生するか、会議・オンライン授業に入り、**Start Listening**をクリック

> mimiは、まだAppleの公証を受けていません。初回起動時にmacOSでブロックされた場合は、「システム設定 → プライバシーとセキュリティ」から「このまま開く」を選んでください。

## プライバシーと設定

mimiはマイクを使いません。アカウント登録も不要で、音声や字幕の履歴を保存することもありません。API KeyはmacOSのキーチェーンに保管されます。

[API Keyを作成](https://help.aliyun.com/en/model-studio/get-api-key) · [Workspace IDを確認](https://help.aliyun.com/en/model-studio/obtain-the-app-id-and-workspace-id)

> Workspace IDとAPI Keyは、同じ中国（北京）リージョンのワークスペースで発行されたものを使用してください。モデルの利用には料金がかかる場合があります。

## 使い方のヒント

- 上部中央の短いハンドルをドラッグして移動。四辺と四隅から自由にサイズ変更できます
- 上部をダブルクリックして、字幕ウィンドウを折りたたみ / 展開できます
- 固定すると、字幕の上からそのまま動画や会議画面を操作できます
- 左上で言語を切り替え、右上で一時停止や字幕の消去ができます
- 字幕が二重に表示されたら、Chromeの「自動字幕起こし / リアルタイム翻訳」をオフにしてください

<details>
<summary>ソースからビルド</summary>

macOS 14以降と、Xcode 16またはSwift 6を含むXcode Command Line Toolsが必要です。

```bash
git clone https://github.com/yuxino/mimi.git
cd mimi
swift run mimi-core-tests
swift build -c release -Xswiftc -warnings-as-errors
./scripts/package-app.sh
open dist/mimi.app
```

</details>

## mimiに参加する

[Issue](https://github.com/yuxino/mimi/issues)やPull Requestを歓迎します。開発を始める前に[CONTRIBUTING.md](CONTRIBUTING.md)を読み、セキュリティ上の問題は[SECURITY.md](SECURITY.md)に従って非公開で報告してください。

[MIT](LICENSE) © 2026 yuxino
