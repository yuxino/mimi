import { Icon, type IconName } from "../../components/Icon";
import { Switch } from "../../components/Switch";
import {
  I18N,
  providerDisplayName,
  sessionConnectingText,
  setStoredUiLanguage,
  type UiLanguage,
} from "../../lib/i18n";
import { useStore } from "../../lib/store";
import {
  activeServiceProfile,
  effectiveTranslationModeForSettings,
  sourceLanguagesForSettings,
  targetLanguagesForSettings,
  translationModesForSettings,
} from "../../lib/providerCapabilities";
import {
  SOURCE_LANGUAGE_DISPLAY_NAMES,
  TARGET_LANGUAGE_DISPLAY_NAMES,
  TRANSLATION_MODE_DISPLAY_NAMES,
  type ServiceProfile,
  type SessionStateEvent,
  type SettingsSnapshot,
  type SourceLanguage,
  type TargetLanguage,
  type TranslationMode,
} from "../../lib/types";
import { sourceLanguageButtonTitle } from "../overlay/overlayModel";
import { ServiceProfiles } from "./ServiceProfiles";
import {
  SettingsRow,
  SettingsSection,
  SettingsSelect,
} from "./SettingsPrimitives";
import "./settings.css";

/** Professional settings console shared by the macOS and Windows shells. */
export function SettingsView() {
  // Subscribe only to state rendered in this window. Subtitle text updates do
  // not re-render settings while a stream is active.
  const sessionStatus = useStore((state) => state.session.status);
  const sessionIsPaused = useStore((state) => state.session.isPaused);
  const sessionIsActive = useStore((state) => state.session.isActive);
  const settings = useStore((state) => state.settings);
  const start = useStore((state) => state.start);
  const stop = useStore((state) => state.stop);
  const switchSourceLanguage = useStore((state) => state.switchSourceLanguage);
  const saveSettings = useStore((state) => state.saveSettings);
  const setOverlayLocked = useStore((state) => state.setOverlayLocked);

  const activeProfile = activeServiceProfile(settings);
  const sourceLanguages = sourceLanguagesForSettings(settings);
  const targetLanguages = targetLanguagesForSettings(settings);
  const translationModes = translationModesForSettings(settings);
  const effectiveTranslationMode =
    effectiveTranslationModeForSettings(settings);
  const isChangingSession =
    sessionStatus.kind === "connecting" || sessionStatus.kind === "stopping";
  const canStart = activeProfile?.credentialState === "present";

  const startListening = () => {
    if (!canStart) return;
    void start();
  };

  const focusServiceProfiles = () => {
    const section = document.getElementById("service-profiles");
    section?.scrollIntoView({
      behavior: window.matchMedia("(prefers-reduced-motion: reduce)").matches
        ? "auto"
        : "smooth",
      block: "start",
    });
    window.requestAnimationFrame(() => {
      document.getElementById("profile-api-key")?.focus();
    });
  };

  const visualState = sessionVisualState(sessionStatus, sessionIsPaused);

  return (
    <main className="settings-console">
      <div className="settings-console__scroll">
        <div className="settings-console__frame">
          <header className="settings-titlebar">
            <div className="settings-wordmark" aria-hidden="true">
              m
            </div>
            <div>
              <h1>{I18N.settings.windowTitle}</h1>
              <p>{I18N.settings.windowSubtitle}</p>
            </div>
          </header>

          <section
            className="session-hero"
            data-state={visualState}
            aria-labelledby="session-hero-title"
          >
            <div className="session-hero__signal" aria-hidden="true">
              <span className="session-hero__signal-ring" />
              <Icon name="captions-bubble" />
            </div>

            <div className="session-hero__copy">
              <span className="session-hero__eyebrow">
                <StatusDot state={visualState} />
                {I18N.settings.sessionEyebrow}
              </span>
              <h2 id="session-hero-title">{I18N.settings.sessionTitle}</h2>
              <p aria-live="polite">
                {sessionStatusText(
                  sessionStatus,
                  sessionIsPaused,
                  settings,
                  activeProfile,
                )}
              </p>
            </div>

            <div className="session-hero__service">
              <span>{I18N.settings.currentProfile}</span>
              <strong>
                {activeProfile?.name ?? I18N.settings.noActiveProfile}
              </strong>
              {activeProfile && (
                <small>{providerDisplayName(activeProfile.provider)}</small>
              )}
            </div>

            <button
              type="button"
              className="session-hero__action"
              data-intent={
                sessionIsActive ? "stop" : canStart ? "start" : "configure"
              }
              disabled={sessionStatus.kind === "stopping"}
              onClick={() =>
                sessionIsActive
                  ? void stop()
                  : canStart
                    ? startListening()
                    : focusServiceProfiles()
              }
            >
              <Icon
                name={sessionIsActive ? "stop" : canStart ? "play" : "key"}
              />
              {sessionIsActive
                ? I18N.settings.stop
                : canStart
                  ? I18N.settings.start
                  : I18N.settings.configureService}
            </button>
          </section>

          <div className="settings-layout">
            <div className="settings-layout__primary">
              <SettingsSection
                id="subtitle-settings"
                icon="languages"
                title={I18N.settings.subtitleTitle}
                description={I18N.settings.subtitleDescription}
                action={
                  <span className="translation-badge">
                    <Icon name={translationBadgeIcon(settings)} />
                    {translationBadgeText(settings)}
                  </span>
                }
              >
                <div className="settings-field-group">
                  <span className="settings-field-group__label">
                    {I18N.settings.sourceLanguage}
                  </span>
                  <div
                    className="source-language-grid"
                    data-count={sourceLanguages.length}
                  >
                    {sourceLanguages.map((language) => (
                      <SourceLanguageButton
                        key={language}
                        language={language}
                        selected={settings.sourceLanguage === language}
                        disabled={
                          isChangingSession || sourceLanguages.length === 1
                        }
                        onSelect={() => void switchSourceLanguage(language)}
                      />
                    ))}
                  </div>
                  <p className="settings-help">
                    {sourceLanguageHelp(sessionStatus, settings)}
                  </p>
                </div>

                <div className="settings-divider" />

                <SettingsRow label={I18N.settings.translateTo}>
                  <SettingsSelect
                    value={settings.targetLanguage}
                    disabled={
                      sessionIsActive || settings.sourceLanguage === "zh"
                    }
                    label={I18N.settings.translateTo}
                    onChange={(value) =>
                      void saveSettings({
                        targetLanguage: value as TargetLanguage,
                      })
                    }
                    options={targetLanguages.map((language) => ({
                      value: language,
                      label: TARGET_LANGUAGE_DISPLAY_NAMES[language],
                    }))}
                  />
                </SettingsRow>

                <div className="settings-divider" />

                <SettingsRow
                  label={I18N.settings.translationMode}
                  description={translationModeHelp(effectiveTranslationMode)}
                  align="start"
                >
                  <SettingsSelect
                    value={effectiveTranslationMode}
                    disabled={sessionIsActive}
                    label={I18N.settings.translationMode}
                    onChange={(value) =>
                      void saveSettings({
                        translationMode: value as TranslationMode,
                      })
                    }
                    options={translationModes.map((mode) => ({
                      value: mode,
                      label: TRANSLATION_MODE_DISPLAY_NAMES[mode],
                    }))}
                  />
                </SettingsRow>

                <div className="settings-divider" />

                <SettingsRow label={I18N.settings.fontSize}>
                  <div className="font-size-control">
                    <span className="font-size-control__sample">A</span>
                    <input
                      type="range"
                      min={14}
                      max={20}
                      step={1}
                      value={settings.fontSize}
                      aria-label={I18N.settings.fontSize}
                      onChange={(event) =>
                        void saveSettings({
                          fontSize: Number(event.target.value),
                        })
                      }
                    />
                    <output>{Math.round(settings.fontSize)}</output>
                  </div>
                </SettingsRow>

                <div className="settings-divider" />

                <SettingsRow
                  label={I18N.settings.lockPosition}
                  description={I18N.settings.lockHelp}
                  align="start"
                >
                  <Switch
                    checked={settings.isOverlayLocked}
                    aria-label={I18N.settings.lockPosition}
                    onChange={(checked) => {
                      void setOverlayLocked(checked).catch(() => {});
                    }}
                  />
                </SettingsRow>
              </SettingsSection>
            </div>

            <div className="settings-layout__secondary">
              <ServiceProfiles
                settings={settings}
                sessionIsActive={sessionIsActive}
              />

              <SettingsSection
                id="application-settings"
                icon="app-window"
                title={I18N.settings.applicationTitle}
                description={I18N.settings.applicationDescription}
              >
                <SettingsRow
                  label={I18N.settings.appLanguage}
                  description={I18N.settings.languageHelp}
                  align="start"
                >
                  <SettingsSelect
                    value={settings.uiLanguage ?? "system"}
                    label={I18N.settings.appLanguage}
                    onChange={(value) => {
                      const language = value as UiLanguage;
                      void saveSettings({ uiLanguage: language })
                        .then(() => {
                          setStoredUiLanguage(language);
                          window.location.reload();
                        })
                        .catch(() => {});
                    }}
                    options={[
                      {
                        value: "system",
                        label: I18N.settings.systemLanguage,
                      },
                      { value: "zh", label: I18N.settings.chinese },
                      { value: "en", label: I18N.settings.english },
                      { value: "ja", label: I18N.settings.japanese },
                    ]}
                  />
                </SettingsRow>
              </SettingsSection>
            </div>
          </div>
        </div>
      </div>
    </main>
  );
}

