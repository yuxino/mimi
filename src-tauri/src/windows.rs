//! Overlay, child-control, settings, and tray-panel window management.
//!
//! # Overlay geometry state machine
//!
//! The overlay window has several *visual* shapes but exactly one *user*
//! frame. [`OverlayState`] owns that state:
//!
//! * `user_frame` — the user's chosen expanded size/position. The only frame
//!   that is ever persisted. Mutated exclusively by resize drags and window
//!   moves (debounced); the child control and collapse never touch it.
//! * `mode` — [`OverlayMode`]. Collapse is a temporary visual state *derived*
//!   from the user frame; the child control owns separate transient state.
//!
//! [`OverlayWindowManager::apply`] is the single place that writes OS window
//! geometry: it derives size/position/min/max from `(mode, user_frame)`. OS
//! `Moved`/`Resized` events never persist directly — they only arm a debounced
//! commit (so native drags are still remembered), and control-panel mode is
//! never persisted at all. This replaces the earlier design where resize,
//! popover enlargement, collapse animation and event persistence mutated the
//! window directly and negotiated via a global `POPOVER_RESIZING` flag, which
//! caused oscillation, stuck drags, height corruption and lost frames.

/// True when this is a dev build: a debug build (`tauri::is_dev`) or the
/// bundled dev wrapper (bundle id `app.yuxino.mimi.dev`, which is a release
/// build so macOS renders its Dock icon with the standard mask).
pub fn is_dev_build() -> bool {
    if tauri::is_dev() {
        return true;
    }
    #[cfg(target_os = "macos")]
    {
        use objc2::msg_send;
        use objc2::runtime::{AnyClass, AnyObject};
        use std::ffi::{c_char, CStr};
        unsafe {
            let Some(bundle_class) = AnyClass::get(c"NSBundle") else {
                return false;
            };
            let main_bundle: *mut AnyObject = msg_send![bundle_class, mainBundle];
            if main_bundle.is_null() {
                return false;
            }
            let identifier: *mut AnyObject = msg_send![main_bundle, bundleIdentifier];
            if identifier.is_null() {
                return false;
            }
            let ptr: *const c_char = msg_send![identifier, UTF8String];
            if ptr.is_null() {
                return false;
            }
            CStr::from_ptr(ptr).to_str() == Ok("app.yuxino.mimi.dev")
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        false
    }
}

/// Window and tooltip title for the current build: dev builds get a "(dev)"
/// marker so they can be told apart from the installed release app at a
/// glance.
pub fn dev_title(base: &str) -> String {
    if is_dev_build() {
        format!("{base} (dev)")
    } else {
        base.to_string()
    }
}

use crate::pipeline_log;
use crate::settings_store::{OverlayFrame, SettingsStore};
use crate::windows::resize::{apply_drag, ResizeRegion};
use serde::Serialize;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

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

/// The overlay's visual state. The language/mode controls live in their own
/// child window (see [`OverlayControlWindowManager`]), so the overlay has no
/// menu-related state and its height is never affected by the panel.
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
    /// Whether a debounced geometry-commit task is already in flight; avoids
    /// spawning one task per Moved/Resized event during drags (60+/s).
    geometry_task_pending: bool,
    /// Last time a resize-move was logged (throttles the per-event log).
    resize_log_at: Option<std::time::Instant>,
}

/// The lightweight control surface that floats above the subtitle canvas.
/// It reuses the former language-popover WebView instead of creating another
/// renderer: the same child/owned window is either a compact status island or
/// an expanded control panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum OverlayControlMode {
    Hidden,
    Island,
    Panel,
}

#[derive(Debug)]
struct OverlayControlStateInner {
    mode: OverlayControlMode,
    panel_height: f64,
    generation: u64,
}

/// Tauri-managed state for the overlay control child window. Keeping this
/// separate from [`OverlayState`] ensures transient panel resizing can never
/// mutate or persist the user's subtitle frame.
#[derive(Debug)]
pub struct OverlayControlState(std::sync::Mutex<OverlayControlStateInner>);

/// Cached native presentation state. Session snapshots can arrive many times
/// per second while subtitles stream; the cache keeps identical snapshots
/// from repeatedly crossing into the OS window API.
#[derive(Debug, Default)]
pub struct OverlayPresentationState(std::sync::Mutex<Option<bool>>);

impl OverlayPresentationState {
    fn apply_click_through_if_changed(&self, window: &tauri::WebviewWindow, enabled: bool) {
        let mut current = self.0.lock().unwrap();
        if *current == Some(enabled) {
            return;
        }
        if window.set_ignore_cursor_events(enabled).is_ok() {
            *current = Some(enabled);
        }
    }

