/**
 * IPC contract types for the mimi Tauri frontend.
 *
 * These mirror `docs/plans/2026-08-13-tauri-multiplatform.md` (the single source
 * of truth for the frontend/backend interface) and `Sources/MimiCore/Models.swift`
 * (language enums, display names, status semantics).
 */

import { I18N, isChineseSystem } from "./i18n";

// ---------------------------------------------------------------------------
// Session state
// ---------------------------------------------------------------------------

export type SessionStatus =
  | { kind: "idle" }
  | { kind: "connecting" }
  | { kind: "listening" }
  | { kind: "stopping" }
  | { kind: "error"; message: string };

export interface SubtitleLineSnapshot {
  text: string;
  isFinal: boolean;
}

export interface SubtitleHistoryItem {
  source: string;
  translation: string;
  /** Epoch milliseconds. */
  createdAt: number;
}

export interface SubtitleSnapshot {
  source: SubtitleLineSnapshot;
  translation: SubtitleLineSnapshot;
  history: SubtitleHistoryItem[];
}

export interface SessionStateEvent {
  status: SessionStatus;
  isActive: boolean;
  isPaused: boolean;
  isOverlayCollapsed: boolean;
  subtitles: SubtitleSnapshot;
  /** "zh" | "ja" | "en" | "ko" | ... (normalized language code). */
  detectedLanguage: string | null;
  isTranslationPending: boolean;
}

// ---------------------------------------------------------------------------
// Settings
// ---------------------------------------------------------------------------

export interface SettingsSnapshot {
  /** Loaded from the OS keychain so the settings field can be prefilled. */
  apiKey: string;
  hasAPIKey: boolean;
  sourceLanguage: SourceLanguage;
  targetLanguage: TargetLanguage;
  translationMode: TranslationMode;
  /** 14..20 */
  fontSize: number;
  isOverlayLocked: boolean;
  credentialLoadError: string | null;
  /** UI language override; `null` means follow the system language. */
  uiLanguage: UiLanguage | null;
}

export type UiLanguage = "system" | "zh" | "en";

export interface SettingsDraft {
  /** Only transmitted on settings_save; never present elsewhere. */
  apiKey?: string;
  sourceLanguage?: SourceLanguage;
  targetLanguage?: TargetLanguage;
  translationMode?: TranslationMode;
  fontSize?: number;
  isOverlayLocked?: boolean;
  uiLanguage?: UiLanguage;
}

// ---------------------------------------------------------------------------
// Languages
// ---------------------------------------------------------------------------

export type SourceLanguage = "auto" | "zh" | "en" | "ja" | "ko";
export type TargetLanguage = "original" | "zh" | "en" | "ja";
export type TranslationMode = "lowLatency" | "highQuality" | "turbo";

/** Picker order including automatic detection ("自动识别"), which runs the
 * low-latency pipeline and detects the language per utterance. */
export const SOURCE_LANGUAGE_QUICK_CASES: readonly SourceLanguage[] = [
  "auto",
  "ja",
  "en",
  "ko",
  "zh",
];

export const TARGET_LANGUAGE_CASES: readonly TargetLanguage[] = [
  "original",
  "zh",
  "en",
  "ja",
];

export const TRANSLATION_MODE_CASES: readonly TranslationMode[] = [
  "lowLatency",
  "highQuality",
  "turbo",
];

/** `SourceLanguage.displayName` (Models.swift), following the system language. */
export const SOURCE_LANGUAGE_DISPLAY_NAMES: Record<SourceLanguage, string> = isChineseSystem()
  ? {
      auto: "自动识别",
      zh: "中文",
      en: "英语",
      ja: "日语",
      ko: "韩语",
    }
  : {
      auto: "Auto Detect",
      zh: "Chinese",
      en: "English",
      ja: "Japanese",
      ko: "Korean",
    };

/** `TargetLanguage.displayName` (Models.swift), following the system language. */
export const TARGET_LANGUAGE_DISPLAY_NAMES: Record<TargetLanguage, string> = isChineseSystem()
  ? {
      original: "原文（不翻译）",
      zh: "简体中文",
      en: "英语",
      ja: "日语",
    }
  : {
      original: "Original (no translation)",
      zh: "Simplified Chinese",
      en: "English",
      ja: "Japanese",
    };

/** `TranslationMode.displayName` (Models.swift), following the system language. */
export const TRANSLATION_MODE_DISPLAY_NAMES: Record<TranslationMode, string> = isChineseSystem()
  ? {
      lowLatency: "低延迟",
      highQuality: "高质量",
      turbo: "极速",
    }
  : {
      lowLatency: "Low latency",
      highQuality: "High quality",
      turbo: "Turbo",
    };

/**
 * `DetectedLanguage.displayName` (Models.swift). The incoming code is already
 * normalized to the primary language segment (e.g. "zh", "en", "ja").
 */
