import { describe, expect, it } from "vitest";
import { isTauri } from "./ipc";
import { useStore } from "./store";

describe("local preview store", () => {
  it("keeps the synthetic provider ready outside Tauri", () => {
    expect(isTauri).toBe(false);
    expect(
      useStore.getState().settings.profiles[0]?.credentialState,
    ).toBe("present");
  });
});
