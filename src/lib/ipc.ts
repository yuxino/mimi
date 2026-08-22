/**
 * Thin wrappers over the Tauri IPC surface defined by the contract in
 * `docs/plans/2026-08-22-multi-provider-professional-settings-design.md`.
 * Command names and payload
 * keys are fixed by that contract and must not drift.
 */

import { invoke } from "@tauri-apps/api/core";
import { emit, listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  SessionStateEvent,
  ServiceProvider,
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

export function profileCreate(
  provider: ServiceProvider,
  name: string,
): Promise<SettingsSnapshot> {
  return invoke<SettingsSnapshot>("profile_create", { provider, name });
}

export function profileUpdate(
  profileId: string,
  name: string,
): Promise<SettingsSnapshot> {
  return invoke<SettingsSnapshot>("profile_update", { profileId, name });
}

export function profileSelect(profileId: string): Promise<SettingsSnapshot> {
  return invoke<SettingsSnapshot>("profile_select", { profileId });
}

export function profileDelete(profileId: string): Promise<SettingsSnapshot> {
  return invoke<SettingsSnapshot>("profile_delete", { profileId });
}

export function profileSaveAPIKey(
  profileId: string,
  apiKey: string,
): Promise<SettingsSnapshot> {
  return invoke<SettingsSnapshot>("profile_save_api_key", {
    profileId,
    apiKey,
  });
}

export function profileDeleteAPIKey(
  profileId: string,
): Promise<SettingsSnapshot> {
  return invoke<SettingsSnapshot>("profile_delete_api_key", { profileId });
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

/** Toggles the language/mode popover window, anchored under the overlay's
 * language capsule (the anchor is derived from the overlay window's own
 * position backend-side). The overlay window itself is never resized for the
 * menu. */
export function overlayPopoverToggle(): Promise<void> {
  return invoke("overlay_popover_toggle");
}

/** Hides the language/mode popover window. */
export function overlayPopoverHide(): Promise<void> {
  return invoke("overlay_popover_hide");
}

/** Fetches the current session state snapshot (for windows that boot after
 * the last session-state broadcast). */
export function sessionGetState(): Promise<SessionStateEvent> {
  return invoke<SessionStateEvent>("session_get_state");
}

export function trayPanelHide(): Promise<void> {
  return invoke("tray_panel_hide");
}

export function appQuit(): Promise<void> {
  return invoke("app_quit");
}

export type SettingsNavigationTarget = "service";

export function appShowSettings(
  target?: SettingsNavigationTarget,
): Promise<void> {
  return invoke("app_show_settings", { target: target ?? null });
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

/** Receives an explicit category intent when another app window opens the
 * settings window for a specific task. */
export function listenSettingsNavigation(
  handler: (target: SettingsNavigationTarget) => void,
): Promise<UnlistenFn> {
  return listen<SettingsNavigationTarget>("settings-navigate", (event) =>
    handler(event.payload),
  );
}

/** Completes the navigation handshake after SettingsView has installed its
 * listener. This matters only when the native window had to be recreated. */
export function announceSettingsNavigationReady(): Promise<void> {
  return emit("settings-navigation-ready");
}
