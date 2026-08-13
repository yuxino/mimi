//! Windows system-audio capture via WASAPI loopback (cpal): opening an input
//! stream on the default output device transparently enables loopback mode,
//! capturing the mix of everything playing. The mix format (typically 48 kHz
//! stereo f32) is resampled to 16 kHz mono and quantized to PCM16.

use crate::audio::SystemAudioCaptureError;
use crate::core::pcm16::PCM16Encoder;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rubato::audioadapter::Adapter as _;
use rubato::audioadapter_buffers::direct::SequentialSliceOfVecs;
use rubato::{FixedSync, Resampler};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

const TARGET_SAMPLE_RATE: f64 = 16_000.0;
const CHUNK_SIZE_IN: usize = 1024;

#[derive(Clone)]
pub struct WindowsSystemAudioCapture {
    stream: Mutex<Option<cpal::Stream>>,
    active: Arc<AtomicBool>,
    audio_tx: Mutex<Option<mpsc::UnboundedSender<Vec<u8>>>>,
    error_tx: Mutex<Option<mpsc::UnboundedSender<String>>>,
}

impl WindowsSystemAudioCapture {
    pub fn new() -> Self {
        Self {
            stream: Mutex::new(None),
            active: Arc::new(AtomicBool::new(false)),
            audio_tx: Mutex::new(None),
            error_tx: Mutex::new(None),
        }
    }

    pub async fn start(
        &self,
        audio_tx: mpsc::UnboundedSender<Vec<u8>>,
        error_tx: mpsc::UnboundedSender<String>,
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
        let sample_rate = input_config.sample_rate().0 as f64;

        let resampler = build_resampler(sample_rate, channel_count)?;
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
                        &input_config.into(),
                        move |data: &[f32], _info| {
                            if !active_for_cb.load(Ordering::SeqCst) {
                                return;
                            }
                            process_frames_f32(
                                data,
                                channel_count,
                                &resampler_for_cb,
                                &audio_tx_for_cb,
                            );
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
                        &input_config.into(),
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
                                channel_count,
                                &resampler_for_cb,
                                &audio_tx_for_cb,
                            );
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

fn build_resampler(
    sample_rate: f64,
    channel_count: usize,
) -> Result<rubato::Fft<f32>, SystemAudioCaptureError> {
    rubato::Fft::<f32>::new(
        sample_rate as usize,
        TARGET_SAMPLE_RATE as usize,
        CHUNK_SIZE_IN,
        channel_count,
        FixedSync::Input,
    )
    .map_err(|error| SystemAudioCaptureError::Other(error.to_string()))
}

/// Buffers interleaved frames, resamples in `CHUNK_SIZE_IN`-frame windows, and
/// emits mono PCM16.
fn process_frames_f32(
    data: &[f32],
    channel_count: usize,
    resampler: &Arc<Mutex<rubato::Fft<f32>>>,
    audio_tx: &mpsc::UnboundedSender<Vec<u8>>,
) {
    // Accumulate frames in the resampler's expected interleaved layout.
    let frames = data.len() / channel_count;
    let channels: Vec<Vec<f32>> = (0..channel_count)
        .map(|channel| {
            (0..frames)
                .map(|frame| data[frame * channel_count + channel])
                .collect()
        })
        .collect();

    let mut resampler = resampler.lock().unwrap();
    let input_frames_available = frames;
    let mut consumed = 0usize;
    // SincFixedIn needs exactly `CHUNK_SIZE_IN` frames per call; feed all
    // available complete chunks and drop the trailing partial chunk (it is
    // consumed by the ring buffer across callbacks instead).
    let _ = input_frames_available;
    while consumed + CHUNK_SIZE_IN <= frames {
        let chunk: Vec<Vec<f32>> = channels
            .iter()
            .map(|channel| channel[consumed..consumed + CHUNK_SIZE_IN].to_vec())
            .collect();
        let input = match SequentialSliceOfVecs::new(&chunk, channel_count, CHUNK_SIZE_IN) {
            Ok(input) => input,
            Err(_) => break,
        };
        match resampler.process(&input, None) {
            Ok(output) => {
                let data = output.take_data();
                let out_frames = data.len() / channel_count;
                let per_channel: Vec<Vec<f32>> = (0..channel_count)
                    .map(|channel| {
                        (0..out_frames)
                            .map(|frame| data[frame * channel_count + channel])
                            .collect()
                    })
                    .collect();
                let pcm = PCM16Encoder::encode(&per_channel);
                if !pcm.is_empty() {
                    let _ = audio_tx.send(pcm);
                }
            }
            Err(_) => break,
        }
        consumed += CHUNK_SIZE_IN;
    }
}
