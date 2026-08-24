import type { SubtitleAlignment } from "../../lib/types";

/** Keeps the timestamp gutter outside the subtitle's visual alignment area. */
export function rowHorizontalPadding(
  alignment: SubtitleAlignment,
  side: "left" | "right",
  blendsWithBackground: boolean,
): number {
  if (blendsWithBackground) return 18;
  if (alignment === "center") return 57;
  if (alignment === side) return 57;
  return 18;
}
