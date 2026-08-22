import { Icon } from "../../components/Icon";
import { Switch } from "../../components/Switch";
import { I18N, setStoredUiLanguage, type UiLanguage } from "../../lib/i18n";
import { isTauri } from "../../lib/ipc";
import {
  activeServiceProfile,
  effectiveTranslationModeForSettings,
  sourceLanguagesForSettings,
} from "../../lib/providerCapabilities";
import { useStore } from "../../lib/store";
import {
  TRANSLATION_MODE_DISPLAY_NAMES,
  detectedLanguageDisplayName,
  targetLanguageTranslatesAudio,
  type SessionStateEvent,
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
  // Narrow selectors: this window never shows subtitle text, so subscribing
  // to the whole session object would re-render it on every streaming event.
  const sessionStatus = useStore((state) => state.session.status);
  const isPaused = useStore((state) => state.session.isPaused);
  const detectedLanguage = useStore((state) => state.session.detectedLanguage);
  const settings = useStore((state) => state.settings);
  const switchSourceLanguage = useStore((state) => state.switchSourceLanguage);
  const setOverlayLocked = useStore((state) => state.setOverlayLocked);
  const saveSettings = useStore((state) => state.saveSettings);
  const showOverlay = useStore((state) => state.showOverlay);
  const clearSubtitles = useStore((state) => state.clearSubtitles);
  const showSettings = useStore((state) => state.showSettings);
  const quit = useStore((state) => state.quit);

  const isChangingSession =
    sessionStatus.kind === "connecting" || sessionStatus.kind === "stopping";
  const isListening = sessionStatus.kind === "listening";
  const sourceLanguages = sourceLanguagesForSettings(settings);
  const effectiveMode = effectiveTranslationModeForSettings(settings);

  const handleLock = (checked: boolean) => {
    void setOverlayLocked(checked).catch(() => {});
  };

  const handleUiLanguage = (language: UiLanguage) => {
    void saveSettings({ uiLanguage: language })
      .then(() => {
        setStoredUiLanguage(language);
        window.location.reload();
      })
      .catch(() => {});
  };

  const pickerValue = settings.sourceLanguage;

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
        <div style={{ display: "flex", flexDirection: "column", gap: 2 }}>
          <span style={{ fontSize: 16, fontWeight: 600, color: "#ffffff" }}>
            {I18N.tray.appName}
          </span>
          <span
            style={{
              fontSize: 12,
              color: statusColor(sessionStatus, isPaused),
              lineHeight: 1.3,
            }}
          >
            {statusText(sessionStatus, isPaused, settings)}
          </span>
        </div>

        <Divider />

        <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
          <span style={{ fontSize: 13, color: "rgba(255,255,255,0.85)" }}>
            {I18N.tray.sourceLanguage}
          </span>
          <select
            value={pickerValue}
            disabled={
              isChangingSession || isPaused || sourceLanguages.length === 1
            }
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
              opacity:
                isChangingSession || isPaused || sourceLanguages.length === 1
                  ? 0.5
                  : 1,
            }}
          >
            {sourceLanguages.map((language) => (
              <option key={language} value={language}>
                {sourceLanguageButtonTitle(language)}
              </option>
            ))}
          </select>
        </div>

        <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
          <span style={{ fontSize: 13, color: "rgba(255,255,255,0.85)" }}>
            {I18N.settings.appLanguage}
          </span>
          <select
            value={settings.uiLanguage ?? "system"}
            aria-label={I18N.settings.appLanguage}
            onChange={(event) =>
              handleUiLanguage(event.target.value as UiLanguage)
            }
            style={{
              height: 28,
              padding: "0 8px",
              borderRadius: 6,
              background: "rgba(255,255,255,0.06)",
              color: "rgba(255,255,255,0.9)",
              border: "1px solid rgba(255,255,255,0.12)",
              fontSize: 13,
            }}
          >
            <option value="system">{I18N.settings.systemLanguage}</option>
            <option value="zh">{I18N.settings.chinese}</option>
            <option value="en">{I18N.settings.english}</option>
            <option value="ja">{I18N.settings.japanese}</option>
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
          {(() => {
            const detected =
              settings.sourceLanguage === "auto" && detectedLanguage
                ? `${detectedLanguageDisplayName(detectedLanguage)} · `
                : "";
            return targetLanguageTranslatesAudio(settings.targetLanguage)
              ? `${detected}${TRANSLATION_MODE_DISPLAY_NAMES[effectiveMode]}${I18N.overlay.translationSuffix}`
              : `${detected}${I18N.tray.originalOnly}`;
          })()}
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
  label,
  checked,
  onChange,
}: {
  label: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
}) {
  return (
    <div style={{ display: "flex", gap: 10, alignItems: "center" }}>
      <span style={{ flex: 1, fontSize: 13, color: "rgba(255,255,255,0.9)" }}>
        {label}
      </span>
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
      className="ux-hover ux-hover-bg flex w-full items-center"
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
  status: SessionStateEvent["status"],
  isPaused: boolean,
  settings: SettingsSnapshot,
): string {
  if (isPaused) return I18N.tray.paused;

  switch (status.kind) {
    case "idle":
      return activeServiceProfile(settings)?.credentialState !== "present"
        ? I18N.tray.setupRequired
        : I18N.tray.ready;
    case "connecting":
      return I18N.tray.connecting;
    case "listening":
      return I18N.tray.listening;
    case "stopping":
      return I18N.tray.stopping;
    case "error":
      return status.message;
  }
}

function statusColor(
  status: SessionStateEvent["status"],
  isPaused: boolean,
): string {
  if (isPaused) return ORANGE;
  switch (status.kind) {
    case "listening":
      return GREEN;
    case "error":
      return RED;
    default:
      return SECONDARY;
  }
}
