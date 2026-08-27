import { useEffect, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { I18N } from "../../lib/i18n";
import {
  appCheckForUpdates,
  appOpenReleases,
  isTauri,
} from "../../lib/ipc";
import { InlineFeedback, SettingsRow } from "./SettingsPrimitives";
import {
  stateFromUpdateResult,
  updateInteraction,
  type UpdateCheckState,
} from "./softwareUpdateModel";

/** User-initiated release check. It never polls or performs an installation. */
export function SoftwareUpdate() {
  const [currentVersion, setCurrentVersion] = useState<string>();
  const [state, setState] = useState<UpdateCheckState>({ kind: "idle" });

  useEffect(() => {
    if (!isTauri) return;

    let disposed = false;
    void getVersion()
      .then((version) => {
        if (!disposed) setCurrentVersion(version);
      })
      .catch(() => {});

    return () => {
      disposed = true;
    };
  }, []);

  const interaction = updateInteraction(state);

  const handleAction = async () => {
    if (interaction.action === "openRelease") {
      if (!("latestVersion" in state)) return;
      const latestVersion = state.latestVersion;
      setState({ kind: "opening", latestVersion });
      try {
        await appOpenReleases();
        setState({ kind: "available", latestVersion });
      } catch {
        setState({ kind: "openError", latestVersion });
      }
      return;
    }

    // Plain Vite preview is a deterministic UI fixture, not an update client.
    if (!isTauri) {
      setState({ kind: "noUpdate", latestVersion: "preview" });
      return;
    }

    setState({ kind: "checking" });
    try {
      const result = await appCheckForUpdates();
      setCurrentVersion(result.currentVersion);
      setState(stateFromUpdateResult(result));
    } catch {
      setState({ kind: "checkError" });
    }
  };

  const busyMessage =
    state.kind === "checking"
      ? I18N.settings.checkingForUpdates
      : state.kind === "opening"
        ? I18N.settings.openingUpdate
        : "";

  return (
    <>
      <SettingsRow
        label={I18N.settings.softwareUpdate}
        description={
          currentVersion
            ? I18N.settings.currentVersion(currentVersion)
            : I18N.settings.updateDescription
        }
      >
        <button
          type="button"
          className={`settings-button software-update-button ${
            interaction.emphasized
              ? "settings-button--primary"
              : "settings-button--quiet"
          }`}
          disabled={interaction.busy}
          aria-busy={interaction.busy}
          onClick={() => void handleAction()}
        >
          {state.kind === "checking"
            ? I18N.settings.checkingForUpdates
            : state.kind === "opening"
              ? I18N.settings.openingUpdate
              : interaction.action === "openRelease"
              ? I18N.settings.viewUpdate
              : I18N.settings.checkForUpdates}
        </button>
      </SettingsRow>

      <span
        className="software-update-live-status"
        role="status"
        aria-live="polite"
        aria-atomic="true"
      >
        {busyMessage}
      </span>

      <UpdateFeedback state={state} />
    </>
  );
}

function UpdateFeedback({ state }: { state: UpdateCheckState }) {
  switch (state.kind) {
    case "idle":
    case "checking":
    case "opening":
      return null;
    case "noUpdate":
      return (
        <InlineFeedback tone="success">
          {I18N.settings.noUpdateAvailable}
        </InlineFeedback>
      );
    case "available":
      return (
        <InlineFeedback tone="info">
          {I18N.settings.updateAvailable(state.latestVersion)}
        </InlineFeedback>
      );
    case "checkError":
      return (
        <InlineFeedback tone="error">
          {I18N.settings.updateCheckFailed}
        </InlineFeedback>
      );
    case "openError":
      return (
        <InlineFeedback tone="error">
          {I18N.settings.openUpdateFailed}
        </InlineFeedback>
      );
  }
}