    fn invalidate(&self) {
        *self.0.lock().unwrap() = None;
    }
}

impl Default for OverlayControlState {
    fn default() -> Self {
        Self(std::sync::Mutex::new(OverlayControlStateInner {
            mode: OverlayControlMode::Hidden,
            panel_height: OverlayControlWindowManager::DEFAULT_PANEL_HEIGHT,
            generation: 0,
        }))
    }
}

impl OverlayControlState {
    pub fn mode(&self) -> OverlayControlMode {
        self.0.lock().unwrap().mode
    }

    fn snapshot(&self) -> (OverlayControlMode, f64, u64) {
        let state = self.0.lock().unwrap();
        (state.mode, state.panel_height, state.generation)
    }

    fn set_mode(&self, mode: OverlayControlMode) -> bool {
        let mut state = self.0.lock().unwrap();
        if state.mode == mode {
            return false;
        }
        state.mode = mode;
        state.generation = state.generation.wrapping_add(1);
        true
    }

    fn set_panel_height(&self, height: f64) -> bool {
        let mut state = self.0.lock().unwrap();
        if (state.panel_height - height).abs() < 0.5 {
            return false;
        }
        state.panel_height = height;
        true
    }

    fn cancel_scheduled_dismiss(&self) {
        let mut state = self.0.lock().unwrap();
        state.generation = state.generation.wrapping_add(1);
    }
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
            geometry_task_pending: false,
            resize_log_at: None,
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
                .title(dev_title("mimi Subtitles"))
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
                .visible_on_all_workspaces(true)
                .skip_taskbar(true)
                .shadow(false)
                .resizable(true)
                .visible(false);

