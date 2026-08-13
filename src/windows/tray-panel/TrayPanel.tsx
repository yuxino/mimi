import { useEffect } from "react";
import { Icon } from "../../components/Icon";
import { Switch } from "../../components/Switch";
import { I18N } from "../../lib/i18n";
import { isTauri } from "../../lib/ipc";
import { useStore } from "../../lib/store";
import {
  SOURCE_LANGUAGE_QUICK_CASES,
  TRANSLATION_MODE_DISPLAY_NAMES,
  targetLanguageTranslatesAudio,
  type SessionStateEvent,
  type SettingsDraft,
  type SettingsSnapshot,
  type SourceLanguage,
} from "../../lib/types";
import { sourceLanguageButtonTitle } from "../overlay/overlayModel";

const GREEN = "#30D158";
const RED = "#FF453A";
const ORANGE = "#FF9F0A";
const SECONDARY = "rgba(255,255,255,0.55)";

/** Menu-bar style control panel; 1:1 port of `MenuBarView.swift`. */
export function TrayPanel() {
  const session = useStore((state) => state.session);
  const settings = useStore((state) => state.settings);
  const start = useStore((state) => state.start);
  const stop = useStore((state) => state.stop);
  const switchSourceLanguage = useStore((state) => state.switchSourceLanguage);
  const setOverlayLocked = useStore((state) => state.setOverlayLocked);
  const saveSettings = useStore((state) => state.saveSettings);
  const showOverlay = useStore((state) => state.showOverlay);
  const clearSubtitles = useStore((state) => state.clearSubtitles);
  const showSettings = useStore((state) => state.showSettings);
  const quit = useStore((state) => state.quit);

  const isChangingSession =
    session.status.kind === "connecting" || session.status.kind === "stopping";
  const isListening = session.status.kind === "listening";

  useEffect(() => {
    prepareLanguagePreferences(settings, saveSettings);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const handleLiveSubtitles = (checked: boolean) => {
    if (checked) {
      prepareLanguagePreferences(settings, saveSettings);
      void start();
    } else {
      void stop();
    }
  };

  const handleLock = (checked: boolean) => {
    void setOverlayLocked(checked);
    void saveSettings({ isOverlayLocked: checked }).catch(() => {});
  };

  const pickerValue =
    settings.sourceLanguage === "auto" ? "ja" : settings.sourceLanguage;

  const panel = (
    <div
      style={{
        width: 290,
        padding: 14,
        borderRadius: 14,
        background: "#1e1e1e",
        border: "1px solid rgba(255,255,255,0.08)",
        boxShadow: "0 12px 32px rgba(0,0,0,0.45)",
      }}
    >
      <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
        <div style={{ display: "flex", gap: 10, alignItems: "center" }}>
          <Icon
            name="ear-waveform"
            style={{ fontSize: 22, color: "rgba(255,255,255,0.85)" }}
          />
          <div style={{ display: "flex", flexDirection: "column", gap: 2 }}>
            <span style={{ fontSize: 16, fontWeight: 600, color: "#ffffff" }}>
              {I18N.tray.appName}
            </span>
            <span
              style={{
                fontSize: 12,
                color: statusColor(session),
                lineHeight: 1.3,
              }}
            >
              {statusText(session, settings)}
            </span>
          </div>
        </div>

        <Divider />

        <ToggleRow
          icon={session.isActive ? "waveform" : "waveform-slash"}
          label={I18N.tray.liveSubtitles}
          hint={shortcutHint()}
          checked={session.isActive}
          onChange={handleLiveSubtitles}
        />

        <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
          <span style={{ fontSize: 13, color: "rgba(255,255,255,0.85)" }}>
            {I18N.tray.sourceLanguage}
          </span>
          <select
            value={pickerValue}
            disabled={isChangingSession || session.isPaused}
            aria-label={I18N.tray.sourceLanguage}
            onChange={(event) =>
              void switchSourceLanguage(event.target.value as SourceLanguage)
            }
            style={{
              height: 28,
              padding: "0 8px",
              borderRadius: 6,
              background: "rgba(255,255,255,0.06)",
              color: "rgba(255,255,255,0.9)",
              border: "1px solid rgba(255,255,255,0.12)",
              fontSize: 13,
              opacity: isChangingSession || session.isPaused ? 0.5 : 1,
            }}
          >
            {SOURCE_LANGUAGE_QUICK_CASES.map((language) => (
              <option key={language} value={language}>
                {sourceLanguageButtonTitle(language)}
              </option>
            ))}
          </select>
        </div>

        <div
          style={{
            display: "flex",
            gap: 6,
            alignItems: "center",
            fontSize: 12,
            color: SECONDARY,
          }}
        >
          <Icon
            name={
              targetLanguageTranslatesAudio(settings.targetLanguage)
                ? "sparkles"
                : "text-quote"
            }
            style={{ fontSize: 12 }}
          />
          {targetLanguageTranslatesAudio(settings.targetLanguage)
            ? `${TRANSLATION_MODE_DISPLAY_NAMES[settings.translationMode]}翻译`
            : I18N.tray.originalOnly}
        </div>

        <ToggleRow
          label={I18N.tray.lockPosition}
          checked={settings.isOverlayLocked}
          onChange={handleLock}
        />

        <TrayButton
          label={I18N.tray.showSubtitleWindow}
          disabled={!isListening}
          onClick={() => void showOverlay()}
        />
        <TrayButton
          label={I18N.tray.clearSubtitles}
          onClick={() => void clearSubtitles()}
        />

        <Divider />

        <TrayButton
          label={I18N.tray.settings}
          icon="gear"
          onClick={() => void showSettings()}
        />
        <TrayButton label={I18N.tray.quit} onClick={() => void quit()} />
      </div>
    </div>
  );

  if (isTauri) {
    return <div className="h-full w-full bg-transparent p-0">{panel}</div>;
  }
  return (
    <div className="flex h-screen w-screen items-start justify-center pt-8">
      {panel}
    </div>
  );
}

function ToggleRow({
  icon,
  label,
  hint,
  checked,
  onChange,
}: {
  icon?: "waveform" | "waveform-slash";
  label: string;
  hint?: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
}) {
  return (
    <div style={{ display: "flex", gap: 10, alignItems: "center" }}>
      {icon && <Icon name={icon} style={{ fontSize: 14 }} />}
      <span style={{ flex: 1, fontSize: 13, color: "rgba(255,255,255,0.9)" }}>
        {label}
      </span>
      {hint && <span style={{ fontSize: 11, color: SECONDARY }}>{hint}</span>}
      <Switch checked={checked} onChange={onChange} aria-label={label} />
    </div>
  );
}

function TrayButton({
  label,
  icon,
  disabled = false,
  onClick,
}: {
  label: string;
  icon?: "gear";
  disabled?: boolean;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      disabled={disabled}
      onClick={onClick}
      className="flex w-full items-center"
      style={{
        gap: 8,
        height: 26,
        padding: "0 2px",
        border: "none",
        background: "transparent",
        color: disabled ? "rgba(255,255,255,0.3)" : "rgba(255,255,255,0.9)",
        fontSize: 13,
        cursor: disabled ? "default" : "pointer",
      }}
    >
      {icon && <Icon name={icon} style={{ fontSize: 14 }} />}
      {label}
    </button>
  );
}

function Divider() {
  return <div style={{ height: 1, background: "rgba(255,255,255,0.1)" }} />;
}

function statusText(
  session: SessionStateEvent,
  settings: SettingsSnapshot,
): string {
  if (session.isPaused) return I18N.tray.paused;

  switch (session.status.kind) {
    case "idle":
      return settings.workspaceID.trim() === "" || !settings.hasAPIKey
        ? I18N.tray.setupRequired
        : I18N.tray.ready;
    case "connecting":
      return I18N.tray.connecting;
    case "listening":
      return I18N.tray.listening;
    case "stopping":
      return I18N.tray.stopping;
    case "error":
      return session.status.message;
  }
}

function statusColor(session: SessionStateEvent): string {
  if (session.isPaused) return ORANGE;
  switch (session.status.kind) {
    case "listening":
      return GREEN;
    case "error":
      return RED;
    default:
      return SECONDARY;
  }
}

function shortcutHint(): string {
  const isMac = navigator.platform.toLowerCase().includes("mac");
  return isMac ? "⌘⇧ Space" : "Ctrl+Shift+Space";
}

function prepareLanguagePreferences(
  settings: SettingsSnapshot,
  saveSettings: (draft: SettingsDraft) => Promise<void>,
): void {
  let source = settings.sourceLanguage;
  let target = settings.targetLanguage;
  if (source === "auto") source = "ja";
  if (source === "zh") target = "original";
  if (source !== settings.sourceLanguage || target !== settings.targetLanguage) {
    void saveSettings({ sourceLanguage: source, targetLanguage: target }).catch(
      () => {},
    );
  }
}
