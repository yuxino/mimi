import type { AppUpdateCheck } from "../../lib/ipc";

export type UpdateCheckState =
  | { kind: "idle" }
  | { kind: "checking" }
  | { kind: "noUpdate"; latestVersion: string }
  | { kind: "available"; latestVersion: string }
  | { kind: "opening"; latestVersion: string }
  | { kind: "checkError" }
  | { kind: "openError"; latestVersion: string };

interface UpdateInteraction {
  action: "check" | "openRelease";
  busy: boolean;
  emphasized: boolean;
}

export function updateInteraction(state: UpdateCheckState): UpdateInteraction {
  switch (state.kind) {
    case "checking":
      return { action: "check", busy: true, emphasized: false };
    case "available":
    case "openError":
      return { action: "openRelease", busy: false, emphasized: true };
    case "opening":
      return { action: "openRelease", busy: true, emphasized: true };
    case "idle":
    case "noUpdate":
    case "checkError":
      return { action: "check", busy: false, emphasized: false };
  }
}

export function stateFromUpdateResult(
  result: AppUpdateCheck,
): UpdateCheckState {
  return result.updateAvailable
    ? { kind: "available", latestVersion: result.latestVersion }
    : { kind: "noUpdate", latestVersion: result.latestVersion };
}
