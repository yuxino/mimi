//! Placeholder capture backend for unsupported platforms.

use crate::audio::send_pipeline::AudioIngress;
use crate::audio::{AudioCaptureFormat, CaptureFailureSender, SystemAudioCaptureError};

#[derive(Clone)]
pub struct UnsupportedSystemAudioCapture;

impl UnsupportedSystemAudioCapture {
    pub fn new() -> Self {
        Self
    }

    pub async fn start(
        &self,
        _audio_ingress: AudioIngress,
        _failure_tx: CaptureFailureSender,
        _format: AudioCaptureFormat,
    ) -> Result<(), SystemAudioCaptureError> {
        Err(SystemAudioCaptureError::UnsupportedPlatform)
    }

    pub async fn stop(&self) {}
}

impl Default for UnsupportedSystemAudioCapture {
    fn default() -> Self {
        Self::new()
    }
}
