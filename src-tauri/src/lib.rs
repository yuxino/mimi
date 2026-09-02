//! mimi — live translated subtitles for anything playing on your device.
//! Tauri v2 shell wiring: plugins, tray, global shortcut, windows, and state.

mod audio;
mod clients;
mod commands;
mod core;
mod session_manager;
mod settings_store;
mod windows;
#[cfg(target_os = "windows")]
mod windows_startup;

use commands::AppState;
use session_manager::SessionManager;
use settings_store::{SettingsStore, DEVELOPMENT_APPLICATION_IDENTIFIER};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::menu::{MenuBuilder, MenuItem, MenuItemBuilder};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Listener, Manager, WindowEvent};

/// Runs the mimi Tauri application.
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let context = tauri::generate_context!();
    #[cfg(target_os = "windows")]
    let startup_gate = windows_startup::StartupGate::acquire(context.config().identifier.as_str())
        .unwrap_or_else(|label| panic!("mimi startup gate failed: {label}"));

    let builder = tauri::Builder::default();
    // This must stay first so a secondary Windows launch exits before any
    // other plugin, renderer, tray, or settings store is initialized.
    #[cfg(target_os = "windows")]
    let builder = builder.plugin(windows_startup::single_instance_plugin());
    let builder = builder
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_positioner::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build());
    #[cfg(target_os = "macos")]
    let builder = builder.plugin(tauri_nspanel::init());

    builder
        .setup(move |app| {
            tracing::info!("mimi starting");

            // macOS only admits accessory utilities into another app's true
            // full-screen presentation. mimi already exposes its lifecycle
            // through the menu-bar tray, so it does not need a Dock or Cmd-Tab
            // presence of its own.
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            // Dev-build marker: the settings window is created from the
            // static config title, so adjust it at runtime so the dev binary
            // is distinguishable from the installed release app.
            if windows::is_dev_build() {
                if let Some(window) = app.get_webview_window("settings") {
                    let _ = window.set_title(&windows::dev_title("mimi 设置"));
                }
            }

            let app_handle = app.handle().clone();
            if windows::is_dev_build()
                && app.config().identifier.as_str() != DEVELOPMENT_APPLICATION_IDENTIFIER
            {
                return Err("development builds require the isolated Tauri identifier".into());
            }
            let is_ui_test = std::env::var("MIMI_UI_TEST").as_deref() == Ok("1");
            if is_ui_test {
                if let Some(window) = app.get_webview_window("settings") {
                    let _ = window.set_title("mimi UI test settings");
                }
            }
            let settings = Arc::new(SettingsStore::load(
                app.path().app_config_dir().unwrap_or_default(),
                is_ui_test,
                &app.config().identifier,
            ));
            // A deterministic standard-overlay fixture is useful for native
            // window-level checks. It changes only the in-memory UI-test
            // snapshot; `SettingsStore` never persists UI-test writes.
            if is_ui_test && std::env::var("MIMI_UI_TEST_STANDARD_OVERLAY").as_deref() == Ok("1") {
                let _ = settings.save_preferences(|preferences| {
                    preferences.subtitle_blends_with_background = false;
                    preferences.overlay_locked = false;
                });
            }
            let session = SessionManager::new(app_handle.clone(), Arc::clone(&settings));

            let overlay = Arc::new(std::sync::Mutex::new(windows::OverlayState::load(
                &app_handle,
                &settings,
            )));
            app.manage(AppState {
                settings: Arc::clone(&settings),
                session: Arc::clone(&session),
                overlay: Arc::clone(&overlay),
            });
            app.manage(windows::OverlayControlState::default());
            app.manage(windows::OverlayPresentationState::default());
            windows::OverlayWindowManager::ensure_overlay(&app_handle, &overlay);
            #[cfg(target_os = "macos")]
            windows::install_active_space_observer(&app_handle, &overlay, &settings);
            let preferences = settings.preferences();
            windows::OverlayWindowManager::update_locked(
                &app_handle,
                preferences.overlay_locked || preferences.subtitle_blends_with_background,
            );
            windows::TrayPanelManager::ensure(&app_handle);
            windows::OverlayControlWindowManager::ensure(&app_handle);
            #[cfg(target_os = "windows")]
            app.manage(windows::install_windows_workspace_follower(&app_handle));

            setup_tray(&app_handle)?;
            setup_global_shortcuts(&app_handle, Arc::clone(&session))?;

            // Test-only probe: `SessionManager` handles UI-test starts as a
            // synthetic local state transition. It never reads the keychain,
            // opens a network connection, or starts system-audio capture.
            if is_ui_test && std::env::var("MIMI_AUTO_START").as_deref() == Ok("1") {
                let session_for_probe = Arc::clone(&session);
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    let _ = session_for_probe.start(true).await;
                });
            }

            // Release queued cold launches only after setup returns to the
            // Windows event loop. A secondary uses a bounded synchronous,
            // payload-free activation message; releasing earlier can block it
            // while this main thread is still creating WebViews, tray state,
            // and shortcuts.
            #[cfg(target_os = "windows")]
            {
                let app_handle_for_gate = app.handle().clone();
                std::thread::spawn(move || {
                    if app_handle_for_gate
                        .run_on_main_thread(move || drop(startup_gate))
                        .is_err()
                    {
                        // The event loop cannot become a usable primary, and
                        // this mutex is owned by its main thread. Abort so the
                        // kernel abandons it for the next launch.
                        std::process::abort();
                    }
                });
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            let app = window.app_handle();
            match event {
                // The overlay geometry manager folds the final frame in after
                // a debounce; transient states (control panel, collapse
                // animation steps) are never persisted.
                WindowEvent::Moved(_)
                | WindowEvent::Resized(_)
                | WindowEvent::ScaleFactorChanged { .. }
                    if window.label() == "overlay" =>
                {
                    if let Some(state) = app.try_state::<AppState>() {
                        windows::OverlayWindowManager::on_geometry_event(
                            app,
                            &state.overlay,
                            &state.settings,
                        );
                    }
                    // AppKit moves an NSWindow child in the same native drag
                    // transaction as its parent. Repositioning it again from
                    // the later Moved event makes the control visibly trail.
                    // Windows owned windows and Linux transient windows still
                    // need explicit live following.
                    #[cfg(not(target_os = "macos"))]
                    windows::OverlayControlWindowManager::follow_overlay(app);
                }
                WindowEvent::Focused(false) if window.label() == "tray-panel" => {
                    windows::TrayPanelManager::hide(app);
                }
                // An expanded control panel returns to its compact island on
                // focus loss. The island itself remains available even while
                // the subtitle canvas is click-through.
                WindowEvent::Focused(false) if window.label() == "overlay-control" => {
                    windows::OverlayControlWindowManager::schedule_dismiss(app);
                }
                WindowEvent::Focused(true) if window.label() == "overlay-control" => {
                    windows::OverlayControlWindowManager::cancel_scheduled_dismiss(app);
                }
                WindowEvent::CloseRequested { api, .. } if window.label() == "settings" => {
                    // Hiding instead of closing keeps the window alive so
                    // the tray or a repeated launch can restore it instantly.
                    // Windows users may keep tray icons in the notification
                    // overflow, so explicitly keep the native icon visible.
                    api.prevent_close();
                    let tray_ready = if let Some(tray) = app.tray_by_id("mimi-tray") {
                        if let Err(error) = tray.set_visible(true) {
                            tracing::warn!(
                                error = %error,
                                "settings close could not keep tray icon visible"
                            );
                            false
                        } else {
                            record_ui_test_tray_visible();
                            true
                        }
                    } else {
                        tracing::warn!("settings close could not find tray icon");
                        false
                    };
                    // Never strand the user with neither Settings nor a tray
                    // entry. `prevent_close` keeps this window visible when
                    // the native tray handle cannot be confirmed.
                    if !tray_ready {
                        return;
                    }
                    if let Err(error) = window.hide() {
                        tracing::warn!(error = %error, "settings close could not hide window");
                        // A later tray/relaunch activation recreates a missing
                        // settings window, avoiding an uncloseable stale shell.
                        if let Err(error) = window.destroy() {
                            tracing::warn!(
                                error = %error,
                                "settings close could not destroy stale window"
                            );
                        }
                    }
                }
                WindowEvent::CloseRequested { api, .. }
                    if window.label() == "overlay"
                        || window.label() == "overlay-control"
                        || window.label() == "tray-panel" =>
                {
                    api.prevent_close();
                    if let Err(error) = window.hide() {
                        tracing::warn!(
                            window_label = window.label(),
                            error = %error,
                            "auxiliary window close could not hide window"
                        );
                    }
                }
                _ => {}
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::settings_get,
            commands::app_is_ui_test,
            commands::app_open_releases,
            commands::settings_save,
            commands::profile_create,
            commands::profile_update,
            commands::profile_select,
            commands::profile_delete,
            commands::profile_save_credentials,
            commands::profile_delete_api_key,
            commands::session_start,
            commands::session_stop,
            commands::session_toggle_paused,
            commands::session_clear_subtitles,
            commands::session_switch_source_language,
            commands::session_switch_translation_mode,
            commands::overlay_set_collapsed,
            commands::overlay_set_locked,
            commands::overlay_show,
            commands::overlay_move_start,
            commands::overlay_popover_toggle,
            commands::overlay_popover_hide,
            commands::overlay_control_state,
            commands::overlay_control_set_panel_height,
            commands::session_get_state,
            commands::resize_start,
            commands::resize_move,
            commands::resize_end,
            commands::tray_panel_hide,
            commands::app_show_settings,
            commands::app_quit,
        ])
        .run(context)
        .expect("error while running tauri application");
}

