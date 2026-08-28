/**
 * Pure derived state for the subtitle overlay. Keeping these transformations
 * outside React makes the phase and row logic deterministic and testable.
 */

import type {
  OverlayActivityPhaseKind,
  SessionStateEvent,
  SettingsSnapshot,
  SubtitleSnapshot,
} from "../../lib/types";
import { I18N } from "../../lib/i18n";
import {
  SOURCE_LANGUAGE_DISPLAY_NAMES,
  TARGET_LANGUAGE_DISPLAY_NAMES,
  sourceLanguageStatusDisplayName,
} from "../../lib/types";
import { segments } from "./segmenter";

export interface SubtitleRow {
  id: string;
  text: string;
  /** Epoch ms for the first row of a history pair; `null` otherwise. */
  createdAt: number | null;
}

function isSameLanguageMode(
  settings: Pick<SettingsSnapshot, "sourceLanguage" | "targetLanguage">,
  detectedLanguage: string | null,
): boolean {
  if (settings.targetLanguage === "original") return true;
  if (detectedLanguage !== null) {
    return detectedLanguage === settings.targetLanguage;
  }
  return (
    settings.sourceLanguage !== "auto" &&
    settings.sourceLanguage === settings.targetLanguage
  );
}

export function isWaitingForFinalTranslation(
  settings: Pick<SettingsSnapshot, "sourceLanguage" | "targetLanguage">,
  detectedLanguage: string | null,
  isTranslationPending: boolean,
): boolean {
  if (isSameLanguageMode(settings, detectedLanguage)) return false;
  return isTranslationPending;
}

export function computeActivityPhase(
  session: SessionStateEvent,
  settings: Pick<SettingsSnapshot, "sourceLanguage" | "targetLanguage">,
): OverlayActivityPhaseKind {
  const source = session.subtitles.source;
  return computeActivityPhaseFromSignals(
    {
      statusKind: session.status.kind,
      isPaused: session.isPaused,
      detectedLanguage: session.detectedLanguage,
      isTranslationPending: session.isTranslationPending,
      hasRecognizingSourceDraft: source.text !== "" && !source.isFinal,
    },
    settings,
  );
}

export interface ActivityPhaseSignals {
  statusKind: SessionStateEvent["status"]["kind"];
  isPaused: boolean;
  detectedLanguage: string | null;
  isTranslationPending: boolean;
  hasRecognizingSourceDraft: boolean;
}

export function computeActivityPhaseFromSignals(
  signals: ActivityPhaseSignals,
  settings: Pick<SettingsSnapshot, "sourceLanguage" | "targetLanguage">,
): OverlayActivityPhaseKind {
  if (signals.isPaused) return "paused";

  switch (signals.statusKind) {
    case "connecting":
    case "stopping":
      return "connecting";
    case "listening": {
      if (
        isWaitingForFinalTranslation(
          settings,
          signals.detectedLanguage,
          signals.isTranslationPending,
        )
      ) {
        return "translating";
      }
      if (signals.hasRecognizingSourceDraft) {
        return "recognizing";
      }
      return "listening";
    }
    case "idle":
    case "error":
      return "listening";
  }
}

export function emptyStateText(
  session: SessionStateEvent,
  settings: Pick<SettingsSnapshot, "sourceLanguage" | "targetLanguage">,
): string {
  if (session.isPaused) return I18N.overlay.paused;

  switch (session.status.kind) {
    case "connecting":
      return I18N.overlay.connecting;
    case "listening":
      return isWaitingForFinalTranslation(
        settings,
        session.detectedLanguage,
        session.isTranslationPending,
      )
        ? I18N.overlay.translatingEmpty
        : I18N.overlay.listeningEmpty;
    case "stopping":
      return I18N.overlay.stopping;
    case "error":
      return session.status.message;
    case "idle":
      return I18N.overlay.idle;
  }
}

export function emptyStateIsError(session: SessionStateEvent): boolean {
  return session.status.kind === "error";
}

export type EmptyStateDensity = "minimal" | "compact" | "comfortable";

/**
 * Empty-state chrome adapts to the freely resized overlay height. At the
 * 100px native minimum only one status line fits below the control band, so
 * the decorative pulse yields to the text instead of being clipped.
 */
export function emptyStateDensity(overlayHeight: number): EmptyStateDensity {
  if (overlayHeight <= 112) return "minimal";
  if (overlayHeight < 176) return "compact";
  return "comfortable";
}

export function timelineClassName(blendsWithBackground: boolean): string {
  return [
    "min-h-0 flex-1 overflow-y-auto",
    blendsWithBackground ? "overlay-timeline--immersive" : "",
  ]
    .filter(Boolean)
    .join(" ");
}