        match builder.build() {
            Ok(window) => {
                if let Some(presentation) = app.try_state::<OverlayPresentationState>() {
                    presentation.invalidate();
                }
                configure_overlay_window(&window);
                pipeline_log!("overlay window created");
            }
            Err(_) => pipeline_log!("overlay window failed label=create_failed"),
        }
    }

    /// Applies the complete native overlay presentation in one place:
    /// visibility, click-through behavior, and the child control surface.
    /// The control island is visible only for an active, expanded overlay;
    /// an already-open panel remains open across ordinary state broadcasts.
    pub fn sync_presentation(
        app: &AppHandle,
        is_active: bool,
        is_collapsed: bool,
        click_through: bool,
    ) {
        if is_active {
            if click_through {
                // Preserve the escape hatch invariant: the subtitle canvas
                // cannot become click-through until its independent control
                // island/panel is visible and interactive.
                Self::sync_overlay_visibility(app, true);
                OverlayControlWindowManager::sync_presentation(app, true, is_collapsed);
                Self::update_locked(app, true);
            } else {
                // Restore canvas interaction before changing either visible
                // surface, so unlocking never leaves an inert frame behind.
                Self::update_locked(app, false);
                Self::sync_overlay_visibility(app, true);
                OverlayControlWindowManager::sync_presentation(app, true, is_collapsed);
            }
        } else {
            // Hide the child first so it cannot linger for a frame after its
            // subtitle parent disappears.
            OverlayControlWindowManager::sync_presentation(app, false, is_collapsed);
            Self::sync_overlay_visibility(app, false);
            Self::update_locked(app, click_through);
        }
    }

    fn sync_overlay_visibility(app: &AppHandle, is_active: bool) {
        let Some(window) = app.get_webview_window("overlay") else {
            return;
        };
        let visible = window.is_visible().unwrap_or(false);
        // Only act when the visibility actually changes: `show()` on an
        // already-visible window re-activates it and steals focus from the
        // child control on every subtitle update.
        if is_active && !visible {
            configure_overlay_window(&window);
            let _ = window.show();
        } else if !is_active && visible {
            let _ = window.hide();
        }
    }

    /// Locks the overlay: the window ignores mouse events so clicks pass
    /// through to whatever plays underneath.
    pub fn update_locked(app: &AppHandle, locked: bool) {
        let Some(window) = app.get_webview_window("overlay") else {
            return;
        };
        if let Some(presentation) = app.try_state::<OverlayPresentationState>() {
            presentation.apply_click_through_if_changed(&window, locked);
        } else {
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
    /// change uses the shared 180 ms ease-in-out timing, and expansion clamps
    /// the remembered frame back onto the visible screen.
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
            // One debounced commit task is enough: the in-flight task tails
            // newer timestamps until events settle, so no commit is lost and
            // no task is spawned per event.
            if state.geometry_task_pending {
                return;
            }
            state.geometry_task_pending = true;
        }
        let app = app.clone();
        let state = Arc::clone(state);
        let settings = Arc::clone(settings);
        tauri::async_runtime::spawn(async move {
            let mut expected = now;
            loop {
                tokio::time::sleep(std::time::Duration::from_millis(350)).await;
                let mut state = state.lock().unwrap();
                if state.last_geometry_event != Some(expected) {
                    // Newer events arrived while we slept; tail them instead
                    // of dropping their commit (the single-flight flag made
                    // them skip spawning their own task).
                    expected = state.last_geometry_event.expect("checked as Some");
                    continue;
                }
                state.geometry_task_pending = false;
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
                return;
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
        let work_area = current_work_area_logical(app, &window);
        let mut local_start_frame = start_frame;
        local_start_frame.x -= work_area.x;
        local_start_frame.y -= work_area.y;
        let mut frame = apply_drag(
            region,
            (start_x - work_area.x, start_y - work_area.y),
            &local_start_frame,
            (x - work_area.x, y - work_area.y),
            (
                SubtitleOverlayMetrics::MINIMUM_WIDTH,
                SubtitleOverlayMetrics::MINIMUM_HEIGHT,
            ),
            (
                SubtitleOverlayMetrics::MAXIMUM_WIDTH,
                SubtitleOverlayMetrics::MAXIMUM_HEIGHT,
            ),
            (work_area.width, work_area.height),
            48.0,
        );
        frame.x += work_area.x;
        frame.y += work_area.y;
        // Skip sub-pixel jitter: unchanged frames do not need window ops.
        if (frame.x - state.user_frame.x).abs() < 0.5
            && (frame.y - state.user_frame.y).abs() < 0.5
            && (frame.width - state.user_frame.width).abs() < 0.5
            && (frame.height - state.user_frame.height).abs() < 0.5
        {
            return;
        }
        state.user_frame = frame;
        // Only position and size change during a drag; skip the full apply()
        // (which also resets min/max) to halve the window ops per event.
        let _ = window.set_position(tauri::LogicalPosition::new(frame.x, frame.y));
        let _ = window.set_size(tauri::LogicalSize::new(frame.width, frame.height));
        let log_now = std::time::Instant::now();
        let due = state
            .resize_log_at
            .is_none_or(|at| log_now.duration_since(at).as_millis() >= 250);
        if due {
            state.resize_log_at = Some(log_now);
            tracing::info!(
                "resize move frame={:?}",
                (frame.x, frame.y, frame.width, frame.height)
            );
        }
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
}

/// Reasserts presentation-level window behavior whenever the overlay is
/// created or explicitly shown. macOS needs `FullScreenAuxiliary` in addition
/// to Tauri's all-workspaces flag to accompany another app's full-screen
/// window instead of remaining on the previous Space.
fn configure_overlay_window(window: &tauri::WebviewWindow) {
    let _ = window.set_always_on_top(true);
    let _ = window.set_visible_on_all_workspaces(true);

    #[cfg(target_os = "macos")]
    unsafe {
        use objc2_app_kit::{NSWindow, NSWindowCollectionBehavior};

        let Ok(pointer) = window.ns_window() else {
            return;
        };
        let ns_window: &NSWindow = &*pointer.cast();
        let behavior = ns_window.collectionBehavior()
            | NSWindowCollectionBehavior::CanJoinAllSpaces
            | NSWindowCollectionBehavior::FullScreenAuxiliary;
        ns_window.setCollectionBehavior(behavior);
    }
}

// ---------------------------------------------------------------------------
// Geometry helpers
// ---------------------------------------------------------------------------

/// Logical work area of the monitor the overlay currently sits on. The
/// desktop-space origin is preserved, including negative coordinates for
/// monitors positioned to the left of or above the primary display.
fn current_work_area_logical(app: &AppHandle, window: &tauri::WebviewWindow) -> LogicalWorkArea {
    window
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| app.primary_monitor().ok().flatten())
        .map(|monitor| logical_work_area(&monitor))
        .unwrap_or(LogicalWorkArea {
            x: 0.0,
            y: 0.0,
            width: 1440.0,
            height: 900.0,
        })
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
    if settings
        .save_preferences(|prefs| {
            prefs.overlay_frame = Some(frame);
            prefs.frame_layout_version = OVERLAY_FRAME_LAYOUT_VERSION;
        })
        .is_err()
    {
        tracing::warn!("preferences unavailable label=overlay_frame_write_failed");
    }
}

/// Clamps the frame origin so the whole frame sits on the screen the overlay
/// currently occupies (used on expand, when the screen may have changed).
fn clamp_user_frame_to_screen(app: &AppHandle, frame: &mut OverlayFrame) {
    let Some(window) = app.get_webview_window("overlay") else {
        return;
    };
    let work_area = current_work_area_logical(app, &window);
    clamp_frame_origin_to_work_area(frame, work_area);
}

/// Animated 180 ms ease-in-out size transition, followed by a settle pass.
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

/// Cubic ease-in-out used by the native geometry transition.
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
        // Prefer the work area that contains most of the saved frame. This
        // preserves deliberate placement on a left/upper secondary monitor;
        // if that display disappeared, fall back to the primary work area.
        if let Some(work_area) = work_area_for_saved_frame(app, saved) {
            let mut restored = *saved;
            clamp_frame_origin_to_work_area(&mut restored, work_area);
            return (restored.x, restored.y);
        }
        return (saved.x, saved.y);
    }
    // Default to the bottom-center of the primary monitor.
    if let Some(monitor) = app.primary_monitor().ok().flatten() {
        let work_area = logical_work_area(&monitor);
        return default_overlay_origin_in_work_area(width, height, work_area);
    }
    (0.0, 0.0)
}

