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
use settings_store::{SettingsStore, DEVELOPMENT_APPLICATION_IDENTIFIER};
use std::sync::Arc;
use tauri::menu::{
    CheckMenuItem, CheckMenuItemBuilder, MenuBuilder, MenuItem, MenuItemBuilder, Submenu,
    SubmenuBuilder,
};
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
            if windows::is_dev_build()
                && app.config().identifier.as_str() != DEVELOPMENT_APPLICATION_IDENTIFIER
            {
                return Err("development builds require the isolated Tauri identifier".into());
            }
            let is_ui_test = std::env::var("MIMI_UI_TEST").as_deref() == Ok("1");
            let settings = Arc::new(SettingsStore::load(
                app.path().app_config_dir().unwrap_or_default(),
                is_ui_test,
                &app.config().identifier,
            ));
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
            windows::OverlayWindowManager::ensure_overlay(&app_handle, &overlay);
            windows::TrayPanelManager::ensure(&app_handle);
            windows::LanguagePopoverManager::ensure(&app_handle);

            setup_tray(&app_handle)?;
            setup_global_shortcut(&app_handle, Arc::clone(&session))?;

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
            commands::profile_create,
            commands::profile_update,
            commands::profile_select,
            commands::profile_delete,
            commands::profile_save_api_key,
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
            commands::overlay_set_size,
            commands::overlay_popover_toggle,
            commands::overlay_popover_hide,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NativeMenuLanguage {
    Chinese,
    English,
    Japanese,
}

#[derive(Debug, Clone, Copy)]
struct NativeMenuLabels {
    live_subtitles: &'static str,
    lock_position: &'static str,
    language_menu: &'static str,
    lang_auto: &'static str,
    lang_ja: &'static str,
    lang_en: &'static str,
    lang_ko: &'static str,
    lang_zh: &'static str,
    show_subtitles: &'static str,
    clear_subtitles: &'static str,
    toggle_devtools: &'static str,
    settings: &'static str,
    quit: &'static str,
}

#[derive(Clone)]
struct NativeTrayMenuItems {
    live_subtitles: CheckMenuItem<tauri::Wry>,
    lock_position: CheckMenuItem<tauri::Wry>,
    language_menu: Submenu<tauri::Wry>,
    lang_auto: MenuItem<tauri::Wry>,
    lang_ja: MenuItem<tauri::Wry>,
    lang_en: MenuItem<tauri::Wry>,
    lang_ko: MenuItem<tauri::Wry>,
    lang_zh: MenuItem<tauri::Wry>,
    show_subtitles: MenuItem<tauri::Wry>,
    clear_subtitles: MenuItem<tauri::Wry>,
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
            live_subtitles: "实时字幕",
            lock_position: "锁定字幕位置",
            language_menu: "识别语言",
            lang_auto: "自动识别",
            lang_ja: "日语",
            lang_en: "英语",
            lang_ko: "韩语",
            lang_zh: "中文原文",
            show_subtitles: "显示字幕窗口",
            clear_subtitles: "清空字幕",
            toggle_devtools: "打开调试工具",
            settings: "设置…",
            quit: "退出 mimi",
        },
        NativeMenuLanguage::Japanese => NativeMenuLabels {
            live_subtitles: "ライブ字幕",
            lock_position: "字幕位置を固定",
            language_menu: "認識言語",
            lang_auto: "自動検出",
            lang_ja: "日本語",
            lang_en: "英語",
            lang_ko: "韓国語",
            lang_zh: "中国語（原文）",
            show_subtitles: "字幕ウィンドウを表示",
            clear_subtitles: "字幕を消去",
            toggle_devtools: "開発者ツールを開く",
            settings: "設定…",
            quit: "mimiを終了",
        },
        NativeMenuLanguage::English => NativeMenuLabels {
            live_subtitles: "Live Subtitles",
            lock_position: "Lock Subtitle Position",
            language_menu: "Recognition Language",
            lang_auto: "Auto Detect",
            lang_ja: "Japanese",
            lang_en: "English",
            lang_ko: "Korean",
            lang_zh: "Chinese (Original)",
            show_subtitles: "Show Subtitle Window",
            clear_subtitles: "Clear Subtitles",
            toggle_devtools: "Open DevTools",
            settings: "Settings…",
            quit: "Quit mimi",
        },
    }
}

