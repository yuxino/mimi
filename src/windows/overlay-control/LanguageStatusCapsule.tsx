import { Icon } from "../../components/Icon";
import { I18N } from "../../lib/i18n";
import {
  OVERLAY_ACTIVITY_PHASES,
  TRANSLATION_MODE_DISPLAY_NAMES,
  targetLanguageTranslatesAudio,
  type OverlayActivityPhaseKind,
  type SettingsSnapshot,
} from "../../lib/types";
import { PulseRing } from "../overlay/PulseRing";
import type { LanguageStatus } from "../overlay/overlayModel";

interface LanguageStatusCapsuleProps {
  phase: OverlayActivityPhaseKind;
  status: LanguageStatus;
  settings: SettingsSnapshot;
  effectiveMode: SettingsSnapshot["translationMode"];
  isPaused: boolean;
  isWaitingForFinalTranslation: boolean;
  expanded: boolean;
  onToggle: () => void;
}

/** Compact, always-reachable entry point for the subtitle control panel. */
export function LanguageStatusCapsule({
  phase,
  status,
  settings,
  effectiveMode,
  isPaused,
  isWaitingForFinalTranslation,
  expanded,
  onToggle,
}: LanguageStatusCapsuleProps) {
  const translatesAudio = targetLanguageTranslatesAudio(settings.targetLanguage);
  const modeLabel = translatesAudio
    ? TRANSLATION_MODE_DISPLAY_NAMES[effectiveMode]
    : I18N.overlay.originalOnly;
  const transientLabel = isPaused
    ? I18N.overlay.paused
    : isWaitingForFinalTranslation
      ? I18N.overlay.translating
      : null;
  const actionLabel = expanded
    ? I18N.overlay.closeControls
    : I18N.overlay.openControls;

  return (
    <button
      type="button"
      className={expanded ? "overlay-control-header" : "overlay-control-island"}
      onClick={onToggle}
      title={actionLabel}
      aria-label={`${OVERLAY_ACTIVITY_PHASES[phase].accessibilityLabel}${I18N.overlay.accessibilityCurrentLanguagePrefix}${status.source} ${status.separator} ${status.target}, ${modeLabel}. ${actionLabel}`}
      aria-haspopup={expanded ? undefined : "dialog"}
      aria-expanded={expanded ? undefined : false}
      aria-controls={expanded ? undefined : "overlay-control-panel"}
    >
      <PulseRing phase={phase} compact />
      {transientLabel && (
        <span className="overlay-control-island__phase">{transientLabel}</span>
      )}
      <span className="overlay-control-island__summary">
        <strong>{status.source}</strong>
        <span aria-hidden="true">{status.separator}</span>
        <span>{status.target}</span>
      </span>
      <span className="overlay-control-island__divider" aria-hidden="true" />
      <span className="overlay-control-island__mode">{modeLabel}</span>
      <Icon name={expanded ? "chevron-up" : "chevron-down"} />
    </button>
  );
}
