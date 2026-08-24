/**
 * UI copy grouped by window. Tray strings, language/mode display names, and
 * the overlay copy follow the effective UI language (Chinese, English, or
 * Japanese). The overlay activity-phase labels live in `types.ts` so they
 * stay adjacent to their color and amplitude parameters.
 */

import type { ServiceProvider, UiLanguage } from "./types";

export type { UiLanguage } from "./types";

const UI_LANGUAGE_STORAGE_KEY = "mimi-ui-language";

/** Returns the user-selected UI language override, if any. */
function getStoredUiLanguage(): UiLanguage | null {
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
  originalOnly: "只显示原文",
  subtitleAlignment: "字幕对齐",
  alignLeft: "左对齐",
  alignCenter: "居中对齐",
  alignRight: "右对齐",
  blendBackground: "融入背景",
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
  originalOnly: "Original only",
  subtitleAlignment: "Subtitle Alignment",
  alignLeft: "Align Left",
  alignCenter: "Align Center",
  alignRight: "Align Right",
  blendBackground: "Blend into Background",
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
  originalOnly: "原文のみ表示",
  subtitleAlignment: "字幕の配置",
  alignLeft: "左揃え",
  alignCenter: "中央揃え",
  alignRight: "右揃え",
  blendBackground: "背景になじませる",
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
  clearSubtitles: "清空字幕",
  openSettings: "打开 mimi 设置",
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
  clearSubtitles: "字幕をクリア",
  openSettings: "mimi の設定を開く",
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
  windowTitle: "设置",
  currentProfile: "当前配置",
  noActiveProfile: "尚未选择服务",
  appLanguage: "界面语言",
  systemLanguage: "跟随系统",
  chinese: "中文",
  english: "English",
  japanese: "日本語",
  languageHelp: "切换后立即生效。",
  start: "开始",
  stop: "停止",
  configureService: "配置服务",
  subtitleTitle: "字幕",
  sourceLanguage: "识别语言",
  translateTo: "翻译成",
  translationMode: "翻译模式",
  fontSize: "字幕字号",
  subtitleAlignment: "字幕对齐",
  alignLeft: "左对齐",
  alignCenter: "居中对齐",
  alignRight: "右对齐",
  blendBackground: "字幕融入背景",
  blendBackgroundHelp: "去掉半透明卡片和功能按钮，只保留画面上的字幕。可从翻译面板随时关闭。",
  lockPosition: "锁定字幕位置",
  lockHelp: "关闭锁定后，可拖动字幕顶部来移动位置，也可从边缘或四角调整大小。",
  serviceProfilesTitle: "翻译服务",
  serviceProfilesDescription: "选择服务，并安全保存对应的 API Key。",
  manageServiceProfiles: "管理服务配置",
  addProfile: "添加配置",
  chooseProvider: "添加翻译服务",
  chooseProviderDescription: "选择服务商后即可保存 API Key。",
  cancel: "取消",
  profileName: "配置名称",
  profileNamePlaceholder: "输入配置名称",
  saveName: "保存",
  useProfile: "使用此配置",
  activeProfile: "使用中",
  deleteProfile: "删除配置",
  deleteProfileConfirm: (name: string): string =>
    `确定删除“${name}”及其专属凭证吗？凭证不会显示或导出。`,
  profileCount: (count: number): string => `${count} 个配置`,
  defaultAlibabaProfileName: "Alibaba Cloud",
  defaultOpenAIProfileName: "OpenAI Realtime",
  providerAlibaba: "Alibaba Cloud",
  providerOpenAI: "OpenAI Realtime",
  providerAlibabaDescription: "使用 API Key，无需 Workspace ID。",
  providerOpenAIDescription: "自动识别，支持中英日目标语言与极速模式。",
  credentials: "连接凭证",
  credentialPresent: "凭证已保存",
  credentialMissing: "尚未配置凭证",
  credentialUnavailable: "凭证状态不可用",
  credentialUnavailableHelp: "无法读取系统安全存储，请稍后重试。",
  apiKey: "API Key",
  apiKeyPlaceholder: "输入新的 API Key",
  credentialNote: "输入框始终为空。凭证只会写入这台设备的系统安全存储，不会回显。",
  saveCredentials: "保存凭证",
  replaceCredentials: "更新凭证",
  deleteCredentials: "移除凭证",
  deleteCredentialsConfirm: "确定移除此配置的凭证吗？",
  credentialsSaved: "凭证已安全保存。",
  credentialsDeleted: "凭证已移除。",
  profileNameSaved: "配置名称已更新。",
  profileCreated: "服务配置已创建。",
  profileSelected: "当前服务已切换。",
  profileSelectedWithAdjustments: "当前服务已切换，并已按该服务支持范围调整字幕设置。",
  profileDeleted: "服务配置已删除。",
  profileActionFailed: "操作未完成，请重试。",
  profileMutationsLocked: "实时字幕运行期间，服务配置暂时锁定。",
  profileLimitReached: "最多可创建 20 个服务配置。",
  applicationTitle: "通用",
  recognizingChineseListening: "正在识别中文，只显示原文，不发送翻译请求。",
  recognizingChineseIdle: "只识别并显示中文原文，不发送翻译请求。",
  sourceHelpReconnecting: "切换后会自动重新连接，继续使用当前翻译模式。",
  sourceHelpIdle: "选择主要语种，整句翻译更准确。",
  switchToChineseHelp: "切换到中文识别，只显示原文",
  switchToLanguageHelp: (language: string): string =>
    `切换到 ${language} 识别，保持当前翻译模式`,
};

