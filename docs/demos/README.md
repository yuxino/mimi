# mimi expanded interface demonstration

This replaces the earlier three-to-four-scene, 2x demo with **10 recorded scenes** from the actual production frontend at `4bbcde72b9bcb9a2540f0b230c8ce45d842a50dc`.

**Pacing:** every source action interval is played at **10x**, followed by a **0.8-second result hold**. The final clip lasts 16.77 seconds; it is not a uniformly accelerated full video. The GIF and MP4 share the same timing. The fast-forward and sample-data labels remain visible.

**Scope:** Settings UI · built-in browser preview · no live transcription. The browser harness substitutes native API boundaries with original local examples; this is not native macOS/Windows end-to-end validation. No user credentials, personal files, live provider output or upstream comic content are included. Satori question composition is shown without submitting an AI request; no answer is fabricated.

## Scenes

1. 01 / Source and target languages / 日语输入，英语字幕
2. 02 / Change the language pair / 随时切换翻译方向
3. 03 / Translation quality controls / 按需要选择翻译模式
4. 04 / Enlarge subtitles / 字号放大，远一点也能看清
5. 05 / Three subtitle alignments / 左、中、右，选习惯的对齐方式
6. 06 / Immersive and position-lock controls / 沉浸显示，也能锁定字幕位置
7. 07 / Translation service settings / 翻译服务集中管理
8. 08 / Explore supported provider choices / 查看可添加的实时服务
9. 09 / General settings / 通用设置与界面语言
10. 10 / Switch the interface language / 中英文界面，一键切换

## Files

`preview.gif` is the inline README preview; `demo.mp4` is the complete silent H.264 video. `poster.png` is an actual recorded result frame. `provenance.json` records source, pacing, scene boundaries and media hashes.

The reproducible documentation-only recorder is `yuxino/kiri/docs/demos/capture/expanded.py`. It is not loaded by the applications. The shipped application code, versions, signing and update workflows remain unchanged.
