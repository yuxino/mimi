/**
 * UI copy grouped by window. Tray strings, language/mode display names, and
 * the overlay copy follow the effective UI language (Chinese, English, or
 * Japanese). The overlay activity-phase labels live in `types.ts` so they
 * stay adjacent to their color and amplitude parameters.
 */

import type { ServiceProvider, UiLanguage } from "./types";

export type { UiLanguage } from "./types";

export const UI_LANGUAGE_STORAGE_KEY = "mimi-ui-language";

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
  windowTitle: "mimi 控制台",
  windowSubtitle: "管理字幕体验、服务档案与应用偏好。",
  sessionEyebrow: "当前状态",
  sessionTitle: "实时字幕",
  currentProfile: "当前服务",
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
  subtitleDescription: "调整识别、翻译和浮窗显示。",
  sourceLanguage: "识别语言",
  translateTo: "翻译成",
  translationMode: "翻译模式",
  fontSize: "字幕字号",
  lockPosition: "锁定字幕位置",
  lockHelp: "关闭锁定后，可拖动字幕顶部来移动位置，也可从边缘或四角调整大小。",
  serviceProfilesTitle: "服务档案",
  serviceProfilesDescription: "为不同的实时字幕服务分别管理连接凭证。",
  addProfile: "添加档案",
  chooseProvider: "选择服务",
  chooseProviderDescription: "创建后仍可修改档案名称，服务类型不可更改。",
  cancel: "取消",
  profileName: "档案名称",
  profileNamePlaceholder: "输入档案名称",
  saveName: "保存名称",
  useProfile: "设为当前服务",
  activeProfile: "当前",
  deleteProfile: "删除档案",
  deleteProfileConfirm: (name: string): string =>
    `确定删除“${name}”及其当前专属凭证吗？凭证不会显示或导出。`,
  profileCount: (count: number): string => `${count} 个档案`,
  defaultAlibabaProfileName: "Alibaba Cloud",
  defaultOpenAIProfileName: "OpenAI Realtime",
  providerAlibaba: "Alibaba Cloud",
  providerOpenAI: "OpenAI Realtime",
  providerAlibabaDescription: "覆盖现有全部识别语言、翻译目标与模式。",
  providerOpenAIDescription: "自动识别，支持中英日目标语言与极速模式。",
  profileProvider: "服务类型",
  credentials: "连接凭证",
  credentialPresent: "凭证已保存",
  credentialMissing: "尚未配置凭证",
  credentialUnavailable: "凭证状态不可用",
  credentialUnavailableHelp: "无法读取系统钥匙串，请稍后重试。",
  apiKey: "API Key",
  apiKeyPlaceholder: "输入新的 API Key",
  credentialNote: "输入框始终为空。凭证只会写入这台设备的系统钥匙串，不会回显。",
  saveCredentials: "保存凭证",
  replaceCredentials: "更新凭证",
  deleteCredentials: "移除凭证",
  deleteCredentialsConfirm: "确定移除此档案的凭证吗？",
  credentialsSaved: "凭证已安全保存。",
  credentialsDeleted: "凭证已移除。",
  profileNameSaved: "档案名称已更新。",
  profileCreated: "服务档案已创建。",
  profileSelected: "当前服务已切换。",
  profileSelectedWithAdjustments: "当前服务已切换，并已按该服务支持范围调整字幕设置。",
  profileDeleted: "服务档案已删除。",
  profileActionFailed: "操作未完成，请重试。",
  profileMutationsLocked: "实时字幕运行期间，服务档案暂时锁定。",
  profileLimitReached: "最多可创建 20 个服务档案。",
  applicationTitle: "应用",
  applicationDescription: "设置界面语言与本机使用偏好。",
  sessionReady: "准备就绪",
  sessionNeedsCredential: "请先为当前服务档案配置凭证",
  sessionCredentialUnavailable: "暂时无法读取当前服务凭证",
  sessionListening: "正在识别并翻译",
  sessionStopping: "正在停止…",
  originalOnlyBadge: "仅显示原文",
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
  windowTitle: "mimi Console",
  windowSubtitle: "Manage subtitles, service profiles, and app preferences.",
  sessionEyebrow: "Current status",
  sessionTitle: "Live Subtitles",
  currentProfile: "Current service",
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
  subtitleDescription: "Tune recognition, translation, and the subtitle overlay.",
  sourceLanguage: "Recognition Language",
  translateTo: "Translate To",
  translationMode: "Translation Mode",
  fontSize: "Subtitle Size",
  lockPosition: "Lock Subtitle Position",
  lockHelp: "When unlocked, drag the subtitle top to move it, or resize from any edge or corner.",
  serviceProfilesTitle: "Service Profiles",
  serviceProfilesDescription: "Keep connection credentials separate for each live subtitle service.",
  addProfile: "Add Profile",
  chooseProvider: "Choose a service",
  chooseProviderDescription: "You can rename the profile later. Its service type cannot be changed.",
  cancel: "Cancel",
  profileName: "Profile Name",
  profileNamePlaceholder: "Enter a profile name",
  saveName: "Save Name",
  useProfile: "Make Current",
  activeProfile: "Current",
  deleteProfile: "Delete Profile",
  deleteProfileConfirm: (name: string): string =>
    `Delete “${name}” and its current profile credential? The credential will never be shown or exported.`,
  profileCount: (count: number): string => `${count} ${count === 1 ? "profile" : "profiles"}`,
  defaultAlibabaProfileName: "Alibaba Cloud",
  defaultOpenAIProfileName: "OpenAI Realtime",
  providerAlibaba: "Alibaba Cloud",
  providerOpenAI: "OpenAI Realtime",
  providerAlibabaDescription: "Supports every existing recognition language, target, and mode.",
  providerOpenAIDescription: "Automatic recognition with Chinese, English, or Japanese targets in Turbo mode.",
  profileProvider: "Service Type",
  credentials: "Connection Credentials",
  credentialPresent: "Credential saved",
  credentialMissing: "Credential not configured",
  credentialUnavailable: "Credential status unavailable",
  credentialUnavailableHelp: "The system keychain could not be read. Try again later.",
  apiKey: "API Key",
  apiKeyPlaceholder: "Enter a new API Key",
  credentialNote: "This field is always blank. Credentials are written only to this device's system keychain and never displayed.",
  saveCredentials: "Save Credentials",
  replaceCredentials: "Update Credentials",
  deleteCredentials: "Remove Credentials",
  deleteCredentialsConfirm: "Remove the credential from this profile?",
  credentialsSaved: "Credentials saved securely.",
  credentialsDeleted: "Credentials removed.",
  profileNameSaved: "Profile name updated.",
  profileCreated: "Service profile created.",
  profileSelected: "Current service changed.",
  profileSelectedWithAdjustments: "Current service changed, and subtitle settings were adjusted to match its supported options.",
  profileDeleted: "Service profile deleted.",
  profileActionFailed: "The action could not be completed. Try again.",
  profileMutationsLocked: "Service profiles are locked while live subtitles are running.",
  profileLimitReached: "You can create up to 20 service profiles.",
  applicationTitle: "Application",
  applicationDescription: "Set the interface language and local preferences.",
  sessionReady: "Ready",
  sessionNeedsCredential: "Add credentials to the current service profile to begin",
  sessionCredentialUnavailable: "The current service credential is temporarily unavailable",
  sessionListening: "Listening and translating",
  sessionStopping: "Stopping…",
  originalOnlyBadge: "Original only",
  recognizingChineseListening: "Recognizing Chinese; showing original text only and not sending translation requests.",
  recognizingChineseIdle: "Recognizes Chinese and shows original text only; no translation requests are sent.",
  sourceHelpReconnecting: "Switching will reconnect automatically and keep the current translation mode.",
  sourceHelpIdle: "Choose the main language for more accurate sentence translation.",
  switchToChineseHelp: "Switch to Chinese recognition and show original text only",
  switchToLanguageHelp: (language: string): string =>
    `Switch to ${language} recognition and keep the current translation mode`,
} satisfies SettingsCopy;