type SettingsCopy = {
  [Key in keyof typeof SETTINGS_ZH]: (typeof SETTINGS_ZH)[Key] extends (
    ...args: infer Args
  ) => string
    ? (...args: Args) => string
    : string;
};

const SETTINGS_EN = {
  windowTitle: "Settings",
  currentProfile: "Current configuration",
  noActiveProfile: "No service selected",
  appLanguage: "Interface Language",
  systemLanguage: "System",
  chinese: "中文",
  english: "English",
  japanese: "日本語",
  languageHelp: "Applies immediately after switching.",
  start: "Start",
  stop: "Stop",
  configureService: "Configure Service",
  subtitleTitle: "Subtitles",
  sourceLanguage: "Recognition Language",
  translateTo: "Translate To",
  translationMode: "Translation Mode",
  fontSize: "Subtitle Size",
  subtitleAlignment: "Subtitle Alignment",
  alignLeft: "Align Left",
  alignCenter: "Align Center",
  alignRight: "Align Right",
  blendBackground: "Blend Subtitles into Background",
  blendBackgroundHelp: "Removes the translucent card and controls, leaving only subtitles over the picture. Turn it off anytime from the translation panel.",
  lockPosition: "Lock Subtitle Position",
  lockHelp: "When unlocked, drag the subtitle top to move it, or resize from any edge or corner.",
  serviceProfilesTitle: "Translation Service",
  serviceProfilesDescription: "Choose a service and save its API key securely.",
  manageServiceProfiles: "Manage Configurations",
  addProfile: "Add Configuration",
  chooseProvider: "Add a translation service",
  chooseProviderDescription: "Choose a provider, then save its API key.",
  cancel: "Cancel",
  profileName: "Configuration Name",
  profileNamePlaceholder: "Enter a configuration name",
  saveName: "Save",
  useProfile: "Use This Configuration",
  activeProfile: "In Use",
  deleteProfile: "Delete Configuration",
  deleteProfileConfirm: (name: string): string =>
    `Delete “${name}” and its credential? The credential will never be shown or exported.`,
  profileCount: (count: number): string => `${count} ${count === 1 ? "configuration" : "configurations"}`,
  defaultAlibabaProfileName: "Alibaba Cloud",
  defaultOpenAIProfileName: "OpenAI Realtime",
  providerAlibaba: "Alibaba Cloud",
  providerOpenAI: "OpenAI Realtime",
  providerAlibabaDescription: "Uses an API key; no Workspace ID is required.",
  providerOpenAIDescription: "Automatic recognition with Chinese, English, or Japanese targets in Turbo mode.",
  credentials: "Connection Credentials",
  credentialPresent: "Credential saved",
  credentialMissing: "Credential not configured",
  credentialUnavailable: "Credential status unavailable",
  credentialUnavailableHelp: "Secure credential storage could not be read. Try again later.",
  apiKey: "API Key",
  apiKeyPlaceholder: "Enter a new API Key",
  credentialNote: "This field is always blank. Credentials are written only to this device's secure credential storage and never displayed.",
  saveCredentials: "Save Credentials",
  replaceCredentials: "Update Credentials",
  deleteCredentials: "Remove Credentials",
  deleteCredentialsConfirm: "Remove the credential from this configuration?",
  credentialsSaved: "Credentials saved securely.",
  credentialsDeleted: "Credentials removed.",
  profileNameSaved: "Configuration name updated.",
  profileCreated: "Service configuration created.",
  profileSelected: "Current service changed.",
  profileSelectedWithAdjustments: "Current service changed, and subtitle settings were adjusted to match its supported options.",
  profileDeleted: "Service configuration deleted.",
  profileActionFailed: "The action could not be completed. Try again.",
  profileMutationsLocked: "Service configurations are locked while live subtitles are running.",
  profileLimitReached: "You can create up to 20 service configurations.",
  applicationTitle: "General",
  recognizingChineseListening: "Recognizing Chinese; showing original text only and not sending translation requests.",
  recognizingChineseIdle: "Recognizes Chinese and shows original text only; no translation requests are sent.",
  sourceHelpReconnecting: "Switching will reconnect automatically and keep the current translation mode.",
  sourceHelpIdle: "Choose the main language for more accurate sentence translation.",
  switchToChineseHelp: "Switch to Chinese recognition and show original text only",
  switchToLanguageHelp: (language: string): string =>
    `Switch to ${language} recognition and keep the current translation mode`,
} satisfies SettingsCopy;

