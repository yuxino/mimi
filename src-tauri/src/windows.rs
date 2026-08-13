//! Overlay and tray-panel window management, ported from
//! `Sources/MimiApp/OverlayWindowController.swift` and the menu-bar popover
//! behavior in `MimiApp.swift`.

use crate::pipeline_log;
use crate::settings_store::{OverlayFrame, SettingsStore};
use std::sync::Arc;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

/// Logical-coordinate frame layout version. Frames saved before this version
/// stored physical pixels and must be discarded (they restore off-screen on
/// Retina displays).
pub const OVERLAY_FRAME_LAYOUT_VERSION: u64 = 1;

pub struct SubtitleOverlayMetrics;

impl SubtitleOverlayMetrics {
    pub const REFERENCE_WIDTH: f64 = 640.0;
    pub const REFERENCE_HEIGHT: f64 = 136.0;
    pub const MINIMUM_WIDTH: f64 = 360.0;
    pub const MINIMUM_HEIGHT: f64 = 100.0;
    pub const MAXIMUM_WIDTH: f64 = 1_200.0;
    pub const MAXIMUM_HEIGHT: f64 = 600.0;
    pub const COLLAPSED_WIDTH: f64 = 280.0;
    pub const COLLAPSED_HEIGHT: f64 = 54.0;
}

pub struct OverlayWindowManager;

impl OverlayWindowManager {
    pub fn ensure_overlay(app: &AppHandle, settings: &SettingsStore) {
        if app.get_webview_window("overlay").is_some() {
            return;
        }
        let prefs = settings.preferences();
        // Frames from before the logical-coordinate migration are unreliable
        // and can sit off-screen; fall back to the default placement.
        let trusted_frame = (prefs.frame_layout_version >= OVERLAY_FRAME_LAYOUT_VERSION)
            .then(|| prefs.overlay_frame.clone())
            .flatten();
        let frame = trusted_frame.clone().unwrap_or(OverlayFrame {
            x: 0.0,
            y: 0.0,
            width: SubtitleOverlayMetrics::REFERENCE_WIDTH,
            height: SubtitleOverlayMetrics::REFERENCE_HEIGHT,
        });
        let (x, y) = default_overlay_origin(app, frame.width, frame.height, &trusted_frame);

        let builder =
            WebviewWindowBuilder::new(app, "overlay", WebviewUrl::App("index.html".into()))
                .title("mimi Subtitles")
                .inner_size(frame.width, frame.height)
                .min_inner_size(
                    SubtitleOverlayMetrics::MINIMUM_WIDTH,
                    SubtitleOverlayMetrics::MINIMUM_HEIGHT,
                )
                .max_inner_size(
                    SubtitleOverlayMetrics::MAXIMUM_WIDTH,
                    SubtitleOverlayMetrics::MAXIMUM_HEIGHT,
                )
                .position(x, y)
                .transparent(true)
                .decorations(false)
                .always_on_top(true)
                .skip_taskbar(true)
                .shadow(false)
                .resizable(true)
                .visible(false);

        match builder.build() {
            Ok(_) => pipeline_log!("overlay window created"),
            Err(error) => pipeline_log!("overlay window failed error={}", error),
        }
    }

    pub fn sync_visibility(app: &AppHandle, is_active: bool) {
        if is_active {
            if let Some(window) = app.get_webview_window("overlay") {
                let _ = window.show();
            }
        } else if let Some(window) = app.get_webview_window("overlay") {
            let _ = window.hide();
        }
    }

    pub fn show(app: &AppHandle) {
        if let Some(window) = app.get_webview_window("overlay") {
            let _ = window.show();
        }
    }

    pub fn hide(app: &AppHandle) {
        if let Some(window) = app.get_webview_window("overlay") {
            let _ = window.hide();
        }
    }

    /// Locks the overlay: the window ignores mouse events so clicks pass
    /// through to whatever plays underneath.
    pub fn update_locked(app: &AppHandle, locked: bool) {
        if let Some(window) = app.get_webview_window("overlay") {
            let _ = window.set_ignore_cursor_events(locked);
        }
    }

