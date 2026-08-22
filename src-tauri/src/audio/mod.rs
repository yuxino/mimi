//! Platform audio capture (system audio only, never microphone) and the
//! bounded PCM send pipeline.

pub mod send_pipeline;

#[cfg(any(target_os = "windows", test))]
mod streaming_resampler;

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
    #[cfg(target_os = "macos")]
    #[error("mimi could not find a display to use for system audio capture.")]
    NoDisplay,
    #[error("The system returned an unsupported audio format.")]
    UnsupportedAudioFormat,
    #[cfg(target_os = "windows")]
    #[error("No default playback device is available for system audio capture.")]
    NoPlaybackDevice,
    #[error("{0}")]
    Other(String),
}

/// Provider-requested wire format. Both supported backends capture system
/// audio only, mix to mono, resample to this rate, and encode little-endian
/// PCM16 before emitting buffers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AudioCaptureFormat {
    pub sample_rate_hz: u32,
}

impl AudioCaptureFormat {
    pub fn pcm16_mono(sample_rate_hz: u32) -> Result<Self, SystemAudioCaptureError> {
        if !matches!(sample_rate_hz, 16_000 | 24_000) {
            return Err(SystemAudioCaptureError::UnsupportedAudioFormat);
        }
        Ok(Self { sample_rate_hz })
    }
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
            // The AppHandle is macOS-only (ScreenCaptureKit main-thread
            // dispatch); Windows WASAPI needs no app handle.
            let _ = app;
            windows::WindowsSystemAudioCapture::new()
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            let _ = app;
            unsupported::UnsupportedSystemAudioCapture::new()
        }
    }
}

#[cfg(test)]
mod format_tests {
    use super::*;

    #[test]
    fn provider_sample_rates_are_supported() {
        assert_eq!(
            AudioCaptureFormat::pcm16_mono(16_000)
                .unwrap()
                .sample_rate_hz,
            16_000
        );
        assert_eq!(
            AudioCaptureFormat::pcm16_mono(24_000)
                .unwrap()
                .sample_rate_hz,
            24_000
        );
        assert!(AudioCaptureFormat::pcm16_mono(48_000).is_err());
    }
}
