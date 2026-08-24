//! Windows system-audio capture via WASAPI loopback (cpal): opening an input
//! stream on the default output device transparently enables loopback mode,
//! capturing the mix of everything playing. The mix format (typically 48 kHz
//! stereo f32) is resampled to the provider rate, mixed to mono, and
//! quantized to PCM16.

use crate::audio::send_pipeline::{AudioIngress, AudioIngressError};
use crate::audio::streaming_resampler::StreamingPcm16Resampler;
use crate::audio::{
    AudioCaptureFormat, CaptureFailureSender, SystemAudioCaptureError, SystemAudioCaptureFailure,
};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct WindowsSystemAudioCapture {
    stream: Arc<Mutex<Option<cpal::Stream>>>,
    active: Arc<AtomicBool>,
}

impl WindowsSystemAudioCapture {
    pub fn new() -> Self {
        Self {
            stream: Arc::new(Mutex::new(None)),
            active: Arc::new(AtomicBool::new(false)),
        }
    }

    pub async fn start(
        &self,
        audio_ingress: AudioIngress,
        failure_tx: CaptureFailureSender,
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
            .map_err(|_| SystemAudioCaptureError::NativeStartFailed)?;
        let sample_format = input_config.sample_format();
        let channel_count = input_config.channels() as usize;
        // cpal 0.18: `SampleRate` is a `u32` type alias (not a tuple struct).
        let sample_rate = input_config.sample_rate();

        let resampler =
            StreamingPcm16Resampler::new(sample_rate, format.sample_rate_hz, channel_count)?;
        let resampler = Arc::new(Mutex::new(resampler));
        let active = self.active.clone();

        let stream = match sample_format {
            cpal::SampleFormat::F32 => {
                let resampler_for_cb = resampler.clone();
                let audio_ingress_for_cb = audio_ingress.clone();
                let failure_tx_for_cb = failure_tx.clone();
                let failure_tx_for_error = failure_tx.clone();
                let active_for_cb = active.clone();
                output_device
                    .build_input_stream(
                        input_config.into(),
                        move |data: &[f32], _info| {
                            if !active_for_cb.load(Ordering::SeqCst) {
                                return;
                            }
                            process_frames_f32(
                                data,
                                &resampler_for_cb,
                                &audio_ingress_for_cb,
                                &failure_tx_for_cb,
                            );
                        },
                        move |_error| {
                            failure_tx_for_error.report(SystemAudioCaptureFailure::NativeStopped);
                        },
                        None,
                    )
                    .map_err(|_| SystemAudioCaptureError::NativeStartFailed)?
            }
            cpal::SampleFormat::I16 => {
                let resampler_for_cb = resampler.clone();
                let audio_ingress_for_cb = audio_ingress.clone();
                let failure_tx_for_cb = failure_tx.clone();
                let failure_tx_for_error = failure_tx.clone();
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
                            process_frames_f32(
                                &frames,
                                &resampler_for_cb,
                                &audio_ingress_for_cb,
                                &failure_tx_for_cb,
                            );
                        },
                        move |_error| {
                            failure_tx_for_error.report(SystemAudioCaptureFailure::NativeStopped);
                        },
                        None,
                    )
                    .map_err(|_| SystemAudioCaptureError::NativeStartFailed)?
            }
            _ => return Err(SystemAudioCaptureError::UnsupportedAudioFormat),
        };

        active.store(true, Ordering::SeqCst);
        if stream.play().is_err() {
            active.store(false, Ordering::SeqCst);
            return Err(SystemAudioCaptureError::NativeStartFailed);
        }
        *self.stream.lock().unwrap() = Some(stream);
        Ok(())
    }

    pub async fn stop(&self) {
        self.active.store(false, Ordering::SeqCst);
        if let Some(stream) = self.stream.lock().unwrap().take() {
            drop(stream);
        }
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
    audio_ingress: &AudioIngress,
    failure_tx: &CaptureFailureSender,
) {
    if failure_tx.has_reported() {
        return;
    }
    let Ok(mut resampler) = resampler.lock() else {
        failure_tx.report(SystemAudioCaptureFailure::AudioProcessingFailed);
        return;
    };
    let Ok(buffers) = resampler.push_interleaved(data) else {
        failure_tx.report(SystemAudioCaptureFailure::AudioProcessingFailed);
        return;
    };
    for pcm in buffers {
        match audio_ingress.try_send(pcm) {
            Ok(()) => {}
            Err(AudioIngressError::Backpressure) => {
                failure_tx.report(SystemAudioCaptureFailure::Backpressure);
                return;
            }
            Err(AudioIngressError::Closed) => return,
        }
    }
}
