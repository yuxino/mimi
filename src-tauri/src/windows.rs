//! Overlay and tray-panel window management, ported from
//! `Sources/MimiApp/OverlayWindowController.swift` and the menu-bar popover
//! behavior in `MimiApp.swift`.
//!
//! # Overlay geometry state machine
//!
//! The overlay window has several *visual* shapes but exactly one *user*
//! frame. [`OverlayState`] owns that state:
//!
//! * `user_frame` — the user's chosen expanded size/position. The only frame
//!   that is ever persisted. Mutated exclusively by resize drags and window
//!   moves (debounced); the popover and collapse never touch it.
//! * `mode` — [`OverlayMode`]. Collapse and the language/mode popover are
//!   temporary visual states *derived* from the user frame.
//!
//! [`OverlayWindowManager::apply`] is the single place that writes OS window
//! geometry: it derives size/position/min/max from `(mode, user_frame)`. OS
//! `Moved`/`Resized` events never persist directly — they only arm a debounced
//! commit (so native drags are still remembered), and the popover mode is
//! never persisted at all. This replaces the earlier design where resize,
//! popover, collapse animation and event persistence all mutated the window
//! directly and negotiated via a global `POPOVER_RESIZING` flag, which caused
//! oscillation, stuck drags, height corruption and lost frames.

use crate::pipeline_log;
use crate::settings_store::{OverlayFrame, SettingsStore};
use crate::windows::resize::{apply_drag, ResizeRegion};
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

/// The overlay's visual state. The language/mode menu lives in its own
/// window (see [`LanguagePopoverManager`], mirroring the Swift NSPopover), so
/// the overlay window has no menu-related state and its height is never
/// affected by the menu.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OverlayMode {
    Expanded,
    Collapsed,
}

/// All overlay geometry state: the single source of truth for the overlay
/// window's size and position.
#[derive(Debug)]
pub struct OverlayState {
    pub mode: OverlayMode,
    /// The user's chosen expanded frame; the only persisted frame.
    pub user_frame: OverlayFrame,
    /// Active resize drag, if any (only valid while `mode == Expanded`).
    pub resize_drag: Option<ResizeRegion>,
    /// Pointer position and frame snapshot taken at `resize_start`.
    pub resize_start: Option<(f64, f64, OverlayFrame)>,
    /// Timestamp of the most recent Moved/Resized event; debounced commits
    /// compare against it to drop superseded tasks.
    last_geometry_event: Option<std::time::Instant>,
}

impl OverlayState {
    /// Loads the persisted frame (or the default bottom-center placement) and
    /// starts in the expanded mode.
    pub fn load(app: &AppHandle, settings: &SettingsStore) -> Self {
        let prefs = settings.preferences();
        let trusted = (prefs.frame_layout_version >= OVERLAY_FRAME_LAYOUT_VERSION)
            .then_some(prefs.overlay_frame)
            .flatten()
            .filter(|frame| {
                frame.width >= SubtitleOverlayMetrics::MINIMUM_WIDTH
                    && frame.height >= SubtitleOverlayMetrics::MINIMUM_HEIGHT
            });
        let width = trusted
            .as_ref()
            .map_or(SubtitleOverlayMetrics::REFERENCE_WIDTH, |f| f.width);
        let height = trusted
            .as_ref()
            .map_or(SubtitleOverlayMetrics::REFERENCE_HEIGHT, |f| f.height);
        let (x, y) = default_overlay_origin(app, width, height, &trusted);
        Self {
            mode: OverlayMode::Expanded,
            user_frame: OverlayFrame {
                x,
                y,
                width,
                height,
            },
            resize_drag: None,
            resize_start: None,
            last_geometry_event: None,
        }
    }
}

/// OS window geometry derived from `(mode, user_frame)`; all logical pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
struct WindowGeometry {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    min: (f64, f64),
    max: (f64, f64),
}

