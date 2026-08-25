import { useCallback, useEffect, useState } from "react";
import {
  isTauri,
  listenOverlayControlMode,
  overlayControlGetState,
  overlayPopoverHide,
  overlayPopoverToggle,
  type OverlayControlMode,
} from "../../lib/ipc";
import { useStore } from "../../lib/store";
import {
  computeActivityPhase,
  isWaitingForFinalTranslation,
  languageStatus,
} from "../overlay/overlayModel";
import { LanguageStatusCapsule } from "./LanguageStatusCapsule";
import { OverlayControlPanel } from "./OverlayControlPanel";
import { overlayControlPanelModel } from "./overlayControlModel";
import "./overlay-control.css";

/** Child window that morphs between a compact status island and its panel. */
export function OverlayControlWindow() {
  const session = useStore((state) => state.session);
  const settings = useStore((state) => state.settings);
  const switchSourceLanguage = useStore((state) => state.switchSourceLanguage);
  const switchTranslationMode = useStore(
    (state) => state.switchTranslationMode,
  );
  const saveSettings = useStore((state) => state.saveSettings);
  const setOverlayLocked = useStore((state) => state.setOverlayLocked);
  const showSettings = useStore((state) => state.showSettings);
  const [mode, setMode] = useState<OverlayControlMode>(initialPreviewMode);

  const toggle = useCallback(() => {
    if (isTauri) {
      void overlayPopoverToggle().catch(() => {});
    } else {
      setMode((current) => (current === "panel" ? "island" : "panel"));
    }
  }, []);

  const dismiss = useCallback(() => {
    if (isTauri) {
      void overlayPopoverHide().catch(() => {});
    } else {
      setMode("island");
    }
  }, []);

  useEffect(() => {
    if (!isTauri) return;
    let disposed = false;
    let eventSeen = false;
    let removeListener: (() => void) | undefined;
    void listenOverlayControlMode((nextMode) => {
      eventSeen = true;
      if (!disposed) setMode(nextMode);
    }).then((unlisten) => {
      if (disposed) {
        unlisten();
        return;
      }
      removeListener = unlisten;
      void overlayControlGetState()
        .then((nextMode) => {
          if (!disposed && !eventSeen) setMode(nextMode);
        })
        .catch(() => {});
    });
    return () => {
      disposed = true;
      removeListener?.();
    };
  }, []);

  useEffect(() => {
    if (mode !== "panel") return;
    const dismissOnEscape = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      event.preventDefault();
      dismiss();
    };
    window.addEventListener("keydown", dismissOnEscape);
    return () => window.removeEventListener("keydown", dismissOnEscape);
  }, [dismiss, mode]);

  if (mode === "hidden") return null;

  const phase = computeActivityPhase(session, settings);
  const status = languageStatus(settings, session.detectedLanguage);
  if (status === null) return null;
  const isWaiting = isWaitingForFinalTranslation(
    settings,
    session.detectedLanguage,
    session.isTranslationPending,
  );
  const model = overlayControlPanelModel(settings);
  const isChangingSession =
    session.status.kind === "connecting" || session.status.kind === "stopping";

  if (mode === "panel") {
    return (
      <OverlayControlPanel
        phase={phase}
        status={status}
        settings={settings}
        model={model}
        isPaused={session.isPaused}
        isWaitingForFinalTranslation={isWaiting}
        isChangingSession={isChangingSession}
        onDismiss={dismiss}
        onSwitchSourceLanguage={switchSourceLanguage}
        onSwitchTranslationMode={switchTranslationMode}
        onSetImmersiveMode={(subtitleBlendsWithBackground) =>
          saveSettings({ subtitleBlendsWithBackground })
        }
        onSetOverlayLocked={setOverlayLocked}
        onShowSettings={showSettings}
      />
    );
  }

  return (
    <LanguageStatusCapsule
      phase={phase}
      status={status}
      settings={settings}
      effectiveMode={model.effectiveTranslationMode}
      isPaused={session.isPaused}
      isWaitingForFinalTranslation={isWaiting}
      expanded={false}
      onToggle={toggle}
    />
  );
}

function initialPreviewMode(): OverlayControlMode {
  if (isTauri) return "hidden";
  const mode = new URLSearchParams(window.location.search).get("mode");
  return mode === "panel" ? "panel" : "island";
}
