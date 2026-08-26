import { useCallback, useEffect, useRef, useState } from "react";
import { Icon } from "../../components/Icon";
import { Switch } from "../../components/Switch";
import { I18N, setStoredUiLanguage, type UiLanguage } from "../../lib/i18n";
import {
  announceSettingsNavigationReady,
  isTauri,
  listenSettingsNavigation,
} from "../../lib/ipc";
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
  type SubtitleAlignment,
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

type SettingsCategory = "subtitles" | "service" | "general";

const CATEGORY_SECTION_IDS: Record<SettingsCategory, string> = {
  subtitles: "subtitle-settings",
  service: "service-profiles",
  general: "application-settings",
};

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

  const activeProfile =
    settings.profiles.find(
      (profile) => profile.id === settings.activeProfileId,
    ) ?? settings.profiles[0];
  const locationCategory = settingsCategoryFromHash(window.location.hash);
  const preferredCategory: SettingsCategory =
    activeProfile?.credentialState === "present" ? "subtitles" : "service";
  const [activeCategory, setActiveCategory] = useState<SettingsCategory>(
    locationCategory ?? preferredCategory,
  );
  const locationSelectedCategory = useRef(locationCategory !== null);
  const initialCredentialState = useRef(activeProfile?.credentialState);

  // Native settings arrive after the first render. Resolve the initial
  // fail-closed `unavailable` placeholder once, without later pulling users
  // away from a category they chose or changing category after key edits.
  useEffect(() => {
    if (
      locationSelectedCategory.current ||
      initialCredentialState.current !== "unavailable" ||
      activeProfile?.credentialState === "unavailable"
    ) {
      return;
    }
    initialCredentialState.current = activeProfile?.credentialState;
    setActiveCategory(
      activeProfile?.credentialState === "present" ? "subtitles" : "service",
    );
  }, [activeProfile?.credentialState]);

  const sourceLanguages = sourceLanguagesForSettings(settings);
  const targetLanguages = targetLanguagesForSettings(settings);
  const chineseIsOriginalOnly = targetLanguages.includes("original");
  const translationModes = translationModesForSettings(settings);
  const effectiveTranslationMode =
    effectiveTranslationModeForSettings(settings);
  const isChangingSession =
    sessionStatus.kind === "connecting" || sessionStatus.kind === "stopping";

  const categories: readonly {
    id: SettingsCategory;
    label: string;
    icon: "captions-bubble" | "languages" | "gear";
  }[] = [
    {
      id: "subtitles",
      label: I18N.settings.subtitleTitle,
      icon: "captions-bubble",
    },
    {
      id: "service",
      label: I18N.settings.serviceProfilesTitle,
      icon: "languages",
    },
    {
      id: "general",
      label: I18N.settings.applicationTitle,
      icon: "gear",
    },
  ];

  const selectCategory = useCallback((category: SettingsCategory) => {
    locationSelectedCategory.current = true;
    setActiveCategory(category);
    window.history.replaceState(null, "", `#${CATEGORY_SECTION_IDS[category]}`);
  }, []);

  useEffect(() => {
    if (!isTauri) return;

    let disposed = false;
    let unlisten: (() => void) | undefined;

    void listenSettingsNavigation(() => {
      selectCategory("service");
      window.requestAnimationFrame(() => {
        document.getElementById("settings-category-service")?.focus();
      });
    })
      .then(async (installedUnlisten) => {
        if (disposed) {
          installedUnlisten();
          return;
        }
        unlisten = installedUnlisten;
        await announceSettingsNavigationReady();
      })
      .catch(() => {});

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [selectCategory]);

  return (
    <main className="settings-console">
      <div className="settings-console__scroll">
        <div className="settings-console__frame">
          <header className="settings-page-header">
            <h1>{I18N.settings.windowTitle}</h1>
          </header>

          <div className="settings-workspace">
            <nav
              className="settings-category-nav"
              aria-label={I18N.settings.windowTitle}
            >
              {categories.map((category) => {
                const selected = activeCategory === category.id;
                return (
                  <button
                    key={category.id}
                    id={`settings-category-${category.id}`}
                    type="button"
                    className="settings-category-nav__item"
                    data-selected={selected}
                    aria-current={selected ? "page" : undefined}
                    aria-controls={`${CATEGORY_SECTION_IDS[category.id]}-panel`}
                    onClick={() => selectCategory(category.id)}
                  >
                    <Icon name={category.icon} />
                    <span>{category.label}</span>
                  </button>
                );
              })}
            </nav>

            <div className="settings-layout">
              <div
                id="subtitle-settings-panel"
                className="settings-category-panel"
                hidden={activeCategory !== "subtitles"}
              >
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
                          chineseIsOriginalOnly={chineseIsOriginalOnly}
                          disabled={
                            isChangingSession || sourceLanguages.length === 1
                          }
                          onSelect={() => void switchSourceLanguage(language)}
                        />
                      ))}
                    </div>
                    <p className="settings-help">
                      {sourceLanguageHelp(
                        sessionStatus,
                        settings,
                        chineseIsOriginalOnly,
                      )}
                    </p>
                  </div>

                  <div className="settings-divider" />

                  <SettingsRow label={I18N.settings.translateTo}>
                    <SettingsSelect
                      value={settings.targetLanguage}
                      disabled={
                        sessionIsActive ||
                        (settings.sourceLanguage === "zh" &&
                          targetLanguages.includes("original"))
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
                      <span
                        className="font-size-control__sample"
                        aria-hidden="true"
                      >
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

                  <SettingsRow label={I18N.settings.subtitleAlignment}>
                    <SubtitleAlignmentControl
                      value={settings.subtitleAlignment}
                      onChange={(subtitleAlignment) =>
                        void saveSettings({ subtitleAlignment })
                      }
                    />
                  </SettingsRow>

                  <div className="settings-divider" />

                  <SettingsRow
                    label={I18N.settings.blendBackground}
                    description={I18N.settings.blendBackgroundHelp}
                    align="start"
                  >
                    <Switch
                      checked={settings.subtitleBlendsWithBackground}
                      aria-label={I18N.settings.blendBackground}
                      onChange={(subtitleBlendsWithBackground) =>
                        void saveSettings({ subtitleBlendsWithBackground })
                      }
                    />
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

              <div
                id="service-profiles-panel"
                className="settings-category-panel"
                hidden={activeCategory !== "service"}
              >
                <ServiceProfiles
                  settings={settings}
                  sessionIsActive={sessionIsActive}
                />
              </div>

              <div
                id="application-settings-panel"
                className="settings-category-panel"
                hidden={activeCategory !== "general"}
              >
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
        </div>
      </div>
    </main>
  );
}

