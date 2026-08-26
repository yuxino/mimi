import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { Icon } from "../../components/Icon";
import { I18N } from "../../lib/i18n";
import {
  isTauri,
  overlayControlSetPanelHeight,
} from "../../lib/ipc";
import { targetLanguagesForSettings } from "../../lib/providerCapabilities";
import {
  TRANSLATION_MODE_DISPLAY_NAMES,
  type OverlayActivityPhaseKind,
  type SettingsSnapshot,
  type SourceLanguage,
  type TranslationMode,
} from "../../lib/types";
import {
  sourceLanguageButtonTitle,
  type LanguageStatus,
} from "../overlay/overlayModel";
import { LanguageStatusCapsule } from "./LanguageStatusCapsule";
import type { OverlayControlPanelModel } from "./overlayControlModel";

type PendingAction =
  | "source"
  | "mode"
  | "immersive"
  | "lock"
  | "settings";

interface OverlayControlPanelProps {
  phase: OverlayActivityPhaseKind;
  status: LanguageStatus;
  settings: SettingsSnapshot;
  model: OverlayControlPanelModel;
  isPaused: boolean;
  isWaitingForFinalTranslation: boolean;
  isChangingSession: boolean;
  onDismiss: () => void;
  onSwitchSourceLanguage: (language: SourceLanguage) => Promise<void>;
  onSwitchTranslationMode: (mode: TranslationMode) => Promise<void>;
  onSetImmersiveMode: (enabled: boolean) => Promise<void>;
  onSetOverlayLocked: (locked: boolean) => Promise<void>;
  onShowSettings: () => Promise<void>;
}