    /// Collapses to 280×54 or expands to the remembered size, mirroring
    /// `SubtitleOverlayCollapseLayout` plus the frame settle pass.
    pub fn set_collapsed(app: &AppHandle, settings: &SettingsStore, collapsed: bool) {
        let Some(window) = app.get_webview_window("overlay") else {
            return;
        };
        if collapsed {
            if let (Ok(size), Ok(position)) = (window.inner_size(), window.outer_position()) {
                let scale = window.scale_factor().unwrap_or(1.0).max(1.0);
                remember_frame(
                    settings,
                    Some(tauri::LogicalPosition::new(
                        position.x as f64 / scale,
                        position.y as f64 / scale,
                    )),
                    tauri::LogicalSize::new(size.width as f64 / scale, size.height as f64 / scale),
                );
            }
            let _ = window.set_min_size(Some(tauri::LogicalSize::new(
                SubtitleOverlayMetrics::COLLAPSED_WIDTH,
                SubtitleOverlayMetrics::COLLAPSED_HEIGHT,
            )));
            let _ = window.set_max_size(Some(tauri::LogicalSize::new(
                SubtitleOverlayMetrics::COLLAPSED_WIDTH,
                SubtitleOverlayMetrics::COLLAPSED_HEIGHT,
            )));
            let _ = window.set_size(tauri::LogicalSize::new(
                SubtitleOverlayMetrics::COLLAPSED_WIDTH,
                SubtitleOverlayMetrics::COLLAPSED_HEIGHT,
            ));
        } else {
            let _ = window.set_min_size(Some(tauri::LogicalSize::new(
                SubtitleOverlayMetrics::MINIMUM_WIDTH,
                SubtitleOverlayMetrics::MINIMUM_HEIGHT,
            )));
            let _ = window.set_max_size(Some(tauri::LogicalSize::new(
                SubtitleOverlayMetrics::MAXIMUM_WIDTH,
                SubtitleOverlayMetrics::MAXIMUM_HEIGHT,
            )));
            let frame = settings
                .preferences()
                .overlay_frame
                .unwrap_or(OverlayFrame {
                    x: 0.0,
                    y: 0.0,
                    width: SubtitleOverlayMetrics::REFERENCE_WIDTH,
                    height: SubtitleOverlayMetrics::REFERENCE_HEIGHT,
                });
            let _ = window.set_size(tauri::LogicalSize::new(frame.width, frame.height));
        }

        // Settle the exact size once animations have had time to finish,
        // mirroring the Swift `settleFrame` pass: the collapsed bar, or the
        // remembered expanded size when un-collapsing.
        let window_settle = window.clone();
        let target = if collapsed {
            tauri::LogicalSize::new(
                SubtitleOverlayMetrics::COLLAPSED_WIDTH,
                SubtitleOverlayMetrics::COLLAPSED_HEIGHT,
            )
        } else {
            let prefs = settings.preferences();
            let trusted = (prefs.frame_layout_version >= OVERLAY_FRAME_LAYOUT_VERSION)
                .then(|| prefs.overlay_frame.clone())
                .flatten();
            let frame = trusted.unwrap_or(OverlayFrame {
                x: 0.0,
                y: 0.0,
                width: SubtitleOverlayMetrics::REFERENCE_WIDTH,
                height: SubtitleOverlayMetrics::REFERENCE_HEIGHT,
            });
            tauri::LogicalSize::new(frame.width, frame.height)
        };
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(400)).await;
            let _ = window_settle.set_size(target);
        });
    }

    /// The frontend draws its own resize handles and calls this with the new
    /// size while dragging.
    pub fn set_size(app: &AppHandle, width: f64, height: f64) {
        let width = width.clamp(
            SubtitleOverlayMetrics::MINIMUM_WIDTH,
            SubtitleOverlayMetrics::MAXIMUM_WIDTH,
        );
        let height = height.clamp(
            SubtitleOverlayMetrics::MINIMUM_HEIGHT,
            SubtitleOverlayMetrics::MAXIMUM_HEIGHT,
        );
        if let Some(window) = app.get_webview_window("overlay") {
            let _ = window.set_size(tauri::LogicalSize::new(width, height));
        }
    }

    pub fn persist_frame(app: &AppHandle, settings: &SettingsStore) {
        let Some(window) = app.get_webview_window("overlay") else {
            return;
        };
        if let (Ok(size), Ok(position)) = (window.inner_size(), window.outer_position()) {
            // Store logical coordinates: `set_position`/`set_size` (used when
            // restoring) interpret their arguments in logical pixels.
            let scale = window.scale_factor().unwrap_or(1.0).max(1.0);
            remember_frame(
                settings,
                Some(tauri::LogicalPosition::new(
                    position.x as f64 / scale,
                    position.y as f64 / scale,
                )),
                tauri::LogicalSize::new(size.width as f64 / scale, size.height as f64 / scale),
            );
        }
    }
}

