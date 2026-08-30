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
use cpal::{FromSample, Sample};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct WindowsSystemAudioCapture {
    stream: Arc<Mutex<Option<cpal::Stream>>>,
    active: Arc<AtomicBool>,
}

struct WindowsAudioProcessor {
    resampler: StreamingPcm16Resampler,
    normalized_samples: Vec<f32>,
}

struct WindowsStreamContext {
    active: Arc<AtomicBool>,
    processor: Arc<Mutex<WindowsAudioProcessor>>,
    audio_ingress: AudioIngress,
    failure_tx: CaptureFailureSender,
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

        // CPAL enables WASAPI loopback when an input stream is built on an
        // output device. Its configuration must still come from that output
        // endpoint: `default_input_config()` rejects render devices before a
        // loopback stream can be created.
        let input_config = output_device
            .default_output_config()
            .map_err(|_| SystemAudioCaptureError::NativeStartFailed)?;
        let sample_format = input_config.sample_format();
        let channel_count = input_config.channels() as usize;
        // cpal 0.18: `SampleRate` is a `u32` type alias (not a tuple struct).
        let sample_rate = input_config.sample_rate();

        if !supports_sample_format(sample_format) {
            return Err(SystemAudioCaptureError::UnsupportedAudioFormat);
        }

        let processor = Arc::new(Mutex::new(WindowsAudioProcessor {
            resampler: StreamingPcm16Resampler::new(
                sample_rate,
                format.sample_rate_hz,
                channel_count,
            )?,
            normalized_samples: Vec::new(),
        }));
        let stream_context = WindowsStreamContext {
            active: self.active.clone(),
            processor,
            audio_ingress,
            failure_tx,
        };
        let stream_config: cpal::StreamConfig = input_config.into();

        let stream = match sample_format {
            cpal::SampleFormat::F32 => {
                build_f32_input_stream(&output_device, stream_config, stream_context)?
            }
            cpal::SampleFormat::I8 => {
                build_converting_input_stream::<i8>(&output_device, stream_config, stream_context)?
            }
            cpal::SampleFormat::I16 => {
                build_converting_input_stream::<i16>(&output_device, stream_config, stream_context)?
            }
            cpal::SampleFormat::I24 => build_converting_input_stream::<cpal::I24>(
                &output_device,
                stream_config,
                stream_context,
            )?,
            cpal::SampleFormat::I32 => {
                build_converting_input_stream::<i32>(&output_device, stream_config, stream_context)?
            }
            cpal::SampleFormat::I64 => {
                build_converting_input_stream::<i64>(&output_device, stream_config, stream_context)?
            }
            cpal::SampleFormat::U8 => {
                build_converting_input_stream::<u8>(&output_device, stream_config, stream_context)?
            }
            cpal::SampleFormat::U16 => {
                build_converting_input_stream::<u16>(&output_device, stream_config, stream_context)?
            }
            cpal::SampleFormat::U24 => build_converting_input_stream::<cpal::U24>(
                &output_device,
                stream_config,
                stream_context,
            )?,
            cpal::SampleFormat::U32 => {
                build_converting_input_stream::<u32>(&output_device, stream_config, stream_context)?
            }
            cpal::SampleFormat::U64 => {
                build_converting_input_stream::<u64>(&output_device, stream_config, stream_context)?
            }
            cpal::SampleFormat::F64 => {
                build_converting_input_stream::<f64>(&output_device, stream_config, stream_context)?
            }
            _ => return Err(SystemAudioCaptureError::UnsupportedAudioFormat),
        };

