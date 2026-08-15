---
name: i18n
description: 处理 mimi 的国际化/多语言界面。当新增 UI 文案、新增界面语言、修改语言切换逻辑、排查某个窗口语言不跟随问题时使用。
---

# mimi 国际化(界面语言)

## 支持的界面语言

`src/lib/i18n.ts` 中 `UiLanguage = "system" | "zh" | "en" | "ja"`。界面语言三选一:中文(zh)、英文(en)、日文(ja);`system` 跟随系统语言(zh-* → zh、ja-* → ja、其他 → en)。

## 新增/修改 UI 文案

文案按窗口分组存放在 `i18n.ts`:

- `TRAY_ZH / TRAY_EN / TRAY_JA` — 托盘面板
- `OVERLAY_ZH / OVERLAY_EN / OVERLAY_JA` — 字幕 overlay
- `SETTINGS_ZH / SETTINGS_EN / SETTINGS_JA` — 设置窗口
- `MODES_ZH / MODES_EN / MODES_JA` — 翻译模式帮助文案
- `types.ts` 里 `SOURCE_LANGUAGE_DISPLAY_NAMES` / `TARGET_LANGUAGE_DISPLAY_NAMES` / `TRANSLATION_MODE_DISPLAY_NAMES` — 语言/模式显示名(也按 zh/en/ja 三态)

**规则**:
1. **三个语言组必须同步加/删同一个键**——只加 zh 不加 en/ja 会编译失败或运行时 undefined。
2. 用 `I18N.tray.xxx` / `I18N.overlay.xxx` / `I18N.settings.xxx` / `I18N.modes.xxx` 引用(模块级常量,窗口加载时按当前语言计算一次)。
3. 带参数的文案(如 `switchToLanguageHelp(language)`)用函数。
4. 语言名(中文/English/日本語)在设置页下拉里用原生写法,不随界面语言翻译。
5. `OVERLAY_ACTIVITY_PHASES`(阶段标签)在 `types.ts`,与颜色/振幅参数放一起,同样三态。

## 语言判定

- `effectiveUiLanguage(): "zh" | "en" | "ja"` — 三态解析(存储覆盖优先,其次系统语言)。
- `isChineseSystem(): boolean` — 兼容布尔,仅判断是否中文;判断日语要用 `effectiveUiLanguage() === "ja"`。
- 显示名表(`types.ts` 里三个 DISPLAY_NAMES)是三态嵌套三元:先判 ja,再判 zh,默认 en。

## 语言切换与跨窗口同步(关键)

`I18N` 是**模块级常量**,只在窗口加载时计算。切换语言的路径:

1. 设置窗口 / 托盘面板里的语言下拉 → `setStoredUiLanguage(language)`(写 localStorage)+ `saveSettings({ uiLanguage })`(写后端)+ reload 当前窗口。
2. 后端向**所有窗口**广播 `settings-changed` 事件。
3. 每个窗口的 store(`src/lib/store.ts`)在 `listenSettingsChanged` 回调里调用 `syncUiLanguageFromSettings`:**比较后端目标语言与本窗口 `renderedUiLanguage`(模块加载时捕获的渲染语言),不同则 reload 本窗口**,让模块级 I18N 重新计算。

**为什么不能比较 localStorage 与后端**:所有 Tauri 窗口共享同一个 localStorage origin,发起切换的窗口已经写入了新值,其他窗口比较永远一致,永远不会 reload。必须用 `renderedUiLanguage`(渲染时快照)对比。

**新增界面语言时需要改的地方**(以加 ja 为例):
1. `i18n.ts`:`UiLanguage` 类型、`getStoredUiLanguage` 校验、`effectiveUiLanguage` 系统检测、四个 JA 文案组、`I18N` 选择表达式、`sessionConnectingText` / `credentialLoadErrorMessage`。
2. `types.ts`:`UiLanguage` 类型、三个 DISPLAY_NAMES 的三态分支。
3. 设置窗口 + 托盘面板的语言下拉加对应 option。
4. 验证:`npm run build`(tsc 保证键同步)+ lint + test;在 `MIMI_UI_TEST=1` 下切语言看所有窗口是否跟随。

## 排查"切换语言后某窗口没跟上"

1. 确认该窗口能收到 `settings-changed`(后端 emit 是否覆盖所有窗口)。
2. 确认该窗口 store 里的 `syncUiLanguageFromSettings` 逻辑存在且用 `renderedUiLanguage` 对比(不是 localStorage 对比)。
3. 确认切换后该窗口确实 reload 了(模块级 I18N 才会重算)。

## 注意

- 语言/模式显示名表在 `types.ts` 而不在 `i18n.ts`,是历史结构,改显示名时两个文件都要看。
- 文案改动不要改动 wire protocol(JSON 键、模型名、翻译 prompt),那些与上游服务镜像,不属于界面文案。