/** Maximum characters per segment for the current target/display language. */
export function subtitleSegmentLength(
  targetLanguage: SettingsSnapshot["targetLanguage"],
  detectedLanguage: string | null,
): number {
  switch (targetLanguage) {
    case "zh":
      return 28;
    case "en":
      return 64;
    case "ja":
      return 32;
    case "original":
      switch (detectedLanguage) {
        case "en":
          return 64;
        case "ja":
          return 32;
        default:
          return 28;
      }
  }
}

export function computeVisibleRows(
  history: SubtitleSnapshot["history"],
  segmentLength: number,
): SubtitleRow[] {
  const rows: SubtitleRow[] = [];

  for (const pair of history) {
    const pairSegments = segments(pair.translation, segmentLength);
    pairSegments.forEach((text, index) => {
      rows.push({
        id: `history-${pair.createdAt}-${index}`,
        text,
        createdAt: index === 0 ? pair.createdAt : null,
      });
    });
  }

  return rows;
}

/**
 * The live preview line: the current unconfirmed translation (or a just-final
 * line that has not yet entered history). The overlay renders it as the
 * timeline's last row — dimmed with a trailing ellipsis — so streaming
 * updates never look like a separate pile at the bottom. Returns `null` when
 * there is nothing to preview.
 */
export function visibleDraft(
  translation: SubtitleSnapshot["translation"],
  history: SubtitleSnapshot["history"],
): { text: string; isFinal: boolean } | null {
  if (translation.text === "") return null;
  const currentIsAlreadyInHistory =
    translation.isFinal &&
    history[history.length - 1]?.translation === translation.text;
  if (currentIsAlreadyInHistory) return null;
  return { text: translation.text, isFinal: translation.isFinal };
}

export interface LiveSubtitlePreview {
  text: string;
  isFinal: boolean;
  kind: "translation" | "source";
}

/**
 * Selects the active subtitle tail. Translation remains the preferred output,
 * but source recognition is a latency and failure fallback: long utterances,
 * same-language audio, and an empty provider translation must never leave the
 * subtitle canvas blank while usable recognition text already exists.
 */
export function visibleLiveSubtitle(
  subtitles: SubtitleSnapshot,
  settings: Pick<SettingsSnapshot, "sourceLanguage" | "targetLanguage">,
  detectedLanguage: string | null,
  isTranslationPending: boolean,
  isTranslationTimedOut: boolean,
): LiveSubtitlePreview | null {
  const translation = visibleDraft(
    subtitles.translation,
    subtitles.history,
  );
  if (translation !== null) {
    return { ...translation, kind: "translation" };
  }

  const source = subtitles.source;
  if (source.text === "") return null;

  const latestPair = subtitles.history[subtitles.history.length - 1];
  const currentTranslationMatchesLatestPair =
    subtitles.translation.isFinal &&
    subtitles.translation.text !== "" &&
    latestPair?.translation === subtitles.translation.text;
  const sourceIsAlreadyCommitted =
    source.isFinal &&
    !isTranslationPending &&
    !isTranslationTimedOut &&
    latestPair?.source === source.text &&
    currentTranslationMatchesLatestPair;
  if (sourceIsAlreadyCommitted) return null;

  return {
    text: source.text,
    // A final source is durable display text only when no translation is
    // semantically needed. Otherwise keep the preview treatment until the
    // provider supplies the translated replacement.
    isFinal:
      source.isFinal &&
      isSameLanguageMode(settings, detectedLanguage),
    kind: "source",
  };
}

export interface LanguageStatus {
  source: string;
  separator: string;
  target: string;
}

export function languageStatus(
  settings: SettingsSnapshot,
  detectedLanguage: string | null,
): LanguageStatus | null {
  const sourceName = sourceLanguageStatusDisplayName(
    settings.sourceLanguage,
    detectedLanguage,
    settings.targetLanguage,
  );

  if (settings.targetLanguage === "original") {
    return {
      source: sourceName,
      separator: I18N.overlay.dotSeparator,
      target: I18N.overlay.original,
    };
  }
  return {
    source: sourceName,
    separator: I18N.overlay.separator,
    target: TARGET_LANGUAGE_DISPLAY_NAMES[settings.targetLanguage],
  };
}

export function sourceLanguageButtonTitle(
  sourceLanguage: SettingsSnapshot["sourceLanguage"],
  chineseIsOriginalOnly = true,
): string {
  return sourceLanguage === "zh"
    ? chineseIsOriginalOnly
      ? I18N.overlay.chineseSource
      : SOURCE_LANGUAGE_DISPLAY_NAMES.zh
    : SOURCE_LANGUAGE_DISPLAY_NAMES[sourceLanguage];
}

export function hasSubtitleContent(subtitles: SubtitleSnapshot): boolean {
  return (
    subtitles.source.text !== "" ||
    subtitles.translation.text !== "" ||
    subtitles.history.length > 0
  );
}