        self.active.store(true, Ordering::SeqCst);
        if stream.play().is_err() {
            self.active.store(false, Ordering::SeqCst);
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

fn supports_sample_format(sample_format: cpal::SampleFormat) -> bool {
    matches!(
        sample_format,
        cpal::SampleFormat::I8
            | cpal::SampleFormat::I16
            | cpal::SampleFormat::I24
            | cpal::SampleFormat::I32
            | cpal::SampleFormat::I64
            | cpal::SampleFormat::U8
            | cpal::SampleFormat::U16
            | cpal::SampleFormat::U24
            | cpal::SampleFormat::U32
            | cpal::SampleFormat::U64
            | cpal::SampleFormat::F32
            | cpal::SampleFormat::F64
    )
}

fn build_f32_input_stream(
    output_device: &cpal::Device,
    stream_config: cpal::StreamConfig,
    context: WindowsStreamContext,
) -> Result<cpal::Stream, SystemAudioCaptureError> {
    let WindowsStreamContext {
        active,
        processor,
        audio_ingress,
        failure_tx,
    } = context;
    let failure_tx_for_error = failure_tx.clone();
    output_device
        .build_input_stream(
            stream_config,
            move |data: &[f32], _info| {
                if !active.load(Ordering::SeqCst) {
                    return;
                }
                process_frames_f32(data, &processor, &audio_ingress, &failure_tx);
            },
            move |_error| {
                failure_tx_for_error.report(SystemAudioCaptureFailure::NativeStopped);
            },
            None,
        )
        .map_err(|_| SystemAudioCaptureError::NativeStartFailed)
}

fn build_converting_input_stream<T>(
    output_device: &cpal::Device,
    stream_config: cpal::StreamConfig,
    context: WindowsStreamContext,
) -> Result<cpal::Stream, SystemAudioCaptureError>
where
    T: cpal::SizedSample,
    f32: FromSample<T>,
{
    let WindowsStreamContext {
        active,
        processor,
        audio_ingress,
        failure_tx,
    } = context;
    let failure_tx_for_error = failure_tx.clone();
    output_device
        .build_input_stream(
            stream_config,
            move |data: &[T], _info| {
                if !active.load(Ordering::SeqCst) {
                    return;
                }
                process_frames(data, &processor, &audio_ingress, &failure_tx);
            },
            move |_error| {
                failure_tx_for_error.report(SystemAudioCaptureFailure::NativeStopped);
            },
            None,
        )
        .map_err(|_| SystemAudioCaptureError::NativeStartFailed)
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
    processor: &Arc<Mutex<WindowsAudioProcessor>>,
    audio_ingress: &AudioIngress,
    failure_tx: &CaptureFailureSender,
) {
    if failure_tx.has_reported() {
        return;
    }
    let Ok(mut processor) = processor.lock() else {
        failure_tx.report(SystemAudioCaptureFailure::AudioProcessingFailed);
        return;
    };
    let Ok(buffers) = processor.resampler.push_interleaved(data) else {
        failure_tx.report(SystemAudioCaptureFailure::AudioProcessingFailed);
        return;
    };
    drop(processor);
    emit_buffers(buffers, audio_ingress, failure_tx);
}

fn process_frames<T>(
    data: &[T],
    processor: &Arc<Mutex<WindowsAudioProcessor>>,
    audio_ingress: &AudioIngress,
    failure_tx: &CaptureFailureSender,
) where
    T: Sample,
    f32: FromSample<T>,
{
    if failure_tx.has_reported() {
        return;
    }
    let Ok(mut processor) = processor.lock() else {
        failure_tx.report(SystemAudioCaptureFailure::AudioProcessingFailed);
        return;
    };
    let WindowsAudioProcessor {
        resampler,
        normalized_samples,
    } = &mut *processor;
    normalize_samples(data, normalized_samples);
    let buffers = resampler.push_interleaved(normalized_samples);
    // Keep the reusable allocation, but do not retain a logical copy of the
    // most recent system-audio callback after it enters the PCM pipeline.
    normalized_samples.clear();
    let Ok(buffers) = buffers else {
        failure_tx.report(SystemAudioCaptureFailure::AudioProcessingFailed);
        return;
    };
    drop(processor);
    emit_buffers(buffers, audio_ingress, failure_tx);
}

fn normalize_samples<T>(data: &[T], output: &mut Vec<f32>)
where
    T: Sample,
    f32: FromSample<T>,
{
    output.clear();
    output.extend(data.iter().copied().map(f32::from_sample));
}

fn emit_buffers(
    buffers: Vec<Vec<u8>>,
    audio_ingress: &AudioIngress,
    failure_tx: &CaptureFailureSender,
) {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_normalized<T>(samples: &[T])
    where
        T: Sample,
        f32: FromSample<T>,
    {
        let mut output = Vec::new();
        normalize_samples(samples, &mut output);

        assert_eq!(output.len(), 3);
        assert!((output[0] + 1.0).abs() < 0.000_001, "{output:?}");
        assert!(output[1].abs() < f32::EPSILON, "{output:?}");
        assert!(output[2] > 0.99 && output[2] <= 1.0, "{output:?}");
    }

    #[test]
    fn every_cpal_pcm_and_float_format_is_supported() {
        for format in [
            cpal::SampleFormat::I8,
            cpal::SampleFormat::I16,
            cpal::SampleFormat::I24,
            cpal::SampleFormat::I32,
            cpal::SampleFormat::I64,
            cpal::SampleFormat::U8,
            cpal::SampleFormat::U16,
            cpal::SampleFormat::U24,
            cpal::SampleFormat::U32,
            cpal::SampleFormat::U64,
            cpal::SampleFormat::F32,
            cpal::SampleFormat::F64,
        ] {
            assert!(supports_sample_format(format), "unsupported {format}");
        }
    }

    #[test]
    fn dsd_formats_are_rejected_before_stream_creation() {
        for format in [
            cpal::SampleFormat::DsdU8,
            cpal::SampleFormat::DsdU16,
            cpal::SampleFormat::DsdU32,
        ] {
            assert!(!supports_sample_format(format), "accepted {format}");
        }
    }

    #[test]
    fn integer_samples_are_normalized_around_their_equilibrium() {
        assert_normalized(&[i8::MIN, 0, i8::MAX]);
        assert_normalized(&[i16::MIN, 0, i16::MAX]);
        assert_normalized(&[
            cpal::I24::new(-8_388_608).unwrap(),
            cpal::I24::new(0).unwrap(),
            cpal::I24::new(8_388_607).unwrap(),
        ]);
        assert_normalized(&[i32::MIN, 0, i32::MAX]);
        assert_normalized(&[i64::MIN, 0, i64::MAX]);
        assert_normalized(&[u8::MIN, 128, u8::MAX]);
        assert_normalized(&[u16::MIN, 32_768, u16::MAX]);
        assert_normalized(&[
            cpal::U24::new(0).unwrap(),
            cpal::U24::new(8_388_608).unwrap(),
            cpal::U24::new(16_777_215).unwrap(),
        ]);
        assert_normalized(&[u32::MIN, 2_147_483_648, u32::MAX]);
        assert_normalized(&[u64::MIN, 9_223_372_036_854_775_808, u64::MAX]);
    }

    #[test]
    fn floating_point_samples_keep_their_amplitude() {
        assert_normalized(&[-1.0_f32, 0.0, 1.0]);
        assert_normalized(&[-1.0_f64, 0.0, 1.0]);
    }

    #[test]
    fn converted_callbacks_reuse_the_normalization_buffer() {
        let mut output = Vec::new();
        normalize_samples(&[i16::MIN, 0, i16::MAX], &mut output);
        let allocation = output.as_ptr();
        let capacity = output.capacity();

        normalize_samples(&[0_i16, 1, 2], &mut output);

        assert_eq!(output.as_ptr(), allocation);
        assert_eq!(output.capacity(), capacity);
    }
}