fn record_ui_test_tray_visible() {
    if std::env::var("MIMI_UI_TEST").as_deref() != Ok("1") {
        return;
    }
    let Some(path) = std::env::var_os("MIMI_UI_TEST_TRAY_VISIBLE_FILE") else {
        return;
    };
    if let Err(error) = std::fs::write(path, b"visible") {
        tracing::warn!(error = %error, "could not write UI-test tray visibility marker");
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NativeMenuLanguage {
    Chinese,
    English,
    Japanese,
}

#[derive(Debug, Clone, Copy)]
struct NativeMenuLabels {
    start_subtitles: &'static str,
    stop_subtitles: &'static str,
    toggle_devtools: &'static str,
    settings: &'static str,
    quit: &'static str,
}

#[derive(Clone)]
struct NativeTrayMenuItems {
    session_action: MenuItem<tauri::Wry>,
    toggle_devtools: Option<MenuItem<tauri::Wry>>,
    settings: MenuItem<tauri::Wry>,
    quit: MenuItem<tauri::Wry>,
}

fn effective_native_menu_language(
    override_language: Option<&str>,
    system_language: Option<&str>,
) -> NativeMenuLanguage {
    let language = match override_language {
        Some("zh") | Some("en") | Some("ja") => override_language,
        _ => system_language,
    }
    .unwrap_or("en")
    .to_ascii_lowercase();
    if language.starts_with("zh") {
        NativeMenuLanguage::Chinese
    } else if language.starts_with("ja") {
        NativeMenuLanguage::Japanese
    } else {
        NativeMenuLanguage::English
    }
}

fn native_menu_labels(language: NativeMenuLanguage) -> NativeMenuLabels {
    match language {
        NativeMenuLanguage::Chinese => NativeMenuLabels {
            start_subtitles: "开始字幕",
            stop_subtitles: "停止字幕",
            toggle_devtools: "打开调试工具",
            settings: "设置…",
            quit: "退出 mimi",
        },
        NativeMenuLanguage::Japanese => NativeMenuLabels {
            start_subtitles: "字幕を開始",
            stop_subtitles: "字幕を停止",
            toggle_devtools: "開発者ツールを開く",
            settings: "設定…",
            quit: "mimiを終了",
        },
        NativeMenuLanguage::English => NativeMenuLabels {
            start_subtitles: "Start Subtitles",
            stop_subtitles: "Stop Subtitles",
            toggle_devtools: "Open DevTools",
            settings: "Settings…",
            quit: "Quit mimi",
        },
    }
}

fn native_session_action_label(labels: NativeMenuLabels, is_active: bool) -> &'static str {
    if is_active {
        labels.stop_subtitles
    } else {
        labels.start_subtitles
    }
}

fn native_tray_session_is_active(status: &str) -> bool {
    matches!(status, "connecting" | "listening" | "stopping")
}

/// Reads the operating-system language code when the preference follows the system.
#[cfg(target_os = "macos")]
fn system_language_code() -> Option<String> {
    use objc2::msg_send;
    use objc2::runtime::{AnyClass, AnyObject};
    use std::ffi::{c_char, CStr};
    unsafe {
        let locale_class = AnyClass::get(c"NSLocale")?;
        let current: *mut AnyObject = msg_send![locale_class, currentLocale];
        if current.is_null() {
            return None;
        }
        let code: *mut AnyObject = msg_send![current, languageCode];
        if code.is_null() {
            return None;
        }
        let ptr: *const c_char = msg_send![code, UTF8String];
        if ptr.is_null() {
            return None;
        }
        Some(CStr::from_ptr(ptr).to_string_lossy().into_owned())
    }
}

#[cfg(target_os = "windows")]
fn system_language_code() -> Option<String> {
    use windows_sys::Win32::Globalization::GetUserDefaultLocaleName;

    // Windows defines LOCALE_NAME_MAX_LENGTH as 85 UTF-16 code units,
    // including the trailing null terminator.
    let mut buffer = [0_u16; 85];
    let length =
        unsafe { GetUserDefaultLocaleName(buffer.as_mut_ptr(), i32::try_from(buffer.len()).ok()?) };
    if length <= 1 {
        return None;
    }
    String::from_utf16(&buffer[..usize::try_from(length - 1).ok()?]).ok()
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn system_language_code() -> Option<String> {
    None
}

/// Tray icon with a compact native menu and a left-click popup control panel.
/// Copy follows the saved UI override, falling back to the system language.
fn tray_icon_bytes(is_windows: bool) -> &'static [u8] {
    if is_windows {
        // Windows does not implement macOS template-image recolouring. Use
        // the branded full-colour icon so it remains visible on dark taskbars.
        include_bytes!("../icons/32x32.png")
    } else {
        include_bytes!("../icons/tray-template.png")
    }
}

fn setup_tray(app: &tauri::AppHandle) -> tauri::Result<()> {
    let (override_language, session_is_active) = app
        .try_state::<AppState>()
        .map(|state| {
            (
                state.settings.preferences().ui_language,
                native_tray_session_is_active(&state.session.status_kind()),
            )
        })
        .unwrap_or((None, false));
    let system_language = system_language_code();
    let labels = native_menu_labels(effective_native_menu_language(
        override_language.as_deref(),
        system_language.as_deref(),
    ));

    let session_action = MenuItemBuilder::with_id(
        "live-subtitles",
        native_session_action_label(labels, session_is_active),
    )
    .build(app)?;

    let mut menu_builder = MenuBuilder::new(app).item(&session_action).separator();

    // Dev builds expose a manual "open inspector" action instead of opening
    // the WebView devtools automatically, so the user decides when to look.
    let toggle_devtools = (cfg!(any(debug_assertions, feature = "devtools"))
        && windows::is_dev_build())
    .then(|| MenuItemBuilder::with_id("toggle-devtools", labels.toggle_devtools).build(app))
    .transpose()?;
    if let Some(item) = &toggle_devtools {
        menu_builder = menu_builder.item(item);
    }

    let settings_item = MenuItemBuilder::with_id("settings", labels.settings).build(app)?;
    let quit_item = MenuItemBuilder::with_id("quit", labels.quit).build(app)?;

    let menu = menu_builder.item(&settings_item).item(&quit_item).build()?;
    // Menu-bar icon: a monochrome waveform template (like the original app's
    // `ear.badge.waveform` SF Symbol). Template icons are rendered by macOS at
    // the native menu-bar resolution — crisp at any size and adapting to the
    // light/dark menu bar — unlike the character squircle, whose fine detail
    // turned into a blurry blob at ~18pt.
    let icon = tauri::image::Image::from_bytes(tray_icon_bytes(cfg!(target_os = "windows")))
        .ok()
        .or_else(|| app.default_window_icon().cloned())
        .expect("the tray icon is bundled");

    let _tray = TrayIconBuilder::with_id("mimi-tray")
        .icon(icon)
        .icon_as_template(cfg!(target_os = "macos"))
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
                            let _ = session.start(true).await;
                        }
                    });
                }
                "toggle-devtools" => {
                    // Manual inspector toggle (dev builds only). Open the
                    // overlay's WebView devtools so the user decides when to
                    // inspect, instead of auto-opening at startup.
                    #[cfg(any(debug_assertions, feature = "devtools"))]
                    {
                        if let Some(window) = app.get_webview_window("overlay") {
                            window.open_devtools();
                        }
                    }
                }
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
            // Preserve the positioner fallback state as well as passing the
            // click rectangle to the edge-aware primary placement path.
            tauri_plugin_positioner::on_tray_event(tray.app_handle(), &event);
            if let TrayIconEvent::Click {
                rect,
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                windows::TrayPanelManager::toggle(app, &rect);
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

    app.manage(NativeTrayMenuItems {
        session_action,
        toggle_devtools,
        settings: settings_item,
        quit: quit_item,
    });

    // Session broadcasts already cover every start/stop path (native menu,
    // tray panel, settings, shortcut, recovery). Update the native action
    // only when activity changes instead of polling all menu state.
    let app_for_session = app.clone();
    let session_was_active = AtomicBool::new(session_is_active);
    app.listen("session-state", move |_| {
        let Some(state) = app_for_session.try_state::<AppState>() else {
            return;
        };
        let is_active = native_tray_session_is_active(&state.session.status_kind());
        if session_was_active.swap(is_active, Ordering::Relaxed) != is_active {
            refresh_native_tray_session_action(&app_for_session, is_active);
        }
    });

    Ok(())
}

