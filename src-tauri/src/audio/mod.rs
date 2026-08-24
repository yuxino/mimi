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

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;

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
    #[cfg(target_os = "macos")]
    #[error("System audio capture permission was denied.")]
    PermissionDenied,
    #[cfg(target_os = "macos")]
    #[error("System audio capture setup timed out.")]
    StartTimedOut,
    #[cfg(target_os = "macos")]
    #[error("System audio capture start was cancelled.")]
    StartCancelled,
    #[cfg(target_os = "macos")]
    #[error("The previous system audio capture is still stopping.")]
    PreviousCaptureStopping,
    #[error("System audio capture could not be started.")]
    NativeStartFailed,
    #[error("System audio capture could not process the device audio format.")]
    AudioProcessingFailed,
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    #[error("System audio capture is not supported on this platform.")]
    UnsupportedPlatform,
}

/// Fatal failures reported after a native capture session has started.
///
/// The variants deliberately contain no platform or provider free text so the
/// same value is safe to use for both recovery decisions and diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SystemAudioCaptureFailure {
    #[error("System audio capture stopped unexpectedly.")]
    NativeStopped,
    #[error("System audio capture could not process the device audio format.")]
    AudioProcessingFailed,
    #[error("Audio streaming fell behind. mimi is reconnecting.")]
    Backpressure,
}

impl SystemAudioCaptureFailure {
    pub fn diagnostic_label(self) -> &'static str {
        match self {
            Self::NativeStopped => "capture.native_stopped",
            Self::AudioProcessingFailed => "capture.audio_processing_failed",
            Self::Backpressure => "capture.backpressure",
        }
    }
}

/// Cloneable, non-blocking, exactly-once failure reporter for native audio
/// callbacks. The bounded channel holds one fatal failure because a capture
/// generation is torn down after the first one.
#[derive(Clone)]
pub struct CaptureFailureSender {
    tx: mpsc::Sender<SystemAudioCaptureFailure>,
    reported: Arc<AtomicBool>,
}

impl CaptureFailureSender {
    pub fn channel() -> (Self, mpsc::Receiver<SystemAudioCaptureFailure>) {
        let (tx, rx) = mpsc::channel(1);
        (
            Self {
                tx,
                reported: Arc::new(AtomicBool::new(false)),
            },
            rx,
        )
    }

    /// Reports the first fatal failure without ever blocking the native audio
    /// callback. Returns true only for the caller that claimed the report.
    pub fn report(&self, failure: SystemAudioCaptureFailure) -> bool {
        if self
            .reported
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return false;
        }
        let _ = self.tx.try_send(failure);
        true
    }

    pub fn has_reported(&self) -> bool {
        self.reported.load(Ordering::SeqCst)
    }
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
                    if app.run_on_main_thread(task).is_err() {
                        tracing::error!("main-thread dispatch failed label=tauri_dispatch_failed");
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

    #[tokio::test]
    async fn capture_failure_sender_is_bounded_and_reports_once() {
        let (sender, mut receiver) = CaptureFailureSender::channel();

        assert!(sender.report(SystemAudioCaptureFailure::NativeStopped));
        assert!(!sender.report(SystemAudioCaptureFailure::Backpressure));
        assert!(sender.has_reported());
        assert_eq!(
            receiver.recv().await,
            Some(SystemAudioCaptureFailure::NativeStopped)
        );
        assert!(receiver.try_recv().is_err());
    }
}
