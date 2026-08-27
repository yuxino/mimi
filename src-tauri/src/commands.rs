//! Tauri command handlers exposed to the frontend. The IPC contract is
//! documented in docs/plans/2026-08-22-multi-provider-professional-settings-design.md.

use crate::core::credentials::ProviderCredentials;
use crate::core::models::{SourceLanguage, TargetLanguage, TranslationMode};
use crate::core::provider::{ProviderKind, ServiceProfile};
use crate::core::update::compare_release_versions;
use crate::session_manager::{SessionManager, SessionStateEvent};
use crate::settings_store::{CredentialState, SettingsStore, SubtitleAlignment};
use crate::windows::{
    OverlayControlMode, OverlayControlWindowManager, OverlayWindowManager, TrayPanelManager,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Listener, Manager, State};
use tauri_plugin_opener::OpenerExt;

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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppUpdateCheckPayload {
    pub current_version: String,
    pub latest_version: String,
    pub update_available: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn background_blend_forces_expanded_overlay() {
        assert!(!normalize_overlay_collapsed(true, true));
        assert!(!normalize_overlay_collapsed(false, true));
        assert!(normalize_overlay_collapsed(true, false));
    }

    #[test]
    fn immersive_mode_toggle_inverts_the_current_preference() {
        assert!(toggled_immersive_mode(false));
        assert!(!toggled_immersive_mode(true));
    }

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
    fn update_check_payload_uses_the_frontend_contract() {
        let payload = AppUpdateCheckPayload {
            current_version: "1.3.1".into(),
            latest_version: "1.4.0".into(),
            update_available: true,
        };

        let json = serde_json::to_value(payload).unwrap();
        assert_eq!(json["currentVersion"], "1.3.1");
        assert_eq!(json["latestVersion"], "1.4.0");
        assert_eq!(json["updateAvailable"], true);
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

/// Checks the fixed public mimi repository only when requested by the settings
/// window. No credentials, settings, or subtitle content leave the app.
#[tauri::command]
pub async fn app_check_for_updates(app: AppHandle) -> Result<AppUpdateCheckPayload, String> {
    let release_tag = crate::clients::update_client::latest_release_tag()
        .await
        .map_err(|error| {
            tracing::warn!(
                label = error.diagnostic_label(),
                status = ?error.status_code(),
                "update check failed"
            );
            "Could not check for updates.".to_string()
        })?;
    let availability =
        compare_release_versions(&app.package_info().version.to_string(), &release_tag).map_err(
            |error| {
                tracing::warn!(label = %error, "update version comparison failed");
                "Could not check for updates.".to_string()
            },
        )?;

    Ok(AppUpdateCheckPayload {
        current_version: availability.current_version,
        latest_version: availability.latest_version,
        update_available: availability.update_available,
    })
}

/// Opens a single hard-coded release destination. The frontend cannot supply
/// or widen the URL, and no generic opener permission is exposed to WebViews.
#[tauri::command]
pub fn app_open_releases(app: AppHandle) -> Result<(), String> {
    app.opener()
        .open_url(
            crate::clients::update_client::RELEASES_LATEST_URL,
            None::<&str>,
        )
        .map_err(|_| {
            tracing::warn!(label = "system_opener_failed", "release page open failed");
            "Could not open the release page.".to_string()
        })
}

/// Saves non-secret preferences only. Credentials use dedicated write-only
/// commands so a general settings draft can never echo or overwrite a key.
#[tauri::command]
pub async fn settings_save(
    app: AppHandle,
    state: State<'_, AppState>,
    draft: SettingsDraft,
) -> Result<SettingsSnapshotPayload, String> {
    apply_settings_draft(&app, &state, draft).await
}

async fn apply_settings_draft(
    app: &AppHandle,
    state: &AppState,
    draft: SettingsDraft,
) -> Result<SettingsSnapshotPayload, String> {
    let changes_listening_settings = draft.source_language.is_some()
        || draft.target_language.is_some()
        || draft.translation_mode.is_some();
    let _lifecycle = state
        .session
        .settings_mutation_guard(changes_listening_settings)
        .await?;
    apply_settings_draft_guarded(app, state, draft)
}

/// Applies a settings draft while the caller owns the settings mutation
/// guard. Keeping the read and write inside one guard makes native toggles
/// atomic with ordinary settings saves.
fn apply_settings_draft_guarded(
    app: &AppHandle,
    state: &AppState,
    draft: SettingsDraft,
) -> Result<SettingsSnapshotPayload, String> {
    let changes_ui_language = draft.ui_language.is_some();
    let enables_background_blend = draft.subtitle_blends_with_background == Some(true);
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
    // Background blending has no meaningful collapsed presentation. Enforce
    // this natively so the invariant also holds while the WebView is hidden
    // or reloading; do not rely on a React effect to repair geometry later.
    if enables_background_blend && state.session.is_overlay_collapsed() {
        state.session.set_overlay_collapsed(false);
        OverlayWindowManager::set_collapsed(app, &state.overlay, &state.settings, false);
        state.session.publish_state();
    }
    if draft.is_overlay_locked.is_some() || draft.subtitle_blends_with_background.is_some() {
        let preferences = state.settings.preferences();
        OverlayWindowManager::sync_presentation(
            app,
            state.session.is_active(),
            state.session.is_overlay_collapsed(),
            preferences.overlay_locked || preferences.subtitle_blends_with_background,
            preferences.subtitle_blends_with_background,
        );
    }
    if changes_ui_language {
        crate::refresh_native_tray_language(app);
    }

    let payload = SettingsSnapshotPayload::try_from_store(&state.settings)?;
    let _ = app.emit("settings-changed", payload.clone());
    Ok(payload)
}

/// Toggles the persisted Immersive Mode presentation from native surfaces
/// such as the global shortcut. This deliberately reuses the settings mutation
/// path so collapsed geometry, click-through state, and settings broadcasts
/// stay identical to UI-triggered changes.
pub(crate) async fn toggle_immersive_mode(app: &AppHandle) -> Result<(), String> {
    let state = app
        .try_state::<AppState>()
        .ok_or_else(|| "Application state is unavailable.".to_string())?;
    let _lifecycle = state.session.settings_mutation_guard(false).await?;
    let enabled =
        toggled_immersive_mode(state.settings.preferences().subtitle_blends_with_background);
    apply_settings_draft_guarded(
        app,
        &state,
        SettingsDraft {
            subtitle_blends_with_background: Some(enabled),
            ..SettingsDraft::default()
        },
    )?;
    Ok(())
}

fn toggled_immersive_mode(current: bool) -> bool {
    !current
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
pub async fn profile_save_credentials(
    app: AppHandle,
    state: State<'_, AppState>,
    profile_id: String,
    credentials: ProviderCredentials,
) -> Result<SettingsSnapshotPayload, String> {
    let _lifecycle = state.session.settings_mutation_guard(true).await?;
    ensure_profile_mutation_allowed(state.session.has_active_session())?;
    state.settings.save_credentials(&profile_id, &credentials)?;
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
    let preferences = state.settings.preferences();
    let collapsed =
        normalize_overlay_collapsed(collapsed, preferences.subtitle_blends_with_background);
    state.session.set_overlay_collapsed(collapsed);
    OverlayWindowManager::set_collapsed(&app, &state.overlay, &state.settings, collapsed);
    OverlayWindowManager::sync_presentation(
        &app,
        state.session.is_active(),
        collapsed,
        preferences.overlay_locked || preferences.subtitle_blends_with_background,
        preferences.subtitle_blends_with_background,
    );
    // The frontend's collapse UI state only updates through the
    // session-state event; without this the overlay renders the wrong
    // layout after collapsing/expanding.
    state.session.publish_state();
    Ok(())
}

fn normalize_overlay_collapsed(requested: bool, background_blend: bool) -> bool {
    requested && !background_blend
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
    OverlayWindowManager::sync_presentation(
        &app,
        state.session.is_active(),
        state.session.is_overlay_collapsed(),
        locked || preferences.subtitle_blends_with_background,
        preferences.subtitle_blends_with_background,
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
        let preferences = state.settings.preferences();
        OverlayWindowManager::sync_presentation(
            &app,
            true,
            state.session.is_overlay_collapsed(),
            preferences.overlay_locked || preferences.subtitle_blends_with_background,
            preferences.subtitle_blends_with_background,
        );
        OverlayWindowManager::follow_active_space(&app, &state.overlay, &state.settings);
    }
    Ok(())
}

/// Marks an explicit user drag before Tauri hands the gesture to AppKit.
#[tauri::command]
pub fn overlay_move_start(
    app: AppHandle,
    window: tauri::WebviewWindow,
    state: State<'_, AppState>,
) -> Result<(), String> {
    OverlayWindowManager::move_start(&app, &state.overlay);
    if window.start_dragging().is_err() {
        OverlayWindowManager::move_cancel(&state.overlay);
        return Err("Could not start overlay drag.".to_string());
    }
    Ok(())
}

/// Toggles the child overlay control between its compact island and expanded
/// panel. The legacy command name remains part of the IPC contract.
#[tauri::command]
pub fn overlay_popover_toggle(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    OverlayControlWindowManager::toggle_panel(&app);
    // Refresh snapshots after a hidden/reloaded WebView so its checkmarks and
    // lifecycle guards never depend solely on an older broadcast.
    state.session.publish_settings();
    state.session.publish_state();
    Ok(())
}

/// Returns the expanded panel to its compact island. The legacy command name
/// is kept for backwards-compatible frontend bundles.
#[tauri::command]
pub fn overlay_popover_hide(app: AppHandle) -> Result<(), String> {
    OverlayControlWindowManager::dismiss_panel(&app);
    Ok(())
}

/// Current native mode for initial control-window hydration after a reload.
#[tauri::command]
pub fn overlay_control_state(app: AppHandle) -> Result<OverlayControlMode, String> {
    Ok(OverlayControlWindowManager::mode(&app))
}

/// Applies a tightly-fitted panel height measured by the control WebView.
#[tauri::command]
pub fn overlay_control_set_panel_height(app: AppHandle, height: f64) -> Result<(), String> {
    OverlayControlWindowManager::set_panel_height(&app, height);
    Ok(())
}

/// The current session state snapshot, for windows that boot after the last
/// broadcast (e.g. the overlay control). Async: cloning the full controller
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
    // no-op. Collapse the overlay control panel for the same reason.
    TrayPanelManager::hide(&app);
    OverlayControlWindowManager::dismiss_panel(&app);

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
