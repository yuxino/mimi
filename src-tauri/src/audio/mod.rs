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
