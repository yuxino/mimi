import type {
  CredentialState,
  SessionStateEvent,
  SubtitleSnapshot,
} from "../../lib/types";

type TrayVisualState =
  | "ready"
  | "setup"
  | "connecting"
  | "listening"
  | "paused"
  | "stopping"
  | "error";

export type TrayStatusKind =
  | "ready"
  | "setupRequired"
  | "connecting"
  | "listening"
  | "paused"
  | "stopping"
  | "error";

export type TraySessionAction =
  | "start"
  | "configure"
  | "pause"
  | "resume"
  | "stop"
  | "connecting"
  | "stopping";

export interface TrayActionPresentation {
  action: TraySessionAction;
  disabled: boolean;
}

interface TrayPresentation {
  visualState: TrayVisualState;
  statusKind: TrayStatusKind;
  primaryAction: TrayActionPresentation;
  secondaryAction: TrayActionPresentation | null;
  canChangeSourceLanguage: boolean;
  canShowOverlay: boolean;
  canClearSubtitles: boolean;
}

interface TrayPresentationInput {
  status: SessionStateEvent["status"];
  isPaused: boolean;
  credentialState: CredentialState | undefined;
  hasSubtitleContent: boolean;
}

/**
 * Converts backend state into the small, explicit set of tray states. Keeping
 * this pure prevents transitional or stale snapshots from accidentally
 * exposing actions that the session manager cannot safely perform.
 */
export function deriveTrayPresentation({
  status,
  isPaused,
  credentialState,
  hasSubtitleContent,
}: TrayPresentationInput): TrayPresentation {
  const shared = {
    canClearSubtitles: hasSubtitleContent,
  };

  // Transitional states take priority over a briefly stale pause flag.
  if (status.kind === "connecting") {
    return {
      ...shared,
      visualState: "connecting",
      statusKind: "connecting",
      primaryAction: { action: "connecting", disabled: true },
      secondaryAction: null,
      canChangeSourceLanguage: false,
      canShowOverlay: false,
    };
  }

  if (status.kind === "stopping") {
    return {
      ...shared,
      visualState: "stopping",
      statusKind: "stopping",
      primaryAction: { action: "stopping", disabled: true },
      secondaryAction: null,
      canChangeSourceLanguage: false,
      canShowOverlay: false,
    };
  }

  if (isPaused) {
    return {
      ...shared,
      visualState: "paused",
      statusKind: "paused",
      primaryAction: { action: "resume", disabled: false },
      secondaryAction: { action: "stop", disabled: false },
      canChangeSourceLanguage: false,
      canShowOverlay: status.kind === "listening",
    };
  }

  if (status.kind === "listening") {
    return {
      ...shared,
      visualState: "listening",
      statusKind: "listening",
      primaryAction: { action: "pause", disabled: false },
      secondaryAction: { action: "stop", disabled: false },
      canChangeSourceLanguage: true,
      canShowOverlay: true,
    };
  }

  const hasCredential = credentialState === "present";
  const primaryAction: TrayActionPresentation = hasCredential
    ? { action: "start", disabled: false }
    : { action: "configure", disabled: false };

  if (status.kind === "error") {
    return {
      ...shared,
      visualState: "error",
      statusKind: "error",
      primaryAction,
      secondaryAction: null,
      canChangeSourceLanguage: true,
      canShowOverlay: hasSubtitleContent,
    };
  }

  return {
    ...shared,
    visualState: hasCredential ? "ready" : "setup",
    statusKind: hasCredential ? "ready" : "setupRequired",
    primaryAction,
    secondaryAction: null,
    canChangeSourceLanguage: true,
    canShowOverlay: hasSubtitleContent,
  };
}

export function hasSubtitleContent(subtitles: SubtitleSnapshot): boolean {
  return (
    subtitles.history.length > 0 ||
    subtitles.source.text.trim().length > 0 ||
    subtitles.translation.text.trim().length > 0
  );
}

export function actionErrorMessage(error: unknown, fallback: string): string {
  if (error instanceof Error && error.message.trim()) return error.message;
  if (typeof error === "string" && error.trim()) return error;
  return fallback;
}