pub(crate) fn refresh_native_tray_language(app: &tauri::AppHandle) {
    let (override_language, session_is_active) = app
        .try_state::<AppState>()
        .map(|state| {
            (
                state.settings.preferences().ui_language,
                native_tray_session_is_active(&state.session.status_kind()),
            )
        })
        .unwrap_or((None, false));
    let system_language = system_language_code();
    let labels = native_menu_labels(effective_native_menu_language(
        override_language.as_deref(),
        system_language.as_deref(),
    ));
    let Some(items) = app.try_state::<NativeTrayMenuItems>() else {
        return;
    };

    let _ = items
        .session_action
        .set_text(native_session_action_label(labels, session_is_active));
    if let Some(item) = &items.toggle_devtools {
        let _ = item.set_text(labels.toggle_devtools);
    }
    let _ = items.settings.set_text(labels.settings);
    let _ = items.quit.set_text(labels.quit);
}

fn refresh_native_tray_session_action(app: &tauri::AppHandle, is_active: bool) {
    let override_language = app
        .try_state::<AppState>()
        .and_then(|state| state.settings.preferences().ui_language);
    let system_language = system_language_code();
    let labels = native_menu_labels(effective_native_menu_language(
        override_language.as_deref(),
        system_language.as_deref(),
    ));
    if let Some(items) = app.try_state::<NativeTrayMenuItems>() {
        let _ = items
            .session_action
            .set_text(native_session_action_label(labels, is_active));
    }
}

