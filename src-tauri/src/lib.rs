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

            // Dev-build marker: the settings window is created from the
            // static config title, so adjust it at runtime so the dev binary
            // is distinguishable from the installed release app.
            if windows::is_dev_build() {
                if let Some(window) = app.get_webview_window("settings") {
                    let _ = window.set_title(&windows::dev_title("mimi 设置"));
                }
            }

            let app_handle = app.handle().clone();
            let is_ui_test = std::env::var("MIMI_UI_TEST").as_deref() == Ok("1");
            let settings = Arc::new(SettingsStore::load(
                app.path().app_config_dir().unwrap_or_default(),
                is_ui_test,
            ));
            let session = SessionManager::new(app_handle.clone(), Arc::clone(&settings));

            let overlay = Arc::new(std::sync::Mutex::new(windows::OverlayState::load(
                &app_handle,
                &settings,
            )));
            windows::OverlayWindowManager::ensure_overlay(&app_handle, &overlay);
            windows::TrayPanelManager::ensure(&app_handle);
            windows::LanguagePopoverManager::ensure(&app_handle);

            setup_tray(&app_handle)?;
            setup_global_shortcut(&app_handle, Arc::clone(&session))?;

            // Test-only probe: with MIMI_UI_TEST=1 and MIMI_AUTO_START=1, start a
            // session automatically so the pipeline (connect → error handling →
            // state events) can be exercised without UI interaction. The fake
            // API key makes the service reject the connection, exercising the
            // full failure path deterministically.
            if is_ui_test && std::env::var("MIMI_AUTO_START").as_deref() == Ok("1") {
                let session_for_probe = Arc::clone(&session);
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    let _ = session_for_probe.start(true).await;
                });
            }

            app.manage(AppState {
                settings,
                session,
                overlay,
            });
            Ok(())
        })
        .on_window_event(|window, event| {
            let app = window.app_handle();
            match event {
                // The overlay geometry manager folds the final frame in after
                // a debounce; transient states (popover enlargement, collapse
                // animation steps) are never persisted.
                WindowEvent::Moved(_) | WindowEvent::Resized(_) if window.label() == "overlay" => {
                    if let Some(state) = app.try_state::<AppState>() {
                        windows::OverlayWindowManager::on_geometry_event(
                            app,
                            &state.overlay,
                            &state.settings,
                        );
                    }
                    // Keep the open language/mode menu glued to the capsule.
                    windows::LanguagePopoverManager::follow_overlay(app);
                }
                WindowEvent::Focused(false) if window.label() == "tray-panel" => {
                    windows::TrayPanelManager::hide(app);
                }
                // The language/mode menu closes when focus moves elsewhere
                // (mirrors the Swift NSPopover's transient behavior). The
                // delayed hide lets a capsule click that stole focus re-open
                // or toggle the menu without a hide/show flicker.
                WindowEvent::Focused(false) if window.label() == "language-popover" => {
                    windows::LanguagePopoverManager::schedule_hide(app);
                }
                WindowEvent::CloseRequested { api, .. }
                    if window.label() == "overlay"
                        || window.label() == "tray-panel"
                        || window.label() == "settings" =>
                {
                    // Hiding instead of closing keeps the window alive so
                    // later "settings"/"show" actions can always re-show it.
                    // A destroyed settings window would make the tray's
                    // "设置" a silent no-op.
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
            commands::overlay_popover_toggle,
            commands::overlay_popover_hide,
            commands::overlay_commit_frame,
            commands::session_get_state,
            commands::resize_start,
            commands::resize_move,
            commands::resize_end,
            commands::tray_panel_hide,
            commands::app_show_settings,
            commands::app_quit,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// True when the macOS system language is Chinese (zh-*). The native tray
/// menu is built once at startup and follows the system language like the
/// webview panels do (see `src/lib/i18n.ts`).
#[cfg(target_os = "macos")]
fn system_language_is_zh() -> bool {
    use objc2::msg_send;
    use objc2::runtime::{AnyClass, AnyObject};
    use std::ffi::{c_char, CStr};
    unsafe {
        let Some(locale_class) = AnyClass::get(c"NSLocale") else {
            return false;
        };
        let current: *mut AnyObject = msg_send![locale_class, currentLocale];
        if current.is_null() {
            return false;
        }
        let code: *mut AnyObject = msg_send![current, languageCode];
        if code.is_null() {
            return false;
        }
        let ptr: *const c_char = msg_send![code, UTF8String];
        if ptr.is_null() {
            return false;
        }
        CStr::from_ptr(ptr)
            .to_string_lossy()
            .to_ascii_lowercase()
            .starts_with("zh")
    }
}

/// Tray icon with the mimi menu (start/stop, language, lock, settings, quit)
/// and a left-click popup control panel. Menu copy follows the system
/// language (Chinese UI on zh-* systems, English otherwise).
fn setup_tray(app: &tauri::AppHandle) -> tauri::Result<()> {
    #[cfg(target_os = "macos")]
    let zh = system_language_is_zh();
    #[cfg(not(target_os = "macos"))]
    let zh = false;

    let live_subtitles = CheckMenuItemBuilder::with_id(
        "live-subtitles",
        if zh { "实时字幕" } else { "Live Subtitles" },
    )
    .build(app)?;
    let lock_position = CheckMenuItemBuilder::with_id(
        "lock-position",
        if zh {
            "锁定字幕位置"
        } else {
            "Lock Subtitle Position"
        },
    )
    .build(app)?;

    let language_menu = SubmenuBuilder::new(
        app,
        if zh {
            "识别语言"
        } else {
            "Recognition Language"
        },
    )
    .item(
        &MenuItemBuilder::with_id("lang-auto", if zh { "自动识别" } else { "Auto Detect" })
            .build(app)?,
    )
    .item(&MenuItemBuilder::with_id("lang-ja", if zh { "日语" } else { "Japanese" }).build(app)?)
    .item(&MenuItemBuilder::with_id("lang-en", if zh { "英语" } else { "English" }).build(app)?)
    .item(&MenuItemBuilder::with_id("lang-ko", if zh { "韩语" } else { "Korean" }).build(app)?)
    .item(
        &MenuItemBuilder::with_id(
            "lang-zh",
            if zh {
                "中文原文"
            } else {
                "Chinese (Original)"
            },
        )
        .build(app)?,
    )
    .build()?;

    let menu = MenuBuilder::new(app)
        .item(&live_subtitles)
        .item(&language_menu)
        .item(&lock_position)
        .item(
            &MenuItemBuilder::with_id(
                "show-subtitles",
                if zh {
                    "显示字幕窗口"
                } else {
                    "Show Subtitle Window"
                },
            )
            .build(app)?,
        )
        .item(
            &MenuItemBuilder::with_id(
                "clear-subtitles",
                if zh {
                    "清空字幕"
                } else {
                    "Clear Subtitles"
                },
            )
            .build(app)?,
        )
        .separator()
        .item(
            &MenuItemBuilder::with_id("settings", if zh { "设置…" } else { "Settings…" })
                .build(app)?,
        )
        .item(
            &MenuItemBuilder::with_id("quit", if zh { "退出 mimi" } else { "Quit mimi" })
                .build(app)?,
        )
        .build()?;

    // Menu-bar icon: a monochrome waveform template (like the original app's
    // `ear.badge.waveform` SF Symbol). Template icons are rendered by macOS at
    // the native menu-bar resolution — crisp at any size and adapting to the
    // light/dark menu bar — unlike the character squircle, whose fine detail
    // turned into a blurry blob at ~18pt.
    let icon = tauri::image::Image::from_bytes(include_bytes!("../icons/tray-template.png"))
        .ok()
        .or_else(|| app.default_window_icon().cloned())
        .expect("the tray icon is bundled");

    let live_subtitles_clone = live_subtitles.clone();
    let lock_position_clone = lock_position.clone();
    let _tray = TrayIconBuilder::with_id("mimi-tray")
        .icon(icon)
        .icon_as_template(true)
        .tooltip(windows::dev_title("mimi"))
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
                "lang-auto" => {
                    switch_language(state, crate::core::models::SourceLanguage::Automatic)
                }
                "lang-ja" => switch_language(state, crate::core::models::SourceLanguage::Japanese),
                "lang-en" => switch_language(state, crate::core::models::SourceLanguage::English),
                "lang-ko" => switch_language(state, crate::core::models::SourceLanguage::Korean),
                "lang-zh" => switch_language(state, crate::core::models::SourceLanguage::Chinese),
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
                    // The tray panel is always-on-top; hide it so the
                    // settings window is not obscured behind it.
                    windows::TrayPanelManager::hide(app);
                    windows::ensure_settings_window(app);
                }
                "quit" => app.exit(0),
                _ => {}
            }
        })
        .on_tray_icon_event(|tray, event| {
            // Record the tray icon's screen position for the positioner
            // plugin; without this, TrayBottomCenter (used by the tray panel)
            // fails with "Tray position not set" and the panel stays at its
            // default window position.
            tauri_plugin_positioner::on_tray_event(tray.app_handle(), &event);
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                windows::TrayPanelManager::toggle(app);
                // Refresh the panel's snapshots so its pickers and check
                // states always match the current session, even if its
                // webview missed events while hidden.
                if let Some(state) = app.try_state::<AppState>() {
                    state.session.publish_settings();
                    state.session.publish_state();
                }
            }
        })
        .build(app)?;

    // Keep the live-subtitles check state in sync with the session. The
    // polling interval is deliberately coarse: the check states change only
    // on user actions, so 2s staleness is invisible while the loop stays
    // nearly free.
    let app_for_state = app.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(2000)).await;
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
    state: tauri::State<'_, AppState>,
    language: crate::core::models::SourceLanguage,
) {
    // The session manager broadcasts settings-changed itself, immediately
    // after the preference write.
    let session = Arc::clone(&state.session);
    tauri::async_runtime::spawn(async move {
        session.switch_source_language(language).await;
    });
}

