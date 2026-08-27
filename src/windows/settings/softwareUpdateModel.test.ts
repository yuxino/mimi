import { describe, expect, it } from "vitest";
import {
  stateFromUpdateResult,
  updateInteraction,
  type UpdateCheckState,
} from "./softwareUpdateModel";

describe("software update interaction", () => {
  it.each<{
    state: UpdateCheckState;
    action: "check" | "openRelease";
    busy: boolean;
    emphasized: boolean;
  }>([
    {
      state: { kind: "idle" },
      action: "check",
      busy: false,
      emphasized: false,
    },
    {
      state: { kind: "checking" },
      action: "check",
      busy: true,
      emphasized: false,
    },
    {
      state: { kind: "noUpdate", latestVersion: "1.3.1" },
      action: "check",
      busy: false,
      emphasized: false,
    },
    {
      state: { kind: "available", latestVersion: "1.4.0" },
      action: "openRelease",
      busy: false,
      emphasized: true,
    },
    {
      state: { kind: "opening", latestVersion: "1.4.0" },
      action: "openRelease",
      busy: true,
      emphasized: true,
    },
    {
      state: { kind: "openError", latestVersion: "1.4.0" },
      action: "openRelease",
      busy: false,
      emphasized: true,
    },
    {
      state: { kind: "checkError" },
      action: "check",
      busy: false,
      emphasized: false,
    },
  ])("derives $state.kind button behavior", (expected) => {
    expect(updateInteraction(expected.state)).toEqual({
      action: expected.action,
      busy: expected.busy,
      emphasized: expected.emphasized,
    });
  });

  it("maps backend availability to the matching visible state", () => {
    expect(
      stateFromUpdateResult({
        currentVersion: "1.3.1",
        latestVersion: "1.4.0",
        updateAvailable: true,
      }),
    ).toEqual({ kind: "available", latestVersion: "1.4.0" });
    expect(
      stateFromUpdateResult({
        currentVersion: "1.3.1",
        latestVersion: "1.3.0",
        updateAvailable: false,
      }),
    ).toEqual({ kind: "noUpdate", latestVersion: "1.3.0" });
  });
});
