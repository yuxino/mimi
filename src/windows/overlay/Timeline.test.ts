import { describe, expect, it } from "vitest";
import { rowHorizontalPadding } from "./alignment";

describe("subtitle row alignment", () => {
  it("reserves the timestamp gutter on the aligned edge", () => {
    expect(rowHorizontalPadding("left", "left", false)).toBe(57);
    expect(rowHorizontalPadding("left", "right", false)).toBe(18);
    expect(rowHorizontalPadding("right", "left", false)).toBe(18);
    expect(rowHorizontalPadding("right", "right", false)).toBe(57);
  });

  it("keeps centered and background-blended subtitles symmetric", () => {
    expect(rowHorizontalPadding("center", "left", false)).toBe(57);
    expect(rowHorizontalPadding("center", "right", false)).toBe(57);
    expect(rowHorizontalPadding("right", "left", true)).toBe(18);
    expect(rowHorizontalPadding("right", "right", true)).toBe(18);
  });
});
