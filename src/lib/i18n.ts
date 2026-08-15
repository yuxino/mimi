/**
 * UI copy, copied verbatim from the Swift views. Strings are grouped by
 * window. Tray strings, language/mode display names, and the overlay copy
 * follow the system language (Chinese UI on zh-* systems, Japanese on ja-*
 * systems, English otherwise). The overlay activity-phase labels live in
 * `types.ts` (the `OverlayActivityPhase` table) so they stay adjacent to
 * their color/amplitude parameters.
 */

export const UI_LANGUAGE_STORAGE_KEY = "mimi-ui-language";

export type UiLanguage = "system" | "zh" | "en" | "ja";

/** Returns the user-selected UI language override, if any. */
export function getStoredUiLanguage(): UiLanguage | null {
  try {
    const value = localStorage.getItem(UI_LANGUAGE_STORAGE_KEY);
    return value === "zh" || value === "en" || value === "ja" || value === "system"
      ? value
      : null;
  } catch {
    return null;
  }
}

/** Persists the UI language override before a reload applies it. */
export function setStoredUiLanguage(language: UiLanguage): void {
  try {
    localStorage.setItem(UI_LANGUAGE_STORAGE_KEY, language);
  } catch {
    // Storage may be unavailable; the backend preference still persists.
  }
}

/** The effective UI language, honoring a stored override over the OS/webview
 * language. */
export function effectiveUiLanguage(): "zh" | "en" | "ja" {
  const stored = getStoredUiLanguage();
  if (stored === "zh" || stored === "en" || stored === "ja") return stored;
  const system =
    typeof navigator !== "undefined" ? (navigator.language ?? "") : "";
  if (system.toLowerCase().startsWith("zh")) return "zh";
  if (system.toLowerCase().startsWith("ja")) return "ja";
  return "en";
}

/** True when the effective UI language is Chinese (zh-*). */
export function isChineseSystem(): boolean {
  return effectiveUiLanguage() === "zh";
}

const TRAY_ZH = {
  appName: "mimi",
  sourceLanguage: "识别语言",
  chineseSource: "中文原文",
  originalOnly: "只显示中文原文",
  lockPosition: "锁定字幕位置",
  showSubtitleWindow: "显示字幕窗口",
  clearSubtitles: "清空字幕",
  settings: "设置…",
  quit: "退出 mimi",
  setupRequired: "需要先完成设置",
  ready: "就绪",
  connecting: "正在连接…",
  listening: "正在聆听并翻译",
  stopping: "正在停止…",
  paused: "已暂停",
};

const TRAY_EN = {
  appName: "mimi",
  sourceLanguage: "Recognition Language",
  chineseSource: "Chinese (Original)",
  originalOnly: "Original Chinese only",
  lockPosition: "Lock Subtitle Position",
  showSubtitleWindow: "Show Subtitle Window",
  clearSubtitles: "Clear Subtitles",
  settings: "Settings…",
  quit: "Quit mimi",
  setupRequired: "Setup required",
  ready: "Ready",
  connecting: "Connecting…",
  listening: "Listening and translating",
  stopping: "Stopping…",
  paused: "Paused",
};

const TRAY_JA = {
  appName: "mimi",
  sourceLanguage: "認識言語",
  chineseSource: "中国語（原文）",
  originalOnly: "中国語原文のみ表示",
  lockPosition: "字幕の位置を固定",
  showSubtitleWindow: "字幕ウィンドウを表示",
  clearSubtitles: "字幕をクリア",
  settings: "設定…",
  quit: "mimi を終了",
  setupRequired: "先に設定が必要です",
  ready: "準備完了",
  connecting: "接続中…",
  listening: "聞き取り・翻訳中",
  stopping: "停止中…",
  paused: "一時停止中",
};