const DETECTED_LANGUAGE_DISPLAY_NAMES: Record<string, string> = {
  zh: "中文",
  chinese: "中文",
  mandarin: "中文",
  yue: "粤语",
  cantonese: "粤语",
  en: "English",
  english: "English",
  ja: "日本語",
  japanese: "日本語",
  ko: "한국어",
  korean: "한국어",
  de: "Deutsch",
  fr: "Français",
  es: "Español",
  pt: "Português",
  it: "Italiano",
  ru: "Русский",
  ar: "العربية",
  hi: "हिन्दी",
  id: "Bahasa Indonesia",
  th: "ไทย",
  tr: "Türkçe",
  vi: "Tiếng Việt",
  uk: "Українська",
  cs: "Čeština",
  da: "Dansk",
  tl: "Filipino",
  fil: "Filipino",
  fi: "Suomi",
  is: "Íslenska",
  ms: "Bahasa Melayu",
  no: "Norsk",
  nb: "Norsk",
  pl: "Polski",
  sv: "Svenska",
};

export function detectedLanguageDisplayName(code: string): string {
  return DETECTED_LANGUAGE_DISPLAY_NAMES[code] ?? code.toUpperCase();
}

/**
 * `SourceLanguage.statusDisplayName(detectedLanguage:targetLanguage:)`
 * (Models.swift).
 */
export function sourceLanguageStatusDisplayName(
  sourceLanguage: SourceLanguage,
  detectedLanguage: string | null,
  targetLanguage: TargetLanguage,
): string {
  if (sourceLanguage !== "auto") {
    return SOURCE_LANGUAGE_DISPLAY_NAMES[sourceLanguage];
  }
  if (detectedLanguage === null) {
    return I18N.overlay.autoDetecting;
  }
  if (targetLanguage === "zh" && detectedLanguage === "zh") {
    return I18N.overlay.autoDetecting;
  }
  return `${I18N.overlay.autoDetectedPrefix}${detectedLanguageDisplayName(detectedLanguage)}${I18N.overlay.autoDetectedSuffix}`;
}

/**
 * `SourceLanguage.targetLanguageAfterQuickSwitch(from:currentTarget:)`
 * (Models.swift).
 */
export function targetLanguageAfterQuickSwitch(
  language: SourceLanguage,
  previousSource: SourceLanguage,
  currentTarget: TargetLanguage,
): TargetLanguage {
  if (language === "zh") {
    return "original";
  }
  if (previousSource === "zh" && currentTarget === "original") {
    return "zh";
  }
  return currentTarget;
}

/** `TargetLanguage.translatesAudio` (Models.swift). */
export function targetLanguageTranslatesAudio(target: TargetLanguage): boolean {
  return target !== "original";
}

// ---------------------------------------------------------------------------
// Overlay activity phase (SubtitleOverlayView.swift)
// ---------------------------------------------------------------------------

export type OverlayActivityPhaseKind =
  | "connecting"
  | "listening"
  | "recognizing"
  | "translating"
  | "paused";

export interface OverlayActivityPhaseInfo {
  accessibilityLabel: string;
  /** Base RGB as `#RRGGBB`. */
  color: string;
  /** Opacity already applied by `OverlayActivityPhase.color` in Swift. */
  baseOpacity: number;
  animationSpeed: number;
  amplitude: number;
}

/**
 * The `OverlayActivityPhase` table from `SubtitleOverlayView.swift`. Colors,
 * animation speed and amplitude are copied verbatim; `listening` and
 * `connecting` carry a pre-applied opacity just like their Swift counterparts.
 */
export const OVERLAY_ACTIVITY_PHASES: Record<
  OverlayActivityPhaseKind,
  OverlayActivityPhaseInfo
> = {
  connecting: {
    accessibilityLabel: I18N.overlay.phaseConnecting,
    color: "#FFFFFF",
    baseOpacity: 0.5,
    animationSpeed: 2.6,
    amplitude: 3,
  },
  listening: {
    accessibilityLabel: I18N.overlay.phaseListening,
    color: "#7AA8FF",
    baseOpacity: 0.62,
    animationSpeed: 2.6,
    amplitude: 2,
  },
  recognizing: {
    accessibilityLabel: I18N.overlay.phaseRecognizing,
    color: "#7AA8FF",
    baseOpacity: 1,
    animationSpeed: 2.6,
    amplitude: 6,
  },
  translating: {
    accessibilityLabel: I18N.overlay.phaseTranslating,
    color: "#B894FF",
    baseOpacity: 1,
    animationSpeed: 2.6,
    amplitude: 4,
  },
  paused: {
    accessibilityLabel: I18N.overlay.phasePaused,
    color: "#FFB852",
    baseOpacity: 1,
    animationSpeed: 0,
    amplitude: 0,
  },
};

// ---------------------------------------------------------------------------
// Small color helpers
// ---------------------------------------------------------------------------

/** Converts a `#RRGGBB` color into an `rgba()` string. */
export function hexToRgba(hex: string, alpha: number): string {
  const r = Number.parseInt(hex.slice(1, 3), 16);
  const g = Number.parseInt(hex.slice(3, 5), 16);
  const b = Number.parseInt(hex.slice(5, 7), 16);
  return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}

/** Resolves a phase color with an *additional* opacity multiplier applied. */
export function overlayPhaseColor(
  phase: OverlayActivityPhaseKind,
  opacity: number,
): string {
  const info = OVERLAY_ACTIVITY_PHASES[phase];
  return hexToRgba(info.color, info.baseOpacity * opacity);
}