const SETTINGS_JA = {
  windowTitle: "設定",
  currentProfile: "現在の設定",
  noActiveProfile: "サービスが選択されていません",
  appLanguage: "表示言語",
  systemLanguage: "システムに従う",
  chinese: "中文",
  english: "English",
  japanese: "日本語",
  languageHelp: "切り替えるとすぐに反映されます。",
  start: "開始",
  stop: "停止",
  configureService: "サービスを設定",
  subtitleTitle: "字幕",
  sourceLanguage: "認識言語",
  translateTo: "翻訳先",
  translationMode: "翻訳モード",
  fontSize: "字幕サイズ",
  subtitleAlignment: "字幕の配置",
  alignLeft: "左揃え",
  alignCenter: "中央揃え",
  alignRight: "右揃え",
  blendBackground: "字幕を背景になじませる",
  blendBackgroundHelp: "半透明カードと操作ボタンを消し、映像上の字幕だけを残します。翻訳パネルからいつでも解除できます。",
  lockPosition: "字幕の位置を固定",
  lockHelp: "固定を解除すると、字幕の上部をドラッグして移動したり、端や角からサイズを変更できます。",
  serviceProfilesTitle: "翻訳サービス",
  serviceProfilesDescription: "サービスを選び、API Key を安全に保存します。",
  manageServiceProfiles: "サービス設定を管理",
  addProfile: "設定を追加",
  chooseProvider: "翻訳サービスを追加",
  chooseProviderDescription: "プロバイダーを選び、API Key を保存してください。",
  cancel: "キャンセル",
  profileName: "設定名",
  profileNamePlaceholder: "設定名を入力",
  saveName: "保存",
  useProfile: "この設定を使用",
  activeProfile: "使用中",
  deleteProfile: "設定を削除",
  deleteProfileConfirm: (name: string): string =>
    `「${name}」と専用の認証情報を削除しますか？認証情報は表示・書き出しされません。`,
  profileCount: (count: number): string => `${count} 件の設定`,
  defaultAlibabaProfileName: "Alibaba Cloud",
  defaultOpenAIProfileName: "OpenAI Realtime",
  providerAlibaba: "Alibaba Cloud",
  providerOpenAI: "OpenAI Realtime",
  providerAlibabaDescription: "API Key のみを使用し、Workspace ID は不要です。",
  providerOpenAIDescription: "自動認識、中国語・英語・日本語への翻訳、最速モードに対応します。",
  credentials: "接続認証情報",
  credentialPresent: "認証情報は保存済みです",
  credentialMissing: "認証情報が未設定です",
  credentialUnavailable: "認証情報の状態を確認できません",
  credentialUnavailableHelp: "OS の安全な認証情報ストレージを読み取れませんでした。後でもう一度お試しください。",
  apiKey: "API Key",
  apiKeyPlaceholder: "新しい API Key を入力",
  credentialNote: "入力欄は常に空です。認証情報はこの端末の安全な認証情報ストレージにのみ書き込まれ、表示されません。",
  saveCredentials: "認証情報を保存",
  replaceCredentials: "認証情報を更新",
  deleteCredentials: "認証情報を削除",
  deleteCredentialsConfirm: "この設定の認証情報を削除しますか？",
  credentialsSaved: "認証情報が安全に保存されました。",
  credentialsDeleted: "認証情報を削除しました。",
  profileNameSaved: "設定名を更新しました。",
  profileCreated: "サービス設定を作成しました。",
  profileSelected: "現在のサービスを切り替えました。",
  profileSelectedWithAdjustments: "現在のサービスを切り替え、対応範囲に合わせて字幕設定を調整しました。",
  profileDeleted: "サービス設定を削除しました。",
  profileActionFailed: "操作を完了できませんでした。もう一度お試しください。",
  profileMutationsLocked: "リアルタイム字幕の実行中は、サービス設定がロックされます。",
  profileLimitReached: "サービス設定は最大 20 件まで作成できます。",
  applicationTitle: "一般",
  recognizingChineseListening: "中国語を認識中。原文のみ表示し、翻訳リクエストは送信しません。",
  recognizingChineseIdle: "中国語のみ認識して原文を表示します。翻訳リクエストは送信されません。",
  sourceHelpReconnecting: "切り替えると自動的に再接続し、現在の翻訳モードを維持します。",
  sourceHelpIdle: "主要言語を選ぶと、文単位の翻訳がより正確になります。",
  switchToChineseHelp: "中国語認識に切り替え、原文のみ表示",
  switchToLanguageHelp: (language: string): string =>
    `「${language}」認識に切り替え、現在の翻訳モードを維持`,
} satisfies SettingsCopy;

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

export function providerDisplayName(provider: ServiceProvider): string {
  return provider === "alibabaCloud"
    ? I18N.settings.providerAlibaba
    : I18N.settings.providerOpenAI;
}