const OVERLAY_ZH = {
  expandSubtitle: "展开字幕",
  collapseSubtitle: "收起字幕",
  collapsedAccessibilityPrefix: "字幕已收起，",
  dragTooltip: "拖动字幕；双击收起或展开",
  paused: "已暂停",
  translating: "翻译中",
  dotSeparator: "·",
  clearSubtitles: "Clear subtitles",
  openSettings: "Open mimi Settings",
  listeningEmpty: "正在聆听，译文会保留在这里",
  connecting: "正在连接",
  translatingEmpty: "正在翻译",
  stopping: "正在结束",
  idle: "mimi",
  original: "原文",
  separator: "→",
  sourceLanguage: "识别语言",
  translationMode: "翻译模式",
  chineseSource: "中文原文",
  resume: "继续翻译",
  pause: "暂停翻译",
  pickerHelpTranslating: "切换识别语言和翻译模式",
  pickerHelpOriginal: "切换识别语言",
  originalOnly: "只显示原文",
  autoDetecting: "自动识别中",
  autoDetectedPrefix: "自动识别（",
  autoDetectedSuffix: "）",
  phaseConnecting: "正在连接",
  phaseListening: "正在聆听",
  phaseRecognizing: "正在识别",
  phaseTranslating: "正在翻译",
  phasePaused: "已暂停",
  accessibilityCurrentLanguagePrefix: "，当前语言：",
  accessibilityOpenToSwitch: "。打开以切换识别语言。",
  translationSuffix: "翻译",
};

const OVERLAY_EN = {
  expandSubtitle: "Expand subtitles",
  collapseSubtitle: "Collapse subtitles",
  collapsedAccessibilityPrefix: "Subtitles collapsed, ",
  dragTooltip: "Drag to move; double-click to collapse or expand",
  paused: "Paused",
  translating: "Translating",
  dotSeparator: "·",
  clearSubtitles: "Clear subtitles",
  openSettings: "Open mimi Settings",
  listeningEmpty: "Listening — translations will stay here",
  connecting: "Connecting",
  translatingEmpty: "Translating",
  stopping: "Stopping",
  idle: "mimi",
  original: "Original",
  separator: "→",
  sourceLanguage: "Recognition Language",
  translationMode: "Translation Mode",
  chineseSource: "Chinese (Original)",
  resume: "Resume translating",
  pause: "Pause translating",
  pickerHelpTranslating: "Switch recognition language and translation mode",
  pickerHelpOriginal: "Switch recognition language",
  originalOnly: "Original only",
  autoDetecting: "Auto detecting",
  autoDetectedPrefix: "Auto detect (",
  autoDetectedSuffix: ")",
  phaseConnecting: "Connecting",
  phaseListening: "Listening",
  phaseRecognizing: "Recognizing",
  phaseTranslating: "Translating",
  phasePaused: "Paused",
  accessibilityCurrentLanguagePrefix: ", current: ",
  accessibilityOpenToSwitch: ". Open to switch recognition language.",
  translationSuffix: " translation",
};

const OVERLAY_JA = {
  expandSubtitle: "字幕を展開",
  collapseSubtitle: "字幕を折りたたむ",
  collapsedAccessibilityPrefix: "字幕は折りたたまれています、",
  dragTooltip: "ドラッグで移動。ダブルクリックで折りたたみ・展開",
  paused: "一時停止中",
  translating: "翻訳中",
  dotSeparator: "·",
  clearSubtitles: "Clear subtitles",
  openSettings: "Open mimi Settings",
  listeningEmpty: "聞き取り中 — 翻訳はここに表示されます",
  connecting: "接続中",
  translatingEmpty: "翻訳中",
  stopping: "終了中",
  idle: "mimi",
  original: "原文",
  separator: "→",
  sourceLanguage: "認識言語",
  translationMode: "翻訳モード",
  chineseSource: "中国語（原文）",
  resume: "翻訳を再開",
  pause: "翻訳を一時停止",
  pickerHelpTranslating: "認識言語と翻訳モードを切り替え",
  pickerHelpOriginal: "認識言語を切り替え",
  originalOnly: "原文のみ表示",
  autoDetecting: "自動認識中",
  autoDetectedPrefix: "自動認識（",
  autoDetectedSuffix: "）",
  phaseConnecting: "接続中",
  phaseListening: "聞き取り中",
  phaseRecognizing: "認識中",
  phaseTranslating: "翻訳中",
  phasePaused: "一時停止中",
  accessibilityCurrentLanguagePrefix: "、現在の言語：",
  accessibilityOpenToSwitch: "。開いて認識言語を切り替え。",
  translationSuffix: "翻訳",
};

