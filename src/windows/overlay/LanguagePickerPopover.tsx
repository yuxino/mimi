import { useState } from "react";
import { Icon } from "../../components/Icon";
import { I18N } from "../../lib/i18n";
import { isTauri, overlayPopoverToggle } from "../../lib/ipc";
import {
  effectiveTranslationModeForSettings,
  sourceLanguagesForSettings,
  translationModesForSettings,
} from "../../lib/providerCapabilities";
import {
  OVERLAY_ACTIVITY_PHASES,
  TRANSLATION_MODE_DISPLAY_NAMES,
  hexToRgba,
  overlayPhaseColor,
  targetLanguageTranslatesAudio,
  type OverlayActivityPhaseKind,
  type SettingsSnapshot,
  type SourceLanguage,
  type TranslationMode,
} from "../../lib/types";
import { PulseRing } from "./PulseRing";
import {
  languageStatus,
  sourceLanguageButtonTitle,
  type LanguageStatus,
} from "./overlayModel";

const ACCENT = "#7AA8FF";

interface LanguagePickerPopoverProps {
  phase: OverlayActivityPhaseKind;
  isHovering: boolean;
  isPaused: boolean;
  isChangingSession: boolean;
  isWaitingForFinalTranslation: boolean;
  settings: SettingsSnapshot;
  detectedLanguage: string | null;
  onSwitchSourceLanguage: (language: SourceLanguage) => void;
  onSwitchTranslationMode: (mode: TranslationMode) => void;
}

/**
 * The top-left language capsule. In Tauri the menu lives in its own anchored
 * window, so opening it never changes the subtitle overlay's geometry.
 * In the plain `vite dev` preview the menu renders inline instead.
 */
export function LanguagePickerPopover({
  phase,
  isHovering,
  isPaused,
  isChangingSession,
  isWaitingForFinalTranslation,
  settings,
  detectedLanguage,
  onSwitchSourceLanguage,
  onSwitchTranslationMode,
}: LanguagePickerPopoverProps) {
  const [open, setOpen] = useState(false);
  const status = languageStatus(settings, detectedLanguage);

  if (status === null) return null;

  const canInteract = !isPaused && !isChangingSession;
  const translatesAudio = targetLanguageTranslatesAudio(settings.targetLanguage);
  const sourceLanguages = sourceLanguagesForSettings(settings);
  const translationModes = translationModesForSettings(settings);
  const displayMode = effectiveTranslationModeForSettings(settings);
  const help = translatesAudio
    ? I18N.overlay.pickerHelpTranslating
    : I18N.overlay.pickerHelpOriginal;

  const handleToggle = () => {
    if (!canInteract) return;
    if (isTauri) {
      // The backend anchors the menu under this capsule from the overlay's
      // position, so the overlay size is never affected by the menu.
      void overlayPopoverToggle();
      return;
    }
    setOpen((value) => !value);
  };

  return (
    <div className="relative">
      <button
        type="button"
        disabled={!canInteract}
        onClick={handleToggle}
        title={help}
        aria-label={accessibilityLabel(phase, status, settings)}
        className="flex items-center"
        style={{
          gap: 4,
          fontSize: 10,
          fontWeight: 500,
          lineHeight: "20px",
          height: 20,
          padding: "0 8px",
          borderRadius: 999,
          border: `0.5px solid ${hexToRgba(ACCENT, isHovering ? 0.22 : 0.14)}`,
          background: hexToRgba(ACCENT, isHovering ? 0.11 : 0.075),
          whiteSpace: "nowrap",
          cursor: canInteract ? "pointer" : "default",
        }}
      >
        <PulseRing phase={phase} compact />
        {(isPaused || isWaitingForFinalTranslation) && (
          <>
            <span style={{ color: overlayPhaseColor(phase, 0.96) }}>
              {isPaused ? I18N.overlay.paused : I18N.overlay.translating}
            </span>
            <span style={{ color: "rgba(255,255,255,0.34)" }}>
              {I18N.overlay.dotSeparator}
            </span>
          </>
        )}
        <span style={{ color: hexToRgba(ACCENT, isHovering ? 0.96 : 0.8) }}>
          {status.source}
        </span>
        <span
          style={{ color: `rgba(255,255,255,${isHovering ? 0.48 : 0.32})` }}
        >
          {status.separator}
        </span>
        <span
          style={{ color: `rgba(255,255,255,${isHovering ? 0.74 : 0.56})` }}
        >
          {status.target}
        </span>
        {translatesAudio && (
          <>
            <div
              style={{
                width: 0.5,
                height: 9,
                background: "rgba(255,255,255,0.14)",
              }}
            />
            <Icon
              name="sparkles"
              style={{ fontSize: 7, color: hexToRgba(ACCENT, 0.74) }}
            />
            <span
              style={{
                color: `rgba(255,255,255,${isHovering ? 0.72 : 0.52})`,
              }}
            >
              {TRANSLATION_MODE_DISPLAY_NAMES[displayMode]}
            </span>
          </>
        )}
        {!translatesAudio && (
          // Original mode: the mode slot stays visible with an explicit
          // "原文" label instead of silently disappearing, so the capsule
          // layout is stable and the current state is always explained.
          <>
            <div
              style={{
                width: 0.5,
                height: 9,
                background: "rgba(255,255,255,0.14)",
              }}
            />
            <span
              style={{
                color: `rgba(255,255,255,${isHovering ? 0.72 : 0.52})`,
              }}
            >
              {I18N.overlay.originalOnly}
            </span>
          </>
        )}
        <Icon
          name="chevron-down"
          style={{
            fontSize: 6,
            color: `rgba(255,255,255,${isHovering ? 0.5 : 0.3})`,
          }}
        />
      </button>

      {!isTauri && open && (
        <div
          className="absolute left-0 top-full z-10 mt-1.5"
          style={{
            width: 168,
            padding: 8,
            borderRadius: 10,
            background: "#242424",
            border: "0.5px solid rgba(255,255,255,0.14)",
            boxShadow: "0 8px 24px rgba(0,0,0,0.4)",
          }}
        >
          <PopoverHeader>{I18N.overlay.sourceLanguage}</PopoverHeader>
          {sourceLanguages.map((language) => (
            <PickerRow
              key={language}
              title={sourceLanguageButtonTitle(language)}
              selected={settings.sourceLanguage === language}
              disabled={!canInteract || sourceLanguages.length === 1}
              onSelect={() => {
                setOpen(false);
                onSwitchSourceLanguage(language);
              }}
            />
          ))}

          <div
            style={{
              height: 1,
              background: "rgba(255,255,255,0.1)",
              margin: "5px 0",
            }}
          />
          <PopoverHeader>{I18N.overlay.translationMode}</PopoverHeader>
          {translationModes.map((mode) => (
            <PickerRow
              key={mode}
              title={TRANSLATION_MODE_DISPLAY_NAMES[mode]}
              selected={displayMode === mode}
              disabled={!canInteract || translationModes.length === 1}
              onSelect={() => {
                setOpen(false);
                onSwitchTranslationMode(mode);
              }}
            />
          ))}
        </div>
      )}
    </div>
  );
}

