import { useState } from "react";
import { Icon } from "../../components/Icon";
import { I18N } from "../../lib/i18n";
import {
  OVERLAY_ACTIVITY_PHASES,
  SOURCE_LANGUAGE_MANUAL_CASES,
  TRANSLATION_MODE_CASES,
  TRANSLATION_MODE_DISPLAY_NAMES,
  hexToRgba,
  targetLanguageTranslatesAudio,
  type OverlayActivityPhaseKind,
  type SessionStatus,
  type SettingsSnapshot,
  type SourceLanguage,
  type TranslationMode,
} from "../../lib/types";
import { RecognitionActivityIndicator } from "./RecognitionActivityIndicator";
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
  isWaitingForFinalTranslation: boolean;
  statusKind: SessionStatus["kind"];
  settings: SettingsSnapshot;
  detectedLanguage: string | null;
  onSwitchSourceLanguage: (language: SourceLanguage) => void;
  onSwitchTranslationMode: (mode: TranslationMode) => void;
}

/** The top-left language capsule and its source-language / mode popover. */
export function LanguagePickerPopover({
  phase,
  isHovering,
  isPaused,
  isWaitingForFinalTranslation,
  statusKind,
  settings,
  detectedLanguage,
  onSwitchSourceLanguage,
  onSwitchTranslationMode,
}: LanguagePickerPopoverProps) {
  const [open, setOpen] = useState(false);
  const status = languageStatus(settings, detectedLanguage);
  if (status === null) return null;

  // Interactive whenever not mid-connection: idle (before starting)
  // and listening both allow quick language/mode switching.
  const canInteract =
    !isPaused && statusKind !== "connecting" && statusKind !== "stopping";
  const translatesAudio = targetLanguageTranslatesAudio(settings.targetLanguage);
  const help = translatesAudio
    ? I18N.overlay.pickerHelpTranslating
    : I18N.overlay.pickerHelpOriginal;

  return (
    <div className="relative">
      <button
        type="button"
        disabled={!canInteract}
        onClick={() => setOpen((value) => !value)}
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
        <RecognitionActivityIndicator phase={phase} />
        {(isPaused || isWaitingForFinalTranslation) && (
          <>
            <span style={{ color: hexToRgba(ACCENT, 0.96) }}>
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
              {TRANSLATION_MODE_DISPLAY_NAMES[settings.translationMode]}
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

      {open && (
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
          {SOURCE_LANGUAGE_MANUAL_CASES.map((language) => (
            <PickerRow
              key={language}
              title={sourceLanguageButtonTitle(language)}
              selected={settings.sourceLanguage === language}
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
          {TRANSLATION_MODE_CASES.map((mode) => (
            <PickerRow
              key={mode}
              title={TRANSLATION_MODE_DISPLAY_NAMES[mode]}
              selected={settings.translationMode === mode}
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
  onSelect,
}: {
  title: string;
  selected: boolean;
  onSelect: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onSelect}
      className="flex w-full items-center"
      style={{
        gap: 8,
        height: 26,
        padding: "0 8px",
        borderRadius: 6,
        border: "none",
        background: "transparent",
        color: "rgba(255,255,255,0.9)",
        fontSize: 13,
        cursor: "pointer",
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
  const mode = targetLanguageTranslatesAudio(settings.targetLanguage)
    ? `${TRANSLATION_MODE_DISPLAY_NAMES[settings.translationMode]}翻译`
    : I18N.overlay.originalOnly;
  return `${OVERLAY_ACTIVITY_PHASES[phase].accessibilityLabel}，当前语言：${status.source} ${status.separator} ${status.target}，${mode}。打开以切换识别语言。`;
}