const SETTINGS_ZH = {
  sessionTitle: "实时字幕",
  appLanguage: "界面语言",
  systemLanguage: "跟随系统",
  chinese: "中文",
  english: "English",
  japanese: "日本語",
  languageHelp: "切换后立即生效。",
  start: "开始",
  stop: "停止",
  subtitleTitle: "字幕",
  sourceLanguage: "识别语言",
  translateTo: "翻译成",
  translationMode: "翻译模式",
  fontSize: "字幕字号",
  lockPosition: "锁定字幕位置",
  lockHelp: "关闭锁定后，可拖动字幕顶部来移动位置，也可从边缘或四角调整大小。",
  serviceSettings: "服务设置",
  configured: "已配置",
  apiKey: "DashScope API Key",
  apiKeyPlaceholder: "输入 API Key",
  credentialNote: "凭证仅保存在这台设备上，API Key 会存入钥匙串。",
  saveCredentials: "保存凭证",
  credentialsSaved: "凭证已安全保存。",
  sessionReady: "准备就绪",
  sessionListening: "正在识别并翻译",
  sessionStopping: "正在停止…",
  sessionError: "翻译暂时不可用，请重试",
  originalOnlyBadge: "仅显示原文",
  recognizingChineseListening: "正在识别中文，只显示原文，不发送翻译请求。",
  recognizingChineseIdle: "只识别并显示中文原文，不发送翻译请求。",
  sourceHelpReconnecting: "切换后会自动重新连接，继续使用当前翻译模式。",
  sourceHelpIdle: "选择主要语种，整句翻译更准确。",
  switchToChineseHelp: "切换到中文识别，只显示原文",
  switchToLanguageHelp: (language: string): string =>
    `切换到 ${language} 识别，保持当前翻译模式`,
};

const SETTINGS_EN = {
  sessionTitle: "Live Subtitles",
  appLanguage: "Interface Language",
  systemLanguage: "System",
  chinese: "中文",
  english: "English",
  japanese: "日本語",
  languageHelp: "Applies immediately after switching.",
  start: "Start",
  stop: "Stop",
  subtitleTitle: "Subtitles",
  sourceLanguage: "Recognition Language",
  translateTo: "Translate To",
  translationMode: "Translation Mode",
  fontSize: "Subtitle Size",
  lockPosition: "Lock Subtitle Position",
  lockHelp: "When unlocked, drag the subtitle top to move it, or resize from any edge or corner.",
  serviceSettings: "Service Settings",
  configured: "Configured",
  apiKey: "DashScope API Key",
  apiKeyPlaceholder: "Enter API Key",
  credentialNote: "Credentials are stored only on this device; the API Key is saved to your keychain.",
  saveCredentials: "Save Credentials",
  credentialsSaved: "Credentials saved securely.",
  sessionReady: "Ready",
  sessionListening: "Listening and translating",
  sessionStopping: "Stopping…",
  sessionError: "Translation is temporarily unavailable. Try again.",
  originalOnlyBadge: "Original only",
  recognizingChineseListening: "Recognizing Chinese; showing original text only and not sending translation requests.",
  recognizingChineseIdle: "Recognizes Chinese and shows original text only; no translation requests are sent.",
  sourceHelpReconnecting: "Switching will reconnect automatically and keep the current translation mode.",
  sourceHelpIdle: "Choose the main language for more accurate sentence translation.",
  switchToChineseHelp: "Switch to Chinese recognition and show original text only",
  switchToLanguageHelp: (language: string): string =>
    `Switch to ${language} recognition and keep the current translation mode`,
};

