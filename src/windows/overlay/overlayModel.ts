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
import { isChineseSystem } from "../../lib/i18n";
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
  if (session.isPaused) return "已暂停";

  switch (session.status.kind) {
    case "connecting":
      return "正在连接";
    case "listening":
      return isWaitingForFinalTranslation(
        settings,
        session.detectedLanguage,
        session.isTranslationPending,
      )
        ? "正在翻译"
        : "正在聆听，译文会保留在这里";
    case "stopping":
      return "正在结束";
    case "error":
      return session.status.message;
    case "idle":
      return "mimi";
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
  subtitles: SubtitleSnapshot,
  segmentLength: number,
): SubtitleRow[] {
  const rows: SubtitleRow[] = [];

  for (const pair of subtitles.history) {
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
  subtitles: SubtitleSnapshot,
): { text: string; isFinal: boolean } | null {
  const currentLine = subtitles.translation;
  if (currentLine.text === "") return null;
  const currentIsAlreadyInHistory =
    currentLine.isFinal &&
    subtitles.history[subtitles.history.length - 1]?.translation ===
      currentLine.text;
  if (currentIsAlreadyInHistory) return null;
  return { text: currentLine.text, isFinal: currentLine.isFinal };
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
    return { source: sourceName, separator: "·", target: "原文" };
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
    ? isChineseSystem()
      ? "中文原文"
      : "Chinese (Original)"
    : SOURCE_LANGUAGE_DISPLAY_NAMES[sourceLanguage];
}

export function hasSubtitleContent(subtitles: SubtitleSnapshot): boolean {
  return (
    subtitles.source.text !== "" ||
    subtitles.translation.text !== "" ||
    subtitles.history.length > 0
  );
}
