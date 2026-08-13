//! mimi — live translated subtitles for anything playing on your device.
//! Tauri v2 shell wiring: plugins, tray, global shortcut, windows, and state.

pub mod audio;
pub mod clients;
pub mod commands;
pub mod core;
pub mod session_manager;
pub mod settings_store;
pub mod windows;

use commands::AppState;
use session_manager::SessionManager;
use settings_store::SettingsStore;
use std::sync::Arc;
use tauri::menu::{CheckMenuItemBuilder, MenuBuilder, MenuItemBuilder, SubmenuBuilder};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Emitter, Manager, WindowEvent};

/// Runs the mimi Tauri application.
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_positioner::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            tracing::info!("mimi starting");

            let app_handle = app.handle().clone();
            let is_ui_test = std::env::var("MIMI_UI_TEST").as_deref() == Ok("1");
            let settings = Arc::new(SettingsStore::load(
                app.path().app_config_dir().unwrap_or_default(),
                is_ui_test,
            ));
            let session = SessionManager::new(app_handle.clone(), Arc::clone(&settings));

            windows::OverlayWindowManager::ensure_overlay(&app_handle, &settings);
            windows::TrayPanelManager::ensure(&app_handle);

            setup_tray(&app_handle)?;
            setup_global_shortcut(&app_handle, Arc::clone(&session))?;

            app.manage(AppState { settings, session });
            Ok(())
        })
        .on_window_event(|window, event| {
            let app = window.app_handle();
            match event {
                WindowEvent::Moved(_) | WindowEvent::Resized(_) if window.label() == "overlay" => {
                    if let Some(state) = app.try_state::<AppState>() {
                        windows::OverlayWindowManager::persist_frame(app, &state.settings);
                    }
                }
                WindowEvent::Focused(false) if window.label() == "tray-panel" => {
                    windows::TrayPanelManager::hide(app);
                }
                WindowEvent::CloseRequested { api, .. }
                    if window.label() == "overlay" || window.label() == "tray-panel" =>
                {
                    api.prevent_close();
                    let _ = window.hide();
                }
                _ => {}
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::settings_get,
            commands::settings_save,
            commands::session_start,
            commands::session_stop,
            commands::session_toggle_paused,
            commands::session_clear_subtitles,
            commands::session_switch_source_language,
            commands::session_switch_translation_mode,
            commands::overlay_set_collapsed,
            commands::overlay_set_locked,
            commands::overlay_show,
            commands::overlay_set_size,
            commands::tray_panel_hide,
            commands::app_show_settings,
            commands::app_quit,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Tray icon with the mimi menu (start/stop, language, lock, settings, quit)
/// and a left-click popup control panel.
fn setup_tray(app: &tauri::AppHandle) -> tauri::Result<()> {
    let live_subtitles =
        CheckMenuItemBuilder::with_id("live-subtitles", "Live Subtitles").build(app)?;
    let lock_position =
        CheckMenuItemBuilder::with_id("lock-position", "Lock Subtitle Position").build(app)?;

    let language_menu = SubmenuBuilder::new(app, "识别语言")
        .item(&MenuItemBuilder::with_id("lang-ja", "日本語").build(app)?)
        .item(&MenuItemBuilder::with_id("lang-en", "English").build(app)?)
        .item(&MenuItemBuilder::with_id("lang-ko", "한국어").build(app)?)
        .item(&MenuItemBuilder::with_id("lang-zh", "中文原文").build(app)?)
        .build()?;

    let menu = MenuBuilder::new(app)
        .item(&live_subtitles)
        .item(&language_menu)
        .item(&lock_position)
        .item(&MenuItemBuilder::with_id("show-subtitles", "Show Subtitle Window").build(app)?)
        .item(&MenuItemBuilder::with_id("clear-subtitles", "Clear Subtitles").build(app)?)
        .separator()
        .item(&MenuItemBuilder::with_id("settings", "Settings…").build(app)?)
        .item(&MenuItemBuilder::with_id("quit", "Quit mimi").build(app)?)
        .build()?;

    let icon = app
        .default_window_icon()
        .cloned()
        .expect("the default window icon is bundled");

    let live_subtitles_clone = live_subtitles.clone();
    let lock_position_clone = lock_position.clone();
    let _tray = TrayIconBuilder::with_id("mimi-tray")
        .icon(icon)
        .tooltip("mimi")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event| {
            let Some(state) = app.try_state::<AppState>() else {
                return;
            };
            match event.id().as_ref() {
                "live-subtitles" => {
                    let session = Arc::clone(&state.session);
                    tauri::async_runtime::spawn(async move {
                        if session.is_active() {
                            session.stop().await;
                        } else {
                            state_settings_prepare(&session);
                            let _ = session.start(true).await;
                        }
                    });
                }
                "lang-ja" => {
                    switch_language(app, state, crate::core::models::SourceLanguage::Japanese)
                }
                "lang-en" => {
                    switch_language(app, state, crate::core::models::SourceLanguage::English)
                }
                "lang-ko" => {
                    switch_language(app, state, crate::core::models::SourceLanguage::Korean)
                }
                "lang-zh" => {
                    switch_language(app, state, crate::core::models::SourceLanguage::Chinese)
                }
                "lock-position" => {
                    let locked = !state.settings.preferences().overlay_locked;
                    state
                        .settings
                        .update_preferences(|prefs| prefs.overlay_locked = locked);
                    state.settings.persist();
                    windows::OverlayWindowManager::update_locked(app, locked);
                    let _ = app.emit(
                        "settings-changed",
                        commands::SettingsSnapshotPayload::from_store(&state.settings),
                    );
                }
                "show-subtitles" => windows::OverlayWindowManager::show(app),
                "clear-subtitles" => state.session.clear_subtitles(),
                "settings" => {
                    if let Some(window) = app.get_webview_window("settings") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
                "quit" => app.exit(0),
                _ => {}
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                windows::TrayPanelManager::toggle(app);
            }
        })
        .build(app)?;

    // Keep the live-subtitles check state in sync with the session.
    let app_for_state = app.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(800)).await;
            let Some(state) = app_for_state.try_state::<AppState>() else {
                continue;
            };
            let active = state.session.is_active();
            if live_subtitles_clone.is_checked().unwrap_or(false) != active {
                let _ = live_subtitles_clone.set_checked(active);
            }
            let locked = state.settings.preferences().overlay_locked;
            if lock_position_clone.is_checked().unwrap_or(false) != locked {
                let _ = lock_position_clone.set_checked(locked);
            }
        }
    });

    Ok(())
}