/// Logical work area, including its desktop-space origin. Unlike the former
/// popover placement helpers, this remains correct for monitors positioned to
/// the left of or above the primary display.
#[derive(Debug, Clone, Copy, PartialEq)]
struct LogicalWorkArea {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

fn logical_work_area(monitor: &tauri::Monitor) -> LogicalWorkArea {
    let scale = monitor.scale_factor().max(1.0);
    let area = monitor.work_area();
    LogicalWorkArea {
        x: area.position.x as f64 / scale,
        y: area.position.y as f64 / scale,
        width: area.size.width as f64 / scale,
        height: area.size.height as f64 / scale,
    }
}

fn work_area_for_saved_frame(app: &AppHandle, frame: &OverlayFrame) -> Option<LogicalWorkArea> {
    let primary = app
        .primary_monitor()
        .ok()
        .flatten()
        .map(|monitor| logical_work_area(&monitor));
    let areas = app
        .available_monitors()
        .unwrap_or_default()
        .into_iter()
        .map(|monitor| logical_work_area(&monitor));
    choose_work_area_for_frame(frame, areas, primary)
}

fn choose_work_area_for_frame(
    frame: &OverlayFrame,
    areas: impl IntoIterator<Item = LogicalWorkArea>,
    fallback: Option<LogicalWorkArea>,
) -> Option<LogicalWorkArea> {
    areas
        .into_iter()
        .map(|area| (frame_intersection_area(frame, area), area))
        .filter(|(intersection, _)| *intersection > 0.0)
        .max_by(|left, right| left.0.total_cmp(&right.0))
        .map(|(_, area)| area)
        .or(fallback)
}

fn frame_intersection_area(frame: &OverlayFrame, area: LogicalWorkArea) -> f64 {
    let width = (frame.x + frame.width).min(area.x + area.width) - frame.x.max(area.x);
    let height = (frame.y + frame.height).min(area.y + area.height) - frame.y.max(area.y);
    width.max(0.0) * height.max(0.0)
}

fn clamp_frame_origin_to_work_area(frame: &mut OverlayFrame, area: LogicalWorkArea) {
    let max_x = (area.x + area.width - frame.width).max(area.x);
    let max_y = (area.y + area.height - frame.height).max(area.y);
    frame.x = frame.x.clamp(area.x, max_x);
    frame.y = frame.y.clamp(area.y, max_y);
}

fn default_overlay_origin_in_work_area(
    width: f64,
    height: f64,
    area: LogicalWorkArea,
) -> (f64, f64) {
    let mut frame = OverlayFrame {
        x: area.x + (area.width - width) / 2.0,
        y: area.y + area.height - height - 72.0,
        width,
        height,
    };
    clamp_frame_origin_to_work_area(&mut frame, area);
    (frame.x, frame.y)
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct OverlayControlGeometry {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

const OVERLAY_CONTROL_MODE_EVENT: &str = "overlay-control-mode";

/// Child/owned control surface for the subtitle canvas. The native bounds
/// always match the visible surface, so no transparent shadow margin steals
/// clicks from the video underneath.
pub struct OverlayControlWindowManager;

impl OverlayControlWindowManager {
    pub const ISLAND_WIDTH: f64 = 236.0;
    pub const ISLAND_HEIGHT: f64 = 30.0;
    pub const PANEL_WIDTH: f64 = 276.0;
    // Matches the first-open height of the full Alibaba control set; React
    // immediately replaces it with the measured provider/locale-specific
    // height, but this default avoids a visible clipped first frame.
    pub const DEFAULT_PANEL_HEIGHT: f64 = 374.0;
    const MIN_PANEL_HEIGHT: f64 = 132.0;
    const MAX_PANEL_HEIGHT: f64 = 520.0;
    const ANCHOR_OFFSET_X: f64 = 18.0;
    const ANCHOR_OFFSET_Y: f64 = 16.0;
    const WORK_AREA_MARGIN: f64 = 8.0;

    pub fn ensure(app: &AppHandle) {
        if app.get_webview_window("overlay-control").is_some() {
            return;
        }
        let Some(overlay) = app.get_webview_window("overlay") else {
            pipeline_log!("overlay control failed label=missing_parent");
            return;
        };
        let builder =
            WebviewWindowBuilder::new(app, "overlay-control", WebviewUrl::App("index.html".into()))
                .title(dev_title("mimi"))
                .inner_size(Self::ISLAND_WIDTH, Self::ISLAND_HEIGHT)
                .resizable(false)
                .transparent(true)
                .decorations(false)
                .always_on_top(true)
                .visible_on_all_workspaces(true)
                .skip_taskbar(true)
                .shadow(false)
                .focused(false)
                .visible(false);
        let builder = match builder.parent(&overlay) {
            Ok(builder) => builder,
            Err(_) => {
                pipeline_log!("overlay control failed label=parent_unavailable");
                return;
            }
        };

        match builder.build() {
            Ok(window) => {
                configure_overlay_window(&window);
                pipeline_log!("overlay control created");
            }
            Err(_) => pipeline_log!("overlay control failed label=create_failed"),
        }
    }

    pub fn mode(app: &AppHandle) -> OverlayControlMode {
        app.try_state::<OverlayControlState>()
            .map(|state| state.mode())
            .unwrap_or(OverlayControlMode::Hidden)
    }

    /// Derives the persistent island visibility from the overlay lifecycle.
    /// An open panel survives ordinary subtitle/status broadcasts, but any
    /// inactive or collapsed transition hides the entire child surface.
    pub fn sync_presentation(app: &AppHandle, is_active: bool, is_collapsed: bool) {
        let Some(state) = app.try_state::<OverlayControlState>() else {
            return;
        };
        let current = state.mode();
        let next = control_mode_for_presentation(current, is_active, is_collapsed);
        let mode_changed = state.set_mode(next);
        let visible_surface_needs_restore = !mode_changed
            && next != OverlayControlMode::Hidden
            && app
                .get_webview_window("overlay-control")
                .and_then(|window| window.is_visible().ok())
                != Some(true);
        if mode_changed || visible_surface_needs_restore {
            Self::apply_mode(app, next, false);
        }
    }

    pub fn toggle_panel(app: &AppHandle) {
        let Some(state) = app.try_state::<OverlayControlState>() else {
            return;
        };
        let next = match state.mode() {
            OverlayControlMode::Island => OverlayControlMode::Panel,
            OverlayControlMode::Panel => OverlayControlMode::Island,
            OverlayControlMode::Hidden => return,
        };
        if state.set_mode(next) {
            Self::apply_mode(app, next, next == OverlayControlMode::Panel);
        }
    }

    /// Returns an expanded panel to its compact island. Hidden surfaces stay
    /// hidden, so a delayed focus-loss task can never resurrect an inactive
    /// session.
    pub fn dismiss_panel(app: &AppHandle) {
        let Some(state) = app.try_state::<OverlayControlState>() else {
            return;
        };
        if state.mode() == OverlayControlMode::Panel && state.set_mode(OverlayControlMode::Island) {
            Self::apply_mode(app, OverlayControlMode::Island, false);
        }
    }

    /// Delayed focus-loss handling avoids a hide/show race when a click inside
    /// the control WebView changes focus during the island/panel resize.
    pub fn schedule_dismiss(app: &AppHandle) {
        let Some(state) = app.try_state::<OverlayControlState>() else {
            return;
        };
        let (_, _, generation) = state.snapshot();
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(180)).await;
            let Some(state) = app.try_state::<OverlayControlState>() else {
                return;
            };
            let (mode, _, current_generation) = state.snapshot();
            if current_generation != generation || mode != OverlayControlMode::Panel {
                return;
            }
            if state.set_mode(OverlayControlMode::Island) {
                Self::apply_mode(&app, OverlayControlMode::Island, false);
            }
        });
    }