function PopoverHeader({ children }: { children: string }) {
  return (
    <p
      style={{
        fontSize: 11,
        fontWeight: 600,
        color: "rgba(255,255,255,0.5)",
        padding: "0 8px 2px",
        margin: 0,
      }}
    >
      {children}
    </p>
  );
}

function PickerRow({
  title,
  selected,
  disabled = false,
  onSelect,
}: {
  title: string;
  selected: boolean;
  disabled?: boolean;
  onSelect: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onSelect}
      disabled={disabled}
      className="ux-hover ux-hover-bg flex w-full items-center"
      style={{
        gap: 8,
        height: 26,
        padding: "0 8px",
        borderRadius: 6,
        border: "none",
        background: "transparent",
        color: "rgba(255,255,255,0.9)",
        fontSize: 13,
        cursor: disabled ? "default" : "pointer",
      }}
    >
      <span className="flex-1 text-left">{title}</span>
      {selected && <Icon name="checkmark" style={{ color: ACCENT }} />}
    </button>
  );
}

function accessibilityLabel(
  phase: OverlayActivityPhaseKind,
  status: LanguageStatus,
  settings: SettingsSnapshot,
): string {
  const effectiveMode = effectiveTranslationModeForSettings(settings);
  const mode = targetLanguageTranslatesAudio(settings.targetLanguage)
    ? `${TRANSLATION_MODE_DISPLAY_NAMES[effectiveMode]}${I18N.overlay.translationSuffix}`
    : I18N.overlay.originalOnly;
  return `${OVERLAY_ACTIVITY_PHASES[phase].accessibilityLabel}${I18N.overlay.accessibilityCurrentLanguagePrefix}${status.source} ${status.separator} ${status.target}，${mode}${I18N.overlay.accessibilityOpenToSwitch}`;
}
