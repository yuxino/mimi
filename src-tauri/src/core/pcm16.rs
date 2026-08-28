//! Float samples to signed 16-bit little-endian PCM.

pub enum PCM16Encoder {}

impl PCM16Encoder {
    /// Averages the channels per frame, clamps to [-1, 1], and quantizes to
    /// i16 little-endian bytes. Positive samples use `round(x * 32767)` and
    /// negative samples use `round(x * 32768)` to span the signed range.
    pub fn encode(channels: &[Vec<f32>]) -> Vec<u8> {
        if channels.is_empty() {
            return Vec::new();
        }
        let frame_count = channels.iter().map(Vec::len).min().unwrap_or(0);
        if frame_count == 0 {
            return Vec::new();
        }

        let mut data = Vec::with_capacity(frame_count * 2);
        let channel_count = channels.len() as f32;
        for frame in 0..frame_count {
            let mixed: f32 =
                channels.iter().map(|channel| channel[frame]).sum::<f32>() / channel_count;
            let sample = Self::quantize(mixed);
            data.extend_from_slice(&sample.to_le_bytes());
        }
        data
    }

    fn quantize(sample: f32) -> i16 {
        let clamped = sample.clamp(-1.0, 1.0);
        if clamped >= 0.0 {
            (clamped * f32::from(i16::MAX)).round() as i16
        } else {
            (clamped * 32_768.0).round() as i16
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn samples_to_i16(bytes: &[u8]) -> Vec<i16> {
        bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|sample| i16::from_le_bytes(*sample))
            .collect()
    }

    #[test]
    fn empty_input_produces_no_bytes() {
        assert!(PCM16Encoder::encode(&[]).is_empty());
        assert!(PCM16Encoder::encode(&[Vec::new()]).is_empty());
    }

    #[test]
    fn silence_is_zero() {
        let bytes = PCM16Encoder::encode(&[vec![0.0f32; 4]]);
        assert_eq!(samples_to_i16(&bytes), vec![0, 0, 0, 0]);
    }

    #[test]
    fn positive_full_scale_quantizes_to_i16_max() {
        let bytes = PCM16Encoder::encode(&[vec![1.0f32]]);
        assert_eq!(samples_to_i16(&bytes), vec![i16::MAX]);
    }

    #[test]
    fn negative_full_scale_quantizes_to_i16_min() {
        let bytes = PCM16Encoder::encode(&[vec![-1.0f32]]);
        assert_eq!(samples_to_i16(&bytes), vec![i16::MIN]);
    }

    #[test]
    fn overdriven_samples_are_clamped() {
        let bytes = PCM16Encoder::encode(&[vec![1.5f32, -1.5f32]]);
        assert_eq!(samples_to_i16(&bytes), vec![i16::MAX, i16::MIN]);
    }

    #[test]
    fn half_scale_rounds_half_away_from_zero() {
        // 0.5 * 32767 = 16383.5 → 16384 (round half away from zero).
        let bytes = PCM16Encoder::encode(&[vec![0.5f32]]);
        assert_eq!(samples_to_i16(&bytes), vec![16384]);
        // -0.5 * 32768 = -16384.
        let bytes = PCM16Encoder::encode(&[vec![-0.5f32]]);
        assert_eq!(samples_to_i16(&bytes), vec![-16384]);
    }

    #[test]
    fn multiple_channels_are_averaged() {
        let bytes = PCM16Encoder::encode(&[vec![1.0f32, -1.0f32], vec![0.0f32, 0.0f32]]);
        assert_eq!(samples_to_i16(&bytes), vec![16384, -16384]);
    }

    #[test]
    fn channel_lengths_use_the_minimum_frame_count() {
        let bytes = PCM16Encoder::encode(&[vec![1.0f32, 1.0f32], vec![1.0f32]]);
        assert_eq!(samples_to_i16(&bytes), vec![i16::MAX]);
    }

    #[test]
    fn output_is_little_endian() {
        let bytes = PCM16Encoder::encode(&[vec![1.0f32]]);
        assert_eq!(bytes, vec![0xFF, 0x7F]);
    }
}