/// Registers the global start/stop and Immersive Mode shortcuts. Each action
/// owns an independent 500ms debounce so presentation switching never blocks
/// a session lifecycle action (or vice versa).
fn setup_global_shortcuts(
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
    let session_shortcut = Shortcut::new(Some(modifiers), Code::Space);
    let immersive_shortcut = Shortcut::new(Some(modifiers), Code::KeyM);

    let last_trigger = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
    let session_for_handler = Arc::clone(&session);

    // Register the global shortcut. A failure must not abort startup: another
    // app (e.g. a second mimi instance) may already own the combination, in
    // which case the OS keeps delivering it to that app.
    let session_register =
        app.global_shortcut()
            .on_shortcut(session_shortcut, move |_app, _shortcut, event| {
                if event.state() != ShortcutState::Pressed {
                    return;
                }
                // Debounce repeated global-shortcut presses.
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
                        let _ = session.start(true).await;
                    }
                });
            });
    match session_register {
        Ok(()) => tracing::info!("global session shortcut registered"),
        Err(error) => tracing::warn!(
            "global session shortcut could not be registered: {error} \
             (another app may already own the start/stop combination)"
        ),
    }

    let immersive_last_trigger = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
    let immersive_register =
        app.global_shortcut()
            .on_shortcut(immersive_shortcut, move |app, _shortcut, event| {
                if event.state() != ShortcutState::Pressed {
                    return;
                }
                let now_ms = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0);
                let previous = immersive_last_trigger.load(std::sync::atomic::Ordering::SeqCst);
                if now_ms.saturating_sub(previous) < 500 {
                    return;
                }
                immersive_last_trigger.store(now_ms, std::sync::atomic::Ordering::SeqCst);
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(error) = commands::toggle_immersive_mode(&app).await {
                        tracing::warn!(
                            "immersive shortcut failed label=settings_unavailable error={error}"
                        );
                    }
                });
            });
    match immersive_register {
        Ok(()) => tracing::info!("global immersive shortcut registered"),
        Err(error) => tracing::warn!(
            "global immersive shortcut could not be registered: {error} \
             (another app may already own the combination)"
        ),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_tray_session_action_follows_status_in_all_three_languages() {
        for (language, start, stop) in [
            (NativeMenuLanguage::Chinese, "开始字幕", "停止字幕"),
            (
                NativeMenuLanguage::English,
                "Start Subtitles",
                "Stop Subtitles",
            ),
            (NativeMenuLanguage::Japanese, "字幕を開始", "字幕を停止"),
        ] {
            let labels = native_menu_labels(language);
            assert_eq!(native_session_action_label(labels, false), start);
            assert_eq!(native_session_action_label(labels, true), stop);
        }

        assert!(!native_tray_session_is_active("idle"));
        assert!(native_tray_session_is_active("connecting"));
        assert!(native_tray_session_is_active("listening"));
        assert!(native_tray_session_is_active("stopping"));
        assert!(!native_tray_session_is_active("error"));
    }

    #[test]
    fn native_tray_language_override_precedes_three_language_system_fallback() {
        assert_eq!(
            effective_native_menu_language(Some("ja"), Some("zh-CN")),
            NativeMenuLanguage::Japanese
        );
        assert_eq!(
            effective_native_menu_language(None, Some("zh-Hans")),
            NativeMenuLanguage::Chinese
        );
        assert_eq!(
            effective_native_menu_language(Some("system"), Some("ja-JP")),
            NativeMenuLanguage::Japanese
        );
        assert_eq!(
            effective_native_menu_language(None, Some("en-US")),
            NativeMenuLanguage::English
        );
        assert_eq!(
            native_menu_labels(NativeMenuLanguage::Japanese).settings,
            "設定…"
        );
    }

    #[test]
    fn development_tauri_config_uses_the_isolated_identifier() {
        let config: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.dev.conf.json")).unwrap();
        assert_eq!(
            config["identifier"].as_str(),
            Some(DEVELOPMENT_APPLICATION_IDENTIFIER)
        );
    }

    #[test]
    fn windows_tray_asset_contains_visible_colour_pixels() {
        let windows_icon = tauri::image::Image::from_bytes(tray_icon_bytes(true)).unwrap();
        let template_icon = tauri::image::Image::from_bytes(tray_icon_bytes(false)).unwrap();

        assert_eq!((windows_icon.width(), windows_icon.height()), (32, 32));
        assert_ne!(windows_icon.rgba(), template_icon.rgba());
        assert!(windows_icon
            .rgba()
            .as_chunks::<4>()
            .0
            .iter()
            .any(|pixel| { pixel[3] != 0 && (pixel[0] != 0 || pixel[1] != 0 || pixel[2] != 0) }));
    }
}
