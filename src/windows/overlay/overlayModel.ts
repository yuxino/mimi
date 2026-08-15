/**
 * Pure derived state for the subtitle overlay, ported from
 * `SubtitleOverlayView.swift`. Keeping these as pure functions makes the
 * phase/row logic testable and keeps the React components lean.
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

export function isSameLanguageMode(
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
  if (session.isPaused) return "paused";

  switch (session.status.kind) {
    case "connecting":
    case "stopping":
      return "connecting";
    case "listening": {
      if (
        isWaitingForFinalTranslation(
          settings,
          session.detectedLanguage,
          session.isTranslationPending,
        )
      ) {
        return "translating";
      }
      const source = session.subtitles.source;
      if (source.text !== "" && !source.isFinal) {
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
    return { source: sourceName, separator: "·", target: I18N.overlay.original };
  }
  return {
    source: sourceName,
    separator: "→",
    target: TARGET_LANGUAGE_DISPLAY_NAMES[settings.targetLanguage],
  };
}

export function sourceLanguageButtonTitle(
  sourceLanguage: SettingsSnapshot["sourceLanguage"],
): string {
  return sourceLanguage === "zh"
    ? I18N.overlay.chineseSource
    : SOURCE_LANGUAGE_DISPLAY_NAMES[sourceLanguage];
}

export function hasSubtitleContent(subtitles: SubtitleSnapshot): boolean {
  return (
    subtitles.source.text !== "" ||
    subtitles.translation.text !== "" ||
    subtitles.history.length > 0
  );
}
