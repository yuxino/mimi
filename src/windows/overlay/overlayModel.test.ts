import { describe, expect, it } from "vitest";
import { I18N } from "../../lib/i18n";
import {
  SOURCE_LANGUAGE_DISPLAY_NAMES,
  type SubtitleSnapshot,
} from "../../lib/types";
import {
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

describe("live subtitle fallback", () => {
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

  it("shows source recognition while a long utterance has no translation", () => {
    expect(
      visibleLiveSubtitle(
        subtitles({ text: "still recognizing", isFinal: false }),
        settings,
        "en",
        false,
        false,
      ),
    ).toEqual({ text: "still recognizing", isFinal: false, kind: "source" });
  });

  it("keeps an unpaired source final visible after translation times out", () => {
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
    ).toEqual({ text: "recognized final", isFinal: false, kind: "source" });
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

  it("keeps a repeated source final visible while its translation is pending", () => {
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
    ).toEqual({ text: "repeated lyric", isFinal: false, kind: "source" });
  });

  it("keeps a repeated source final visible after its translation times out", () => {
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
    ).toEqual({ text: "repeated lyric", isFinal: false, kind: "source" });
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