/// Reads the macOS language code used when the preference follows the system.
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

#[cfg(not(target_os = "macos"))]
fn system_language_code() -> Option<String> {
    None
}

/// Tray icon with the mimi menu (start/stop, language, lock, settings, quit)
/// and a left-click popup control panel. Copy follows the saved UI override,
/// falling back to the system language.
fn setup_tray(app: &tauri::AppHandle) -> tauri::Result<()> {
    let override_language = app
        .try_state::<AppState>()
        .and_then(|state| state.settings.preferences().ui_language);
    let system_language = system_language_code();
    let labels = native_menu_labels(effective_native_menu_language(
        override_language.as_deref(),
        system_language.as_deref(),
    ));

    let live_subtitles =
        CheckMenuItemBuilder::with_id("live-subtitles", labels.live_subtitles).build(app)?;
    let lock_position =
        CheckMenuItemBuilder::with_id("lock-position", labels.lock_position).build(app)?;

    let lang_auto = MenuItemBuilder::with_id("lang-auto", labels.lang_auto).build(app)?;
    let lang_ja = MenuItemBuilder::with_id("lang-ja", labels.lang_ja).build(app)?;
    let lang_en = MenuItemBuilder::with_id("lang-en", labels.lang_en).build(app)?;
    let lang_ko = MenuItemBuilder::with_id("lang-ko", labels.lang_ko).build(app)?;
    let lang_zh = MenuItemBuilder::with_id("lang-zh", labels.lang_zh).build(app)?;

    let language_menu = SubmenuBuilder::new(app, labels.language_menu)
        .item(&lang_auto)
        .item(&lang_ja)
        .item(&lang_en)
        .item(&lang_ko)
        .item(&lang_zh)
        .build()?;

    let show_subtitles =
        MenuItemBuilder::with_id("show-subtitles", labels.show_subtitles).build(app)?;
    let clear_subtitles =
        MenuItemBuilder::with_id("clear-subtitles", labels.clear_subtitles).build(app)?;

    let mut menu_builder = MenuBuilder::new(app)
        .item(&live_subtitles)
        .item(&language_menu)
        .item(&lock_position)
        .item(&show_subtitles)
        .item(&clear_subtitles)
        .separator();

    // Dev builds expose a manual "open inspector" action instead of opening
    // the WebView devtools automatically, so the user decides when to look.
    let toggle_devtools = windows::is_dev_build()
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
    let icon = tauri::image::Image::from_bytes(include_bytes!("../icons/tray-template.png"))
        .ok()
        .or_else(|| app.default_window_icon().cloned())
        .expect("the tray icon is bundled");

    let live_subtitles_clone = live_subtitles.clone();
    let lock_position_clone = lock_position.clone();
    let language_items = [
        (
            crate::core::models::SourceLanguage::Automatic,
            lang_auto.clone(),
        ),
        (
            crate::core::models::SourceLanguage::Japanese,
            lang_ja.clone(),
        ),
        (
            crate::core::models::SourceLanguage::English,
            lang_en.clone(),
        ),
        (crate::core::models::SourceLanguage::Korean, lang_ko.clone()),
        (
            crate::core::models::SourceLanguage::Chinese,
            lang_zh.clone(),
        ),
    ];
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
                    match state
                        .settings
                        .save_preferences(|prefs| prefs.overlay_locked = locked)
                    {
                        Ok(()) => {
                            windows::OverlayWindowManager::update_locked(app, locked);
                            let _ = app.emit(
                                "settings-changed",
                                commands::SettingsSnapshotPayload::from_store(&state.settings),
                            );
                        }
                        Err(_) => {
                            tracing::warn!("preferences unavailable label=tray_lock_write_failed")
                        }
                    }
                }
                "show-subtitles" => windows::OverlayWindowManager::show(app),
                "clear-subtitles" => state.session.clear_subtitles(),
                "toggle-devtools" => {
                    // Manual inspector toggle (dev builds only). Open the
                    // overlay's WebView devtools so the user decides when to
                    // inspect, instead of auto-opening at startup.
                    if let Some(window) = app.get_webview_window("overlay") {
                        window.open_devtools();
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

    app.manage(NativeTrayMenuItems {
        live_subtitles,
        lock_position,
        language_menu,
        lang_auto,
        lang_ja,
        lang_en,
        lang_ko,
        lang_zh,
        show_subtitles,
        clear_subtitles,
        toggle_devtools,
        settings: settings_item,
        quit: quit_item,
    });

    if let Some(state) = app.try_state::<AppState>() {
        update_tray_language_availability(&state, &language_items);
    }

    // Keep the live-subtitles check state in sync with the session. The
    // polling interval is deliberately coarse: the check states change only
    // on user actions, so 2s staleness is invisible while the loop stays
    // nearly free.
    let app_for_state = app.clone();
    let language_items_for_state = language_items.clone();
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
            update_tray_language_availability(&state, &language_items_for_state);
        }
    });

    Ok(())
}