    /// Cancels a delayed focus-loss dismissal when focus returns during the
    /// island-to-panel native resize.
    pub fn cancel_scheduled_dismiss(app: &AppHandle) {
        let Some(state) = app.try_state::<OverlayControlState>() else {
            return;
        };
        if state.mode() == OverlayControlMode::Panel {
            state.cancel_scheduled_dismiss();
        }
    }

    /// Updates the tightly-fitted panel height measured by React. The value is
    /// clamped before any native window mutation.
    pub fn set_panel_height(app: &AppHandle, height: f64) {
        if !height.is_finite() {
            return;
        }
        let height = height.clamp(Self::MIN_PANEL_HEIGHT, Self::MAX_PANEL_HEIGHT);
        let Some(state) = app.try_state::<OverlayControlState>() else {
            return;
        };
        if state.set_panel_height(height) && state.mode() == OverlayControlMode::Panel {
            Self::apply_geometry(app, OverlayControlMode::Panel);
        }
    }

    /// Keeps the child surface attached to the overlay on platforms where an
    /// owned window does not automatically move with its owner (notably
    /// Windows), and re-clamps after monitor or scale changes.
    pub fn follow_overlay(app: &AppHandle) {
        let mode = Self::mode(app);
        if mode != OverlayControlMode::Hidden {
            Self::apply_geometry(app, mode);
        }
    }