/// Pure derivation of the window geometry from the overlay state. `screen` is
/// the logical size of the screen the overlay currently sits on.
fn geometry_for(mode: OverlayMode, user_frame: &OverlayFrame) -> WindowGeometry {
    let min = (
        SubtitleOverlayMetrics::MINIMUM_WIDTH,
        SubtitleOverlayMetrics::MINIMUM_HEIGHT,
    );
    let max = (
        SubtitleOverlayMetrics::MAXIMUM_WIDTH,
        SubtitleOverlayMetrics::MAXIMUM_HEIGHT,
    );
    match mode {
        OverlayMode::Expanded => WindowGeometry {
            x: user_frame.x,
            y: user_frame.y,
            width: user_frame.width,
            height: user_frame.height,
            min,
            max,
        },
        OverlayMode::Collapsed => WindowGeometry {
            x: user_frame.x,
            y: user_frame.y,
            width: SubtitleOverlayMetrics::COLLAPSED_WIDTH,
            height: SubtitleOverlayMetrics::COLLAPSED_HEIGHT,
            // Fixed size: min == max so the OS cannot resize the collapsed bar.
            min: (
                SubtitleOverlayMetrics::COLLAPSED_WIDTH,
                SubtitleOverlayMetrics::COLLAPSED_HEIGHT,
            ),
            max: (
                SubtitleOverlayMetrics::COLLAPSED_WIDTH,
                SubtitleOverlayMetrics::COLLAPSED_HEIGHT,
            ),
        },
    }
}

pub struct OverlayWindowManager;

