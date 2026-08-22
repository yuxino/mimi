import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { Icon, type IconName } from "../../components/Icon";
import { I18N, providerDisplayName } from "../../lib/i18n";
import { isTauri } from "../../lib/ipc";
import {
  activeServiceProfile,
  effectiveTranslationModeForSettings,
  sourceLanguagesForSettings,
} from "../../lib/providerCapabilities";
import { useStore } from "../../lib/store";
import {
  TARGET_LANGUAGE_DISPLAY_NAMES,
  TRANSLATION_MODE_DISPLAY_NAMES,
  targetLanguageTranslatesAudio,
  type SessionStateEvent,
  type SettingsSnapshot,
  type SourceLanguage,
} from "../../lib/types";
import { sourceLanguageButtonTitle } from "../overlay/overlayModel";
import {
  actionErrorMessage,
  deriveTrayPresentation,
  hasSubtitleContent,
  type TrayActionPresentation,
  type TraySessionAction,
  type TrayStatusKind,
} from "./trayModel";
import "./tray-panel.css";

const TRAY_WINDOW_WIDTH = 320;
const TRAY_WINDOW_PADDING = 6;

type PendingAction =
  | TraySessionAction
  | "language"
  | "lock"
  | "show"
  | "clear"
  | "settings"
  | "quit";