const SETTINGS_JA = {
  sessionTitle: "リアルタイム字幕",
  appLanguage: "表示言語",
  systemLanguage: "システムに従う",
  chinese: "中文",
  english: "English",
  japanese: "日本語",
  languageHelp: "切り替えるとすぐに反映されます。",
  start: "開始",
  stop: "停止",
  subtitleTitle: "字幕",
  sourceLanguage: "認識言語",
  translateTo: "翻訳先",
  translationMode: "翻訳モード",
  fontSize: "字幕サイズ",
  lockPosition: "字幕の位置を固定",
  lockHelp: "固定を解除すると、字幕の上部をドラッグして移動したり、端や角からサイズを変更できます。",
  serviceSettings: "サービス設定",
  configured: "設定済み",
  apiKey: "DashScope API Key",
  apiKeyPlaceholder: "API Key を入力",
  credentialNote: "認証情報はこの端末にのみ保存されます。API Key はキーチェーンに保存されます。",
  saveCredentials: "認証情報を保存",
  credentialsSaved: "認証情報が安全に保存されました。",
  sessionReady: "準備完了",
  sessionListening: "認識・翻訳中",
  sessionStopping: "停止中…",
  sessionError: "翻訳は一時的に利用できません。もう一度お試しください。",
  originalOnlyBadge: "原文のみ",
  recognizingChineseListening: "中国語を認識中。原文のみ表示し、翻訳リクエストは送信しません。",
  recognizingChineseIdle: "中国語のみ認識して原文を表示します。翻訳リクエストは送信されません。",
  sourceHelpReconnecting: "切り替えると自動的に再接続し、現在の翻訳モードを維持します。",
  sourceHelpIdle: "主要言語を選ぶと、文単位の翻訳がより正確になります。",
  switchToChineseHelp: "中国語認識に切り替え、原文のみ表示",
  switchToLanguageHelp: (language: string): string =>
    `「${language}」認識に切り替え、現在の翻訳モードを維持`,
};

const MODES_ZH = {
  turboHelp: "极速：边识别边用快模型翻译，速度优先，一句话说完即定稿。",
  highQualityHelp: "高质量：整句翻译完成后再显示，最准确，稍有延迟。",
  lowLatencyHelp: "低延迟：快速预览 + 高质量定稿，速度和准确度兼顾。",
};

const MODES_EN = {
  turboHelp: "Turbo: fast model translates while recognizing, prioritizing speed; finalizes as soon as a sentence finishes.",
  highQualityHelp: "High quality: waits for the full sentence before showing the translation, most accurate with a slight delay.",
  lowLatencyHelp: "Low latency: quick preview + high-quality final, balancing speed and accuracy.",
};

const MODES_JA = {
  turboHelp: "最速：認識しながら高速モデルで翻訳し、速度を優先。一文が終わるとすぐに確定します。",
  highQualityHelp: "高品質：文全体の翻訳が完了してから表示。最も正確で、少し遅延があります。",
  lowLatencyHelp: "低遅延：高速プレビュー＋高品質な確定版。速度と精度のバランス。",
};

export const I18N = {
  overlay: effectiveUiLanguage() === "ja" ? OVERLAY_JA : isChineseSystem() ? OVERLAY_ZH : OVERLAY_EN,

  tray: effectiveUiLanguage() === "ja" ? TRAY_JA : isChineseSystem() ? TRAY_ZH : TRAY_EN,

  settings: effectiveUiLanguage() === "ja" ? SETTINGS_JA : isChineseSystem() ? SETTINGS_ZH : SETTINGS_EN,

  modes: effectiveUiLanguage() === "ja" ? MODES_JA : isChineseSystem() ? MODES_ZH : MODES_EN,
} as const;

export function sessionConnectingText(modeName: string): string {
  if (effectiveUiLanguage() === "ja") return `「${modeName}」翻訳に接続中…`;
  return isChineseSystem()
    ? `正在连接${modeName}翻译…`
    : `Connecting to ${modeName} translation…`;
}

export function credentialLoadErrorMessage(error: string): string {
  if (effectiveUiLanguage() === "ja") return `保存済みの API Key を読み込めません：${error}`;
  return isChineseSystem()
    ? `无法读取已保存的 API Key：${error}`
    : `Couldn't read the saved API Key: ${error}`;
}