impl OverlayWindowManager {
    pub fn ensure_overlay(app: &AppHandle, state: &Arc<std::sync::Mutex<OverlayState>>) {
        if app.get_webview_window("overlay").is_some() {
            return;
        }
        let frame = state.lock().unwrap().user_frame;
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
                .position(frame.x, frame.y)
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

    /// The single writer of OS window geometry. Derives size/position/min/max
    /// from the overlay state and applies them (optionally animated).
    pub fn apply(app: &AppHandle, state: &OverlayState, animate: bool) {
        let Some(window) = app.get_webview_window("overlay") else {
            return;
        };
        let geometry = geometry_for(state.mode, &state.user_frame);
        let _ = window.set_min_size(Some(tauri::LogicalSize::new(
            geometry.min.0,
            geometry.min.1,
        )));
        let _ = window.set_max_size(Some(tauri::LogicalSize::new(
            geometry.max.0,
            geometry.max.1,
        )));
        if animate {
            let scale = window.scale_factor().unwrap_or(1.0).max(1.0);
            let _ = window.set_position(tauri::LogicalPosition::new(geometry.x, geometry.y));
            animate_resize(&window, scale, geometry.width, geometry.height);
        } else {
            let _ = window.set_position(tauri::LogicalPosition::new(geometry.x, geometry.y));
            let _ = window.set_size(tauri::LogicalSize::new(geometry.width, geometry.height));
        }
    }

    /// Collapses to 280×54 or expands to the remembered frame. The size
    /// change is animated (≈180 ms easeInOut, matching the Swift transition),
    /// and on expand the frame is constrained back onto the visible screen.
    pub fn set_collapsed(
        app: &AppHandle,
        state: &Arc<std::sync::Mutex<OverlayState>>,
        collapsed: bool,
    ) {
        let mut state = state.lock().unwrap();
        let new_mode = if collapsed {
            OverlayMode::Collapsed
        } else {
            OverlayMode::Expanded
        };
        if state.mode == new_mode {
            return;
        }
        if collapsed {
            // Adopt the exact current window frame before shrinking so an
            // expand later restores precisely what the user saw.
            sync_user_frame_from_window(app, &mut state);
        } else {
            // The screen configuration may have changed while collapsed;
            // keep the expanded frame on the visible screen.
            clamp_user_frame_to_screen(app, &mut state.user_frame);
        }
        state.mode = new_mode;
        Self::apply(app, &state, true);
    }

    /// Persists the current window frame as the user frame (explicit commit
    /// points: end of a resize drag).
    pub fn commit(
        app: &AppHandle,
        state: &Arc<std::sync::Mutex<OverlayState>>,
        settings: &SettingsStore,
    ) {
        let mut state = state.lock().unwrap();
        if state.mode != OverlayMode::Expanded {
            return;
        }
        sync_user_frame_from_window(app, &mut state);
        persist_user_frame(settings, &state.user_frame);
    }

    /// Handles OS Moved/Resized events for the overlay. Never persists
    /// immediately: a debounced task (350 ms after the last event) folds the
    /// final window frame into the user frame, so native drags are remembered
    /// but transient animation steps are not.
    pub fn on_geometry_event(
        app: &AppHandle,
        state: &Arc<std::sync::Mutex<OverlayState>>,
        settings: &Arc<SettingsStore>,
    ) {
        let now = std::time::Instant::now();
        {
            let mut state = state.lock().unwrap();
            state.last_geometry_event = Some(now);
        }
        let app = app.clone();
        let state = Arc::clone(state);
        let settings = Arc::clone(settings);
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(350)).await;
            let mut state = state.lock().unwrap();
            // A newer event arrived while we slept; it owns the commit.
            if state.last_geometry_event != Some(now) {
                return;
            }
            match state.mode {
                OverlayMode::Expanded => {
                    sync_user_frame_from_window(&app, &mut state);
                    persist_user_frame(&settings, &state.user_frame);
                }
                OverlayMode::Collapsed => {
                    // Only the position is meaningful while collapsed; the
                    // remembered expanded size must survive.
                    sync_position_from_window(&app, &mut state.user_frame);
                    persist_user_frame(&settings, &state.user_frame);
                }
            }
        });
    }

    /// Begins an overlay resize drag. `region` is one of topLeft/top/topRight/
    /// left/right/bottomLeft/bottom/bottomRight; `x`/`y` are the pointer
    /// position in screen (logical) coordinates.
    pub fn resize_start(
        app: &AppHandle,
        state: &Arc<std::sync::Mutex<OverlayState>>,
        region_name: &str,
        x: f64,
        y: f64,
    ) -> Result<(), String> {
        let region = ResizeRegion::from_name(region_name)
            .ok_or_else(|| format!("unknown resize region: {region_name}"))?;
        let mut state = state.lock().unwrap();
        // Resizing is meaningless (and corrupting) in any transient mode.
        if state.mode != OverlayMode::Expanded {
            return Ok(());
        }
        sync_user_frame_from_window(app, &mut state);
        state.resize_drag = Some(region);
        state.resize_start = Some((x, y, state.user_frame));
        tracing::info!(
            "resize start region={region:?} frame={:?}",
            (
                state.user_frame.x,
                state.user_frame.y,
                state.user_frame.width,
                state.user_frame.height
            )
        );
        Ok(())
    }

    /// Continues a resize drag with the current pointer position (screen
    /// logical px). The dragged edge/corner stays anchored, sizes clamp to
    /// min/max, and the frame keeps at least 48 px on screen.
    pub fn resize_move(
        app: &AppHandle,
        state: &Arc<std::sync::Mutex<OverlayState>>,
        x: f64,
        y: f64,
    ) {
        let mut state = state.lock().unwrap();
        let Some(region) = state.resize_drag else {
            return;
        };
        let Some((start_x, start_y, start_frame)) = state.resize_start else {
            return;
        };
        let Some(window) = app.get_webview_window("overlay") else {
            return;
        };
        let scale = window.scale_factor().unwrap_or(1.0).max(1.0);
        let screen = current_screen_logical(app, &window, scale);
        let frame = apply_drag(
            region,
            (start_x, start_y),
            &start_frame,
            (x, y),
            (
                SubtitleOverlayMetrics::MINIMUM_WIDTH,
                SubtitleOverlayMetrics::MINIMUM_HEIGHT,
            ),
            (
                SubtitleOverlayMetrics::MAXIMUM_WIDTH,
                SubtitleOverlayMetrics::MAXIMUM_HEIGHT,
            ),
            screen,
            48.0,
        );
        state.user_frame = frame;
        Self::apply(app, &state, false);
        tracing::info!(
            "resize move frame={:?}",
            (frame.x, frame.y, frame.width, frame.height)
        );
    }

    /// Ends a resize drag and persists the final frame.
    pub fn resize_end(
        app: &AppHandle,
        state: &Arc<std::sync::Mutex<OverlayState>>,
        settings: &SettingsStore,
    ) {
        let mut state = state.lock().unwrap();
        state.resize_drag = None;
        state.resize_start = None;
        if state.mode != OverlayMode::Expanded {
            return;
        }
        sync_user_frame_from_window(app, &mut state);
        persist_user_frame(settings, &state.user_frame);
        tracing::info!("resize end");
    }

    /// Legacy size setter (kept for the IPC contract; the Tauri frontend
    /// resizes through `resize_start/move/end` instead).
    pub fn set_size(
        app: &AppHandle,
        state: &Arc<std::sync::Mutex<OverlayState>>,
        width: f64,
        height: f64,
    ) {
        let mut state = state.lock().unwrap();
        if state.mode != OverlayMode::Expanded {
            return;
        }
        state.user_frame.width = width.clamp(
            SubtitleOverlayMetrics::MINIMUM_WIDTH,
            SubtitleOverlayMetrics::MAXIMUM_WIDTH,
        );
        state.user_frame.height = height.clamp(
            SubtitleOverlayMetrics::MINIMUM_HEIGHT,
            SubtitleOverlayMetrics::MAXIMUM_HEIGHT,
        );
        Self::apply(app, &state, false);
    }
}