/** Compact cross-platform command center shown from the tray icon. */
export function TrayPanel() {
  // Keep streaming subtitle updates from repainting this window after content
  // first becomes available; the selector returns a stable boolean.
  const sessionStatus = useStore((state) => state.session.status);
  const isPaused = useStore((state) => state.session.isPaused);
  const subtitleHasContent = useStore((state) =>
    hasSubtitleContent(state.session.subtitles),
  );
  const settings = useStore((state) => state.settings);
  const start = useStore((state) => state.start);
  const stop = useStore((state) => state.stop);
  const togglePaused = useStore((state) => state.togglePaused);
  const switchSourceLanguage = useStore((state) => state.switchSourceLanguage);
  const setOverlayLocked = useStore((state) => state.setOverlayLocked);
  const showOverlay = useStore((state) => state.showOverlay);
  const clearSubtitles = useStore((state) => state.clearSubtitles);
  const hideTrayPanel = useStore((state) => state.hideTrayPanel);
  const showSettings = useStore((state) => state.showSettings);
  const quit = useStore((state) => state.quit);

  const [pendingAction, setPendingAction] = useState<PendingAction | null>(null);
  const [operationError, setOperationError] = useState<string | null>(null);
  const panelRef = useRef<HTMLElement>(null);

  const activeProfile = activeServiceProfile(settings);
  const sourceLanguages = sourceLanguagesForSettings(settings);
  const presentation = deriveTrayPresentation({
    status: sessionStatus,
    isPaused,
    credentialState: activeProfile?.credentialState,
    hasSubtitleContent: subtitleHasContent,
  });
  const anyActionPending = pendingAction !== null;
  const sourcePickerDisabled =
    anyActionPending ||
    !presentation.canChangeSourceLanguage ||
    sourceLanguages.length === 1;

  const performAction = (
    name: PendingAction,
    operation: () => Promise<void>,
  ) => {
    if (pendingAction !== null) return;
    setPendingAction(name);
    setOperationError(null);
    void operation()
      .catch((error: unknown) => {
        setOperationError(
          actionErrorMessage(error, I18N.settings.profileActionFailed),
        );
      })
      .finally(() => setPendingAction(null));
  };

  const runSessionAction = (action: TraySessionAction) => {
    switch (action) {
      case "start":
        performAction(action, start);
        break;
      case "configure":
        performAction(action, () => showSettings("service"));
        break;
      case "pause":
      case "resume":
        performAction(action, togglePaused);
        break;
      case "stop":
        performAction(action, stop);
        break;
      case "connecting":
      case "stopping":
        break;
    }
  };

  // Match the native window to the rendered surface so inline errors and
  // localized labels never leave a transparent click-catching tail.
  useLayoutEffect(() => {
    if (!isTauri || !panelRef.current) return;

    const currentWindow = getCurrentWindow();
    let lastHeight = 0;
    let animationFrame = 0;
    const resize = () => {
      animationFrame = 0;
      const surface = panelRef.current;
      if (!surface) return;
      const height = Math.ceil(
        surface.getBoundingClientRect().height + TRAY_WINDOW_PADDING * 2,
      );
      if (height === lastHeight) return;
      lastHeight = height;
      void currentWindow
        .setSize(new LogicalSize(TRAY_WINDOW_WIDTH, height))
        .catch(() => {});
    };
    const scheduleResize = () => {
      if (animationFrame !== 0) return;
      animationFrame = window.requestAnimationFrame(resize);
    };
    const observer = new ResizeObserver(scheduleResize);
    observer.observe(panelRef.current);
    scheduleResize();

    return () => {
      observer.disconnect();
      if (animationFrame !== 0) window.cancelAnimationFrame(animationFrame);
    };
  }, []);

  useEffect(() => {
    const dismissOnEscape = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      event.preventDefault();
      void hideTrayPanel().catch((error: unknown) => {
        setOperationError(
          actionErrorMessage(error, I18N.settings.profileActionFailed),
        );
      });
    };
    window.addEventListener("keydown", dismissOnEscape);
    return () => window.removeEventListener("keydown", dismissOnEscape);
  }, [hideTrayPanel]);

  const panel = (
    <section
      ref={panelRef}
      className="tray-panel"
      data-state={presentation.visualState}
      aria-label={I18N.tray.appName}
    >
      <header className="tray-header">
        <span className="tray-wordmark" aria-hidden="true">
          <Icon name="captions-bubble" />
        </span>

        <span className="tray-header__copy">
          <strong>{I18N.tray.appName}</strong>
          <span className="tray-status" aria-live="polite">
            <span className="tray-status__dot" aria-hidden="true" />
            <span>{statusText(presentation.statusKind, sessionStatus)}</span>
          </span>
        </span>

        <span
          className="tray-profile"
          title={
            activeProfile
              ? `${activeProfile.name} · ${providerDisplayName(activeProfile.provider)}`
              : I18N.settings.noActiveProfile
          }
          aria-label={`${I18N.settings.currentProfile}: ${activeProfile?.name ?? I18N.settings.noActiveProfile}`}
        >
          <Icon
            name={
              activeProfile?.provider === "openAIRealtime" ? "waves" : "cloud"
            }
          />
          <span>{activeProfile?.name ?? I18N.settings.noActiveProfile}</span>
        </span>
      </header>

      <div
        className="tray-session-actions"
        data-layout={presentation.secondaryAction ? "split" : "single"}
      >
        <SessionActionButton
          presentation={presentation.primaryAction}
          pending={pendingAction === presentation.primaryAction.action}
          blocked={anyActionPending}
          onClick={runSessionAction}
        />
        {presentation.secondaryAction && (
          <SessionActionButton
            presentation={presentation.secondaryAction}
            pending={pendingAction === presentation.secondaryAction.action}
            blocked={anyActionPending}
            onClick={runSessionAction}
            secondary
          />
        )}
      </div>

      <div className="tray-card" aria-label={I18N.settings.subtitleTitle}>
        <label className="tray-setting-row tray-setting-row--language">
          <span className="tray-setting-row__icon" aria-hidden="true">
            <Icon name="languages" />
          </span>
          <span className="tray-setting-row__copy">
            <span>{I18N.tray.sourceLanguage}</span>
            <small>{translationSummary(settings)}</small>
          </span>
          <span className="tray-select-wrap">
            <select
              value={settings.sourceLanguage}
              disabled={sourcePickerDisabled}
              aria-label={I18N.tray.sourceLanguage}
              onChange={(event) => {
                const language = event.target.value as SourceLanguage;
                performAction("language", () =>
                  switchSourceLanguage(language),
                );
              }}
            >
              {sourceLanguages.map((language) => (
                <option key={language} value={language}>
                  {sourceLanguageButtonTitle(language)}
                </option>
              ))}
            </select>
            <Icon name="chevron-down" />
          </span>
        </label>

        <span className="tray-card__divider" />

        <button
          type="button"
          role="switch"
          aria-checked={settings.isOverlayLocked}
          aria-label={I18N.tray.lockPosition}
          disabled={anyActionPending}
          className="tray-setting-row tray-setting-row--toggle"
          onClick={() =>
            performAction("lock", () =>
              setOverlayLocked(!settings.isOverlayLocked),
            )
          }
        >
          <span className="tray-setting-row__icon" aria-hidden="true">
            <Icon name="lock" />
          </span>
          <span className="tray-setting-row__copy">
            <span>{I18N.tray.lockPosition}</span>
          </span>
          <span className="tray-switch" aria-hidden="true">
            <span />
          </span>
        </button>
      </div>

      {(presentation.canShowOverlay || presentation.canClearSubtitles) && (
        <div className="tray-tools">
          <ToolButton
            icon="app-window"
            label={I18N.tray.showSubtitleWindow}
            disabled={anyActionPending || !presentation.canShowOverlay}
            pending={pendingAction === "show"}
            onClick={() => performAction("show", showOverlay)}
          />
          <ToolButton
            icon="eraser"
            label={I18N.tray.clearSubtitles}
            disabled={anyActionPending || !presentation.canClearSubtitles}
            pending={pendingAction === "clear"}
            onClick={() => performAction("clear", clearSubtitles)}
          />
        </div>
      )}

      {operationError && (
        <div className="tray-alert" role="alert">
          <Icon name="exclamation-triangle" />
          <span>{operationError}</span>
        </div>
      )}

      <footer className="tray-footer">
        <button
          type="button"
          disabled={anyActionPending}
          onClick={() => performAction("settings", showSettings)}
        >
          <Icon name="gear" />
          <span>{I18N.tray.settings}</span>
        </button>
        <button
          type="button"
          disabled={anyActionPending}
          onClick={() => performAction("quit", quit)}
        >
          {I18N.tray.quit}
        </button>
      </footer>
    </section>
  );

  return (
    <div className={isTauri ? "tray-shell" : "tray-preview"}>{panel}</div>
  );
}

