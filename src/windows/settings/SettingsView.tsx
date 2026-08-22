import { Icon } from "../../components/Icon";
import { Switch } from "../../components/Switch";
import { I18N, setStoredUiLanguage, type UiLanguage } from "../../lib/i18n";
import { useStore } from "../../lib/store";
import {
  effectiveTranslationModeForSettings,
  sourceLanguagesForSettings,
  targetLanguagesForSettings,
  translationModesForSettings,
} from "../../lib/providerCapabilities";
import {
  SOURCE_LANGUAGE_DISPLAY_NAMES,
  TARGET_LANGUAGE_DISPLAY_NAMES,
  TRANSLATION_MODE_DISPLAY_NAMES,
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

/** Compact settings surface shared by the macOS and Windows shells. */
export function SettingsView() {
  // Subscribe only to state rendered in this window. Subtitle text updates do
  // not re-render settings while a stream is active.
  const sessionStatus = useStore((state) => state.session.status);
  const sessionIsActive = useStore((state) => state.session.isActive);
  const settings = useStore((state) => state.settings);
  const switchSourceLanguage = useStore((state) => state.switchSourceLanguage);
  const saveSettings = useStore((state) => state.saveSettings);
  const setOverlayLocked = useStore((state) => state.setOverlayLocked);

  const sourceLanguages = sourceLanguagesForSettings(settings);
  const targetLanguages = targetLanguagesForSettings(settings);
  const translationModes = translationModesForSettings(settings);
  const effectiveTranslationMode =
    effectiveTranslationModeForSettings(settings);
  const isChangingSession =
    sessionStatus.kind === "connecting" || sessionStatus.kind === "stopping";
  return (
    <main className="settings-console">
      <div className="settings-console__scroll">
        <div className="settings-console__frame">
          <header className="settings-page-header">
            <h1>{I18N.settings.windowTitle}</h1>
          </header>

          <div className="settings-layout">
            <SettingsSection
              id="subtitle-settings"
              title={I18N.settings.subtitleTitle}
            >
              <div className="settings-field-group">
                <span
                  className="settings-field-group__label"
                  id="source-language-label"
                >
                  {I18N.settings.sourceLanguage}
                </span>
                <div
                  className="source-language-grid"
                  data-count={sourceLanguages.length}
                  role="group"
                  aria-labelledby="source-language-label"
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
                  <span className="font-size-control__sample" aria-hidden="true">
                    A
                  </span>
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
                  <output aria-live="polite">
                    {Math.round(settings.fontSize)}
                  </output>
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

            <ServiceProfiles
              settings={settings}
              sessionIsActive={sessionIsActive}
            />

            <SettingsSection
              id="application-settings"
              title={I18N.settings.applicationTitle}
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
    </main>
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
