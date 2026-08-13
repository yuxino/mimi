//! Placeholder capture backend for unsupported platforms.

use crate::audio::SystemAudioCaptureError;
use tokio::sync::mpsc;

#[derive(Clone)]
pub struct UnsupportedSystemAudioCapture;

impl UnsupportedSystemAudioCapture {
    pub fn new() -> Self {
        Self
    }

    pub async fn start(
        &self,
        _audio_tx: mpsc::UnboundedSender<Vec<u8>>,
        _error_tx: mpsc::UnboundedSender<String>,
    ) -> Result<(), SystemAudioCaptureError> {
        Err(SystemAudioCaptureError::Other(
            "System audio capture is not supported on this platform.".into(),
        ))
    }

    pub async fn stop(&self) {}
}

impl Default for UnsupportedSystemAudioCapture {
    fn default() -> Self {
        Self::new()
    }
}