type SessionVisualState =
  | "idle"
  | "connecting"
  | "listening"
  | "paused"
  | "stopping"
  | "error";

function StatusDot({ state }: { state: SessionVisualState }) {
  return <span className="status-dot" data-state={state} aria-hidden="true" />;
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
      className="source-language-button"
      data-selected={selected}
      aria-pressed={selected}
      disabled={disabled}
      title={sourceLanguageButtonHelp(language)}
      onClick={onSelect}
    >
      <span>{sourceLanguageButtonTitle(language)}</span>
      {selected && <Icon name="checkmark-circle" />}
    </button>
  );
}

function sessionVisualState(
  status: SessionStateEvent["status"],
  isPaused: boolean,
): SessionVisualState {
  if (isPaused) return "paused";
  return status.kind;
}

function sessionStatusText(
  status: SessionStateEvent["status"],
  isPaused: boolean,
  settings: SettingsSnapshot,
  activeProfile: ServiceProfile | undefined,
): string {
  if (isPaused) return I18N.overlay.paused;
  switch (status.kind) {
    case "idle":
      if (activeProfile?.credentialState === "unavailable") {
        return I18N.settings.sessionCredentialUnavailable;
      }
      if (activeProfile?.credentialState !== "present") {
        return I18N.settings.sessionNeedsCredential;
      }
      return I18N.settings.sessionReady;
    case "connecting":
      return sessionConnectingText(
        TRANSLATION_MODE_DISPLAY_NAMES[
          effectiveTranslationModeForSettings(settings)
        ],
      );
    case "listening":
      return I18N.settings.sessionListening;
    case "stopping":
      return I18N.settings.sessionStopping;
    case "error":
      return status.message;
  }
}

function translationBadgeText(settings: SettingsSnapshot): string {
  return settings.sourceLanguage === "zh" &&
    settings.targetLanguage === "original"
    ? I18N.settings.originalOnlyBadge
    : `${TRANSLATION_MODE_DISPLAY_NAMES[effectiveTranslationModeForSettings(settings)]}${I18N.overlay.translationSuffix}`;
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
