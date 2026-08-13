import { describe, expect, it } from "vitest";
import { segments, visibleDraftSegments } from "./segmenter";

describe("SubtitleTextSegmenter", () => {
  it("keeps a short subtitle as a single segment", () => {
    expect(segments("今天的天气很好。", 28)).toEqual(["今天的天气很好。"]);
  });

  it("splits on sentence punctuation", () => {
    expect(
      segments(
        "第一句话已经说完。第二句话也说完了！第三句还在继续",
        28,
      ),
    ).toEqual(["第一句话已经说完。", "第二句话也说完了！", "第三句还在继续"]);
  });

  it("bounds continuous CJK speech without losing text", () => {
    const text =
      "这是一段完全没有句号而且会持续不断增长的字幕内容用来模拟视频里人物一直讲话的情况";
    const result = segments(text, 14);

    expect(result.length).toBeGreaterThan(1);
    for (const segment of result) {
      expect(segment.length).toBeLessThanOrEqual(14);
    }
    expect(result.join("")).toBe(text);
  });

  it("prefers word boundaries for English subtitles", () => {
    const text =
      "This is a continuous English subtitle that should never split a normal word.";
    const result = segments(text, 24);

    expect(result.length).toBeGreaterThan(1);
    expect(result.join(" ")).toBe(text);
    for (const segment of result.slice(0, -1)) {
      expect(segment.endsWith(" ")).toBe(false);
    }
  });

  it("preserves completed segment prefixes when a draft grows", () => {
    const first = segments(
      "持续讲话时字幕会不断增长直到超过一行然后继续",
      12,
    );
    const extended = segments(
      "持续讲话时字幕会不断增长直到超过一行然后继续显示后面的新增内容",
      12,
    );

    expect(extended.slice(0, first.length - 1)).toEqual(first.slice(0, -1));
  });

  it("keeps only the newest two segments of a long draft preview", () => {
    const text = "第一段已经说完。第二段也已经说完。第三段正在继续。第四段是最新内容";
    const full = segments(text, 12);

    expect(visibleDraftSegments(text, 12, 2)).toEqual(full.slice(-2));
  });

  it("keeps a short draft preview intact", () => {
    expect(visibleDraftSegments("短字幕", 12, 2)).toEqual(["短字幕"]);
  });
});