// ---------------------------------------------------------------------------
// Geometry helpers
// ---------------------------------------------------------------------------

/// Logical size of the screen the overlay currently sits on (falls back to
/// the primary monitor, then a conservative default).
fn current_screen_logical(
    app: &AppHandle,
    window: &tauri::WebviewWindow,
    scale: f64,
) -> (f64, f64) {
    let monitor_size = window
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| app.primary_monitor().ok().flatten())
        .map(|monitor| *monitor.size());
    match monitor_size {
        Some(size) => (size.width as f64 / scale, size.height as f64 / scale),
        None => (1440.0, 900.0),
    }
}

/// Adopts the OS window's current frame as the user frame (logical coords).
fn sync_user_frame_from_window(app: &AppHandle, state: &mut OverlayState) {
    let Some(window) = app.get_webview_window("overlay") else {
        return;
    };
    let scale = window.scale_factor().unwrap_or(1.0).max(1.0);
    let (Ok(size), Ok(position)) = (window.inner_size(), window.outer_position()) else {
        return;
    };
    state.user_frame = OverlayFrame {
        x: position.x as f64 / scale,
        y: position.y as f64 / scale,
        width: size.width as f64 / scale,
        height: size.height as f64 / scale,
    };
}

/// Adopts only the OS window's position into `frame` (used while collapsed,
/// where the size is a fixed 280×54 and must not overwrite the remembered
/// expanded size).
fn sync_position_from_window(app: &AppHandle, frame: &mut OverlayFrame) {
    let Some(window) = app.get_webview_window("overlay") else {
        return;
    };
    let scale = window.scale_factor().unwrap_or(1.0).max(1.0);
    let Ok(position) = window.outer_position() else {
        return;
    };
    frame.x = position.x as f64 / scale;
    frame.y = position.y as f64 / scale;
}

