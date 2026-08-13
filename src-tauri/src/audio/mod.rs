//! Platform audio capture (system audio only, never microphone) and the
//! bounded audio send pipeline. Ported from `SystemAudioCapture.swift` and
//! `AppModel.AudioSendPipeline`.

pub mod send_pipeline;

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub mod unsupported;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SystemAudioCaptureError {
    #[error("System audio capture is already running.")]
    AlreadyRunning,
    #[error("mimi could not find a display to use for system audio capture.")]
    NoDisplay,
    #[error("The system returned an unsupported audio format.")]
    UnsupportedAudioFormat,
    #[error("No default playback device is available for system audio capture.")]
    NoPlaybackDevice,
    #[error("{0}")]
    Other(String),
}

/// Cross-platform handle for an active capture session.
#[cfg(target_os = "macos")]
pub type SystemAudioCapture = macos::MacSystemAudioCapture;

#[cfg(target_os = "windows")]
pub type SystemAudioCapture = windows::WindowsSystemAudioCapture;

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub type SystemAudioCapture = unsupported::UnsupportedSystemAudioCapture;

impl SystemAudioCapture {
    /// Creates the platform capture handle for this app. On macOS the
    /// ScreenCaptureKit objects are confined to the main thread through the
    /// app handle's run-on-main-thread executor.
    pub fn for_app(app: &tauri::AppHandle) -> Self {
        #[cfg(target_os = "macos")]
        {
            let app = app.clone();
            macos::MacSystemAudioCapture::new(std::sync::Arc::new(
                move |task: Box<dyn FnOnce() + Send>| {
                    // A dropped dispatch silently leaves the capture running;
                    // surface the failure instead of ignoring it.
                    if let Err(error) = app.run_on_main_thread(task) {
                        tracing::error!("main-thread dispatch failed error={error}");
                    }
                },
            ))
        }
        #[cfg(target_os = "windows")]
        {
            windows::WindowsSystemAudioCapture::new()
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            unsupported::UnsupportedSystemAudioCapture::new()
        }
    }
}
