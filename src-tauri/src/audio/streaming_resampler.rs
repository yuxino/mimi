//! Stateful interleaved-audio downmixing and resampling shared by the
//! Windows capture callback and platform-independent regression tests.

use crate::audio::SystemAudioCaptureError;
use crate::core::pcm16::PCM16Encoder;
use rubato::audioadapter_buffers::direct::SequentialSliceOfVecs;
use rubato::{FixedSync, Resampler};

const RESAMPLE_CHUNK_FRAMES: usize = 1024;

pub(crate) struct StreamingPcm16Resampler {
    channel_count: usize,
    input_sample_rate_hz: u32,
    target_sample_rate_hz: u32,
    pending_interleaved: Vec<f32>,
    pending_mono: Vec<f32>,
    resampler: Option<rubato::Fft<f32>>,
}

impl StreamingPcm16Resampler {
    pub(crate) fn new(
        input_sample_rate_hz: u32,
        target_sample_rate_hz: u32,
        channel_count: usize,
    ) -> Result<Self, SystemAudioCaptureError> {
        if channel_count == 0 || input_sample_rate_hz == 0 || target_sample_rate_hz == 0 {
            return Err(SystemAudioCaptureError::UnsupportedAudioFormat);
        }
        let resampler = if input_sample_rate_hz == target_sample_rate_hz {
            None
        } else {
            Some(
                rubato::Fft::<f32>::new(
                    input_sample_rate_hz as usize,
                    target_sample_rate_hz as usize,
                    RESAMPLE_CHUNK_FRAMES,
                    1,
                    FixedSync::Input,
                )
                .map_err(|_| SystemAudioCaptureError::AudioProcessingFailed)?,
            )
        };
        Ok(Self {
            channel_count,
            input_sample_rate_hz,
            target_sample_rate_hz,
            pending_interleaved: Vec::new(),
            pending_mono: Vec::new(),
            resampler,
        })
    }

    /// Accepts any callback size. Incomplete interleaved frames and incomplete
    /// resampler blocks remain buffered for the next callback.
    pub(crate) fn push_interleaved(
        &mut self,
        data: &[f32],
    ) -> Result<Vec<Vec<u8>>, SystemAudioCaptureError> {
        self.pending_interleaved.extend_from_slice(data);
        let complete_sample_count =
            self.pending_interleaved.len() / self.channel_count * self.channel_count;
        if complete_sample_count > 0 {
            let complete: Vec<f32> = self
                .pending_interleaved
                .drain(..complete_sample_count)
                .collect();
            self.pending_mono.extend(
                complete
                    .chunks_exact(self.channel_count)
                    .map(|frame| frame.iter().copied().sum::<f32>() / self.channel_count as f32),
            );
        }

        if self.input_sample_rate_hz == self.target_sample_rate_hz {
            if self.pending_mono.is_empty() {
                return Ok(Vec::new());
            }
            let mono = std::mem::take(&mut self.pending_mono);
            return Ok(vec![PCM16Encoder::encode(&[mono])]);
        }

        let Some(resampler) = self.resampler.as_mut() else {
            return Err(SystemAudioCaptureError::UnsupportedAudioFormat);
        };
        let mut buffers = Vec::new();
        while self.pending_mono.len() >= RESAMPLE_CHUNK_FRAMES {
            let chunk: Vec<f32> = self.pending_mono.drain(..RESAMPLE_CHUNK_FRAMES).collect();
            let channels = [chunk];
            let input = SequentialSliceOfVecs::new(&channels, 1, RESAMPLE_CHUNK_FRAMES)
                .map_err(|_| SystemAudioCaptureError::AudioProcessingFailed)?;
            let output = resampler
                .process(&input, None)
                .map_err(|_| SystemAudioCaptureError::AudioProcessingFailed)?
                .take_data();
            let pcm = PCM16Encoder::encode(&[output]);
            if !pcm.is_empty() {
                buffers.push(pcm);
            }
        }
        Ok(buffers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stereo_sine(start_frame: usize, frame_count: usize) -> Vec<f32> {
        (start_frame..start_frame + frame_count)
            .flat_map(|frame| {
                let sample = (frame as f64 * 2.0 * std::f64::consts::PI * 1_000.0 / 48_000.0).sin()
                    as f32
                    * 0.5;
                [sample, sample]
            })
            .collect()
    }

    #[test]
    fn four_small_callbacks_fill_one_resampler_block() {
        let mut resampler = StreamingPcm16Resampler::new(48_000, 24_000, 2).unwrap();

        for callback in 0..3 {
            let output = resampler
                .push_interleaved(&stereo_sine(callback * 256, 256))
                .unwrap();
            assert!(output.is_empty());
        }
        let output = resampler
            .push_interleaved(&stereo_sine(3 * 256, 256))
            .unwrap();

        assert_eq!(output.len(), 1);
        assert!(!output[0].is_empty());
    }

    #[test]
    fn resampling_48k_to_24k_preserves_audible_signal() {
        let mut resampler = StreamingPcm16Resampler::new(48_000, 24_000, 2).unwrap();
        let output = resampler
            .push_interleaved(&stereo_sine(0, RESAMPLE_CHUNK_FRAMES))
            .unwrap();
        let peak = output
            .iter()
            .flatten()
            .copied()
            .collect::<Vec<_>>()
            .chunks_exact(2)
            .map(|sample| i32::from(i16::from_le_bytes([sample[0], sample[1]])).abs())
            .max()
            .unwrap_or(0);

        assert!(peak > 100, "resampled PCM unexpectedly silent: peak={peak}");
    }

    #[test]
    fn incomplete_interleaved_frame_is_kept_for_the_next_callback() {
        let mut resampler = StreamingPcm16Resampler::new(24_000, 24_000, 2).unwrap();
        assert!(resampler.push_interleaved(&[0.5]).unwrap().is_empty());
        let output = resampler.push_interleaved(&[0.5]).unwrap();
        assert_eq!(output.len(), 1);
        assert_eq!(output[0].len(), 2);
    }
}
