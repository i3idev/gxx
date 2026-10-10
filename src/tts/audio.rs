//! Audio WAV generation and processing.

use hound::{WavSpec, WavWriter};
use std::io::Cursor;

use crate::error::GxxError;

/// Write audio samples to a WAV file in memory.
/// Returns the WAV bytes.
pub fn write_wav(samples: &[f32], sample_rate: u32) -> Result<Vec<u8>, GxxError> {
    let mut buffer = Cursor::new(Vec::new());

    let spec = WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = WavWriter::new(&mut buffer, spec)?;

    for sample in samples {
        // Convert f32 (-1.0 to 1.0) to i16
        let sample_i16 = (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
        writer.write_sample(sample_i16)?;
    }

    writer.finalize()?;
    Ok(buffer.into_inner())
}

/// Apply time-stretching (speed change) while preserving pitch.
/// Uses rubato's Fft resampler for high-quality pitch-preserving time stretching.
/// speed: 1.0 = normal, 0.7 = slow, 0.5 = very slow
pub fn time_stretch(samples: &[f32], speed: f32, sample_rate: u32) -> Result<Vec<f32>, GxxError> {
    if speed <= 0.0 || speed > 2.0 {
        return Err(GxxError::msg(format!(
            "Invalid playback speed: {} (expected 0 < speed <= 2)",
            speed
        )));
    }

    if samples.is_empty() || (speed - 1.0).abs() < f32::EPSILON {
        return Ok(samples.to_vec());
    }

    let sr = sample_rate as usize;

    // WSOLA parameters.
    //
    // 40 ms analysis frame:
    //   large enough for speech waveform matching.
    //
    // 10 ms synthesis hop:
    //   gives smooth overlap between frames.
    //
    // 8 ms search:
    //   only searches around the expected analysis position.
    let frame_len = ((sr as f32) * 0.040).round() as usize;
    let synth_hop = ((sr as f32) * 0.010).round() as usize;
    let search_radius = ((sr as f32) * 0.008).round() as usize;

    if frame_len == 0 || synth_hop == 0 || samples.len() < frame_len {
        return Ok(samples.to_vec());
    }

    // The analysis position moves according to the requested speed.
    //
    // 1.0 = normal
    // 0.7 = 70% playback speed
    // 0.5 = 50% playback speed
    let analysis_hop = ((synth_hop as f32) * speed).round().max(1.0) as usize;

    let overlap = frame_len - synth_hop;

    // Hann window.
    let window: Vec<f32> = (0..frame_len)
        .map(|i| 0.5 - 0.5 * (2.0 * std::f32::consts::PI * i as f32 / frame_len as f32).cos())
        .collect();

    let estimated_len = ((samples.len() as f32) / speed).ceil() as usize;

    let mut output = vec![0.0f32; estimated_len + frame_len];
    let mut weights = vec![0.0f32; estimated_len + frame_len];

    // First frame.
    for i in 0..frame_len {
        let w = window[i];

        output[i] += samples[i] * w;
        weights[i] += w;
    }

    let mut previous_frame = samples[..frame_len].to_vec();

    let mut analysis_pos = 0usize;
    let mut output_pos = synth_hop;

    while output_pos + frame_len <= output.len() {
        // IMPORTANT:
        //
        // The expected analysis position always advances from the
        // previous expected position. The correlation search must
        // NOT become the new timing clock.
        let expected = analysis_pos.saturating_add(analysis_hop);

        if expected + frame_len >= samples.len() {
            break;
        }

        let min_pos = expected.saturating_sub(search_radius);
        let max_pos = expected
            .saturating_add(search_radius)
            .min(samples.len() - frame_len);

        // Find the candidate that best matches the tail of the
        // previous frame.
        let mut best_pos = expected;
        let mut best_score = f32::NEG_INFINITY;

        for candidate in min_pos..=max_pos {
            let mut dot = 0.0f32;
            let mut aa = 0.0f32;
            let mut bb = 0.0f32;

            for j in 0..overlap {
                let a = previous_frame[synth_hop + j];
                let b = samples[candidate + j];

                dot += a * b;
                aa += a * a;
                bb += b * b;
            }

            let denom = (aa * bb).sqrt();

            if denom > 1.0e-9 {
                let score = dot / denom;

                if score > best_score {
                    best_score = score;
                    best_pos = candidate;
                }
            }
        }

        // Overlap-add.
        for i in 0..frame_len {
            let dst = output_pos + i;

            if dst >= output.len() {
                break;
            }

            let w = window[i];

            output[dst] += samples[best_pos + i] * w;
            weights[dst] += w;
        }

        previous_frame.copy_from_slice(&samples[best_pos..best_pos + frame_len]);

        // Timing clock advances by the expected amount,
        // not by the correlation correction.
        analysis_pos += analysis_hop;

        output_pos += synth_hop;
    }

    // Normalize overlap-add energy.
    for i in 0..output.len() {
        if weights[i] > 1.0e-6 {
            output[i] /= weights[i];
        }
    }

    output.truncate(estimated_len.min(output.len()));

    Ok(output)
}

pub fn apply_volume(samples: &[f32], volume: f32) -> Result<Vec<f32>, GxxError> {
    if volume <= 0.0 || volume > 2.0 {
        return Err(GxxError::msg(format!("Invalid volume factor: {}", volume)));
    }

    if (volume - 1.0).abs() < f32::EPSILON {
        return Ok(samples.to_vec());
    }

    Ok(samples.iter().map(|s| s * volume).collect())
}

/// Process audio with speed and volume adjustments.
/// Returns processed audio samples.
pub fn process_audio(
    samples: &[f32],
    speed: f32,
    volume: f32,
    sample_rate: u32,
) -> Result<Vec<f32>, GxxError> {
    let mut result = samples.to_vec();

    // Apply time-stretching first (pitch-preserving)
    if (speed - 1.0).abs() > f32::EPSILON {
        result = time_stretch(&result, speed, sample_rate)?;
    }

    // Apply volume
    if (volume - 1.0).abs() > f32::EPSILON {
        result = apply_volume(&result, volume)?;
    }

    eprintln!(
        "TTS: audio processed: {:.3}s -> {:.3}s (speed={:.2}, volume={:.2})",
        samples.len() as f32 / sample_rate as f32,
        result.len() as f32 / sample_rate as f32,
        speed,
        volume
    );

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_volume_unity() {
        let samples = vec![0.5f32, -0.5, 1.0, -1.0];
        let result = apply_volume(&samples, 1.0).unwrap();
        assert_eq!(result, samples);
    }

    #[test]
    fn test_apply_volume_half() {
        let samples = vec![1.0f32, -1.0, 0.5];
        let result = apply_volume(&samples, 0.5).unwrap();
        assert!((result[0] - 0.5).abs() < 0.001);
        assert!((result[1] - (-0.5)).abs() < 0.001);
        assert!((result[2] - 0.25).abs() < 0.001);
    }

    #[test]
    fn test_apply_volume_invalid_zero() {
        let samples = vec![1.0f32];
        assert!(apply_volume(&samples, 0.0).is_err());
    }

    #[test]
    fn test_apply_volume_invalid_negative() {
        let samples = vec![1.0f32];
        assert!(apply_volume(&samples, -1.0).is_err());
    }

    #[test]
    fn test_apply_volume_invalid_high() {
        let samples = vec![1.0f32];
        assert!(apply_volume(&samples, 2.1).is_err());
    }

    #[test]
    fn test_write_wav_mono() {
        let samples = vec![0.0f32, 0.5, -0.5, 1.0, -1.0];
        let wav = write_wav(&samples, 22050).unwrap();
        assert!(!wav.is_empty());
        // WAV header is at least 44 bytes
        assert!(wav.len() > 44);
        // Check RIFF header
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
    }

    #[test]
    fn test_time_stretch_unity() {
        let samples = vec![0.0f32; 1000];
        let result = time_stretch(&samples, 1.0, 22050).unwrap();
        assert_eq!(result.len(), 1000);
    }

    #[test]
    fn test_time_stretch_half_speed() {
        let samples = vec![0.0f32; 1000];
        let result = time_stretch(&samples, 0.5, 22050).unwrap();
        // At half speed, output should be approximately double length
        assert!(result.len() > 1500);
    }

    #[test]
    fn test_time_stretch_invalid_speed() {
        let samples = vec![0.0f32; 1000];
        assert!(time_stretch(&samples, 0.0, 22050).is_err());
        assert!(time_stretch(&samples, 2.1, 22050).is_err());
    }

    #[test]
    fn test_time_stretch_empty() {
        let samples: Vec<f32> = vec![];
        let result = time_stretch(&samples, 1.0, 22050).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_process_audio_unity() {
        let samples = vec![0.5f32; 1000];
        let result = process_audio(&samples, 1.0, 1.0, 22050).unwrap();
        assert_eq!(result.len(), 1000);
        assert!((result[0] - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_process_audio_volume() {
        let samples = vec![1.0f32; 1000];
        let result = process_audio(&samples, 1.0, 0.5, 22050).unwrap();
        assert!((result[0] - 0.5).abs() < 0.001);
    }
}
