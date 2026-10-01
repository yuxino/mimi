# PR88：Linux AppImage 标签同步 / 双语可见性

实际原生 Linux GUI，纯合成字幕，截图由云 QA 在 2026-10-01 09:47 UTC 同一进程补验后交接；本机整合者已查看两张原图像素并记录图片 SHA256。

- exact source：`b9d124c220cb90e43ef98618d13e247d50de03b4`。
- run `36838170887`，artifact `11150278936`。
- 下载 ZIP SHA256：`dcb3911640523adc3a32b8e27b45bb29dcc612e048385a8f4b37ce98c9745d00`，整合者实际下载/CRC/GitHub digest 已核对。
- AppImage SHA256：`eb94d7f61bd4c07fc6792cf92a8355ed62d875ec20435e64394998acb910d552`，已核对归档内文件。
- 云 QA 记录运行 PID `102098`，`/proc/<pid>/exe` 实际指向这个 AppImage 的 `usr/bin/mimi`；运行 ELF SHA256 `53461c066241eeadb990358217bdb88fe0f3c83af991867324414c9738a1f0be`。这是 QA 的运行身份记录，整合者未在 Mac 执行 Linux ELF。
- AppImage 与 DEB 内 ELF 分开记录，不能要求二者哈希相同。DEB 直接运行缺 `libwebkit2gtk-4.1.so.0`，没有界面；**不是 DEB 安装/启动验收通过**。
- 启动前后无活跃旧 Mimi；私有测试配置、UI-only 合成数据，无真实 key/provider/音频。

## 实际通过范围

1. 同一进程、设置页保持打开，以 Ctrl+Shift+B 切双语/原文/译文，选择框标签与设置预览同步。
2. 640×136 浮窗双语两行完整可见，18px，关闭字幕动效。
3. 新设置图没有 PC 候选引导入口，符合生产 b9 源码边界。

`identity-settings-bilingual.jpg`（1180×812）和 `identity-overlay-bilingual.jpg`（640×136）分别是新设置与实际浮窗原图。静图不能证明按键操作全时序；操作结果来自上述 QA 记录。

之前标称 b9 的三张交接图未作为 b9 after 公开；其中旧浮窗图与已公开 db44 原图字节相同。新两张图均是不同字节/新 Library 交接，不把旧 PC guide 或旧 keyboard 结果算入此轮。

## 同条件前后对照

before：独立候选 db44 原生截图 `settings-motion-off-bilingual.jpg`，双语预览已选但选择框还显示原文。after：当前 b9 AppImage 新设置原图，Ctrl+Shift+B 后标签为原文+译文。两者同字体/字幕页/合成预览，完整窗口来自不同版本；旧候选包含 PC guide，不冒充生产包。

旧图固定 evidence commit：`2b6c1a961483635cbf2219d829ca3dbf1d4ff8e0`。

未覆盖：展开菜单的外部更新与键盘焦点（组件回归已过，新的 native 尚未补）、真实服务/采音/权限/Keychain、七态/reduce/流式多句、Mac/Windows，以及 DEB 可用性。#91/#93 仍开放，待最终版本范围验收与合入后按规则收尾。