/// Writes the user frame to preferences. Locked overlays are skipped (their
/// position is pinned by design).
fn persist_user_frame(settings: &SettingsStore, frame: &OverlayFrame) {
    if settings.preferences().overlay_locked {
        return;
    }
    let frame = *frame;
    settings.update_preferences(|prefs| {
        prefs.overlay_frame = Some(frame);
        prefs.frame_layout_version = OVERLAY_FRAME_LAYOUT_VERSION;
    });
    settings.persist();
}

/// Clamps the frame origin so the whole frame sits on the screen the overlay
/// currently occupies (used on expand, when the screen may have changed).
fn clamp_user_frame_to_screen(app: &AppHandle, frame: &mut OverlayFrame) {
    let Some(window) = app.get_webview_window("overlay") else {
        return;
    };
    let scale = window.scale_factor().unwrap_or(1.0).max(1.0);
    let (screen_w, screen_h) = current_screen_logical(app, &window, scale);
    frame.x = frame.x.clamp(0.0, (screen_w - frame.width).max(0.0));
    frame.y = frame.y.clamp(0.0, (screen_h - frame.height).max(0.0));
}

/// Animated size transition (≈180 ms easeInOut, matching the Swift overlay
/// transition), then a final settle pass.
fn animate_resize(
    window: &tauri::WebviewWindow,
    scale: f64,
    target_width: f64,
    target_height: f64,
) {
    let start = window.inner_size().unwrap_or_else(|_| {
        tauri::PhysicalSize::new(
            (target_width * scale) as u32,
            (target_height * scale) as u32,
        )
    });
    let end = tauri::PhysicalSize::new(
        (target_width * scale).round() as u32,
        (target_height * scale).round() as u32,
    );
    let window = window.clone();
    tauri::async_runtime::spawn(async move {
        const STEPS: usize = 12;
        for step in 1..=STEPS {
            tokio::time::sleep(std::time::Duration::from_millis(15)).await;
            let t = ease_in_out(step as f64 / STEPS as f64);
            let width = start.width as f64 + (end.width as f64 - start.width as f64) * t;
            let height = start.height as f64 + (end.height as f64 - start.height as f64) * t;
            let _ = window.set_size(tauri::PhysicalSize::new(
                width.round() as u32,
                height.round() as u32,
            ));
        }
        let _ = window.set_size(end);
    });
}

/// Cubic ease-in-out, matching the Swift `easeInOut(duration: 0.18)` curve.
fn ease_in_out(t: f64) -> f64 {
    if t < 0.5 {
        2.0 * t * t
    } else {
        1.0 - ((-2.0 * t + 2.0).powi(2)) / 2.0
    }
}