    fn apply_mode(app: &AppHandle, mode: OverlayControlMode, focus: bool) {
        let Some(window) = app.get_webview_window("overlay-control") else {
            return;
        };
        if mode == OverlayControlMode::Hidden {
            let _ = window.hide();
            let _ = window.emit(OVERLAY_CONTROL_MODE_EVENT, mode);
            return;
        }

        let _ = window.emit(OVERLAY_CONTROL_MODE_EVENT, mode);
        Self::apply_geometry(app, mode);
        if !window.is_visible().unwrap_or(false) {
            configure_overlay_window(&window);
            let _ = window.show();
        }
        if focus {
            let _ = window.set_focus();
        }
    }

    fn apply_geometry(app: &AppHandle, mode: OverlayControlMode) {
        let Some(window) = app.get_webview_window("overlay-control") else {
            return;
        };
        let Some((anchor_x, anchor_y, work_area)) = Self::overlay_anchor(app) else {
            return;
        };
        let panel_height = app
            .try_state::<OverlayControlState>()
            .map(|state| state.snapshot().1)
            .unwrap_or(Self::DEFAULT_PANEL_HEIGHT);
        let geometry = overlay_control_geometry(mode, anchor_x, anchor_y, panel_height, work_area);
        let _ = window.set_size(tauri::LogicalSize::new(geometry.width, geometry.height));
        let _ = window.set_position(tauri::LogicalPosition::new(geometry.x, geometry.y));
    }

    fn overlay_anchor(app: &AppHandle) -> Option<(f64, f64, LogicalWorkArea)> {
        let overlay = app.get_webview_window("overlay")?;
        let scale = overlay.scale_factor().unwrap_or(1.0).max(1.0);
        let position = overlay.outer_position().ok()?;
        let monitor = overlay
            .current_monitor()
            .ok()
            .flatten()
            .or_else(|| app.primary_monitor().ok().flatten())?;
        let work_area = logical_work_area(&monitor);
        Some((
            position.x as f64 / scale + Self::ANCHOR_OFFSET_X,
            position.y as f64 / scale + Self::ANCHOR_OFFSET_Y,
            work_area,
        ))
    }
}

fn control_mode_for_presentation(
    current: OverlayControlMode,
    is_active: bool,
    is_collapsed: bool,
) -> OverlayControlMode {
    if !is_active || is_collapsed {
        OverlayControlMode::Hidden
    } else if current == OverlayControlMode::Panel {
        OverlayControlMode::Panel
    } else {
        OverlayControlMode::Island
    }
}