/// Registers the global start/stop shortcut — ⌘⇧Space on macOS (matching the
/// original app), Ctrl+Shift+Space on Windows. A 2s debounce mirrors the
/// original `GlobalHotKeyController`.
fn setup_global_shortcut(
    app: &tauri::AppHandle,
    session: Arc<SessionManager>,
) -> tauri::Result<()> {
    use tauri_plugin_global_shortcut::{
        Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState,
    };

    // macOS: Cmd+Shift (SUPER is the Command key); Windows: Ctrl+Shift.
    #[cfg(target_os = "macos")]
    let modifiers = Modifiers::SUPER | Modifiers::SHIFT;
    #[cfg(not(target_os = "macos"))]
    let modifiers = Modifiers::CONTROL | Modifiers::SHIFT;
    let shortcut = Shortcut::new(Some(modifiers), Code::Space);

    let last_trigger = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
    let session_for_handler = Arc::clone(&session);

    // Register the global shortcut. A failure must not abort startup: another
    // app (e.g. a second mimi instance) may already own the combo, in which
    // case macOS delivers the key to that app and this one simply has no
    // shortcut.
    let register = app
        .global_shortcut()
        .on_shortcut(shortcut, move |_app, _shortcut, event| {
            if event.state() != ShortcutState::Pressed {
                return;
            }
            // Debounce repeated presses like the Swift hotkey controller.
            let now_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);
            let previous = last_trigger.load(std::sync::atomic::Ordering::SeqCst);
            if now_ms.saturating_sub(previous) < 500 {
                return;
            }
            last_trigger.store(now_ms, std::sync::atomic::Ordering::SeqCst);
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
        });
    match register {
        Ok(()) => tracing::info!("global shortcut registered"),
        Err(error) => tracing::warn!(
            "global shortcut could not be registered: {error} \
             (another mimi instance may already own ⌘⇧Space)"
        ),
    }
    Ok(())
}