const SUBTITLE_ALIGNMENTS: readonly SubtitleAlignment[] = [
  "left",
  "center",
  "right",
];

function SubtitleAlignmentControl({
  value,
  onChange,
}: {
  value: SubtitleAlignment;
  onChange: (alignment: SubtitleAlignment) => void;
}) {
  const labels: Record<SubtitleAlignment, string> = {
    left: I18N.settings.alignLeft,
    center: I18N.settings.alignCenter,
    right: I18N.settings.alignRight,
  };

  return (
    <div
      className="subtitle-alignment-control"
      role="group"
      aria-label={I18N.settings.subtitleAlignment}
    >
      {SUBTITLE_ALIGNMENTS.map((alignment) => (
        <button
          key={alignment}
          type="button"
          data-selected={value === alignment}
          aria-label={labels[alignment]}
          aria-pressed={value === alignment}
          title={labels[alignment]}
          onClick={() => onChange(alignment)}
        >
          <Icon name={`align-${alignment}`} />
        </button>
      ))}
    </div>
  );
}

function settingsCategoryFromHash(hash: string): SettingsCategory | null {
  switch (hash.replace(/^#/, "")) {
    case CATEGORY_SECTION_IDS.subtitles:
      return "subtitles";
    case CATEGORY_SECTION_IDS.service:
      return "service";
    case CATEGORY_SECTION_IDS.general:
      return "general";
    default:
      return null;
  }
}

function SourceLanguageButton({
  language,
  selected,
  chineseIsOriginalOnly,
  disabled,
  onSelect,
}: {
  language: SourceLanguage;
  selected: boolean;
  chineseIsOriginalOnly: boolean;
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
      title={sourceLanguageButtonHelp(language, chineseIsOriginalOnly)}
      onClick={onSelect}
    >
      <span>
        {sourceLanguageButtonTitle(language, chineseIsOriginalOnly)}
      </span>
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
  chineseIsOriginalOnly: boolean,
): string {
  if (settings.sourceLanguage === "zh") {
    if (!chineseIsOriginalOnly) {
      return status.kind === "listening"
        ? I18N.settings.recognizingChineseTranslatedListening
        : I18N.settings.recognizingChineseTranslatedIdle;
    }
    return status.kind === "listening"
      ? I18N.settings.recognizingChineseListening
      : I18N.settings.recognizingChineseIdle;
  }
  if (status.kind === "listening") {
    return I18N.settings.sourceHelpReconnecting;
  }
  return I18N.settings.sourceHelpIdle;
}

function sourceLanguageButtonHelp(
  language: SourceLanguage,
  chineseIsOriginalOnly: boolean,
): string {
  return language === "zh" && chineseIsOriginalOnly
    ? I18N.settings.switchToChineseHelp
    : I18N.settings.switchToLanguageHelp(
        SOURCE_LANGUAGE_DISPLAY_NAMES[language],
      );
}