pub(crate) fn refresh_native_tray_language(app: &tauri::AppHandle) {
    let override_language = app
        .try_state::<AppState>()
        .and_then(|state| state.settings.preferences().ui_language);
    let system_language = system_language_code();
    let labels = native_menu_labels(effective_native_menu_language(
        override_language.as_deref(),
        system_language.as_deref(),
    ));
    let Some(items) = app.try_state::<NativeTrayMenuItems>() else {
        return;
    };

    let _ = items.live_subtitles.set_text(labels.live_subtitles);
    let _ = items.lock_position.set_text(labels.lock_position);
    let _ = items.language_menu.set_text(labels.language_menu);
    let _ = items.lang_auto.set_text(labels.lang_auto);
    let _ = items.lang_ja.set_text(labels.lang_ja);
    let _ = items.lang_en.set_text(labels.lang_en);
    let _ = items.lang_ko.set_text(labels.lang_ko);
    let _ = items.lang_zh.set_text(labels.lang_zh);
    let _ = items.show_subtitles.set_text(labels.show_subtitles);
    let _ = items.clear_subtitles.set_text(labels.clear_subtitles);
    if let Some(item) = &items.toggle_devtools {
        let _ = item.set_text(labels.toggle_devtools);
    }
    let _ = items.settings.set_text(labels.settings);
    let _ = items.quit.set_text(labels.quit);
}

fn update_tray_language_availability(
    state: &AppState,
    items: &[(crate::core::models::SourceLanguage, MenuItem<tauri::Wry>)],
) {
    let status = state.session.status_kind();
    let provider = state
        .settings
        .active_profile()
        .map(|profile| profile.provider);
    for (language, item) in items {
        let enabled = provider
            .as_ref()
            .map(|provider| tray_language_is_enabled(*provider, &status, *language))
            .unwrap_or(false);
        if item.is_enabled().unwrap_or(!enabled) != enabled {
            let _ = item.set_enabled(enabled);
        }
    }
}

fn tray_language_is_enabled(
    provider: crate::core::provider::ProviderKind,
    status: &str,
    language: crate::core::models::SourceLanguage,
) -> bool {
    !matches!(status, "connecting" | "stopping")
        && provider.capabilities().source_languages.contains(&language)
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
/// original app), Ctrl+Shift+Space on Windows. A 500ms debounce mirrors the
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::models::SourceLanguage;
    use crate::core::provider::ProviderKind;

    #[test]
    fn native_tray_languages_follow_provider_capabilities_and_session_state() {
        assert!(tray_language_is_enabled(
            ProviderKind::OpenAIRealtime,
            "idle",
            SourceLanguage::Automatic,
        ));
        assert!(!tray_language_is_enabled(
            ProviderKind::OpenAIRealtime,
            "idle",
            SourceLanguage::Japanese,
        ));
        assert!(!tray_language_is_enabled(
            ProviderKind::AlibabaCloud,
            "connecting",
            SourceLanguage::Japanese,
        ));
        assert!(tray_language_is_enabled(
            ProviderKind::AlibabaCloud,
            "listening",
            SourceLanguage::Japanese,
        ));
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
}
