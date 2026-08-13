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

            #[cfg(target_os = "macos")]
            set_dock_icon(app.handle());

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

            // UI diagnostics probe: report the real DOM state inside each
            // webview a few seconds after launch (content-free: only counts,
            // colors, and readiness). Helps diagnose blank-window issues that
            // cannot be seen from outside the webview.
            {
                let app_handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                    for label in ["settings", "overlay", "tray-panel"] {
                        if let Some(window) = app_handle.get_webview_window(label) {
                            let script = format!(
                                r#"
                                (function() {{
                                    var state = JSON.stringify({{
                                        readyState: document.readyState,
                                        bodyTextLen: document.body.innerText.length,
                                        bodyBg: getComputedStyle(document.body).backgroundColor,
                                        rootChildren: document.getElementById('root') ? document.getElementById('root').children.length : -1,
                                        bodyClass: document.body.className,
                                        viewport: window.innerWidth + 'x' + window.innerHeight,
                                        href: location.href
                                    }});
                                    window.__TAURI_INTERNALS__.invoke('ui_probe_report', {{
                                        window: '{label}',
                                        state: state
                                    }});
                                }})()
                                "#
                            );
                            let _ = window.eval(&script);
                        }
                    }
                });
            }

            app.manage(AppState { settings, session });
            Ok(())
        })
        .on_window_event(|window, event| {
            let app = window.app_handle();
            match event {
                WindowEvent::Moved(_) | WindowEvent::Resized(_) if window.label() == "overlay" => {
                    if let Some(state) = app.try_state::<AppState>() {
                        // Do not overwrite the remembered expanded size while
                        // collapsed: the collapse animation resizes the window
                        // to 280x54 and would otherwise clobber the frame the
                        // next expand restores from.
                        if !state.session.is_overlay_collapsed() {
                            windows::OverlayWindowManager::persist_frame(app, &state.settings);
                        }
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
            commands::ui_probe_report,
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

    app.global_shortcut()
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
            if now_ms.saturating_sub(previous) < 2_000 {
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
        })
        .map_err(|error| std::io::Error::other(error.to_string()))?;
    tracing::info!("global shortcut registered");
    Ok(())
}

/// Sets the macOS Dock icon at runtime so `tauri dev` (which runs the bare
/// binary without an app bundle) shows the mimi icon instead of the generic
/// executable glyph. Uses dynamic class lookup (no extra AppKit binding
/// crates) and never panics: it runs inside the AppKit delegate callback
/// where panics cannot unwind, so every step falls back silently.
#[cfg(target_os = "macos")]
fn set_dock_icon(app: &tauri::AppHandle) {
    use objc2::msg_send;
    use objc2::runtime::{AnyClass, AnyObject};
    use objc2_foundation::{NSSize, NSString};
    use std::ffi::c_void;

    let Some(icon) = app.default_window_icon() else {
        return;
    };
    let width = icon.width() as i64;
    let height = icon.height() as i64;
    let rgba = icon.rgba();

    unsafe {
        let Some(rep_class) = AnyClass::get(c"NSBitmapImageRep") else {
            return;
        };
        let Some(image_class) = AnyClass::get(c"NSImage") else {
            return;
        };
        let Some(app_class) = AnyClass::get(c"NSApplication") else {
            return;
        };

        let color_space = NSString::from_str("NSCalibratedRGBColorSpace");
        let rep: *mut AnyObject = msg_send![rep_class, alloc];
        let rep: *mut AnyObject = msg_send![
            rep,
            initWithBitmapDataPlanes: std::ptr::null::<*mut c_void>(),
            pixelsWide: width,
            pixelsHigh: height,
            bitsPerSample: 8i64,
            samplesPerPixel: 4i64,
            hasAlpha: true,
            isPlanar: false,
            colorSpaceName: &*color_space,
            bytesPerRow: width * 4,
            bitsPerPixel: 32i64
        ];
        if rep.is_null() {
            return;
        }
        let data_ptr: *mut c_void = msg_send![rep, bitmapData];
        if data_ptr.is_null() {
            return;
        }
        std::ptr::copy_nonoverlapping(rgba.as_ptr(), data_ptr as *mut u8, rgba.len());

        let image: *mut AnyObject = msg_send![image_class, alloc];
        let image: *mut AnyObject = msg_send![
            image,
            initWithSize: NSSize::new(width as f64, height as f64)
        ];
        if image.is_null() {
            return;
        }
        let _: () = msg_send![image, addRepresentation: rep];

        let shared: *mut AnyObject = msg_send![app_class, sharedApplication];
        if shared.is_null() {
            return;
        }
        let _: () = msg_send![shared, setApplicationIconImage: image];
        tracing::info!("dock icon applied");
    }
}
