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
    stream: Arc<Mutex<StreamSlot<cpal::Stream>>>,
    active: Arc<AtomicBool>,
}

enum StreamSlotState<S> {
    Idle,
    Starting(u64),
    Running(S),
}

struct StreamSlot<S> {
    next_token: u64,
    state: StreamSlotState<S>,
}

impl<S> Default for StreamSlot<S> {
    fn default() -> Self {
        Self {
            next_token: 0,
            state: StreamSlotState::Idle,
        }
    }
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
            stream: Arc::new(Mutex::new(StreamSlot::default())),
            active: Arc::new(AtomicBool::new(false)),
        }
    }

    pub async fn start(
        &self,
        audio_ingress: AudioIngress,
        failure_tx: CaptureFailureSender,
        format: AudioCaptureFormat,
    ) -> Result<(), SystemAudioCaptureError> {
        build_install_and_play_stream(
            &self.stream,
            &self.active,
            || {
                let host = cpal::default_host();
                let output_device = host
                    .default_output_device()
                    .ok_or(SystemAudioCaptureError::NoPlaybackDevice)?;
                build_stream_on_output_device(
                    &output_device,
                    audio_ingress,
                    failure_tx,
                    format,
                    Arc::clone(&self.active),
                )
            },
            |stream| stream.play().map_err(|_| ()),
        )
    }

    #[cfg(test)]
    fn start_on_output_device(
        &self,
        output_device: &cpal::Device,
        audio_ingress: AudioIngress,
        failure_tx: CaptureFailureSender,
        format: AudioCaptureFormat,
    ) -> Result<(), SystemAudioCaptureError> {
        build_install_and_play_stream(
            &self.stream,
            &self.active,
            || {
                build_stream_on_output_device(
                    output_device,
                    audio_ingress,
                    failure_tx,
                    format,
                    Arc::clone(&self.active),
                )
            },
            |stream| stream.play().map_err(|_| ()),
        )
    }

    pub async fn stop(&self) {
        stop_stream(&self.stream, &self.active);
    }
}

