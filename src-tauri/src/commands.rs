//! Tauri command handlers exposed to the frontend. The IPC contract is
//! documented in docs/plans/2026-08-22-multi-provider-professional-settings-design.md.

use crate::core::models::{SourceLanguage, TargetLanguage, TranslationMode};
use crate::core::provider::{ProviderKind, ServiceProfile};
use crate::session_manager::{SessionManager, SessionStateEvent};
use crate::settings_store::{CredentialState, SettingsStore, SubtitleAlignment};
use crate::windows::{OverlayWindowManager, TrayPanelManager};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Listener, Manager, State};

const SETTINGS_NAVIGATION_EVENT: &str = "settings-navigate";
const SETTINGS_NAVIGATION_READY_EVENT: &str = "settings-navigation-ready";

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SettingsNavigationTarget {
    Service,
}

pub struct AppState {
    pub settings: Arc<SettingsStore>,
    pub session: Arc<SessionManager>,
    /// Single source of truth for the overlay window geometry.
    pub overlay: Arc<std::sync::Mutex<crate::windows::OverlayState>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceProfilePayload {
    pub id: String,
    pub name: String,
    pub provider: ProviderKind,
    pub credential_state: CredentialState,
}

impl ServiceProfilePayload {
    fn from_profile(store: &SettingsStore, profile: ServiceProfile) -> Self {
        let credential_state = store.credential_state(&profile);
        Self {
            id: profile.id,
            name: profile.name,
            provider: profile.provider,
            credential_state,
        }
    }