/// Origin for a saved frame (clamped into the visible screen) or the default
/// bottom-center placement 72 logical px above the bottom edge.
fn default_overlay_origin(
    app: &AppHandle,
    width: f64,
    height: f64,
    saved: &Option<OverlayFrame>,
) -> (f64, f64) {
    if let Some(saved) = saved {
        // Clamp the restored origin back into the visible screen so a frame
        // dragged off-screen earlier cannot hide the overlay.
        if let Some(monitor) = app.primary_monitor().ok().flatten() {
            let scale = monitor.scale_factor().max(1.0);
            let monitor_size = monitor.size();
            let screen_w = monitor_size.width as f64 / scale;
            let screen_h = monitor_size.height as f64 / scale;
            let x = saved.x.clamp(0.0, (screen_w - saved.width).max(0.0));
            let y = saved.y.clamp(0.0, (screen_h - saved.height).max(0.0));
            return (x, y);
        }
        return (saved.x, saved.y);
    }
    // Bottom-center of the primary monitor, matching the Swift default.
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

/// The language/mode picker: a separate frameless window shown under the
/// overlay's language capsule, mirroring the Swift `NSPopover`. Because the
/// menu lives in its own window, the overlay's size is never affected by
/// opening or using the menu.
pub struct LanguagePopoverManager;

/// Bumped on every show/hide decision so a pending delayed hide (scheduled
/// when the popover loses focus) never fights a newer open/toggle.
static POPOVER_GENERATION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

impl LanguagePopoverManager {
    /// Window size: the 184px menu panel plus 8px shadow margins.
    pub const WIDTH: f64 = 200.0;
    pub const HEIGHT: f64 = 266.0;
    /// Height of the language capsule the menu anchors below (for flipping).
    pub const CAPSULE_HEIGHT: f64 = 20.0;
    /// Capsule anchor offset from the overlay origin (logical px): canvas
    /// padding (6) + capsule position (left 12 / top 10) + capsule height
    /// (20) + the 6px gap between the capsule and the menu.
    pub const ANCHOR_OFFSET_X: f64 = 6.0 + 12.0;
    pub const ANCHOR_OFFSET_Y: f64 = 6.0 + 10.0 + 20.0 + 6.0;

    pub fn ensure(app: &AppHandle) {
        if app.get_webview_window("language-popover").is_some() {
            return;
        }
        let builder = WebviewWindowBuilder::new(
            app,
            "language-popover",
            WebviewUrl::App("index.html".into()),
        )
        .title("mimi")
        .inner_size(Self::WIDTH, Self::HEIGHT)
        .resizable(false)
        .transparent(true)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .shadow(false)
        .visible(false);

        match builder.build() {
            Ok(_) => pipeline_log!("language popover created"),
            Err(error) => pipeline_log!("language popover failed error={}", error),
        }
    }

    /// The menu anchor point (the overlay capsule's bottom-left corner) in
    /// screen logical coordinates, derived from the overlay window's own
    /// position — never from DOM `window.screenX`-style coordinates, which
    /// are unreliable inside the webview.
    fn overlay_anchor(app: &AppHandle) -> Option<(f64, f64)> {
        let overlay = app.get_webview_window("overlay")?;
        let scale = overlay.scale_factor().unwrap_or(1.0).max(1.0);
        let position = overlay.outer_position().ok()?;
        Some((
            position.x as f64 / scale + Self::ANCHOR_OFFSET_X,
            position.y as f64 / scale + Self::ANCHOR_OFFSET_Y,
        ))
    }

    /// Shows the popover under the given anchor point (screen logical
    /// coordinates). Flips above the anchor when the menu would run past the
    /// bottom screen edge.
    pub fn show_at(app: &AppHandle, anchor_x: f64, anchor_y: f64) {
        POPOVER_GENERATION.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let Some(window) = app.get_webview_window("language-popover") else {
            return;
        };
        let scale = window.scale_factor().unwrap_or(1.0).max(1.0);
        let screen = current_screen_logical(app, &window, scale);
        let y = language_popover_y(anchor_y, Self::HEIGHT, screen);
        let _ = window.set_position(tauri::LogicalPosition::new(anchor_x, y));
        let _ = window.show();
        let _ = window.set_focus();
    }

    /// Toggles the popover: open when closed, close when open. The anchor is
    /// computed from the overlay window's current position.
    pub fn toggle(app: &AppHandle) {
        let Some((anchor_x, anchor_y)) = Self::overlay_anchor(app) else {
            return;
        };
        let visible = app
            .get_webview_window("language-popover")
            .and_then(|window| window.is_visible().ok())
            .unwrap_or(false);
        if visible {
            Self::hide(app);
        } else {
            Self::show_at(app, anchor_x, anchor_y);
        }
    }

    /// Keeps the open menu glued to the capsule while the overlay moves.
    pub fn follow_overlay(app: &AppHandle) {
        let Some(window) = app.get_webview_window("language-popover") else {
            return;
        };
        if !window.is_visible().unwrap_or(false) {
            return;
        }
        let Some((anchor_x, anchor_y)) = Self::overlay_anchor(app) else {
            return;
        };
        let scale = window.scale_factor().unwrap_or(1.0).max(1.0);
        let screen = current_screen_logical(app, &window, scale);
        let y = language_popover_y(anchor_y, Self::HEIGHT, screen);
        let _ = window.set_position(tauri::LogicalPosition::new(anchor_x, y));
    }

    pub fn hide(app: &AppHandle) {
        POPOVER_GENERATION.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if let Some(window) = app.get_webview_window("language-popover") {
            let _ = window.hide();
        }
    }

    /// Hides the popover shortly after focus loss, unless a newer show/toggle
    /// happened in between (e.g. the capsule click that stole focus was meant
    /// to keep the menu open at the new position).
    pub fn schedule_hide(app: &AppHandle) {
        let generation = POPOVER_GENERATION.load(std::sync::atomic::Ordering::SeqCst);
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(180)).await;
            if POPOVER_GENERATION.load(std::sync::atomic::Ordering::SeqCst) != generation {
                return;
            }
            Self::hide(&app);
        });
    }
}