fn overlay_control_geometry(
    mode: OverlayControlMode,
    anchor_x: f64,
    anchor_y: f64,
    panel_height: f64,
    work_area: LogicalWorkArea,
) -> OverlayControlGeometry {
    let margin = OverlayControlWindowManager::WORK_AREA_MARGIN;
    let (width, requested_height) = match mode {
        OverlayControlMode::Panel => (OverlayControlWindowManager::PANEL_WIDTH, panel_height),
        OverlayControlMode::Hidden | OverlayControlMode::Island => (
            OverlayControlWindowManager::ISLAND_WIDTH,
            OverlayControlWindowManager::ISLAND_HEIGHT,
        ),
    };
    let available_height = (work_area.height - margin * 2.0).max(1.0);
    let height = requested_height.min(available_height);
    let min_x = work_area.x + margin;
    let max_x = (work_area.x + work_area.width - width - margin).max(min_x);
    let x = anchor_x.clamp(min_x, max_x);
    let min_y = work_area.y + margin;
    let max_y = (work_area.y + work_area.height - height - margin).max(min_y);
    let preferred_y = if mode == OverlayControlMode::Panel && anchor_y > max_y {
        // Grow upward while keeping the panel's bottom aligned with the
        // compact island's bottom.
        anchor_y + OverlayControlWindowManager::ISLAND_HEIGHT - height
    } else {
        anchor_y
    };
    let y = preferred_y.clamp(min_y, max_y);
    OverlayControlGeometry {
        x,
        y,
        width,
        height,
    }
}

/// Tray popup panel: a compact frameless always-on-top control surface shown
/// under the tray icon on both desktop platforms.
pub struct TrayPanelManager;

impl TrayPanelManager {
    pub fn ensure(app: &AppHandle) {
        if app.get_webview_window("tray-panel").is_some() {
            return;
        }
        let builder =
            WebviewWindowBuilder::new(app, "tray-panel", WebviewUrl::App("index.html".into()))
                .title(dev_title("mimi"))
                .inner_size(320.0, 410.0)
                .resizable(false)
                .transparent(true)
                .decorations(false)
                .always_on_top(true)
                .skip_taskbar(true)
                .shadow(false)
                .visible(false);

        match builder.build() {
            Ok(_) => pipeline_log!("tray panel created"),
            Err(_) => pipeline_log!("tray panel failed label=create_failed"),
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

        // Position the panel below the tray icon. The positioner plugin
        // records the tray icon's rect from every tray event (see the
        // on_tray_event call in lib.rs); if it is not available yet, fall
        // back to the top-right of the current monitor so the panel never
        // pops up at the window's default location.
        if window.move_window(Position::TrayBottomCenter).is_err() {
            tracing::debug!("tray panel position unavailable label=initial_position_failed");
            Self::position_top_right(&window);
        }
        if let Ok(position) = window.outer_position() {
            tracing::debug!("tray panel: positioned at {position:?}");
        }
        let _ = window.show();
        let _ = window.set_focus();

        // The first move can run before the window reports its final size,
        // which skews TrayBottomCenter's horizontal centering. Re-apply the
        // position shortly after the panel is visible.
        let window = window.clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(60)).await;
            if window.move_window(Position::TrayBottomCenter).is_err() {
                tracing::debug!("tray panel position unavailable label=settle_position_failed");
                Self::position_top_right(&window);
            }
            if let Ok(position) = window.outer_position() {
                tracing::debug!("tray panel: re-positioned at {position:?}");
            }
        });
    }

    /// Fallback placement: top-right of the window's current monitor, just
    /// below the menu bar, so the panel stays visible and near its anchor.
    fn position_top_right(window: &tauri::WebviewWindow) {
        let Some(monitor) = window.current_monitor().ok().flatten() else {
            return;
        };
        let monitor_pos = monitor.position();
        let monitor_size = monitor.size();
        let window_size = window.outer_size().unwrap_or_default();
        let margin = 8.0_f64;
        let x =
            (monitor_pos.x as f64 + monitor_size.width as f64 - window_size.width as f64 - margin)
                .max(monitor_pos.x as f64);
        let y = monitor_pos.y as f64 + margin;
        let _ = window.set_position(tauri::PhysicalPosition::new(x as i32, y as i32));
    }

    pub fn hide(app: &AppHandle) {
        if let Some(window) = app.get_webview_window("tray-panel") {
            let _ = window.hide();
        }
    }
}