fn remember_frame(
    settings: &SettingsStore,
    position: Option<tauri::LogicalPosition<f64>>,
    size: tauri::LogicalSize<f64>,
) {
    let prefs = settings.preferences();
    if prefs.overlay_locked {
        return;
    }
    if let Some(position) = position {
        settings.update_preferences(|prefs| {
            prefs.overlay_frame = Some(OverlayFrame {
                x: position.x,
                y: position.y,
                width: size.width,
                height: size.height,
            });
            prefs.frame_layout_version = OVERLAY_FRAME_LAYOUT_VERSION;
        });
        settings.persist();
    }
}

fn default_overlay_origin(
    app: &AppHandle,
    width: f64,
    height: f64,
    saved: &Option<OverlayFrame>,
) -> (f64, f64) {
    if let Some(saved) = saved {
        if saved.width >= SubtitleOverlayMetrics::MINIMUM_WIDTH
            && saved.height >= SubtitleOverlayMetrics::MINIMUM_HEIGHT
        {
            return (saved.x, saved.y);
        }
    }
    // Bottom-center of the primary monitor, 72 logical px above the bottom,
    // matching the Swift default placement.
    if let Some(monitor) = app.primary_monitor().ok().flatten() {
        let scale = monitor.scale_factor().max(1.0);
        let size = monitor.size();
        let workable_height = size.height as f64 / scale;
        let screen_width = size.width as f64 / scale;
        return (
            (screen_width - width) / 2.0,
            (workable_height - height - 72.0).max(0.0),
        );
    }
    (0.0, 0.0)
}

/// Tray popup panel: a small frameless always-on-top window shown under the
/// tray icon, mirroring the SwiftUI MenuBarExtra window style.
pub struct TrayPanelManager;

impl TrayPanelManager {
    pub fn ensure(app: &AppHandle) {
        if app.get_webview_window("tray-panel").is_some() {
            return;
        }
        let builder =
            WebviewWindowBuilder::new(app, "tray-panel", WebviewUrl::App("index.html".into()))
                .title("mimi")
                .inner_size(290.0, 470.0)
                .resizable(false)
                .transparent(true)
                .decorations(false)
                .always_on_top(true)
                .skip_taskbar(true)
                .shadow(false)
                .visible(false);

        match builder.build() {
            Ok(_) => pipeline_log!("tray panel created"),
            Err(error) => pipeline_log!("tray panel failed error={}", error),
        }
    }

    pub fn toggle(app: &AppHandle) {
        let Some(window) = app.get_webview_window("tray-panel") else {
            return;
        };
        let is_visible = window.is_visible().unwrap_or(false);
        if is_visible {
            let _ = window.hide();
            return;
        }
        use tauri_plugin_positioner::{Position, WindowExt};
        let _ = window.move_window(Position::TrayBottomCenter);
        let _ = window.show();
        let _ = window.set_focus();
    }

    pub fn hide(app: &AppHandle) {
        if let Some(window) = app.get_webview_window("tray-panel") {
            let _ = window.hide();
        }
    }
}

#[allow(dead_code)]
fn _unused(_: Arc<()>) {}
