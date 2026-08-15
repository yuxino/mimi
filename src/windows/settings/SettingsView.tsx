import { useEffect, useState } from "react";
import type { CSSProperties, ReactNode } from "react";
import { Icon, type IconName } from "../../components/Icon";
import { Switch } from "../../components/Switch";
import {
  I18N,
  credentialLoadErrorMessage,
  sessionConnectingText,
} from "../../lib/i18n";
import { useStore } from "../../lib/store";
import {
  SOURCE_LANGUAGE_DISPLAY_NAMES,
  SOURCE_LANGUAGE_QUICK_CASES,
  TARGET_LANGUAGE_CASES,
  TARGET_LANGUAGE_DISPLAY_NAMES,
  TRANSLATION_MODE_CASES,
  TRANSLATION_MODE_DISPLAY_NAMES,
  type SessionStateEvent,
  type SettingsDraft,
  type SettingsSnapshot,
  type SourceLanguage,
  type TargetLanguage,
  type TranslationMode,
} from "../../lib/types";
import { useReducedMotion } from "../overlay/animation";
import { sourceLanguageButtonTitle } from "../overlay/overlayModel";

const ACCENT = "#3478F0";
const GREEN = "#30D158";
const RED = "#FF453A";
const ORANGE = "#FF9F0A";
const SECONDARY = "rgba(255,255,255,0.55)";