/// Shows the settings window, creating it first if it does not exist (the
/// window is hidden on close — see the CloseRequested handler in lib.rs — but
/// a stale or crashed webview could leave it missing). Mirrors the window
/// declared in tauri.conf.json.
pub fn ensure_settings_window(app: &AppHandle) {
    let Some(window) = app.get_webview_window("settings") else {
        let builder =
            WebviewWindowBuilder::new(app, "settings", WebviewUrl::App("index.html".into()))
                .title(dev_title("mimi"))
                .inner_size(760.0, 720.0)
                .min_inner_size(520.0, 560.0)
                .resizable(true)
                .center()
                .visible(true);
        if builder.build().is_err() {
            pipeline_log!("settings window re-create failed label=create_failed");
            return;
        }
        let Some(window) = app.get_webview_window("settings") else {
            return;
        };
        let _ = window.show();
        let _ = window.set_focus();
        return;
    };
    let _ = window.show();
    let _ = window.set_focus();
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

    const WORK_AREA: LogicalWorkArea = LogicalWorkArea {
        x: 0.0,
        y: 24.0,
        width: 1512.0,
        height: 958.0,
    };

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
    fn active_expanded_overlay_keeps_an_open_panel() {
        assert_eq!(
            control_mode_for_presentation(OverlayControlMode::Panel, true, false),
            OverlayControlMode::Panel
        );
        assert_eq!(
            control_mode_for_presentation(OverlayControlMode::Hidden, true, false),
            OverlayControlMode::Island
        );
    }

    #[test]
    fn inactive_or_collapsed_overlay_hides_control_surface() {
        assert_eq!(
            control_mode_for_presentation(OverlayControlMode::Panel, false, false),
            OverlayControlMode::Hidden
        );
        assert_eq!(
            control_mode_for_presentation(OverlayControlMode::Panel, true, true),
            OverlayControlMode::Hidden
        );
    }

    #[test]
    fn saved_frame_prefers_the_secondary_work_area_it_occupies() {
        let primary = LogicalWorkArea {
            x: 0.0,
            y: 24.0,
            width: 1512.0,
            height: 958.0,
        };
        let left = LogicalWorkArea {
            x: -1728.0,
            y: -120.0,
            width: 1728.0,
            height: 1080.0,
        };
        let saved = frame(-1400.0, 680.0, 640.0, 136.0);

        assert_eq!(
            choose_work_area_for_frame(&saved, [primary, left], Some(primary)),
            Some(left)
        );
    }

    #[test]
    fn frame_clamp_honors_negative_work_area_origin() {
        let left = LogicalWorkArea {
            x: -1728.0,
            y: -120.0,
            width: 1728.0,
            height: 1080.0,
        };
        let mut saved = frame(-2200.0, -300.0, 640.0, 136.0);

        clamp_frame_origin_to_work_area(&mut saved, left);

        assert_eq!(saved.x, -1728.0);
        assert_eq!(saved.y, -120.0);
    }

    #[test]
    fn missing_saved_monitor_falls_back_to_primary_work_area() {
        let primary = LogicalWorkArea {
            x: 0.0,
            y: 24.0,
            width: 1512.0,
            height: 958.0,
        };
        let saved = frame(-2200.0, 700.0, 640.0, 136.0);

        assert_eq!(
            choose_work_area_for_frame(&saved, [primary], Some(primary)),
            Some(primary)
        );
    }

    #[test]
    fn default_origin_uses_work_area_origin_and_bottom_inset() {
        let upper = LogicalWorkArea {
            x: -1280.0,
            y: -1024.0,
            width: 1280.0,
            height: 984.0,
        };

        assert_eq!(
            default_overlay_origin_in_work_area(640.0, 136.0, upper),
            (-960.0, -248.0)
        );
    }

    #[test]
    fn panel_grows_upward_when_it_would_overflow_bottom() {
        let geometry =
            overlay_control_geometry(OverlayControlMode::Panel, 400.0, 900.0, 356.0, WORK_AREA);
        assert_eq!(geometry.y, 900.0 + 30.0 - 356.0);
        assert_eq!(geometry.width, 276.0);
        assert_eq!(geometry.height, 356.0);
    }

    #[test]
    fn panel_keeps_its_anchor_when_there_is_room_below() {
        let geometry =
            overlay_control_geometry(OverlayControlMode::Panel, 400.0, 300.0, 356.0, WORK_AREA);
        assert_eq!(geometry.y, 300.0);
    }

    #[test]
    fn control_geometry_honors_negative_monitor_origins() {
        let work_area = LogicalWorkArea {
            x: -1728.0,
            y: -120.0,
            width: 1728.0,
            height: 1080.0,
        };
        let geometry = overlay_control_geometry(
            OverlayControlMode::Island,
            -1800.0,
            -200.0,
            356.0,
            work_area,
        );
        assert_eq!(geometry.x, -1720.0);
        assert_eq!(geometry.y, -112.0);
    }

    #[test]
    fn panel_clamps_inside_right_edge_and_short_work_area() {
        let work_area = LogicalWorkArea {
            x: 100.0,
            y: 50.0,
            width: 320.0,
            height: 180.0,
        };
        let geometry =
            overlay_control_geometry(OverlayControlMode::Panel, 400.0, 160.0, 520.0, work_area);
        assert_eq!(geometry.x, 136.0);
        assert_eq!(geometry.y, 58.0);
        assert_eq!(geometry.height, 164.0);
    }
}