const SETTINGS_JA = {
  windowTitle: "mimi コンソール",
  windowSubtitle: "字幕、サービスプロファイル、アプリ設定を一か所で管理します。",
  sessionEyebrow: "現在の状態",
  sessionTitle: "リアルタイム字幕",
  currentProfile: "現在のサービス",
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
  subtitleDescription: "認識、翻訳、字幕ウィンドウの表示を調整します。",
  sourceLanguage: "認識言語",
  translateTo: "翻訳先",
  translationMode: "翻訳モード",
  fontSize: "字幕サイズ",
  lockPosition: "字幕の位置を固定",
  lockHelp: "固定を解除すると、字幕の上部をドラッグして移動したり、端や角からサイズを変更できます。",
  serviceProfilesTitle: "サービスプロファイル",
  serviceProfilesDescription: "リアルタイム字幕サービスごとに接続認証情報を分けて管理します。",
  addProfile: "プロファイルを追加",
  chooseProvider: "サービスを選択",
  chooseProviderDescription: "作成後も名前は変更できますが、サービスの種類は変更できません。",
  cancel: "キャンセル",
  profileName: "プロファイル名",
  profileNamePlaceholder: "プロファイル名を入力",
  saveName: "名前を保存",
  useProfile: "現在のサービスに設定",
  activeProfile: "現在",
  deleteProfile: "プロファイルを削除",
  deleteProfileConfirm: (name: string): string =>
    `「${name}」と現在の専用認証情報を削除しますか？認証情報は表示・書き出しされません。`,
  profileCount: (count: number): string => `${count} 件のプロファイル`,
  defaultAlibabaProfileName: "Alibaba Cloud",
  defaultOpenAIProfileName: "OpenAI Realtime",
  providerAlibaba: "Alibaba Cloud",
  providerOpenAI: "OpenAI Realtime",
  providerAlibabaDescription: "既存のすべての認識言語、翻訳先、モードに対応します。",
  providerOpenAIDescription: "自動認識、中国語・英語・日本語への翻訳、最速モードに対応します。",
  profileProvider: "サービスの種類",
  credentials: "接続認証情報",
  credentialPresent: "認証情報は保存済みです",
  credentialMissing: "認証情報が未設定です",
  credentialUnavailable: "認証情報の状態を確認できません",
  credentialUnavailableHelp: "システムのキーチェーンを読み取れませんでした。後でもう一度お試しください。",
  apiKey: "API Key",
  apiKeyPlaceholder: "新しい API Key を入力",
  credentialNote: "入力欄は常に空です。認証情報はこの端末のシステムキーチェーンにのみ書き込まれ、表示されません。",
  saveCredentials: "認証情報を保存",
  replaceCredentials: "認証情報を更新",
  deleteCredentials: "認証情報を削除",
  deleteCredentialsConfirm: "このプロファイルの認証情報を削除しますか？",
  credentialsSaved: "認証情報が安全に保存されました。",
  credentialsDeleted: "認証情報を削除しました。",
  profileNameSaved: "プロファイル名を更新しました。",
  profileCreated: "サービスプロファイルを作成しました。",
  profileSelected: "現在のサービスを切り替えました。",
  profileSelectedWithAdjustments: "現在のサービスを切り替え、対応範囲に合わせて字幕設定を調整しました。",
  profileDeleted: "サービスプロファイルを削除しました。",
  profileActionFailed: "操作を完了できませんでした。もう一度お試しください。",
  profileMutationsLocked: "リアルタイム字幕の実行中は、サービスプロファイルがロックされます。",
  profileLimitReached: "サービスプロファイルは最大 20 件まで作成できます。",
  applicationTitle: "アプリ",
  applicationDescription: "表示言語とこの端末での使用設定を変更します。",
  sessionReady: "準備完了",
  sessionNeedsCredential: "現在のサービスプロファイルに認証情報を追加してください",
  sessionCredentialUnavailable: "現在のサービス認証情報を一時的に確認できません",
  sessionListening: "認識・翻訳中",
  sessionStopping: "停止中…",
  originalOnlyBadge: "原文のみ",
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

export function sessionConnectingText(modeName: string): string {
  if (effectiveUiLanguage() === "ja") return `「${modeName}」翻訳に接続中…`;
  return isChineseSystem()
    ? `正在连接${modeName}翻译…`
    : `Connecting to ${modeName} translation…`;
}
