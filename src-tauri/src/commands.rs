//! Tauri command handlers exposed to the frontend. The IPC contract is
//! documented in docs/plans/2026-08-13-tauri-multiplatform.md.

use crate::core::models::{SourceLanguage, TargetLanguage, TranslationMode};
use crate::session_manager::{SessionManager, SessionStateEvent};
use crate::settings_store::SettingsStore;
use crate::windows::{OverlayWindowManager, TrayPanelManager};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State};

pub struct AppState {
    pub settings: Arc<SettingsStore>,
    pub session: Arc<SessionManager>,
    /// Single source of truth for the overlay window geometry.
    pub overlay: Arc<std::sync::Mutex<crate::windows::OverlayState>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsSnapshotPayload {
    #[serde(rename = "workspaceID")]
    pub workspace_id: String,
    /// Loaded from the OS keychain so the settings field can be prefilled,
    /// mirroring the original app's in-memory binding. Never persisted.
    #[serde(rename = "apiKey")]
    pub api_key: String,
    #[serde(rename = "hasAPIKey")]
    pub has_api_key: bool,
    pub source_language: SourceLanguage,
    pub target_language: TargetLanguage,
    pub translation_mode: TranslationMode,
    pub font_size: f64,
    #[serde(rename = "isOverlayLocked")]
    pub is_overlay_locked: bool,
    #[serde(rename = "credentialLoadError")]
    pub credential_load_error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The `workspaceID` key must survive camelCase renaming (serde would
    /// otherwise emit `workspaceId`, which the frontend does not read).
    #[test]
    fn settings_payload_uses_workspace_id_key() {
        let payload = SettingsSnapshotPayload {
            workspace_id: "ws-abc123".into(),
            api_key: "sk-demo".into(),
            has_api_key: true,
            source_language: SourceLanguage::Japanese,
            target_language: TargetLanguage::SimplifiedChinese,
            translation_mode: TranslationMode::HighQuality,
            font_size: 18.0,
            is_overlay_locked: false,
            credential_load_error: None,
        };
        let json = serde_json::to_value(&payload).unwrap();
        assert_eq!(json["workspaceID"], "ws-abc123");
        assert!(json.get("workspaceId").is_none());
        assert_eq!(json["apiKey"], "sk-demo");
        assert_eq!(json["hasAPIKey"], true);
    }

    #[test]
    fn settings_draft_reads_workspace_id_key() {
        let draft: SettingsDraft = serde_json::from_str(r#"{"workspaceID":"ws-abc123"}"#).unwrap();
        assert_eq!(draft.workspace_id.as_deref(), Some("ws-abc123"));
    }
}

impl SettingsSnapshotPayload {
    pub fn from_store(store: &SettingsStore) -> Self {
        let prefs = store.preferences();
        let api_key = store.load_api_key().ok().flatten().unwrap_or_default();
        Self {
            workspace_id: prefs.workspace_id,
            has_api_key: !api_key.is_empty(),
            api_key,
            source_language: prefs.source_language,
            target_language: prefs.target_language,
            translation_mode: prefs.translation_mode,
            font_size: prefs.font_size,
            is_overlay_locked: prefs.overlay_locked,
            credential_load_error: store.credential_load_error(),
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SettingsDraft {
    #[serde(rename = "workspaceID")]
    pub workspace_id: Option<String>,
    pub api_key: Option<String>,
    pub source_language: Option<SourceLanguage>,
    pub target_language: Option<TargetLanguage>,
    pub translation_mode: Option<TranslationMode>,
    pub font_size: Option<f64>,
    pub is_overlay_locked: Option<bool>,
}

#[tauri::command]
pub fn settings_get(state: State<'_, AppState>) -> Result<SettingsSnapshotPayload, String> {
    tracing::info!(
        "settings_get invoked from frontend hasKey={} keyLen={}",
        u8::from(state.settings.has_api_key()),
        state
            .settings
            .load_api_key()
            .ok()
            .flatten()
            .map(|k| k.len())
            .unwrap_or(0)
    );
    Ok(SettingsSnapshotPayload::from_store(&state.settings))
}

#[tauri::command]
pub fn settings_save(
    app: AppHandle,
    state: State<'_, AppState>,
    draft: SettingsDraft,
) -> Result<SettingsSnapshotPayload, String> {
    let mut credential_changed = false;
    {
        let mut needs_save = false;
        state.settings.update_preferences(|prefs| {
            if let Some(workspace_id) = draft.workspace_id.clone() {
                prefs.workspace_id = workspace_id;
                needs_save = true;
                credential_changed = true;
            }
            if let Some(source_language) = draft.source_language {
                prefs.source_language = source_language;
                needs_save = true;
            }
            if let Some(target_language) = draft.target_language {
                prefs.target_language = target_language;
                needs_save = true;
            }
            if let Some(translation_mode) = draft.translation_mode {
                prefs.translation_mode = translation_mode;
                needs_save = true;
            }
            if let Some(font_size) = draft.font_size {
                prefs.font_size = font_size.clamp(
                    *crate::settings_store::FONT_SIZE_RANGE.start(),
                    *crate::settings_store::FONT_SIZE_RANGE.end(),
                );
                needs_save = true;
            }
            if let Some(locked) = draft.is_overlay_locked {
                prefs.overlay_locked = locked;
                needs_save = true;
            }
        });
        if draft.api_key.is_some() {
            credential_changed = true;
            needs_save = true;
        }
        if !needs_save {
            return Ok(SettingsSnapshotPayload::from_store(&state.settings));
        }
    }

    if credential_changed {
        let prefs = state.settings.preferences();
        let api_key = draft.api_key.unwrap_or_else(|| {
            state
                .settings
                .load_api_key()
                .ok()
                .flatten()
                .unwrap_or_default()
        });
        state
            .settings
            .save_credentials(&prefs.workspace_id, &api_key)?;
    } else {
        state.settings.persist();
    }

    if let Some(locked) = draft.is_overlay_locked {
        OverlayWindowManager::update_locked(&app, locked);
    }

    let payload = SettingsSnapshotPayload::from_store(&state.settings);
    let _ = app.emit("settings-changed", payload.clone());
    Ok(payload)
}

#[tauri::command]
pub async fn session_start(state: State<'_, AppState>) -> Result<(), String> {
    state.session.start(true).await
}

#[tauri::command]
pub async fn session_stop(state: State<'_, AppState>) -> Result<(), String> {
    state.session.stop().await;
    Ok(())
}

#[tauri::command]
pub async fn session_toggle_paused(state: State<'_, AppState>) -> Result<(), String> {
    state.session.toggle_paused().await;
    Ok(())
}

#[tauri::command]
pub async fn session_clear_subtitles(state: State<'_, AppState>) -> Result<(), String> {
    state.session.clear_subtitles();
    Ok(())
}

#[tauri::command]
pub async fn session_switch_source_language(
    state: State<'_, AppState>,
    language: SourceLanguage,
) -> Result<(), String> {
    // The session manager broadcasts settings-changed immediately after the
    // preference write, so no window keeps a stale selection while the
    // reconnect (which this awaits) is still in flight.
    state.session.switch_source_language(language).await;
    Ok(())
}

#[tauri::command]
pub async fn session_switch_translation_mode(
    state: State<'_, AppState>,
    mode: TranslationMode,
) -> Result<(), String> {
    state.session.switch_translation_mode(mode).await;
    Ok(())
}

#[tauri::command]
pub fn overlay_set_collapsed(
    app: AppHandle,
    state: State<'_, AppState>,
    collapsed: bool,
) -> Result<(), String> {
    state.session.set_overlay_collapsed(collapsed);
    OverlayWindowManager::set_collapsed(&app, &state.overlay, collapsed);
    // The language/mode menu cannot stay anchored to a capsule that is no
    // longer visible.
    crate::windows::LanguagePopoverManager::hide(&app);
    // The frontend's collapse UI state only updates through the
    // session-state event; without this the overlay renders the wrong
    // layout after collapsing/expanding.
    state.session.publish_state();
    Ok(())
}

#[tauri::command]
pub fn overlay_set_locked(
    app: AppHandle,
    state: State<'_, AppState>,
    locked: bool,
) -> Result<(), String> {
    state.settings.update_preferences(|prefs| {
        prefs.overlay_locked = locked;
    });
    state.settings.persist();
    OverlayWindowManager::update_locked(&app, locked);
    let _ = app.emit(
        "settings-changed",
        SettingsSnapshotPayload::from_store(&state.settings),
    );
    Ok(())
}

#[tauri::command]
pub fn overlay_show(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    if state.session.is_active() {
        OverlayWindowManager::show(&app);
    }
    Ok(())
}

#[tauri::command]
pub fn overlay_set_size(
    app: AppHandle,
    state: State<'_, AppState>,
    width: f64,
    height: f64,
) -> Result<(), String> {
    OverlayWindowManager::set_size(&app, &state.overlay, width, height);
    Ok(())
}

/// Toggles the language/mode popover window, anchored under the overlay's
/// language capsule (the anchor is derived from the overlay window's own
/// position). The overlay window itself is never resized for the menu.
/// Opening refreshes the popover's snapshots so its checkmarks always match
/// the current state, even if its webview missed events while hidden.
#[tauri::command]
pub fn overlay_popover_toggle(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let was_visible = app
        .get_webview_window("language-popover")
        .and_then(|window| window.is_visible().ok())
        .unwrap_or(false);
    crate::windows::LanguagePopoverManager::toggle(&app);
    if !was_visible {
        state.session.publish_settings();
        state.session.publish_state();
    }
    Ok(())
}

/// Hides the language/mode popover window (no-op when already hidden).
#[tauri::command]
pub fn overlay_popover_hide(app: AppHandle) -> Result<(), String> {
    crate::windows::LanguagePopoverManager::hide(&app);
    Ok(())
}

/// The current session state snapshot, for windows that boot after the last
/// broadcast (e.g. the language popover).
#[tauri::command]
pub fn session_get_state(state: State<'_, AppState>) -> Result<SessionStateEvent, String> {
    Ok(state.session.current_state_event())
}

#[tauri::command]
pub fn overlay_commit_frame(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    OverlayWindowManager::commit(&app, &state.overlay, &state.settings);
    Ok(())
}

/// Begins an overlay resize drag. `region` is one of topLeft/top/topRight/
/// left/right/bottomLeft/bottom/bottomRight; `x`/`y` are the pointer position
/// in screen (logical) coordinates. Ignored unless the overlay is expanded.
#[tauri::command]
pub fn resize_start(
    app: AppHandle,
    state: State<'_, AppState>,
    region: String,
    x: f64,
    y: f64,
) -> Result<(), String> {
    OverlayWindowManager::resize_start(&app, &state.overlay, &region, x, y)
}

/// Continues a resize drag with the current pointer position (screen logical
/// px). The dragged edge/corner stays anchored, sizes clamp to min/max, and
/// the frame keeps at least 48 px on screen.
#[tauri::command]
pub fn resize_move(
    app: AppHandle,
    state: State<'_, AppState>,
    x: f64,
    y: f64,
) -> Result<(), String> {
    OverlayWindowManager::resize_move(&app, &state.overlay, x, y);
    Ok(())
}

/// Ends a resize drag and commits the final frame.
#[tauri::command]
pub fn resize_end(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    OverlayWindowManager::resize_end(&app, &state.overlay, &state.settings);
    Ok(())
}

#[tauri::command]
pub fn tray_panel_hide(app: AppHandle) -> Result<(), String> {
    TrayPanelManager::hide(&app);
    Ok(())
}

#[tauri::command]
pub fn app_show_settings(app: AppHandle) -> Result<(), String> {
    // Close the tray panel first: it is always-on-top, so the settings
    // window would otherwise open behind it and the click would look like a
    // no-op. Same for the language popover, if open.
    TrayPanelManager::hide(&app);
    crate::windows::LanguagePopoverManager::hide(&app);
    crate::windows::ensure_settings_window(&app);
    Ok(())
}

#[tauri::command]
pub fn app_quit(app: AppHandle) -> Result<(), String> {
    app.exit(0);
    Ok(())
}

/// Content-free UI diagnostics probe used by the launch-time DOM check.
#[tauri::command]
pub fn ui_probe_report(window: String, state: String) -> Result<(), String> {
    tracing::info!("ui probe window={} state={}", window, state);
    Ok(())
}
