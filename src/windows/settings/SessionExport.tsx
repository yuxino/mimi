import { useEffect, useRef, useState } from "react";
import { Switch } from "../../components/Switch";
import { I18N } from "../../lib/i18n";
import {
  isTauri,
  sessionArchiveClear,
  sessionArchiveState,
  sessionExport,
} from "../../lib/ipc";
import { useStore } from "../../lib/store";
import type { SessionArchiveState, SettingsDraft } from "../../lib/types";
import { InlineFeedback, SettingsRow, SettingsSection } from "./SettingsPrimitives";
import { monitorSessionArchive } from "./sessionArchiveMonitor";

export function SessionExport() {
  const active = useStore((state) => state.session.isActive);
  const retainHistory = useStore((state) => state.settings.retainSessionHistory);
  const recordAudio = useStore((state) => state.settings.recordSessionAudio);
  const saveSettings = useStore((state) => state.saveSettings);
  const [archive, setArchive] = useState<SessionArchiveState>();
  const [readError, setReadError] = useState(false);
  const [busy, setBusy] = useState(false);
  const [feedback, setFeedback] = useState<"saved" | "cleared" | "error" | null>(null);
  const monitor = useRef<ReturnType<typeof monitorSessionArchive> | null>(null);
  const operation = useRef(false);
  const lifetime = useRef(0);

  useEffect(() => {
    lifetime.current += 1;
    if (isTauri) {
      monitor.current = monitorSessionArchive(
        sessionArchiveState,
        (state) => {
          setArchive(state);
          setReadError(false);
        },
        () => setReadError(true),
      );
    }
    return () => {
      lifetime.current += 1;
      monitor.current?.stop();
      monitor.current = null;
    };
  }, []);

  useEffect(() => {
    monitor.current?.refresh();
  }, [active, retainHistory, recordAudio]);

  async function perform(action: () => Promise<"saved" | "cleared" | null>) {
    if (!isTauri || operation.current || useStore.getState().session.isActive) return;
    const currentLifetime = lifetime.current;
    operation.current = true;
    setBusy(true);
    setFeedback(null);
    try {
      const result = await action();
      if (lifetime.current === currentLifetime) setFeedback(result);
    } catch {
      if (lifetime.current === currentLifetime) setFeedback("error");
    } finally {
      operation.current = false;
      if (lifetime.current === currentLifetime) {
        setBusy(false);
        monitor.current?.refresh();
      }
    }
  }

  function changeSetting(draft: SettingsDraft) {
    void perform(async () => {
      await saveSettings(draft);
      return null;
    });
  }

  const disabled = !isTauri || active || busy;
  const hasTranscript = (archive?.transcriptCount ?? 0) > 0;
  const hasAudio = (archive?.audioBytes ?? 0) > 0;

  return (
    <SettingsSection id="session-export" title={I18N.settings.sessionExportTitle}>
      <p className="settings-help session-export__privacy">
        {I18N.settings.sessionExportPrivacy}
      </p>
      <SettingsRow
        label={I18N.settings.retainSessionHistory}
        description={I18N.settings.retainSessionHistoryHelp}
        align="start"
      >
        <Switch
          checked={retainHistory}
          disabled={disabled}
          aria-label={I18N.settings.retainSessionHistory}
          onChange={(retainSessionHistory) => changeSetting({ retainSessionHistory })}
        />
      </SettingsRow>
      <div className="settings-divider" />
      <SettingsRow
        label={I18N.settings.recordSessionAudio}
        description={I18N.settings.recordSessionAudioHelp}
        align="start"
      >
        <Switch
          checked={recordAudio}
          disabled={disabled}
          aria-label={I18N.settings.recordSessionAudio}
          onChange={(recordSessionAudio) => changeSetting({ recordSessionAudio })}
        />
      </SettingsRow>
      <div className="session-export__details">
        <p className="settings-help">{I18N.settings.sessionArchiveLimits}</p>
        {active && <p className="settings-help">{I18N.settings.sessionExportActive}</p>}
        {active && recordAudio && (
          <InlineFeedback tone="info">{I18N.settings.sessionAudioEnabled}</InlineFeedback>
        )}
        {archive && (
          <p className="settings-help">
            {I18N.settings.sessionArchiveSummary(
              archive.transcriptCount,
              (archive.audioBytes / 1_048_576).toFixed(1),
            )}
          </p>
        )}
        {archive?.transcriptLimited && (
          <InlineFeedback tone="info">{I18N.settings.sessionTranscriptLimit}</InlineFeedback>
        )}
        {archive?.audioLimited && (
          <InlineFeedback tone="info">{I18N.settings.sessionAudioLimit}</InlineFeedback>
        )}
        <div className="session-export__actions">
          <button
            type="button"
            className="settings-button settings-button--quiet"
            disabled={disabled || readError || !hasTranscript}
            onClick={() => void perform(async () => (await sessionExport("transcript")) ? "saved" : null)}
          >
            {I18N.settings.exportTranscript}
          </button>
          <button
            type="button"
            className="settings-button settings-button--quiet"
            disabled={disabled || readError || !hasAudio}
            onClick={() => void perform(async () => (await sessionExport("audio")) ? "saved" : null)}
          >
            {I18N.settings.exportAudio}
          </button>
          <button
            type="button"
            className="settings-button settings-button--quiet"
            disabled={disabled || readError || (!hasTranscript && !hasAudio)}
            onClick={() => void perform(async () => {
              await sessionArchiveClear();
              return "cleared";
            })}
          >
            {I18N.settings.clearSessionArchive}
          </button>
        </div>
        {!isTauri && <p className="settings-help">{I18N.settings.sessionExportNativeOnly}</p>}
        {readError && <InlineFeedback tone="error">{I18N.settings.sessionArchiveReadFailed}</InlineFeedback>}
        {feedback && (
          <InlineFeedback tone={feedback === "error" ? "error" : "success"}>
            {feedback === "error" ? I18N.settings.sessionExportFailed
              : feedback === "saved" ? I18N.settings.sessionExportSaved
              : I18N.settings.sessionArchiveCleared}
          </InlineFeedback>
        )}
      </div>
    </SettingsSection>
  );
}
