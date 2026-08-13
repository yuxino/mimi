/**
 * Thin wrappers over the Tauri IPC surface defined by the contract in
 * `docs/plans/2026-08-13-tauri-multiplatform.md`. Command names and payload
 * keys are fixed by that contract and must not drift.
 */

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  SessionStateEvent,
  SettingsDraft,
  SettingsSnapshot,
  SourceLanguage,
  TranslationMode,
} from "./types";

/**
 * Whether the app is running inside Tauri. In a plain `vite dev` session there
 * are no `__TAURI_INTERNALS__`, so the store falls back to mock behavior.
 */
export const isTauri =
  typeof window !== "undefined" &&
  (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ !==
    undefined;

// ---------------------------------------------------------------------------
// Commands (frontend -> Rust)
// ---------------------------------------------------------------------------

export function sessionStart(): Promise<void> {
  return invoke("session_start");
}

export function sessionStop(): Promise<void> {
  return invoke("session_stop");
}

export function sessionTogglePaused(): Promise<void> {
  return invoke("session_toggle_paused");
}

export function sessionClearSubtitles(): Promise<void> {
  return invoke("session_clear_subtitles");
}

export function sessionSwitchSourceLanguage(
  language: SourceLanguage,
): Promise<void> {
  return invoke("session_switch_source_language", { language });
}

export function sessionSwitchTranslationMode(
  mode: TranslationMode,
): Promise<void> {
  return invoke("session_switch_translation_mode", { mode });
}

export function settingsGet(): Promise<SettingsSnapshot> {
  return invoke<SettingsSnapshot>("settings_get");
}

export function settingsSave(draft: SettingsDraft): Promise<SettingsSnapshot> {
  return invoke<SettingsSnapshot>("settings_save", { draft });
}

export function overlaySetCollapsed(collapsed: boolean): Promise<void> {
  return invoke("overlay_set_collapsed", { collapsed });
}

export function overlaySetLocked(locked: boolean): Promise<void> {
  return invoke("overlay_set_locked", { locked });
}

export function overlayShow(): Promise<void> {
  return invoke("overlay_show");
}

export function overlaySetSize(width: number, height: number): Promise<void> {
  return invoke("overlay_set_size", { width, height });
}

/** Temporarily enlarges the overlay so the language/mode popover fits;
 * `height <= 0` restores the remembered expanded height. */
export function overlaySetHeight(height: number): Promise<void> {
  return invoke("overlay_set_height", { height });
}

/** Persists the current overlay frame (end of a drag-resize or popover
 * close). */
export function overlayCommitFrame(): Promise<void> {
  return invoke("overlay_commit_frame");
}

export function trayPanelHide(): Promise<void> {
  return invoke("tray_panel_hide");
}

export function appQuit(): Promise<void> {
  return invoke("app_quit");
}

export function appShowSettings(): Promise<void> {
  return invoke("app_show_settings");
}

// ---------------------------------------------------------------------------
// Events (Rust -> frontend)
// ---------------------------------------------------------------------------

export function listenSessionState(
  handler: (state: SessionStateEvent) => void,
): Promise<UnlistenFn> {
  return listen<SessionStateEvent>("session-state", (event) =>
    handler(event.payload),
  );
}

export function listenSettingsChanged(
  handler: (settings: SettingsSnapshot) => void,
): Promise<UnlistenFn> {
  return listen<SettingsSnapshot>("settings-changed", (event) =>
    handler(event.payload),
  );
}