    fn unavailable(profile: ServiceProfile) -> Self {
        Self {
            id: profile.id,
            name: profile.name,
            provider: profile.provider,
            credential_state: CredentialState::Unavailable,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsSnapshotPayload {
    pub profiles: Vec<ServiceProfilePayload>,
    pub active_profile_id: String,
    pub source_language: SourceLanguage,
    pub target_language: TargetLanguage,
    pub translation_mode: TranslationMode,
    pub font_size: f64,
    pub subtitle_alignment: SubtitleAlignment,
    pub subtitle_blends_with_background: bool,
    #[serde(rename = "isOverlayLocked")]
    pub is_overlay_locked: bool,
    #[serde(rename = "uiLanguage")]
    pub ui_language: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    struct PartiallyUnavailableSecretStore;

    impl crate::settings_store::SecretStore for PartiallyUnavailableSecretStore {
        fn load(
            &self,
            _service: &str,
            account: &str,
        ) -> Result<Option<String>, crate::settings_store::SecretStoreError> {
            if account.contains("alibaba-default") {
                Err(crate::settings_store::SecretStoreError::Unavailable)
            } else {
                Ok(None)
            }
        }

        fn save(
            &self,
            _service: &str,
            _account: &str,
            _value: &str,
        ) -> Result<(), crate::settings_store::SecretStoreError> {
            Ok(())
        }

        fn delete(
            &self,
            _service: &str,
            _account: &str,
        ) -> Result<(), crate::settings_store::SecretStoreError> {
            Ok(())
        }
    }

    #[test]
    fn settings_payload_is_camel_case_and_write_only() {
        let payload = SettingsSnapshotPayload {
            profiles: vec![ServiceProfilePayload {
                id: "alibaba-default".into(),
                name: "Alibaba Cloud".into(),
                provider: ProviderKind::AlibabaCloud,
                credential_state: CredentialState::Present,
            }],
            active_profile_id: "alibaba-default".into(),
            source_language: SourceLanguage::Japanese,
            target_language: TargetLanguage::SimplifiedChinese,
            translation_mode: TranslationMode::HighQuality,
            font_size: 18.0,
            subtitle_alignment: SubtitleAlignment::Center,
            subtitle_blends_with_background: false,
            is_overlay_locked: false,
            ui_language: None,
        };
        let json = serde_json::to_value(&payload).unwrap();
        assert_eq!(json["activeProfileId"], "alibaba-default");
        assert_eq!(json["profiles"][0]["provider"], "alibabaCloud");
        assert_eq!(json["profiles"][0]["credentialState"], "present");
        assert_eq!(json["subtitleAlignment"], "center");
        assert_eq!(json["subtitleBlendsWithBackground"], false);
        assert!(json.get("apiKey").is_none());
        assert!(json.get("hasAPIKey").is_none());
        assert!(!json.to_string().contains("secret"));
    }

    #[test]
    fn active_session_rejects_every_profile_mutation() {
        assert!(ensure_profile_mutation_allowed(true).is_err());
        assert!(ensure_profile_mutation_allowed(false).is_ok());
    }

    #[test]
    fn active_session_rejects_pipeline_settings_but_allows_visual_settings() {
        let pipeline = SettingsDraft {
            source_language: Some(SourceLanguage::Japanese),
            ..SettingsDraft::default()
        };
        assert!(ensure_settings_draft_allowed(&pipeline, true).is_err());

        let visual = SettingsDraft {
            font_size: Some(19.0),
            subtitle_alignment: Some(SubtitleAlignment::Right),
            subtitle_blends_with_background: Some(true),
            is_overlay_locked: Some(true),
            ui_language: Some("ja".into()),
            ..SettingsDraft::default()
        };
        assert!(ensure_settings_draft_allowed(&visual, true).is_ok());
    }

    #[test]
    fn one_unavailable_credential_does_not_fail_the_snapshot() {
        let store = SettingsStore::in_memory(Box::new(PartiallyUnavailableSecretStore), false);
        store
            .create_profile(ProviderKind::OpenAIRealtime, "OpenAI")
            .unwrap();

        let payload = SettingsSnapshotPayload::try_from_store(&store).unwrap();
        assert_eq!(payload.profiles.len(), 2);
        assert_eq!(
            payload.profiles[0].credential_state,
            CredentialState::Unavailable
        );
        assert_eq!(
            payload.profiles[1].credential_state,
            CredentialState::Missing
        );
    }

    #[test]
    fn settings_navigation_target_accepts_only_known_sections() {
        assert_eq!(
            serde_json::from_str::<SettingsNavigationTarget>(r#""service""#).unwrap(),
            SettingsNavigationTarget::Service
        );
        assert!(serde_json::from_str::<SettingsNavigationTarget>(r#""general""#).is_err());
    }
}

impl SettingsSnapshotPayload {
    pub fn from_store(store: &SettingsStore) -> Self {
        match Self::try_from_store(store) {
            Ok(payload) => payload,
            Err(_) => {
                let prefs = store.preferences();
                let (active_profile_id, profiles) = store.profile_catalog_or_default();
                Self {
                    profiles: profiles
                        .into_iter()
                        .map(ServiceProfilePayload::unavailable)
                        .collect(),
                    active_profile_id,
                    source_language: prefs.source_language,
                    target_language: prefs.target_language,
                    translation_mode: prefs.translation_mode,
                    font_size: prefs.font_size,
                    subtitle_alignment: prefs.subtitle_alignment,
                    subtitle_blends_with_background: prefs.subtitle_blends_with_background,
                    is_overlay_locked: prefs.overlay_locked,
                    ui_language: prefs.ui_language,
                }
            }
        }
    }

    pub fn try_from_store(store: &SettingsStore) -> Result<Self, String> {
        let prefs = store.preferences();
        let (active_profile_id, profiles) = store.profile_catalog()?;
        Ok(Self {
            profiles: profiles
                .into_iter()
                .map(|profile| ServiceProfilePayload::from_profile(store, profile))
                .collect(),
            active_profile_id,
            source_language: prefs.source_language,
            target_language: prefs.target_language,
            translation_mode: prefs.translation_mode,
            font_size: prefs.font_size,
            subtitle_alignment: prefs.subtitle_alignment,
            subtitle_blends_with_background: prefs.subtitle_blends_with_background,
            is_overlay_locked: prefs.overlay_locked,
            ui_language: prefs.ui_language,
        })
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SettingsDraft {
    pub source_language: Option<SourceLanguage>,
    pub target_language: Option<TargetLanguage>,
    pub translation_mode: Option<TranslationMode>,
    pub font_size: Option<f64>,
    pub subtitle_alignment: Option<SubtitleAlignment>,
    pub subtitle_blends_with_background: Option<bool>,
    pub is_overlay_locked: Option<bool>,
    pub ui_language: Option<String>,
}

/// Reads public settings and per-profile credential presence. API-key values
/// never enter this payload. Async keeps OS credential-store reads off wry's
/// main-thread path.
#[tauri::command]
pub async fn settings_get(state: State<'_, AppState>) -> Result<SettingsSnapshotPayload, String> {
    Ok(SettingsSnapshotPayload::from_store(&state.settings))
}

/// Saves non-secret preferences only. Credentials use dedicated write-only
/// commands so a general settings draft can never echo or overwrite a key.
#[tauri::command]
pub async fn settings_save(
    app: AppHandle,
    state: State<'_, AppState>,
    draft: SettingsDraft,
) -> Result<SettingsSnapshotPayload, String> {
    let changes_listening_settings = draft.source_language.is_some()
        || draft.target_language.is_some()
        || draft.translation_mode.is_some();
    let changes_ui_language = draft.ui_language.is_some();
    let _lifecycle = state
        .session
        .settings_mutation_guard(changes_listening_settings)
        .await?;
    ensure_settings_draft_allowed(&draft, state.session.has_active_session())?;
    let needs_save = draft.source_language.is_some()
        || draft.target_language.is_some()
        || draft.translation_mode.is_some()
        || draft.font_size.is_some()
        || draft.subtitle_alignment.is_some()
        || draft.subtitle_blends_with_background.is_some()
        || draft.is_overlay_locked.is_some()
        || draft.ui_language.is_some();
    if !needs_save {
        return SettingsSnapshotPayload::try_from_store(&state.settings);
    }

    state
        .settings
        .save_preferences_for_active_profile(|prefs| {
            if let Some(source_language) = draft.source_language {
                prefs.source_language = source_language;
            }
            if let Some(target_language) = draft.target_language {
                prefs.target_language = target_language;
            }
            if let Some(translation_mode) = draft.translation_mode {
                prefs.translation_mode = translation_mode;
            }
            if let Some(font_size) = draft.font_size {
                prefs.font_size = font_size;
            }
            if let Some(alignment) = draft.subtitle_alignment {
                prefs.subtitle_alignment = alignment;
            }
            if let Some(blends) = draft.subtitle_blends_with_background {
                prefs.subtitle_blends_with_background = blends;
            }
            if let Some(locked) = draft.is_overlay_locked {
                prefs.overlay_locked = locked;
            }
            if let Some(language) = &draft.ui_language {
                prefs.ui_language = Some(language.clone());
            }
        })?;
    if draft.is_overlay_locked.is_some() || draft.subtitle_blends_with_background.is_some() {
        let preferences = state.settings.preferences();
        OverlayWindowManager::update_locked(
            &app,
            preferences.overlay_locked || preferences.subtitle_blends_with_background,
        );
    }
    if changes_ui_language {
        crate::refresh_native_tray_language(&app);
    }

    let payload = SettingsSnapshotPayload::try_from_store(&state.settings)?;
    let _ = app.emit("settings-changed", payload.clone());
    Ok(payload)
}

fn ensure_settings_draft_allowed(draft: &SettingsDraft, is_active: bool) -> Result<(), String> {
    if is_active
        && (draft.source_language.is_some()
            || draft.target_language.is_some()
            || draft.translation_mode.is_some())
    {
        Err(
            "Listening settings cannot be changed through settings while a session is active."
                .to_string(),
        )
    } else {
        Ok(())
    }
}

fn ensure_profile_mutation_allowed(is_active: bool) -> Result<(), String> {
    if is_active {
        Err("Service profiles cannot be changed while a session is active.".to_string())
    } else {
        Ok(())
    }
}

fn emit_settings_snapshot(
    app: &AppHandle,
    settings: &SettingsStore,
) -> Result<SettingsSnapshotPayload, String> {
    let payload = SettingsSnapshotPayload::try_from_store(settings)?;
    let _ = app.emit("settings-changed", payload.clone());
    Ok(payload)
}

#[tauri::command]
pub async fn profile_create(
    app: AppHandle,
    state: State<'_, AppState>,
    provider: ProviderKind,
    name: String,
) -> Result<SettingsSnapshotPayload, String> {
    let _lifecycle = state.session.settings_mutation_guard(true).await?;
    ensure_profile_mutation_allowed(state.session.has_active_session())?;
    state.settings.create_profile(provider, &name)?;
    emit_settings_snapshot(&app, &state.settings)
}

#[tauri::command]
pub async fn profile_update(
    app: AppHandle,
    state: State<'_, AppState>,
    profile_id: String,
    name: String,
) -> Result<SettingsSnapshotPayload, String> {
    let _lifecycle = state.session.settings_mutation_guard(true).await?;
    ensure_profile_mutation_allowed(state.session.has_active_session())?;
    state.settings.update_profile(&profile_id, &name)?;
    emit_settings_snapshot(&app, &state.settings)
}

#[tauri::command]
pub async fn profile_select(
    app: AppHandle,
    state: State<'_, AppState>,
    profile_id: String,
) -> Result<SettingsSnapshotPayload, String> {
    let _lifecycle = state.session.settings_mutation_guard(true).await?;
    ensure_profile_mutation_allowed(state.session.has_active_session())?;
    state.settings.select_profile(&profile_id)?;
    emit_settings_snapshot(&app, &state.settings)
}

#[tauri::command]
pub async fn profile_delete(
    app: AppHandle,
    state: State<'_, AppState>,
    profile_id: String,
) -> Result<SettingsSnapshotPayload, String> {
    let _lifecycle = state.session.settings_mutation_guard(true).await?;
    ensure_profile_mutation_allowed(state.session.has_active_session())?;
    state.settings.delete_profile(&profile_id)?;
    emit_settings_snapshot(&app, &state.settings)
}

#[tauri::command]
pub async fn profile_save_api_key(
    app: AppHandle,
    state: State<'_, AppState>,
    profile_id: String,
    api_key: String,
) -> Result<SettingsSnapshotPayload, String> {
    let _lifecycle = state.session.settings_mutation_guard(true).await?;
    ensure_profile_mutation_allowed(state.session.has_active_session())?;
    state.settings.save_api_key(&profile_id, &api_key)?;
    emit_settings_snapshot(&app, &state.settings)
}

#[tauri::command]
pub async fn profile_delete_api_key(
    app: AppHandle,
    state: State<'_, AppState>,
    profile_id: String,
) -> Result<SettingsSnapshotPayload, String> {
    let _lifecycle = state.session.settings_mutation_guard(true).await?;
    ensure_profile_mutation_allowed(state.session.has_active_session())?;
    state.settings.delete_api_key(&profile_id)?;
    emit_settings_snapshot(&app, &state.settings)
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
    state
        .settings
        .save_preferences(|prefs| prefs.overlay_locked = locked)?;
    let preferences = state.settings.preferences();
    OverlayWindowManager::update_locked(
        &app,
        locked || preferences.subtitle_blends_with_background,
    );
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
/// broadcast (e.g. the language popover). Async: cloning the full controller
/// state (subtitle history included) must not run on the main thread.
#[tauri::command]
pub async fn session_get_state(state: State<'_, AppState>) -> Result<SessionStateEvent, String> {
    Ok(state.session.current_state_event())
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

fn emit_settings_navigation(app: &AppHandle, target: SettingsNavigationTarget) {
    if let Some(window) = app.get_webview_window("settings") {
        let _ = window.emit(SETTINGS_NAVIGATION_EVENT, target);
    }
}

#[tauri::command]
pub fn app_show_settings(
    app: AppHandle,
    target: Option<SettingsNavigationTarget>,
) -> Result<(), String> {
    // Close the tray panel first: it is always-on-top, so the settings
    // window would otherwise open behind it and the click would look like a
    // no-op. Same for the language popover, if open.
    TrayPanelManager::hide(&app);
    crate::windows::LanguagePopoverManager::hide(&app);

    // The settings window normally exists for the whole app lifetime and is
    // merely hidden on close. If its webview crashed, however, recreating it
    // and emitting immediately would race the frontend listener. In that
    // recovery path, wait for SettingsView to announce that its navigation
    // listener is installed before delivering the one-shot intent.
    let settings_window_exists = app.get_webview_window("settings").is_some();
    if !settings_window_exists {
        if let Some(target) = target {
            let ready_app = app.clone();
            app.once(SETTINGS_NAVIGATION_READY_EVENT, move |_| {
                emit_settings_navigation(&ready_app, target);
            });
        }
    }

    crate::windows::ensure_settings_window(&app);
    if settings_window_exists {
        if let Some(target) = target {
            emit_settings_navigation(&app, target);
        }
    }
    Ok(())
}

#[tauri::command]
pub fn app_quit(app: AppHandle) -> Result<(), String> {
    app.exit(0);
    Ok(())
}
