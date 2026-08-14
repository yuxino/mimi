/**
 * UI copy, copied verbatim from the Swift views. Strings are grouped by
 * window. The tray strings and the language/mode display names follow the
 * system language (Chinese UI on zh-* systems, English otherwise); the rest
 * of the copy is currently the original mixed set. The overlay
 * activity-phase labels live in `types.ts` (the `OverlayActivityPhase`
 * table) so they stay adjacent to their color/amplitude parameters.
 */

/** True when the OS/webview language is Chinese (zh-*). Strings are chosen
 * once at startup; the app does not switch language at runtime. */
export function isChineseSystem(): boolean {
  return (
    typeof navigator !== "undefined" &&
    (navigator.language ?? "").toLowerCase().startsWith("zh")
  );
}

const TRAY_ZH = {
  appName: "mimi",
  liveSubtitles: "实时字幕",
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
  liveSubtitles: "Live Subtitles",
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

export const I18N = {
  overlay: {
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
  },

  tray: isChineseSystem() ? TRAY_ZH : TRAY_EN,

  settings: {
    sessionTitle: "实时字幕",
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
    workspaceID: "工作空间 ID",
    workspaceIDPlaceholder: "输入 Workspace ID",
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
  },

  modes: {
    turboHelp: "极速：边识别边用快模型翻译，速度优先，一句话说完即定稿。",
    highQualityHelp: "高质量：整句翻译完成后再显示，最准确，稍有延迟。",
    lowLatencyHelp: "低延迟：快速预览 + 高质量定稿，速度和准确度兼顾。",
  },
} as const;

export function sessionConnectingText(modeName: string): string {
  return `正在连接${modeName}翻译…`;
}

export function credentialLoadErrorMessage(error: string): string {
  return `无法读取已保存的 API Key：${error}`;
}
