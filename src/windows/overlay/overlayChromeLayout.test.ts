import { describe, expect, it } from "vitest";
import { overlayTopChromeLayout } from "./overlayChromeLayout";

const CONTROL_ISLAND_RIGHT = 254;
const CHROME_GAP = 6;

function handleEdges(layout: ReturnType<typeof overlayTopChromeLayout>) {
  return {
    left: layout.dragHandleCenterX - layout.dragHandleWidth / 2,
    right: layout.dragHandleCenterX + layout.dragHandleWidth / 2,
  };
}

describe("overlay top chrome layout", () => {
  it("keeps the drag handle reachable to the right of the island at 360px", () => {
    const layout = overlayTopChromeLayout(360, true);
    const handle = handleEdges(layout);

    expect(layout.showActions).toBe(false);
    expect(handle.left).toBeGreaterThanOrEqual(
      CONTROL_ISLAND_RIGHT + CHROME_GAP,
    );
    expect(handle.right).toBeLessThanOrEqual(360 - 10 - CHROME_GAP);
  });

  it("keeps the compact fallback through widths that cannot fit all actions", () => {
    for (const width of [400, 480]) {
      const layout = overlayTopChromeLayout(width, true);
      expect(layout.showActions).toBe(false);
      expect(handleEdges(layout).left).toBeGreaterThanOrEqual(
        CONTROL_ISLAND_RIGHT + CHROME_GAP,
      );
    }
  });

  it("restores actions once the island, handle, and full row fit", () => {
    const layout = overlayTopChromeLayout(520, true);
    const handle = handleEdges(layout);
    const actionRowLeft = 520 - 10 - 164;

    expect(layout.showActions).toBe(true);
    expect(handle.left).toBeGreaterThanOrEqual(
      CONTROL_ISLAND_RIGHT + CHROME_GAP,
    );
    expect(handle.right).toBeLessThanOrEqual(actionRowLeft - CHROME_GAP);
  });

  it("preserves the centered 120px handle and all actions at 640px", () => {
    expect(overlayTopChromeLayout(640, true)).toEqual({
      dragHandleCenterX: 320,
      dragHandleWidth: 120,
      showActions: true,
    });
  });

  it("keeps an inactive overlay centered because it has no control island", () => {
    expect(overlayTopChromeLayout(360, false)).toEqual({
      dragHandleCenterX: 180,
      dragHandleWidth: 100,
      showActions: false,
    });
  });
});
