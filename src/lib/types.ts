/**
 * IPC contract types for the mimi Tauri frontend.
 *
 * These mirror the Tauri command payloads and `src-tauri/src/core/models.rs`
 * (language enums and status semantics).
 */

import { I18N, effectiveUiLanguage, isChineseSystem } from "./i18n";

// ---------------------------------------------------------------------------
// Session state
// ---------------------------------------------------------------------------

type SessionStatus =
  | { kind: "idle" }
  | { kind: "connecting" }
  | { kind: "listening" }
  | { kind: "stopping" }
  | { kind: "error"; message: string };

interface SubtitleLineSnapshot {
  text: string;
  isFinal: boolean;
}

interface SubtitleHistoryItem {
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
  /** Service profiles never contain credential material, only availability. */
  profiles: ServiceProfile[];
  activeProfileId: string;
  sourceLanguage: SourceLanguage;
  targetLanguage: TargetLanguage;
  translationMode: TranslationMode;
  /** 14..20 */
  fontSize: number;
  isOverlayLocked: boolean;
  /** UI language override; `null` or `system` follows the system language. */
  uiLanguage: UiLanguage | null;
}

export type UiLanguage = "system" | "zh" | "en" | "ja";

export interface SettingsDraft {
  sourceLanguage?: SourceLanguage;
  targetLanguage?: TargetLanguage;
  translationMode?: TranslationMode;
  fontSize?: number;
  isOverlayLocked?: boolean;
  uiLanguage?: UiLanguage;
}

export type ServiceProvider = "alibabaCloud" | "openAIRealtime";

/**
 * Sanitized keychain state returned by native snapshots. A replacement key
 * crosses IPC only in the dedicated write-only save command and is never
 * returned to the frontend.
 */
export type CredentialState = "present" | "missing" | "unavailable";

export interface ServiceProfile {
  id: string;
  name: string;
  provider: ServiceProvider;
  credentialState: CredentialState;
}

export interface ProviderCapabilities {
  sourceLanguages: readonly SourceLanguage[];
  targetLanguages: readonly TargetLanguage[];
  translationModes: readonly TranslationMode[];
}

// ---------------------------------------------------------------------------
// Languages
// ---------------------------------------------------------------------------

export type SourceLanguage = "auto" | "zh" | "en" | "ja" | "ko";
export type TargetLanguage = "original" | "zh" | "en" | "ja";
export type TranslationMode = "lowLatency" | "highQuality" | "turbo";

/** Picker order including provider-specific automatic language detection. */
export const SOURCE_LANGUAGE_QUICK_CASES: readonly SourceLanguage[] = [
  "auto",
  "ja",
  "en",
  "ko",
  "zh",
];

export const TRANSLATION_MODE_CASES: readonly TranslationMode[] = [
  "lowLatency",
  "highQuality",
  "turbo",
];

/** Localized source-language labels for the active UI language. */
export const SOURCE_LANGUAGE_DISPLAY_NAMES: Record<SourceLanguage, string> =
  effectiveUiLanguage() === "ja"
    ? {
        auto: "自動認識",
        zh: "中国語",
        en: "英語",
        ja: "日本語",
        ko: "韓国語",
      }
    : isChineseSystem()
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

/** Localized target-language labels for the active UI language. */
export const TARGET_LANGUAGE_DISPLAY_NAMES: Record<TargetLanguage, string> =
  effectiveUiLanguage() === "ja"
    ? {
        original: "原文（翻訳しない）",
        zh: "簡体中国語",
        en: "英語",
        ja: "日本語",
      }
    : isChineseSystem()
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

/** Localized translation-mode labels for the active UI language. */
export const TRANSLATION_MODE_DISPLAY_NAMES: Record<TranslationMode, string> =
  effectiveUiLanguage() === "ja"
    ? {
        lowLatency: "低遅延",
        highQuality: "高品質",
        turbo: "最速",
      }
    : isChineseSystem()
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

/** Display labels for normalized recognition-service language codes. */
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

function detectedLanguageDisplayName(code: string): string {
  return DETECTED_LANGUAGE_DISPLAY_NAMES[code] ?? code.toUpperCase();
}

/** Builds the source-language status shown while automatic detection runs. */
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

/** Keeps quick source changes paired with a meaningful target language. */
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

/** Whether the selected target requires machine translation. */
export function targetLanguageTranslatesAudio(target: TargetLanguage): boolean {
  return target !== "original";
}

// ---------------------------------------------------------------------------
// Overlay activity phase
// ---------------------------------------------------------------------------

export type OverlayActivityPhaseKind =
  | "connecting"
  | "listening"
  | "recognizing"
  | "translating"
  | "paused";

interface OverlayActivityPhaseInfo {
  accessibilityLabel: string;
  /** Base RGB as `#RRGGBB`. */
  color: string;
  /** Opacity applied to the base phase color. */
  baseOpacity: number;
  animationSpeed: number;
  amplitude: number;
}

/**
 * Visual parameters for the overlay activity indicator. Listening and
 * connecting use a quieter opacity than active processing phases.
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
