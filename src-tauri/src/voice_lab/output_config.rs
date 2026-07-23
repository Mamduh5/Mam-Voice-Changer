use cpal::{traits::DeviceTrait, SampleFormat, SupportedStreamConfigRange};

use crate::audio::{
    sample_format,
    stream_config::{self, StreamSpec},
};

#[derive(Clone, Copy, Debug)]
pub struct PreviewOutputCapability {
    pub minimum_sample_rate: u32,
    pub maximum_sample_rate: u32,
    pub channels: u16,
    pub sample_format: SampleFormat,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PreviewOutputChoice {
    pub capability_index: usize,
    pub sample_rate: u32,
}

pub struct NegotiatedPreviewOutput {
    pub spec: StreamSpec,
    pub sample_format_label: String,
}

pub fn negotiate_preview_output(
    device: &cpal::Device,
    target_buffer_frames: u32,
) -> Result<NegotiatedPreviewOutput, String> {
    let output_name = device
        .name()
        .map_err(|error| format!("Voice Lab output device unavailable: {error}"))?;
    let ranges: Vec<SupportedStreamConfigRange> = device
        .supported_output_configs()
        .map_err(|error| {
            format!(
                "Voice Lab could not inspect supported output configurations for '{output_name}': {error}"
            )
        })?
        .filter(|range| sample_format::supported(range.sample_format()))
        .collect();
    let capabilities = ranges
        .iter()
        .map(|range| PreviewOutputCapability {
            minimum_sample_rate: range.min_sample_rate().0,
            maximum_sample_rate: range.max_sample_rate().0,
            channels: range.channels(),
            sample_format: range.sample_format(),
        })
        .collect::<Vec<_>>();
    let default_rate = device
        .default_output_config()
        .ok()
        .map(|configuration| configuration.sample_rate().0);
    let choice = select_preview_output(&capabilities, default_rate).ok_or_else(|| {
        format!("Voice Lab found no supported output configuration for '{output_name}'.")
    })?;
    let range = ranges.get(choice.capability_index).ok_or_else(|| {
        "Voice Lab preview output configuration selection became invalid.".to_owned()
    })?;
    Ok(NegotiatedPreviewOutput {
        spec: stream_config::stream_spec(range, choice.sample_rate, target_buffer_frames),
        sample_format_label: sample_format_label(range.sample_format()).to_owned(),
    })
}

pub fn select_preview_output(
    capabilities: &[PreviewOutputCapability],
    default_rate: Option<u32>,
) -> Option<PreviewOutputChoice> {
    if let Some(choice) = best_at_rate(capabilities, 48_000) {
        return Some(choice);
    }
    if let Some(default_rate) = default_rate {
        if let Some(choice) = best_at_rate(capabilities, default_rate) {
            return Some(choice);
        }
    }
    capabilities
        .iter()
        .enumerate()
        .filter(|(_, capability)| {
            capability.minimum_sample_rate > 0
                && capability.minimum_sample_rate <= capability.maximum_sample_rate
                && capability.channels > 0
                && sample_format::supported(capability.sample_format)
        })
        .map(|(capability_index, capability)| {
            let sample_rate = 48_000_u32.clamp(
                capability.minimum_sample_rate,
                capability.maximum_sample_rate,
            );
            (
                (
                    sample_rate.abs_diff(48_000),
                    format_score(capability.sample_format),
                    channel_score(capability.channels),
                    sample_rate,
                ),
                PreviewOutputChoice {
                    capability_index,
                    sample_rate,
                },
            )
        })
        .min_by_key(|(score, _)| *score)
        .map(|(_, choice)| choice)
}

fn best_at_rate(
    capabilities: &[PreviewOutputCapability],
    sample_rate: u32,
) -> Option<PreviewOutputChoice> {
    capabilities
        .iter()
        .enumerate()
        .filter(|(_, capability)| {
            (capability.minimum_sample_rate..=capability.maximum_sample_rate).contains(&sample_rate)
                && capability.channels > 0
                && sample_format::supported(capability.sample_format)
        })
        .min_by_key(|(_, capability)| {
            (
                format_score(capability.sample_format),
                channel_score(capability.channels),
            )
        })
        .map(|(capability_index, _)| PreviewOutputChoice {
            capability_index,
            sample_rate,
        })
}

fn format_score(format: SampleFormat) -> u8 {
    match format {
        SampleFormat::F32 => 0,
        SampleFormat::I16 => 1,
        SampleFormat::U16 => 2,
        _ => u8::MAX,
    }
}

fn channel_score(channels: u16) -> u16 {
    match channels {
        2 => 0,
        1 => 1,
        other => 2 + other,
    }
}

pub fn sample_format_label(format: SampleFormat) -> &'static str {
    match format {
        SampleFormat::F32 => "f32",
        SampleFormat::I16 => "i16",
        SampleFormat::U16 => "u16",
        _ => "unsupported",
    }
}

#[cfg(test)]
mod tests {
    use cpal::SampleFormat;

    use super::{select_preview_output, PreviewOutputCapability};

    fn capability(
        minimum_sample_rate: u32,
        maximum_sample_rate: u32,
        channels: u16,
        sample_format: SampleFormat,
    ) -> PreviewOutputCapability {
        PreviewOutputCapability {
            minimum_sample_rate,
            maximum_sample_rate,
            channels,
            sample_format,
        }
    }

    #[test]
    fn prefers_48_khz_then_format_and_stereo() {
        let choice = select_preview_output(
            &[
                capability(44_100, 48_000, 1, SampleFormat::I16),
                capability(48_000, 48_000, 2, SampleFormat::F32),
            ],
            Some(44_100),
        )
        .unwrap();
        assert_eq!(choice.sample_rate, 48_000);
        assert_eq!(choice.capability_index, 1);
    }

    #[test]
    fn uses_device_default_when_48_khz_is_unavailable() {
        let choice = select_preview_output(
            &[
                capability(44_100, 44_100, 2, SampleFormat::F32),
                capability(32_000, 32_000, 2, SampleFormat::F32),
            ],
            Some(44_100),
        )
        .unwrap();
        assert_eq!(choice.sample_rate, 44_100);
    }

    #[test]
    fn chooses_another_valid_rate_and_rejects_unsupported_capabilities() {
        let choice =
            select_preview_output(&[capability(96_000, 192_000, 2, SampleFormat::I16)], None)
                .unwrap();
        assert_eq!(choice.sample_rate, 96_000);
        assert!(
            select_preview_output(&[capability(48_000, 44_100, 2, SampleFormat::F32)], None)
                .is_none()
        );
        assert!(
            select_preview_output(&[capability(48_000, 48_000, 0, SampleFormat::F32)], None)
                .is_none()
        );
        assert!(select_preview_output(&[], Some(48_000)).is_none());
    }
}