export function OverlayControlPanel({
  phase,
  status,
  settings,
  model,
  isPaused,
  isWaitingForFinalTranslation,
  isChangingSession,
  onDismiss,
  onSwitchSourceLanguage,
  onSwitchTranslationMode,
  onSetImmersiveMode,
  onSetOverlayLocked,
  onShowSettings,
}: OverlayControlPanelProps) {
  const panelRef = useRef<HTMLElement>(null);
  const contentRef = useRef<HTMLDivElement>(null);
  const selectedSourceRef = useRef<HTMLButtonElement>(null);
  const selectedModeRef = useRef<HTMLButtonElement>(null);
  const immersiveRef = useRef<HTMLButtonElement>(null);
  const lockRef = useRef<HTMLButtonElement>(null);
  const [pendingAction, setPendingAction] = useState<PendingAction | null>(null);
  const [operationError, setOperationError] = useState<string | null>(null);
  const canChangeSessionSettings = !isChangingSession && pendingAction === null;
  const chineseIsOriginalOnly =
    targetLanguagesForSettings(settings).includes("original");

  useLayoutEffect(() => {
    if (!isTauri || !panelRef.current || !contentRef.current) return;
    let animationFrame = 0;
    let lastHeight = 0;
    const measure = () => {
      animationFrame = 0;
      const panel = panelRef.current;
      const content = contentRef.current;
      if (!panel || !content) return;
      const height = Math.ceil(
        content.getBoundingClientRect().height + panel.clientTop * 2,
      );
      if (height === lastHeight) return;
      lastHeight = height;
      void overlayControlSetPanelHeight(height).catch(() => {});
    };
    const scheduleMeasure = () => {
      if (animationFrame !== 0) return;
      animationFrame = window.requestAnimationFrame(measure);
    };
    const observer = new ResizeObserver(scheduleMeasure);
    observer.observe(contentRef.current);
    scheduleMeasure();
    return () => {
      observer.disconnect();
      if (animationFrame !== 0) window.cancelAnimationFrame(animationFrame);
    };
  }, []);

  useEffect(() => {
    const target = [
      selectedSourceRef.current,
      selectedModeRef.current,
      immersiveRef.current,
      lockRef.current,
    ].find((candidate) => candidate !== null && !candidate.disabled);
    const animationFrame = window.requestAnimationFrame(() => target?.focus());
    return () => window.cancelAnimationFrame(animationFrame);
  }, []);

  const performAction = (
    name: PendingAction,
    operation: () => Promise<void>,
    dismissAfter = true,
  ) => {
    if (pendingAction !== null) return;
    setPendingAction(name);
    setOperationError(null);
    void operation()
      .then(() => {
        if (dismissAfter) onDismiss();
      })
      .catch(() => setOperationError(I18N.overlay.controlActionFailed))
      .finally(() => setPendingAction(null));
  };

  return (
    <section
      ref={panelRef}
      id="overlay-control-panel"
      className="overlay-control-panel"
      role="dialog"
      aria-modal="false"
      aria-label={I18N.overlay.controlPanel}
      aria-busy={pendingAction !== null}
    >
      <div ref={contentRef} className="overlay-control-panel__content">
        <LanguageStatusCapsule
          phase={phase}
          status={status}
          settings={settings}
          effectiveMode={model.effectiveTranslationMode}
          isPaused={isPaused}
          isWaitingForFinalTranslation={isWaitingForFinalTranslation}
          expanded
          onToggle={onDismiss}
        />

        {model.sourceOptions.length > 0 && (
          <fieldset className="overlay-control-group">
            <legend>{I18N.overlay.sourceLanguage}</legend>
            <div className="overlay-control-options">
              {model.sourceOptions.map((language) => {
                const selected = settings.sourceLanguage === language;
                return (
                  <button
                    key={language}
                    ref={selected ? selectedSourceRef : undefined}
                    type="button"
                    className="overlay-control-option"
                    data-selected={selected || undefined}
                    aria-pressed={selected}
                    disabled={!canChangeSessionSettings}
                    onClick={() => {
                      if (selected) {
                        onDismiss();
                        return;
                      }
                      performAction("source", () =>
                        onSwitchSourceLanguage(language),
                      );
                    }}
                  >
                    <span>
                      {sourceLanguageButtonTitle(
                        language,
                        chineseIsOriginalOnly,
                      )}
                    </span>
                    {selected && <Icon name="checkmark" />}
                  </button>
                );
              })}
            </div>
          </fieldset>
        )}

        {model.translationModeOptions.length > 0 && (
          <fieldset className="overlay-control-group">
            <legend>{I18N.overlay.translationMode}</legend>
            <div className="overlay-control-options">
              {model.translationModeOptions.map((mode) => {
                const selected = model.effectiveTranslationMode === mode;
                return (
                  <button
                    key={mode}
                    ref={selected ? selectedModeRef : undefined}
                    type="button"
                    className="overlay-control-option"
                    data-selected={selected || undefined}
                    aria-pressed={selected}
                    disabled={!canChangeSessionSettings}
                    onClick={() => {
                      if (selected) {
                        onDismiss();
                        return;
                      }
                      performAction("mode", () =>
                        onSwitchTranslationMode(mode),
                      );
                    }}
                  >
                    <span>{TRANSLATION_MODE_DISPLAY_NAMES[mode]}</span>
                    {selected && <Icon name="checkmark" />}
                  </button>
                );
              })}
            </div>
          </fieldset>
        )}

        <div className="overlay-control-divider" />

        <button
          ref={immersiveRef}
          type="button"
          role="switch"
          aria-checked={model.immersiveModeEnabled}
          aria-label={I18N.overlay.immersiveMode}
          className="overlay-control-setting"
          disabled={pendingAction !== null}
          onClick={() =>
            performAction("immersive", () =>
              onSetImmersiveMode(!model.immersiveModeEnabled),
            )
          }
        >
          <span className="overlay-control-setting__icon" aria-hidden="true">
            <Icon name="blend" />
          </span>
          <span className="overlay-control-setting__copy">
            <strong>{I18N.overlay.immersiveMode}</strong>
            <small>
              {model.immersiveModeEnabled
                ? I18N.overlay.immersiveModeOn
                : I18N.overlay.immersiveModeOff}
            </small>
          </span>
          <span className="overlay-control-switch" aria-hidden="true">
            <span />
          </span>
        </button>

        <button
          ref={lockRef}
          type="button"
          role="switch"
          aria-checked={model.overlayLocked}
          aria-label={I18N.overlay.lockPosition}
          className="overlay-control-setting"
          disabled={pendingAction !== null}
          onClick={() =>
            performAction("lock", () =>
              onSetOverlayLocked(!model.overlayLocked),
            )
          }
        >
          <span className="overlay-control-setting__icon" aria-hidden="true">
            <Icon name={model.overlayLocked ? "unlock" : "lock"} />
          </span>
          <span className="overlay-control-setting__copy">
            <strong>
              {model.overlayLocked
                ? I18N.overlay.unlockPosition
                : I18N.overlay.lockPosition}
            </strong>
            <small>
              {model.overlayLocked
                ? I18N.overlay.positionLocked
                : I18N.overlay.positionUnlocked}
            </small>
          </span>
          <span className="overlay-control-switch" aria-hidden="true">
            <span />
          </span>
        </button>

        <button
          type="button"
          className="overlay-control-settings-link"
          disabled={pendingAction !== null}
          onClick={() => performAction("settings", onShowSettings, false)}
        >
          <Icon name="gear" />
          <span>{I18N.overlay.moreSettings}</span>
        </button>

        {operationError && (
          <div className="overlay-control-alert" role="alert">
            <Icon name="exclamation-triangle" />
            <span>{operationError}</span>
          </div>
        )}
      </div>
    </section>
  );
}
