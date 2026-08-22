//! Windows system-audio capture via WASAPI loopback (cpal): opening an input
//! stream on the default output device transparently enables loopback mode,
//! capturing the mix of everything playing. The mix format (typically 48 kHz
//! stereo f32) is resampled to the provider rate, mixed to mono, and
//! quantized to PCM16.

use crate::audio::streaming_resampler::StreamingPcm16Resampler;
use crate::audio::{AudioCaptureFormat, SystemAudioCaptureError};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

#[derive(Clone)]
pub struct WindowsSystemAudioCapture {
    stream: Arc<Mutex<Option<cpal::Stream>>>,
    active: Arc<AtomicBool>,
    audio_tx: Arc<Mutex<Option<mpsc::UnboundedSender<Vec<u8>>>>>,
    error_tx: Arc<Mutex<Option<mpsc::UnboundedSender<String>>>>,
}

impl WindowsSystemAudioCapture {
    pub fn new() -> Self {
        Self {
            stream: Arc::new(Mutex::new(None)),
            active: Arc::new(AtomicBool::new(false)),
            audio_tx: Arc::new(Mutex::new(None)),
            error_tx: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn start(
        &self,
        audio_tx: mpsc::UnboundedSender<Vec<u8>>,
        error_tx: mpsc::UnboundedSender<String>,
        format: AudioCaptureFormat,
    ) -> Result<(), SystemAudioCaptureError> {
        if self.stream.lock().unwrap().is_some() {
            return Err(SystemAudioCaptureError::AlreadyRunning);
        }

        let host = cpal::default_host();
        let output_device = host
            .default_output_device()
            .ok_or(SystemAudioCaptureError::NoPlaybackDevice)?;

        // Opening an input stream on the output device enables WASAPI
        // loopback: we receive the system mix (this app plays no audio, so
        // there is no echo).
        let input_config = output_device
            .default_input_config()
            .map_err(|error| SystemAudioCaptureError::Other(error.to_string()))?;
        let sample_format = input_config.sample_format();
        let channel_count = input_config.channels() as usize;
        // cpal 0.18: `SampleRate` is a `u32` type alias (not a tuple struct).
        let sample_rate = input_config.sample_rate();

        let resampler =
            StreamingPcm16Resampler::new(sample_rate, format.sample_rate_hz, channel_count)?;
        let resampler = Arc::new(Mutex::new(resampler));
        let active = self.active.clone();
        active.store(true, Ordering::SeqCst);

        let stream = match sample_format {
            cpal::SampleFormat::F32 => {
                let resampler_for_cb = resampler.clone();
                let audio_tx_for_cb = audio_tx.clone();
                let error_tx_for_cb = error_tx.clone();
                let active_for_cb = active.clone();
                output_device
                    .build_input_stream(
                        input_config.into(),
                        move |data: &[f32], _info| {
                            if !active_for_cb.load(Ordering::SeqCst) {
                                return;
                            }
                            process_frames_f32(data, &resampler_for_cb, &audio_tx_for_cb);
                        },
                        move |error| {
                            let _ = error_tx_for_cb
                                .send(format!("System audio capture stopped: {error}"));
                        },
                        None,
                    )
                    .map_err(|error| SystemAudioCaptureError::Other(error.to_string()))?
            }
            cpal::SampleFormat::I16 => {
                let resampler_for_cb = resampler.clone();
                let audio_tx_for_cb = audio_tx.clone();
                let error_tx_for_cb = error_tx.clone();
                let active_for_cb = active.clone();
                output_device
                    .build_input_stream(
                        input_config.into(),
                        move |data: &[i16], _info| {
                            if !active_for_cb.load(Ordering::SeqCst) {
                                return;
                            }
                            let frames: Vec<f32> = data
                                .iter()
                                .map(|sample| *sample as f32 / 32_768.0)
                                .collect();
                            process_frames_f32(&frames, &resampler_for_cb, &audio_tx_for_cb);
                        },
                        move |error| {
                            let _ = error_tx_for_cb
                                .send(format!("System audio capture stopped: {error}"));
                        },
                        None,
                    )
                    .map_err(|error| SystemAudioCaptureError::Other(error.to_string()))?
            }
            other => {
                return Err(SystemAudioCaptureError::Other(format!(
                    "unsupported sample format: {other:?}"
                )));
            }
        };

        stream
            .play()
            .map_err(|error| SystemAudioCaptureError::Other(error.to_string()))?;
        *self.audio_tx.lock().unwrap() = Some(audio_tx);
        *self.error_tx.lock().unwrap() = Some(error_tx);
        *self.stream.lock().unwrap() = Some(stream);
        Ok(())
    }

    pub async fn stop(&self) {
        self.active.store(false, Ordering::SeqCst);
        if let Some(stream) = self.stream.lock().unwrap().take() {
            drop(stream);
        }
        let _ = self.audio_tx.lock().unwrap().take();
        let _ = self.error_tx.lock().unwrap().take();
    }
}

impl Default for WindowsSystemAudioCapture {
    fn default() -> Self {
        Self::new()
    }
}

/// Buffers interleaved frames across callbacks, resamples complete windows,
/// and emits mono PCM16.
fn process_frames_f32(
    data: &[f32],
    resampler: &Arc<Mutex<StreamingPcm16Resampler>>,
    audio_tx: &mpsc::UnboundedSender<Vec<u8>>,
) {
    let mut resampler = resampler.lock().unwrap();
    let Ok(buffers) = resampler.push_interleaved(data) else {
        return;
    };
    for pcm in buffers {
        let _ = audio_tx.send(pcm);
    }
}