fn state_settings_prepare(session: &SessionManager) {
    // Mirrors `prepareForListening`: automatic source becomes Japanese and
    // Chinese source switches to original subtitles. Applied through the
    // settings store before starting.
    session.prepare_for_listening();
}

fn switch_language(
    app: &tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    language: crate::core::models::SourceLanguage,
) {
    let session = Arc::clone(&state.session);
    tauri::async_runtime::spawn(async move {
        session.switch_source_language(language).await;
    });
    let _ = app.emit(
        "settings-changed",
        commands::SettingsSnapshotPayload::from_store(&state.settings),
    );
}

/// Registers the global start/stop shortcut (⌘⇧Space on macOS,
/// Ctrl+Shift+Space on Windows).
fn setup_global_shortcut(
    app: &tauri::AppHandle,
    session: Arc<SessionManager>,
) -> tauri::Result<()> {
    use tauri_plugin_global_shortcut::{
        Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState,
    };

    let shortcut = Shortcut::new(
        Some(Modifiers::CONTROL | Modifiers::SHIFT | Modifiers::SUPER),
        Code::Space,
    );
    let session_for_handler = Arc::clone(&session);

    app.global_shortcut()
        .on_shortcut(shortcut, move |_app, _shortcut, event| {
            if event.state() != ShortcutState::Pressed {
                return;
            }
            let session = Arc::clone(&session_for_handler);
            tauri::async_runtime::spawn(async move {
                let status = session.status_kind();
                if status == "connecting" || status == "stopping" {
                    return;
                }
                if session.is_active() {
                    session.stop().await;
                } else {
                    session.prepare_for_listening();
                    let _ = session.start(true).await;
                }
            });
        })
        .map_err(|error| std::io::Error::other(error.to_string()))?;
    tracing::info!("global shortcut registered");
    Ok(())
}
