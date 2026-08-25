import { describe, expect, it } from "vitest";
import { rowHorizontalPadding } from "./alignment";
import { emptyStateDensity, timelineClassName } from "./overlayModel";

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

describe("overlay responsive presentation", () => {
  it("keeps the status text visible at the native minimum height", () => {
    expect(emptyStateDensity(100)).toBe("minimal");
    expect(emptyStateDensity(112)).toBe("minimal");
    expect(emptyStateDensity(113)).toBe("compact");
    expect(emptyStateDensity(175)).toBe("compact");
    expect(emptyStateDensity(176)).toBe("comfortable");
    expect(emptyStateDensity(240)).toBe("comfortable");
  });

  it("hides only the immersive timeline scrollbar", () => {
    expect(timelineClassName(true)).toContain("overlay-timeline--immersive");
    expect(timelineClassName(false)).not.toContain(
      "overlay-timeline--immersive",
    );
  });
});
