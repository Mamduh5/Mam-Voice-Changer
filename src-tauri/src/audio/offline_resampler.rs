use std::fmt;

/// Errors produced by the bounded, offline-only linear audio-rate converter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OfflineResampleError {
    InvalidRate,
    InvalidChannels,
    IncompleteFrame,
    NonFiniteInput,
    OutputTooLarge {
        requested_frames: usize,
        maximum_frames: usize,
    },
}

impl fmt::Display for OfflineResampleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRate => formatter.write_str("sample rates must be greater than zero"),
            Self::InvalidChannels => formatter.write_str("channel count must be greater than zero"),
            Self::IncompleteFrame => {
                formatter.write_str("audio samples must contain complete interleaved frames")
            }
            Self::NonFiniteInput => formatter.write_str("audio contains a non-finite sample"),
            Self::OutputTooLarge {
                requested_frames,
                maximum_frames,
            } => write!(
                formatter,
                "resampled audio would contain {requested_frames} frames; the limit is {maximum_frames}"
            ),
        }
    }
}

impl std::error::Error for OfflineResampleError {}

/// Deterministic, bounded linear resampling for finite interleaved `f32` audio.
///
/// This converter is deliberately offline-only and replaceable. It is suitable
/// for short previews and canonical Dataset conversion, not mastering-quality
/// sample-rate conversion. It must never be called from a CPAL callback.
pub fn resample_linear_offline(
    samples: &[f32],
    channels: usize,
    source_rate: u32,
    target_rate: u32,
    maximum_output_frames: usize,
) -> Result<Vec<f32>, OfflineResampleError> {
    if source_rate == 0 || target_rate == 0 {
        return Err(OfflineResampleError::InvalidRate);
    }
    if channels == 0 {
        return Err(OfflineResampleError::InvalidChannels);
    }
    if !samples.len().is_multiple_of(channels) {
        return Err(OfflineResampleError::IncompleteFrame);
    }
    if samples.iter().any(|sample| !sample.is_finite()) {
        return Err(OfflineResampleError::NonFiniteInput);
    }

    let source_frames = samples.len() / channels;
    if source_frames == 0 {
        return Ok(Vec::new());
    }
    let numerator = (source_frames as u128).saturating_mul(u128::from(target_rate));
    let output_frames_u128 = numerator / u128::from(source_rate);
    let output_frames =
        usize::try_from(output_frames_u128).map_err(|_| OfflineResampleError::OutputTooLarge {
            requested_frames: usize::MAX,
            maximum_frames: maximum_output_frames,
        })?;
    if output_frames > maximum_output_frames {
        return Err(OfflineResampleError::OutputTooLarge {
            requested_frames: output_frames,
            maximum_frames: maximum_output_frames,
        });
    }

    let output_samples =
        output_frames
            .checked_mul(channels)
            .ok_or(OfflineResampleError::OutputTooLarge {
                requested_frames: output_frames,
                maximum_frames: maximum_output_frames,
            })?;
    let mut output = Vec::with_capacity(output_samples);
    if source_rate == target_rate {
        output.extend(samples.iter().map(|sample| sample.clamp(-1.0, 1.0)));
        return Ok(output);
    }

    for output_frame in 0..output_frames {
        let source_position = output_frame as f64 * f64::from(source_rate) / f64::from(target_rate);
        let left_frame = (source_position.floor() as usize).min(source_frames - 1);
        let right_frame = (left_frame + 1).min(source_frames - 1);
        let fraction = (source_position - left_frame as f64) as f32;
        for channel in 0..channels {
            let left = samples[left_frame * channels + channel];
            let right = samples[right_frame * channels + channel];
            output.push((left * (1.0 - fraction) + right * fraction).clamp(-1.0, 1.0));
        }
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::{resample_linear_offline, OfflineResampleError};

    #[test]
    fn converts_44_1_to_48_and_48_to_44_1_with_duration_tolerance() {
        let up = resample_linear_offline(&vec![0.25; 44_100], 1, 44_100, 48_000, 48_001)
            .expect("44.1 to 48 kHz conversion");
        assert_eq!(up.len(), 48_000);
        let down = resample_linear_offline(&vec![0.25; 48_000], 1, 48_000, 44_100, 44_101)
            .expect("48 to 44.1 kHz conversion");
        assert_eq!(down.len(), 44_100);
        let source_seconds = 44_100_f64 / 44_100_f64;
        let output_seconds = up.len() as f64 / 48_000_f64;
        assert!((source_seconds - output_seconds).abs() <= 1.0 / 48_000.0);
    }

    #[test]
    fn same_rate_is_finite_normalized_and_channel_aligned() {
        let output =
            resample_linear_offline(&[-2.0, 0.5, 2.0, -0.5], 2, 48_000, 48_000, 2).unwrap();
        assert_eq!(output, vec![-1.0, 0.5, 1.0, -0.5]);
        assert!(output.iter().all(|sample| sample.is_finite()));
    }

    #[test]
    fn rejects_non_finite_incomplete_and_unbounded_audio() {
        assert_eq!(
            resample_linear_offline(&[f32::NAN], 1, 48_000, 48_000, 1),
            Err(OfflineResampleError::NonFiniteInput)
        );
        assert_eq!(
            resample_linear_offline(&[0.0], 2, 48_000, 48_000, 1),
            Err(OfflineResampleError::IncompleteFrame)
        );
        assert!(matches!(
            resample_linear_offline(&[0.0; 48_000], 1, 48_000, 96_000, 48_000),
            Err(OfflineResampleError::OutputTooLarge { .. })
        ));
    }
}