function SessionActionButton({
  presentation,
  pending,
  blocked,
  onClick,
  secondary = false,
}: {
  presentation: TrayActionPresentation;
  pending: boolean;
  blocked: boolean;
  onClick: (action: TraySessionAction) => void;
  secondary?: boolean;
}) {
  const disabled = presentation.disabled || blocked;
  return (
    <button
      type="button"
      className="tray-session-button"
      data-action={presentation.action}
      data-secondary={secondary || undefined}
      disabled={disabled}
      aria-busy={pending}
      onClick={() => onClick(presentation.action)}
    >
      {pending || presentation.disabled ? (
        <span className="tray-session-button__spinner" aria-hidden="true" />
      ) : (
        <Icon name={sessionActionIcon(presentation.action)} />
      )}
      <span>{sessionActionLabel(presentation.action)}</span>
    </button>
  );
}

function ToolButton({
  icon,
  label,
  disabled,
  pending,
  onClick,
}: {
  icon: IconName;
  label: string;
  disabled: boolean;
  pending: boolean;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      className="tray-tool-button"
      disabled={disabled}
      aria-busy={pending}
      onClick={onClick}
    >
      <span className="tray-tool-button__icon" aria-hidden="true">
        <Icon name={icon} />
      </span>
      <span>{label}</span>
    </button>
  );
}

function statusText(
  kind: TrayStatusKind,
  status: SessionStateEvent["status"],
): string {
  switch (kind) {
    case "ready":
      return I18N.tray.ready;
    case "setupRequired":
      return I18N.tray.setupRequired;
    case "connecting":
      return I18N.tray.connecting;
    case "listening":
      return I18N.tray.listening;
    case "paused":
      return I18N.tray.paused;
    case "stopping":
      return I18N.tray.stopping;
    case "error":
      return status.kind === "error"
        ? status.message
        : I18N.settings.profileActionFailed;
  }
}

function sessionActionLabel(action: TraySessionAction): string {
  switch (action) {
    case "start":
      return I18N.settings.start;
    case "configure":
      return I18N.settings.configureService;
    case "pause":
      return I18N.overlay.pause;
    case "resume":
      return I18N.overlay.resume;
    case "stop":
      return I18N.settings.stop;
    case "connecting":
      return I18N.tray.connecting;
    case "stopping":
      return I18N.tray.stopping;
  }
}

function sessionActionIcon(action: TraySessionAction): IconName {
  switch (action) {
    case "start":
      return "play";
    case "configure":
      return "key";
    case "pause":
      return "pause";
    case "resume":
      return "play";
    case "stop":
      return "stop";
    case "connecting":
    case "stopping":
      return "waves";
  }
}

function translationSummary(settings: SettingsSnapshot) {
  if (!targetLanguageTranslatesAudio(settings.targetLanguage)) {
    return I18N.tray.originalOnly;
  }
  const target = TARGET_LANGUAGE_DISPLAY_NAMES[settings.targetLanguage];
  const mode =
    TRANSLATION_MODE_DISPLAY_NAMES[
      effectiveTranslationModeForSettings(settings)
    ];
  return `${I18N.settings.translateTo} ${target} · ${mode}`;
}