/** Main settings window; 1:1 port of `SettingsView.swift`. */
export function SettingsView() {
  // Narrow selectors: the settings window never shows subtitle text, so
  // subscribing to the whole session object would re-render it on every
  // streaming event.
  const sessionStatus = useStore((state) => state.session.status);
  const sessionIsPaused = useStore((state) => state.session.isPaused);
  const sessionIsActive = useStore((state) => state.session.isActive);
  const settings = useStore((state) => state.settings);
  const start = useStore((state) => state.start);
  const stop = useStore((state) => state.stop);
  const switchSourceLanguage = useStore((state) => state.switchSourceLanguage);
  const saveSettings = useStore((state) => state.saveSettings);
  const setOverlayLocked = useStore((state) => state.setOverlayLocked);

  const [showsServiceSettings, setShowsServiceSettings] = useState(
    () => !settings.hasAPIKey || settings.credentialLoadError !== null,
  );
  const [credentialMessage, setCredentialMessage] = useState<string | null>(
    null,
  );
  const [credentialMessageIsError, setCredentialMessageIsError] =
    useState(false);
  const [previousAPIKey, setPreviousAPIKey] = useState(settings.apiKey);
  const [apiKey, setApiKey] = useState(settings.apiKey);

  // Keep the editable field in sync with settings loaded asynchronously
  // after mount (React's "adjust state during render" pattern).
  if (settings.apiKey !== previousAPIKey) {
    setPreviousAPIKey(settings.apiKey);
    setApiKey(settings.apiKey);
  }

  const isActive = sessionIsActive;
  const isChangingSession =
    sessionStatus.kind === "connecting" || sessionStatus.kind === "stopping";
  const isListening = sessionStatus.kind === "listening";

  useEffect(() => {
    const draft = listeningPreferencesDraft(settings);
    if (draft) void saveSettings(draft).catch(() => {});
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const startListening = () => {
    const draft = listeningPreferencesDraft(settings);
    if (draft) void saveSettings(draft).catch(() => {});
    void setOverlayLocked(settings.isOverlayLocked);
    void start();
  };

  const saveCredentials = async () => {
    try {
      await saveSettings({ apiKey });
      setCredentialMessage(I18N.settings.credentialsSaved);
      setCredentialMessageIsError(false);
    } catch (error) {
      setCredentialMessage(errorMessage(error));
      setCredentialMessageIsError(true);
    }
  };

  const credentialsAreConfigured = settings.hasAPIKey;

  return (
    <div className="h-full w-full overflow-y-auto">
      <div
        style={{
          display: "flex",
          flexDirection: "column",
          gap: 16,
          padding: 20,
        }}
      >
        <Card>
          <div style={{ display: "flex", gap: 14, alignItems: "center" }}>
            <div
              style={{
                width: 42,
                height: 42,
                borderRadius: "50%",
                background: "rgba(52,120,240,0.12)",
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                color: ACCENT,
                flexShrink: 0,
              }}
            >
              <Icon name="captions-bubble" style={{ fontSize: 18 }} />
            </div>

            <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
              <span style={{ fontSize: 17, fontWeight: 600, color: "#fff" }}>
                {I18N.settings.sessionTitle}
              </span>
              <div style={{ display: "flex", gap: 7, alignItems: "center" }}>
                <SettingsStatusIndicator
                  color={sessionStatusColor(sessionStatus, sessionIsPaused)}
                  isActive={isListening && !sessionIsPaused}
                />
                <span
                  style={{
                    fontSize: 12.5,
                    fontWeight: 500,
                    color: sessionStatusColor(sessionStatus, sessionIsPaused),
                    lineHeight: 1.3,
                  }}
                >
                  {sessionStatusText(sessionStatus, sessionIsPaused, settings)}
                </span>
              </div>
            </div>

            <span style={{ flex: 1, minWidth: 12 }} />

            <button
              type="button"
              disabled={sessionStatus.kind === "stopping"}
              onClick={() => (isActive ? void stop() : startListening())}
              className="ux-hover flex items-center"
              style={{
                gap: 6,
                minWidth: 62,
                height: 34,
                padding: "0 16px",
                borderRadius: 8,
                border: "none",
                background: isActive ? RED : ACCENT,
                color: "#fff",
                fontSize: 14,
                fontWeight: 600,
                cursor:
                  sessionStatus.kind === "stopping" ? "default" : "pointer",
                opacity: sessionStatus.kind === "stopping" ? 0.5 : 1,
              }}
            >
              <Icon name={isActive ? "stop" : "play"} style={{ fontSize: 12 }} />
              {isActive ? I18N.settings.stop : I18N.settings.start}
            </button>
          </div>
        </Card>

        <Card>
          <div style={{ display: "flex", flexDirection: "column", gap: 15 }}>
            <div style={{ display: "flex", alignItems: "center" }}>
              <span style={{ fontSize: 16, fontWeight: 600, color: "#fff" }}>
                {I18N.settings.subtitleTitle}
              </span>
              <span style={{ flex: 1 }} />
              <span
                className="ux-hover flex items-center"
                style={{
                  gap: 5,
                  fontSize: 11,
                  fontWeight: 600,
                  color: ACCENT,
                  padding: "0 9px",
                  height: 24,
                  borderRadius: 999,
                  background: "rgba(52,120,240,0.1)",
                }}
              >
                <Icon
                  name={translationBadgeIcon(settings)}
                  style={{ fontSize: 11 }}
                />
                {translationBadgeText(settings)}
              </span>
            </div>

            <div style={{ display: "flex", flexDirection: "column", gap: 9 }}>
              <SectionLabel>{I18N.settings.sourceLanguage}</SectionLabel>
              <div style={{ display: "flex", gap: 8 }}>
                {SOURCE_LANGUAGE_QUICK_CASES.map((language) => (
                  <SourceLanguageButton
                    key={language}
                    language={language}
                    selected={settings.sourceLanguage === language}
                    disabled={isChangingSession}
                    onSelect={() => void switchSourceLanguage(language)}
                  />
                ))}
              </div>
              <CaptionText>{sourceLanguageHelp(sessionStatus, settings)}</CaptionText>
            </div>

            <Divider />

            <SettingsRow label={I18N.settings.translateTo}>
              <Select
                value={settings.targetLanguage}
                disabled={isActive || settings.sourceLanguage === "zh"}
                onChange={(value) =>
                  void saveSettings({ targetLanguage: value as TargetLanguage })
                }
                options={TARGET_LANGUAGE_CASES.map((language) => ({
                  value: language,
                  label: TARGET_LANGUAGE_DISPLAY_NAMES[language],
                }))}
              />
            </SettingsRow>

            <Divider />

            <SettingsRow label={I18N.settings.translationMode}>
              <Select
                value={settings.translationMode}
                disabled={isActive}
                onChange={(value) =>
                  void saveSettings({
                    translationMode: value as TranslationMode,
                  })
                }
                options={TRANSLATION_MODE_CASES.map((mode) => ({
                  value: mode,
                  label: TRANSLATION_MODE_DISPLAY_NAMES[mode],
                }))}
              />
            </SettingsRow>

            <CaptionText>
              {translationModeHelp(settings.translationMode)}
            </CaptionText>

            <Divider />

            <SettingsRow label={I18N.settings.fontSize}>
              <div style={{ display: "flex", gap: 10, alignItems: "center" }}>
                <span style={{ fontSize: 12, color: SECONDARY }}>A</span>
                <input
                  type="range"
                  min={14}
                  max={20}
                  step={1}
                  value={settings.fontSize}
                  onChange={(event) =>
                    void saveSettings({ fontSize: Number(event.target.value) })
                  }
                  style={{ width: 178, accentColor: ACCENT }}
                  aria-label={I18N.settings.fontSize}
                />
                <span
                  style={{
                    width: 24,
                    textAlign: "right",
                    color: "rgba(255,255,255,0.85)",
                    fontVariantNumeric: "tabular-nums",
                  }}
                >
                  {Math.round(settings.fontSize)}
                </span>
              </div>
            </SettingsRow>

            <Divider />

            <SettingsRow label={I18N.settings.lockPosition}>
              <Switch
                checked={settings.isOverlayLocked}
                onChange={(checked) => {
                  void setOverlayLocked(checked);
                  void saveSettings({ isOverlayLocked: checked }).catch(
                    () => {},
                  );
                }}
                aria-label={I18N.settings.lockPosition}
              />
            </SettingsRow>

            <CaptionText>{I18N.settings.lockHelp}</CaptionText>
          </div>
        </Card>

        <Card>
          <button
            type="button"
            onClick={() => setShowsServiceSettings((value) => !value)}
            className="ux-hover flex w-full items-center"
            style={{
              gap: 10,
              background: "none",
              border: "none",
              padding: 0,
              cursor: "pointer",
            }}
          >
            <Icon name="key" style={{ fontSize: 16, color: SECONDARY }} />
            <span style={{ fontSize: 16, fontWeight: 600, color: "#fff" }}>
              {I18N.settings.serviceSettings}
            </span>
            <span style={{ flex: 1 }} />
            {credentialsAreConfigured && (
              <span style={{ fontSize: 12, color: SECONDARY }}>
                {I18N.settings.configured}
              </span>
            )}
            <Icon
              name={showsServiceSettings ? "chevron-up" : "chevron-down"}
              style={{ fontSize: 12, color: SECONDARY }}
            />
          </button>

          {showsServiceSettings && (
            <div
              style={{
                display: "flex",
                flexDirection: "column",
                gap: 12,
                paddingTop: 8,
              }}
            >
              <Divider />

              <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
                <SectionLabel>{I18N.settings.apiKey}</SectionLabel>
                <input
                  type="password"
                  value={apiKey}
                  placeholder={I18N.settings.apiKeyPlaceholder}
                  onChange={(event) => {
                    setApiKey(event.target.value);
                    setCredentialMessage(null);
                  }}
                  style={textFieldStyle}
                />
              </div>

              <div style={{ display: "flex", gap: 10, alignItems: "center" }}>
                <CaptionText>{I18N.settings.credentialNote}</CaptionText>
                <span style={{ flex: 1 }} />
                <button
                  type="button"
                  onClick={() => void saveCredentials()}
                  className="ux-hover"
                  style={{
                    height: 28,
                    padding: "0 14px",
                    borderRadius: 6,
                    border: "none",
                    background: ACCENT,
                    color: "#fff",
                    fontSize: 13,
                    fontWeight: 500,
                    cursor: "pointer",
                  }}
                >
                  {I18N.settings.saveCredentials}
                </button>
              </div>

              {settings.credentialLoadError && (
                <CredentialFeedback
                  message={credentialLoadErrorMessage(
                    settings.credentialLoadError,
                  )}
                  isError
                />
              )}

              {credentialMessage && (
                <CredentialFeedback
                  message={credentialMessage}
                  isError={credentialMessageIsError}
                />
              )}
            </div>
          )}
        </Card>
      </div>
    </div>
  );
}

function Card({ children }: { children: ReactNode }) {
  return (
    <div
      style={{
        padding: 16,
        borderRadius: 14,
        background: "rgba(255,255,255,0.06)",
        border: "0.5px solid rgba(255,255,255,0.05)",
      }}
    >
      {children}
    </div>
  );
}

function SectionLabel({ children }: { children: string }) {
  return (
    <span style={{ fontSize: 12, fontWeight: 500, color: SECONDARY }}>
      {children}
    </span>
  );
}

function CaptionText({ children }: { children: string }) {
  return (
    <span style={{ fontSize: 12, color: SECONDARY, lineHeight: 1.4 }}>
      {children}
    </span>
  );
}

function Divider() {
  return <div style={{ height: 1, background: "rgba(255,255,255,0.08)" }} />;
}

function SettingsRow({
  label,
  children,
}: {
  label: string;
  children: ReactNode;
}) {
  return (
    <div
      style={{
        display: "flex",
        gap: 16,
        alignItems: "center",
        minHeight: 30,
      }}
    >
      <span
        style={{ fontSize: 13.5, fontWeight: 500, color: "rgba(255,255,255,0.9)" }}
      >
        {label}
      </span>
      <span style={{ flex: 1 }} />
      {children}
    </div>
  );
}

function Select({
  value,
  disabled,
  onChange,
  options,
}: {
  value: string;
  disabled: boolean;
  onChange: (value: string) => void;
  options: Array<{ value: string; label: string }>;
}) {
  return (
    <select
      value={value}
      disabled={disabled}
      onChange={(event) => onChange(event.target.value)}
      style={{
        width: 168,
        height: 28,
        padding: "0 8px",
        borderRadius: 6,
        background: "rgba(255,255,255,0.06)",
        color: "rgba(255,255,255,0.9)",
        border: "1px solid rgba(255,255,255,0.12)",
        fontSize: 13,
        opacity: disabled ? 0.5 : 1,
      }}
    >
      {options.map((option) => (
        <option key={option.value} value={option.value}>
          {option.label}
        </option>
      ))}
    </select>
  );
}

function SourceLanguageButton({
  language,
  selected,
  disabled,
  onSelect,
}: {
  language: SourceLanguage;
  selected: boolean;
  disabled: boolean;
  onSelect: () => void;
}) {
  return (
    <button
      type="button"
      disabled={disabled}
      onClick={onSelect}
      className="ux-hover flex flex-1 items-center justify-center"
      title={sourceLanguageButtonHelp(language)}
      style={{
        gap: 6,
        height: 34,
        borderRadius: 9,
        border: `0.75px solid ${
          selected ? "rgba(52,120,240,0.34)" : "rgba(255,255,255,0.07)"
        }`,
        background: selected
          ? "rgba(52,120,240,0.12)"
          : "rgba(255,255,255,0.035)",
        color: selected ? ACCENT : "rgba(255,255,255,0.78)",
        fontSize: 13,
        fontWeight: selected ? 600 : 500,
        cursor: disabled ? "default" : "pointer",
        opacity: disabled ? 0.5 : 1,
      }}
    >
      {sourceLanguageButtonTitle(language)}
      {selected && <Icon name="checkmark-circle" style={{ fontSize: 11 }} />}
    </button>
  );
}

function CredentialFeedback({
  message,
  isError,
}: {
  message: string;
  isError: boolean;
}) {
  return (
    <div
      className="ux-hover flex items-center"
      style={{ gap: 6, fontSize: 12, color: isError ? RED : GREEN }}
    >
      <Icon
        name={isError ? "exclamation-triangle" : "checkmark-circle"}
        style={{ fontSize: 12 }}
      />
      {message}
    </div>
  );
}

function SettingsStatusIndicator({
  color,
  isActive,
}: {
  color: string;
  isActive: boolean;
}) {
  const reduceMotion = useReducedMotion();
  const [expanded, setExpanded] = useState(false);

  useEffect(() => {
    if (!isActive || reduceMotion) return;
    const id = setInterval(() => setExpanded((value) => !value), 400);
    return () => clearInterval(id);
  }, [isActive, reduceMotion]);

  const pulseExpanded = isActive && !reduceMotion && expanded;

  return (
    <div style={{ width: 16, height: 16, position: "relative" }}>
      <div
        style={{
          position: "absolute",
          inset: 0,
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
        }}
      >
        <div
          style={{
            width: pulseExpanded ? 16 : 8,
            height: pulseExpanded ? 16 : 8,
            borderRadius: "50%",
            background: color,
            opacity: pulseExpanded ? 0.16 : 0,
            transition: reduceMotion ? "none" : "all 550ms ease-in-out",
          }}
        />
      </div>
      <div
        style={{
          position: "absolute",
          inset: 0,
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
        }}
      >
        <div
          style={{ width: 7, height: 7, borderRadius: "50%", background: color }}
        />
      </div>
    </div>
  );
}

const textFieldStyle: CSSProperties = {
  height: 30,
  padding: "0 10px",
  borderRadius: 6,
  background: "rgba(255,255,255,0.06)",
  color: "rgba(255,255,255,0.9)",
  border: "1px solid rgba(255,255,255,0.12)",
  fontSize: 13,
};

function sessionStatusText(
  status: SessionStateEvent["status"],
  isPaused: boolean,
  settings: SettingsSnapshot,
): string {
  if (isPaused) return I18N.overlay.paused;
  switch (status.kind) {
    case "idle":
      return I18N.settings.sessionReady;
    case "connecting":
      return sessionConnectingText(
        TRANSLATION_MODE_DISPLAY_NAMES[settings.translationMode],
      );
    case "listening":
      return I18N.settings.sessionListening;
    case "stopping":
      return I18N.settings.sessionStopping;
    case "error":
      return I18N.settings.sessionError;
  }
}

function sessionStatusColor(
  status: SessionStateEvent["status"],
  isPaused: boolean,
): string {
  if (isPaused) return ORANGE;
  switch (status.kind) {
    case "listening":
      return GREEN;
    case "connecting":
      return ACCENT;
    case "error":
      return RED;
    default:
      return SECONDARY;
  }
}

function translationBadgeText(settings: SettingsSnapshot): string {
  return settings.sourceLanguage === "zh" &&
    settings.targetLanguage === "original"
    ? I18N.settings.originalOnlyBadge
    : `${TRANSLATION_MODE_DISPLAY_NAMES[settings.translationMode]}${I18N.overlay.translationSuffix}`;
}

function translationBadgeIcon(settings: SettingsSnapshot): IconName {
  return settings.sourceLanguage === "zh" &&
    settings.targetLanguage === "original"
    ? "text-quote"
    : "sparkles";
}

function translationModeHelp(mode: TranslationMode): string {
  switch (mode) {
    case "turbo":
      return I18N.modes.turboHelp;
    case "highQuality":
      return I18N.modes.highQualityHelp;
    case "lowLatency":
      return I18N.modes.lowLatencyHelp;
  }
}

function sourceLanguageHelp(
  status: SessionStateEvent["status"],
  settings: SettingsSnapshot,
): string {
  if (settings.sourceLanguage === "zh") {
    return status.kind === "listening"
      ? I18N.settings.recognizingChineseListening
      : I18N.settings.recognizingChineseIdle;
  }
  if (status.kind === "listening") {
    return I18N.settings.sourceHelpReconnecting;
  }
  return I18N.settings.sourceHelpIdle;
}

function sourceLanguageButtonHelp(language: SourceLanguage): string {
  return language === "zh"
    ? I18N.settings.switchToChineseHelp
    : I18N.settings.switchToLanguageHelp(
        SOURCE_LANGUAGE_DISPLAY_NAMES[language],
      );
}

function errorMessage(error: unknown): string {
  if (error instanceof Error) return error.message;
  return String(error);
}

function listeningPreferencesDraft(
  settings: SettingsSnapshot,
): SettingsDraft | null {
  // Automatic source is the user's persistent choice now (the backend
  // resolves the engine language); only the Chinese-source original-target
  // rule is applied here.
  const source = settings.sourceLanguage;
  let target = settings.targetLanguage;
  if (source === "zh") target = "original";
  if (target !== settings.targetLanguage) {
    return { sourceLanguage: source, targetLanguage: target };
  }
  return null;
}