/// Y origin for the language popover window: below the anchor unless the
/// menu would run past the bottom screen edge, in which case it opens above
/// the anchor, clear of the capsule.
fn language_popover_y(anchor_y: f64, menu_height: f64, screen: (f64, f64)) -> f64 {
    let margin = 8.0;
    if anchor_y + menu_height > screen.1 - margin {
        (anchor_y - LanguagePopoverManager::CAPSULE_HEIGHT - menu_height).max(0.0)
    } else {
        anchor_y
    }
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

pub mod resize;

#[cfg(test)]
mod geometry_tests {
    use super::*;

    fn frame(x: f64, y: f64, w: f64, h: f64) -> OverlayFrame {
        OverlayFrame {
            x,
            y,
            width: w,
            height: h,
        }
    }

    const SCREEN: (f64, f64) = (1512.0, 982.0);

    #[test]
    fn expanded_passes_user_frame_through() {
        let geometry = geometry_for(OverlayMode::Expanded, &frame(400.0, 300.0, 640.0, 136.0));
        assert_eq!(
            (geometry.x, geometry.y, geometry.width, geometry.height),
            (400.0, 300.0, 640.0, 136.0)
        );
        assert_eq!(geometry.min, (360.0, 100.0));
        assert_eq!(geometry.max, (1200.0, 600.0));
    }

    #[test]
    fn collapsed_is_fixed_size_at_user_origin() {
        let geometry = geometry_for(OverlayMode::Collapsed, &frame(400.0, 300.0, 640.0, 136.0));
        assert_eq!(
            (geometry.x, geometry.y, geometry.width, geometry.height),
            (400.0, 300.0, 280.0, 54.0)
        );
        // min == max: the OS cannot resize the collapsed bar.
        assert_eq!(geometry.min, (280.0, 54.0));
        assert_eq!(geometry.max, (280.0, 54.0));
    }

    #[test]
    fn popover_anchor_flips_above_when_menu_would_overflow_bottom() {
        // The menu is a separate window: place it below the anchor unless it
        // would run past the bottom edge, in which case it opens above the
        // anchor, clear of the 20px-tall capsule.
        let anchor_y = 900.0;
        let y = language_popover_y(anchor_y, 260.0, SCREEN);
        assert_eq!(y, 900.0 - 20.0 - 260.0);
    }

    #[test]
    fn popover_anchor_stays_below_when_there_is_room() {
        let y = language_popover_y(100.0, 260.0, SCREEN);
        assert_eq!(y, 100.0);
    }

    #[test]
    fn popover_anchor_flips_on_small_screens_and_never_goes_negative() {
        // A 900px screen with the anchor near the bottom still flips above.
        let y = language_popover_y(800.0, 260.0, (1512.0, 900.0));
        assert_eq!(y, 800.0 - 20.0 - 260.0);
        assert!(y >= 0.0);
    }
}