fn build_stream_on_output_device(
    output_device: &cpal::Device,
    audio_ingress: AudioIngress,
    failure_tx: CaptureFailureSender,
    format: AudioCaptureFormat,
    active: Arc<AtomicBool>,
) -> Result<cpal::Stream, SystemAudioCaptureError> {
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
        resampler: StreamingPcm16Resampler::new(sample_rate, format.sample_rate_hz, channel_count)?,
        normalized_samples: Vec::new(),
    }));
    let stream_context = WindowsStreamContext {
        active,
        processor,
        audio_ingress,
        failure_tx,
    };
    let stream_config: cpal::StreamConfig = input_config.into();

    match sample_format {
        cpal::SampleFormat::F32 => {
            build_f32_input_stream(output_device, stream_config, stream_context)
        }
        cpal::SampleFormat::I8 => {
            build_converting_input_stream::<i8>(output_device, stream_config, stream_context)
        }
        cpal::SampleFormat::I16 => {
            build_converting_input_stream::<i16>(output_device, stream_config, stream_context)
        }
        cpal::SampleFormat::I24 => {
            build_i24_input_stream(output_device, stream_config, stream_context)
        }
        cpal::SampleFormat::I32 => {
            build_converting_input_stream::<i32>(output_device, stream_config, stream_context)
        }
        cpal::SampleFormat::I64 => {
            build_converting_input_stream::<i64>(output_device, stream_config, stream_context)
        }
        cpal::SampleFormat::U8 => {
            build_converting_input_stream::<u8>(output_device, stream_config, stream_context)
        }
        cpal::SampleFormat::U16 => {
            build_converting_input_stream::<u16>(output_device, stream_config, stream_context)
        }
        cpal::SampleFormat::U24 => {
            build_converting_input_stream::<cpal::U24>(output_device, stream_config, stream_context)
        }
        cpal::SampleFormat::U32 => {
            build_converting_input_stream::<u32>(output_device, stream_config, stream_context)
        }
        cpal::SampleFormat::U64 => {
            build_converting_input_stream::<u64>(output_device, stream_config, stream_context)
        }
        cpal::SampleFormat::F64 => {
            build_converting_input_stream::<f64>(output_device, stream_config, stream_context)
        }
        _ => Err(SystemAudioCaptureError::UnsupportedAudioFormat),
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

/// Reserves the capture slot before any native build work. A concurrent stop
/// changes `Starting(token)` back to `Idle`, so the completed build can only
/// be installed when it still owns the same token.
fn build_install_and_play_stream<S, E>(
    stream_slot: &Mutex<StreamSlot<S>>,
    active: &AtomicBool,
    build: impl FnOnce() -> Result<S, SystemAudioCaptureError>,
    play: impl FnOnce(&S) -> Result<(), E>,
) -> Result<(), SystemAudioCaptureError> {
    let token = reserve_stream_start(stream_slot, active)?;
    let stream = match build() {
        Ok(stream) => stream,
        Err(error) => {
            cancel_stream_start(stream_slot, token);
            return Err(error);
        }
    };

    install_and_play_reserved_stream(stream_slot, active, token, stream, play)
}

fn reserve_stream_start<S>(
    stream_slot: &Mutex<StreamSlot<S>>,
    active: &AtomicBool,
) -> Result<u64, SystemAudioCaptureError> {
    let mut slot = stream_slot.lock().unwrap();
    if !matches!(&slot.state, StreamSlotState::Idle) {
        return Err(SystemAudioCaptureError::AlreadyRunning);
    }
    let token = slot.next_token;
    slot.next_token = slot.next_token.wrapping_add(1);
    active.store(false, Ordering::SeqCst);
    slot.state = StreamSlotState::Starting(token);
    Ok(token)
}

fn cancel_stream_start<S>(stream_slot: &Mutex<StreamSlot<S>>, token: u64) {
    let mut slot = stream_slot.lock().unwrap();
    if matches!(&slot.state, StreamSlotState::Starting(current) if *current == token) {
        slot.state = StreamSlotState::Idle;
    }
}

/// Installs before `play` and retains the slot lock while queuing the native
/// start. Native failure cleanup or stop therefore observes either the owned
/// running stream or an idle/cancelled slot, never an unowned stream in flight.
fn install_and_play_reserved_stream<S, E>(
    stream_slot: &Mutex<StreamSlot<S>>,
    active: &AtomicBool,
    token: u64,
    stream: S,
    play: impl FnOnce(&S) -> Result<(), E>,
) -> Result<(), SystemAudioCaptureError> {
    let mut slot = stream_slot.lock().unwrap();
    if !matches!(&slot.state, StreamSlotState::Starting(current) if *current == token) {
        // A stop (or a newer start after that stop) invalidated this build.
        return Err(SystemAudioCaptureError::NativeStartFailed);
    }

    slot.state = StreamSlotState::Running(stream);
    active.store(true, Ordering::SeqCst);
    let play_failed = match &slot.state {
        StreamSlotState::Running(stream) => play(stream).is_err(),
        StreamSlotState::Idle | StreamSlotState::Starting(_) => unreachable!(),
    };
    if play_failed {
        active.store(false, Ordering::SeqCst);
        let failed_stream = match std::mem::replace(&mut slot.state, StreamSlotState::Idle) {
            StreamSlotState::Running(stream) => stream,
            StreamSlotState::Idle | StreamSlotState::Starting(_) => unreachable!(),
        };
        drop(slot);
        drop(failed_stream);
        return Err(SystemAudioCaptureError::NativeStartFailed);
    }
    Ok(())
}

fn stop_stream<S>(stream_slot: &Mutex<StreamSlot<S>>, active: &AtomicBool) {
    let stream = {
        let mut slot = stream_slot.lock().unwrap();
        active.store(false, Ordering::SeqCst);
        match std::mem::replace(&mut slot.state, StreamSlotState::Idle) {
            StreamSlotState::Running(stream) => Some(stream),
            StreamSlotState::Idle | StreamSlotState::Starting(_) => None,
        }
    };
    drop(stream);
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

fn build_i24_input_stream(
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
            move |data: &[cpal::I24], _info| {
                if !active.load(Ordering::SeqCst) {
                    return;
                }
                process_frames_i24(data, &processor, &audio_ingress, &failure_tx);
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
    process_converted_frames(
        data,
        processor,
        audio_ingress,
        failure_tx,
        normalize_samples::<T>,
    );
}

fn process_frames_i24(
    data: &[cpal::I24],
    processor: &Arc<Mutex<WindowsAudioProcessor>>,
    audio_ingress: &AudioIngress,
    failure_tx: &CaptureFailureSender,
) {
    process_converted_frames(
        data,
        processor,
        audio_ingress,
        failure_tx,
        normalize_wasapi_i24_samples,
    );
}

fn process_converted_frames<T>(
    data: &[T],
    processor: &Arc<Mutex<WindowsAudioProcessor>>,
    audio_ingress: &AudioIngress,
    failure_tx: &CaptureFailureSender,
    normalize: impl FnOnce(&[T], &mut Vec<f32>),
) {
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
    normalize(data, normalized_samples);
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

/// CPAL exposes WASAPI's 24-valid-bit PCM as `I24`, but the native shared-mode
/// buffer uses a 32-bit `WAVEFORMATEXTENSIBLE` container. Windows left-aligns
/// valid PCM bits in that container, so recover the signed 24-bit value before
/// applying CPAL's normalized-sample conversion contract.
fn normalize_wasapi_i24_samples(data: &[cpal::I24], output: &mut Vec<f32>) {
    const CONTAINER_PADDING_BITS: u32 = i32::BITS - 24;
    const I24_SCALE: f32 = 8_388_608.0;

    output.clear();
    output.extend(
        data.iter()
            .map(|sample| (sample.inner() >> CONTAINER_PADDING_BITS) as f32 / I24_SCALE),
    );
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
    use crate::audio::send_pipeline::AudioSendPipeline;
    use std::mem::size_of;
    use std::sync::atomic::AtomicUsize;
    use std::sync::Barrier;
    use std::time::{Duration, Instant};
    use tokio::sync::mpsc::error::TryRecvError;

    const TEST_TONE_FREQUENCY_HZ: f32 = 997.0;
    const TEST_TONE_AMPLITUDE: f32 = 0.2;
    const LOOPBACK_SAMPLE_RATE_HZ: f64 = 16_000.0;
    const TEST_TONE_WINDOW_SAMPLES: usize = 4_096;
    const TEST_TONE_MIN_RMS: f64 = 0.01;
    const TEST_TONE_MAX_RMS: f64 = 0.6;
    const TEST_TONE_MAX_PEAK: f64 = 0.95;
    const TEST_TONE_MAX_SATURATION_RATIO: f64 = 0.01;
    const TEST_TONE_MIN_ENERGY_RATIO: f64 = 0.5;

    #[derive(Default)]
    struct TestToneDetector {
        samples_in_window: usize,
        sine_projection: f64,
        cosine_projection: f64,
        square_sum: f64,
        peak: f64,
        saturated_samples: usize,
    }

    impl TestToneDetector {
        fn push_pcm16_le(&mut self, pcm: &[u8]) -> bool {
            for sample_bytes in pcm.as_chunks::<2>().0 {
                let sample = f64::from(i16::from_le_bytes(*sample_bytes)) / 32_768.0;
                let phase = std::f64::consts::TAU
                    * f64::from(TEST_TONE_FREQUENCY_HZ)
                    * self.samples_in_window as f64
                    / LOOPBACK_SAMPLE_RATE_HZ;
                let (phase_sine, phase_cosine) = phase.sin_cos();
                self.sine_projection += sample * phase_sine;
                self.cosine_projection += sample * phase_cosine;
                self.square_sum += sample * sample;
                self.peak = self.peak.max(sample.abs());
                if sample.abs() >= 0.999 {
                    self.saturated_samples += 1;
                }
                self.samples_in_window += 1;

                if self.samples_in_window == TEST_TONE_WINDOW_SAMPLES {
                    let detected = self.window_matches_test_tone();
                    self.reset();
                    if detected {
                        return true;
                    }
                }
            }
            false
        }

        fn window_matches_test_tone(&self) -> bool {
            let sample_count = self.samples_in_window as f64;
            let mean_square = self.square_sum / sample_count;
            let rms = mean_square.sqrt();
            let saturation_ratio = self.saturated_samples as f64 / sample_count;
            if !(TEST_TONE_MIN_RMS..=TEST_TONE_MAX_RMS).contains(&rms)
                || self.peak > TEST_TONE_MAX_PEAK
                || saturation_ratio > TEST_TONE_MAX_SATURATION_RATIO
            {
                return false;
            }

            let projected_mean_square = 2.0
                * (self.sine_projection * self.sine_projection
                    + self.cosine_projection * self.cosine_projection)
                / (sample_count * sample_count);
            projected_mean_square / mean_square >= TEST_TONE_MIN_ENERGY_RATIO
        }

        fn reset(&mut self) {
            self.samples_in_window = 0;
            self.sine_projection = 0.0;
            self.cosine_projection = 0.0;
            self.square_sum = 0.0;
            self.peak = 0.0;
            self.saturated_samples = 0;
        }
    }

    fn sine_pcm16(frequency_hz: f64, amplitude: f64) -> Vec<u8> {
        let mut pcm = Vec::with_capacity(TEST_TONE_WINDOW_SAMPLES * size_of::<i16>());
        for sample_index in 0..TEST_TONE_WINDOW_SAMPLES {
            let phase = std::f64::consts::TAU * frequency_hz * sample_index as f64
                / LOOPBACK_SAMPLE_RATE_HZ;
            let sample = (phase.sin() * amplitude * f64::from(i16::MAX)).round() as i16;
            pcm.extend_from_slice(&sample.to_le_bytes());
        }
        pcm
    }

    fn clipped_square_pcm16(frequency_hz: f64) -> Vec<u8> {
        let mut pcm = Vec::with_capacity(TEST_TONE_WINDOW_SAMPLES * size_of::<i16>());
        for sample_index in 0..TEST_TONE_WINDOW_SAMPLES {
            let phase = std::f64::consts::TAU * frequency_hz * sample_index as f64
                / LOOPBACK_SAMPLE_RATE_HZ;
            let sample = if phase.sin().is_sign_negative() {
                i16::MIN
            } else {
                i16::MAX
            };
            pcm.extend_from_slice(&sample.to_le_bytes());
        }
        pcm
    }

    fn build_test_tone_stream<T>(
        output_device: &cpal::Device,
        stream_config: cpal::StreamConfig,
        output_failed: Arc<AtomicBool>,
        mut encode_sample: impl FnMut(f32) -> T + Send + 'static,
    ) -> Result<cpal::Stream, cpal::Error>
    where
        T: cpal::SizedSample + Copy,
    {
        let channel_count = stream_config.channels as usize;
        let phase_step = TEST_TONE_FREQUENCY_HZ / stream_config.sample_rate as f32;
        let mut phase = 0.0_f32;

        output_device.build_output_stream(
            stream_config,
            move |data: &mut [T], _info| {
                for frame in data.chunks_mut(channel_count) {
                    let value = (std::f32::consts::TAU * phase).sin() * TEST_TONE_AMPLITUDE;
                    phase = (phase + phase_step).fract();
                    let sample = encode_sample(value);
                    frame.fill(sample);
                }
            },
            move |_error| {
                output_failed.store(true, Ordering::SeqCst);
            },
            None,
        )
    }

    fn build_output_test_tone(
        output_device: &cpal::Device,
        output_failed: Arc<AtomicBool>,
    ) -> Result<cpal::Stream, &'static str> {
        let output_config = output_device
            .default_output_config()
            .map_err(|_| "default playback format unavailable for the native tone")?;
        let sample_format = output_config.sample_format();
        let stream_config = output_config.into();

        macro_rules! build_converting_tone {
            ($sample_type:ty) => {
                build_test_tone_stream::<$sample_type>(
                    output_device,
                    stream_config,
                    output_failed,
                    <$sample_type>::from_sample,
                )
            };
        }

        let stream = match sample_format {
            cpal::SampleFormat::I8 => build_converting_tone!(i8),
            cpal::SampleFormat::I16 => build_converting_tone!(i16),
            cpal::SampleFormat::I24 => build_test_tone_stream::<cpal::I24>(
                output_device,
                stream_config,
                output_failed,
                |value| {
                    // Match the native 24-valid-in-32 representation expected
                    // by the vendored WASAPI output backend.
                    let logical = cpal::I24::from_sample(value).inner();
                    cpal::I24::new_unchecked(logical << (i32::BITS - 24))
                },
            ),
            cpal::SampleFormat::I32 => build_converting_tone!(i32),
            cpal::SampleFormat::I64 => build_converting_tone!(i64),
            cpal::SampleFormat::U8 => build_converting_tone!(u8),
            cpal::SampleFormat::U16 => build_converting_tone!(u16),
            cpal::SampleFormat::U24 => build_converting_tone!(cpal::U24),
            cpal::SampleFormat::U32 => build_converting_tone!(u32),
            cpal::SampleFormat::U64 => build_converting_tone!(u64),
            cpal::SampleFormat::F32 => build_converting_tone!(f32),
            cpal::SampleFormat::F64 => build_converting_tone!(f64),
            _ => return Err("default playback format cannot carry the native test tone"),
        }
        .map_err(|_| "native test tone stream could not be built")?;

        Ok(stream)
    }

    fn assert_capture_failure_channel_empty(
        failure_rx: &mut tokio::sync::mpsc::Receiver<SystemAudioCaptureFailure>,
    ) {
        match failure_rx.try_recv() {
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                panic!("the native loopback failure channel disconnected unexpectedly")
            }
            Ok(failure) => panic!(
                "the native loopback stream reported an early fatal failure: {}",
                failure.diagnostic_label()
            ),
        }
    }

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
    fn occupied_stream_slot_rejects_a_second_start_without_playing_it() {
        let stream_slot = Mutex::new(StreamSlot {
            next_token: 0,
            state: StreamSlotState::Running("existing"),
        });
        let active = AtomicBool::new(true);
        let mut build_called = false;
        let mut play_called = false;

        let result = build_install_and_play_stream(
            &stream_slot,
            &active,
            || {
                build_called = true;
                Ok("replacement")
            },
            |_| -> Result<(), ()> {
                play_called = true;
                Ok(())
            },
        );

        assert_eq!(result, Err(SystemAudioCaptureError::AlreadyRunning));
        assert!(!build_called);
        assert!(!play_called);
        assert!(matches!(
            &stream_slot.lock().unwrap().state,
            StreamSlotState::Running(stream) if *stream == "existing"
        ));
        assert!(active.load(Ordering::SeqCst));
    }

    #[test]
    fn immediate_play_failure_clears_the_installed_stream_and_allows_restart() {
        let stream_slot = Mutex::new(StreamSlot::default());
        let active = AtomicBool::new(false);

        let result = build_install_and_play_stream(
            &stream_slot,
            &active,
            || Ok("failed"),
            |_| Err::<(), ()>(()),
        );

        assert_eq!(result, Err(SystemAudioCaptureError::NativeStartFailed));
        assert!(matches!(
            &stream_slot.lock().unwrap().state,
            StreamSlotState::Idle
        ));
        assert!(!active.load(Ordering::SeqCst));

        build_install_and_play_stream(
            &stream_slot,
            &active,
            || Ok("recovered"),
            |_| Ok::<(), ()>(()),
        )
        .expect("a failed play must leave the capture slot restartable");
        assert!(matches!(
            &stream_slot.lock().unwrap().state,
            StreamSlotState::Running(stream) if *stream == "recovered"
        ));
        assert!(active.load(Ordering::SeqCst));
    }

    #[test]
    fn stream_is_installed_and_locked_during_play_then_can_restart_after_cleanup() {
        let stream_slot = Arc::new(Mutex::new(StreamSlot::default()));
        let active = AtomicBool::new(false);
        let slot_observed_by_play = Arc::clone(&stream_slot);

        build_install_and_play_stream(
            &stream_slot,
            &active,
            || Ok("first"),
            move |_| {
                assert!(matches!(
                    slot_observed_by_play.try_lock(),
                    Err(std::sync::TryLockError::WouldBlock)
                ));
                Ok::<(), ()>(())
            },
        )
        .expect("the first stream should start");

        assert!(active.load(Ordering::SeqCst));
        assert!(matches!(
            &stream_slot.lock().unwrap().state,
            StreamSlotState::Running(stream) if *stream == "first"
        ));
        stop_stream(&stream_slot, &active);

        build_install_and_play_stream(&stream_slot, &active, || Ok("second"), |_| Ok::<(), ()>(()))
            .expect("cleanup must leave the slot available for a retry");
        assert!(matches!(
            &stream_slot.lock().unwrap().state,
            StreamSlotState::Running(stream) if *stream == "second"
        ));
        assert!(active.load(Ordering::SeqCst));
    }

    #[test]
    fn stop_cancels_a_blocked_build_without_playing_and_allows_restart() {
        let stream_slot = Arc::new(Mutex::new(StreamSlot::default()));
        let active = Arc::new(AtomicBool::new(false));
        let build_entered = Arc::new(Barrier::new(2));
        let release_build = Arc::new(Barrier::new(2));
        let play_calls = Arc::new(AtomicUsize::new(0));

        let slot_for_start = Arc::clone(&stream_slot);
        let active_for_start = Arc::clone(&active);
        let entered_for_start = Arc::clone(&build_entered);
        let release_for_start = Arc::clone(&release_build);
        let play_calls_for_start = Arc::clone(&play_calls);
        let blocked_start = std::thread::spawn(move || {
            build_install_and_play_stream(
                &slot_for_start,
                &active_for_start,
                || {
                    entered_for_start.wait();
                    release_for_start.wait();
                    Ok("cancelled")
                },
                |_| {
                    play_calls_for_start.fetch_add(1, Ordering::SeqCst);
                    Ok::<(), ()>(())
                },
            )
        });

        build_entered.wait();
        assert!(matches!(
            &stream_slot.lock().unwrap().state,
            StreamSlotState::Starting(_)
        ));
        stop_stream(&stream_slot, &active);
        assert!(matches!(
            &stream_slot.lock().unwrap().state,
            StreamSlotState::Idle
        ));
        assert!(!active.load(Ordering::SeqCst));

        release_build.wait();
        assert_eq!(
            blocked_start.join().unwrap(),
            Err(SystemAudioCaptureError::NativeStartFailed)
        );
        assert_eq!(play_calls.load(Ordering::SeqCst), 0);
        assert!(matches!(
            &stream_slot.lock().unwrap().state,
            StreamSlotState::Idle
        ));
        assert!(!active.load(Ordering::SeqCst));

        build_install_and_play_stream(
            &stream_slot,
            &active,
            || Ok("replacement"),
            |_| {
                play_calls.fetch_add(1, Ordering::SeqCst);
                Ok::<(), ()>(())
            },
        )
        .expect("a cancelled build must leave the capture slot restartable");
        assert_eq!(play_calls.load(Ordering::SeqCst), 1);
        assert!(matches!(
            &stream_slot.lock().unwrap().state,
            StreamSlotState::Running(stream) if *stream == "replacement"
        ));
        assert!(active.load(Ordering::SeqCst));

        stop_stream(&stream_slot, &active);
        assert!(matches!(
            &stream_slot.lock().unwrap().state,
            StreamSlotState::Idle
        ));
        assert!(!active.load(Ordering::SeqCst));
    }

    #[test]
    fn integer_samples_are_normalized_around_their_equilibrium() {
        assert_normalized(&[i8::MIN, 0, i8::MAX]);
        assert_normalized(&[i16::MIN, 0, i16::MAX]);
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
    fn wasapi_i24_left_aligned_container_is_shifted_before_normalization() {
        let samples = [
            cpal::I24::new_unchecked(i32::MIN),
            cpal::I24::new_unchecked(-1_073_741_824),
            cpal::I24::new_unchecked(0),
            cpal::I24::new_unchecked(1_073_741_824),
            cpal::I24::new_unchecked(0x4000_00ff),
            cpal::I24::new_unchecked(2_147_483_392),
        ];
        let mut output = Vec::new();

        normalize_wasapi_i24_samples(&samples, &mut output);

        assert_eq!(output.len(), samples.len());
        assert!((output[0] + 1.0).abs() < f32::EPSILON, "{output:?}");
        assert!((output[1] + 0.5).abs() < f32::EPSILON, "{output:?}");
        assert!(output[2].abs() < f32::EPSILON, "{output:?}");
        assert!((output[3] - 0.5).abs() < f32::EPSILON, "{output:?}");
        assert!((output[4] - 0.5).abs() < f32::EPSILON, "{output:?}");
        assert!(output[5] > 0.99 && output[5] <= 1.0, "{output:?}");
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

    #[test]
    fn test_tone_detector_accepts_997_hz_pcm16_across_callbacks() {
        let pcm = sine_pcm16(f64::from(TEST_TONE_FREQUENCY_HZ), 0.2);
        let split_at = 1_000 * size_of::<i16>();
        let mut detector = TestToneDetector::default();

        assert!(!detector.push_pcm16_le(&pcm[..split_at]));
        assert!(detector.push_pcm16_le(&pcm[split_at..]));
    }

    #[test]
    fn test_tone_detector_rejects_silence() {
        let silence = vec![0; TEST_TONE_WINDOW_SAMPLES * size_of::<i16>()];
        let mut detector = TestToneDetector::default();

        assert!(!detector.push_pcm16_le(&silence));
    }

    #[test]
    fn test_tone_detector_rejects_clearly_different_frequency() {
        let pcm = sine_pcm16(440.0, 0.2);
        let mut detector = TestToneDetector::default();

        assert!(!detector.push_pcm16_le(&pcm));
    }

    #[test]
    fn test_tone_detector_rejects_a_clipped_997_hz_square_wave() {
        let pcm = clipped_square_pcm16(f64::from(TEST_TONE_FREQUENCY_HZ));
        let mut detector = TestToneDetector::default();

        assert!(!detector.push_pcm16_le(&pcm));
    }

    #[test]
    fn test_tone_detector_continues_after_a_failed_window() {
        let off_frequency = sine_pcm16(440.0, 0.2);
        let test_tone = sine_pcm16(f64::from(TEST_TONE_FREQUENCY_HZ), 0.2);
        let mut detector = TestToneDetector::default();

        assert!(!detector.push_pcm16_le(&off_frequency));
        assert!(detector.push_pcm16_le(&test_tone));
    }

    /// Native acceptance probe for the real default render endpoint. This is
    /// ignored in the portable suite because CI runners are not guaranteed to
    /// expose an active playback device.
    #[tokio::test]
    #[ignore = "requires a Windows machine with a default playback endpoint"]
    async fn native_default_output_opens_as_a_wasapi_loopback_stream() {
        let output_device = cpal::default_host()
            .default_output_device()
            .expect("the native probe requires a default playback device");
        let output_failed = Arc::new(AtomicBool::new(false));
        let tone_stream = build_output_test_tone(&output_device, Arc::clone(&output_failed))
            .expect("the in-memory native test tone must use the default output");
        let (pcm_tx, mut pcm_rx) = tokio::sync::mpsc::channel(1);
        let detector = Arc::new(Mutex::new(TestToneDetector::default()));
        let detector_for_pipeline = Arc::clone(&detector);
        let pipeline = AudioSendPipeline::spawn(
            move |pcm| {
                let pcm_tx = pcm_tx.clone();
                let detected = detector_for_pipeline.lock().unwrap().push_pcm16_le(&pcm);
                async move {
                    if detected {
                        let _ = pcm_tx.try_send(());
                    }
                    Ok::<(), ()>(())
                }
            },
            |_failure| {},
        );
        let ingress = pipeline.ingress().expect("pipeline ingress");
        let (failure_tx, mut failure_rx) = CaptureFailureSender::channel();
        let capture = WindowsSystemAudioCapture::new();

        tone_stream
            .play()
            .expect("the in-memory native test tone must start");
        capture
            .start_on_output_device(
                &output_device,
                ingress,
                failure_tx,
                AudioCaptureFormat::pcm16_mono(16_000).unwrap(),
            )
            .expect("default output must open as a loopback input stream");
        assert_capture_failure_channel_empty(&mut failure_rx);

        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            assert_capture_failure_channel_empty(&mut failure_rx);
            match pcm_rx.try_recv() {
                Ok(()) => break,
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {
                    panic!("the native loopback PCM observation channel disconnected")
                }
            }
            assert!(
                !output_failed.load(Ordering::SeqCst),
                "the in-memory native test tone stopped unexpectedly"
            );
            assert!(
                Instant::now() < deadline,
                "WASAPI loopback did not deliver PCM for the in-memory native test tone"
            );
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        assert_capture_failure_channel_empty(&mut failure_rx);

        drop(tone_stream);
        capture.stop().await;
        pipeline.stop();
    }
}
