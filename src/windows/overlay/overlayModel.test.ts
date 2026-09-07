import { describe, expect, it } from "vitest";
import { I18N } from "../../lib/i18n";
import {
  SOURCE_LANGUAGE_DISPLAY_NAMES,
  type SubtitleSnapshot,
} from "../../lib/types";
import {
  computeActivityPhaseFromSignals,
  sourceLanguageButtonTitle,
  visibleLiveSubtitle,
} from "./overlayModel";

const settings = {
  sourceLanguage: "auto" as const,
  targetLanguage: "zh" as const,
};

function subtitles(
  source: SubtitleSnapshot["source"],
  translation: SubtitleSnapshot["translation"] = {
    text: "",
    isFinal: false,
  },
  history: SubtitleSnapshot["history"] = [],
): SubtitleSnapshot {
  return { source, translation, history };
}

describe("live subtitle display mode", () => {
  it("prefers a translation draft over source recognition", () => {
    expect(
      visibleLiveSubtitle(
        subtitles(
          { text: "source draft", isFinal: false },
          { text: "译文草稿", isFinal: false },
        ),
        settings,
        "en",
        false,
        false,
      ),
    ).toEqual({ text: "译文草稿", isFinal: false, kind: "translation" });
  });

  it("does not flash source recognition before the first translation", () => {
    expect(
      visibleLiveSubtitle(
        subtitles({ text: "still recognizing", isFinal: false }),
        settings,
        "en",
        false,
        false,
      ),
    ).toBeNull();
  });

  it("does not replace missing translation with source after timeout", () => {
    expect(
      visibleLiveSubtitle(
        subtitles(
          { text: "recognized final", isFinal: true },
          { text: "", isFinal: true },
          [
            {
              source: "previous source",
              translation: "上一句译文",
              createdAt: 1,
            },
          ],
        ),
        settings,
        "en",
        false,
        true,
      ),
    ).toBeNull();
  });

  it("treats same-language recognition as final subtitle text", () => {
    expect(
      visibleLiveSubtitle(
        subtitles({ text: "中文识别结果", isFinal: true }),
        settings,
        "zh",
        false,
        false,
      ),
    ).toEqual({ text: "中文识别结果", isFinal: true, kind: "source" });
  });

  it("does not duplicate a source final already committed to history", () => {
    const history = [
      {
        source: "finished source",
        translation: "完成的译文",
        createdAt: 1,
      },
    ];
    expect(
      visibleLiveSubtitle(
        subtitles(
          { text: "finished source", isFinal: true },
          { text: "完成的译文", isFinal: true },
          history,
        ),
        settings,
        "en",
        false,
        false,
      ),
    ).toBeNull();
  });

  it("does not append a repeated source while the next translation is pending", () => {
    const history = [
      {
        source: "repeated lyric",
        translation: "重复歌词",
        createdAt: 1,
      },
    ];
    expect(
      visibleLiveSubtitle(
        subtitles(
          { text: "repeated lyric", isFinal: true },
          { text: "重复歌词", isFinal: true },
          history,
        ),
        settings,
        "en",
        true,
        false,
      ),
    ).toBeNull();
  });

  it("does not append a repeated source after its translation times out", () => {
    const history = [
      {
        source: "repeated lyric",
        translation: "上一遍歌词",
        createdAt: 1,
      },
    ];
    expect(
      visibleLiveSubtitle(
        subtitles(
          { text: "repeated lyric", isFinal: true },
          { text: "上一遍歌词", isFinal: true },
          history,
        ),
        settings,
        "en",
        false,
        true,
      ),
    ).toBeNull();
  });

  it("keeps recognition visible in explicitly selected original mode", () => {
    expect(
      visibleLiveSubtitle(
        subtitles({ text: "original draft", isFinal: false }),
        { ...settings, targetLanguage: "original" },
        "en",
        false,
        false,
      ),
    ).toEqual({ text: "original draft", isFinal: false, kind: "source" });
  });

  it("waits for translation while automatic source language is unknown", () => {
    expect(
      visibleLiveSubtitle(
        subtitles({ text: "unclassified draft", isFinal: false }),
        settings,
        null,
        true,
        false,
      ),
    ).toBeNull();
  });
});

describe("source language labels", () => {
  it("labels Chinese as original-only only when the active provider supports that mode", () => {
    expect(sourceLanguageButtonTitle("zh", true)).toBe(
      I18N.overlay.chineseSource,
    );
    expect(sourceLanguageButtonTitle("zh", false)).toBe(
      SOURCE_LANGUAGE_DISPLAY_NAMES.zh,
    );
  });
});

describe("activity phase signals", () => {
  const base = {
    statusKind: "listening" as const,
    isPaused: false,
    detectedLanguage: "ja",
    isTranslationPending: false,
    hasRecognizingSourceDraft: false,
  };

  it("distinguishes recognizing and translating without subtitle text", () => {
    expect(
      computeActivityPhaseFromSignals(
        { ...base, hasRecognizingSourceDraft: true },
        settings,
      ),
    ).toBe("recognizing");
    expect(
      computeActivityPhaseFromSignals(
        { ...base, isTranslationPending: true },
        settings,
      ),
    ).toBe("translating");
  });

  it("keeps pause and lifecycle transitions ahead of stream activity", () => {
    expect(
      computeActivityPhaseFromSignals(
        { ...base, isPaused: true, isTranslationPending: true },
        settings,
      ),
    ).toBe("paused");
    expect(
      computeActivityPhaseFromSignals(
        { ...base, statusKind: "stopping", hasRecognizingSourceDraft: true },
        settings,
      ),
    ).toBe("connecting");
  });
});
