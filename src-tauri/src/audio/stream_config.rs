use cpal::{
    traits::DeviceTrait, BufferSize, SampleFormat, SampleRate, StreamConfig, SupportedBufferSize,
    SupportedStreamConfigRange,
};
use serde::Serialize;

use crate::{
    audio::{device::DeviceDirection, sample_format},
    error::AudioError,
};

type CandidateChoice<'a> = (
    (u32, u8, u16),
    &'a SupportedStreamConfigRange,
    &'a SupportedStreamConfigRange,
    u32,
);

#[derive(Clone)]
pub struct StreamSpec {
    pub config: StreamConfig,
    pub sample_format: SampleFormat,
    pub buffer_frames: u32,
}

pub struct NegotiatedStreamConfig {
    pub input: StreamSpec,
    pub output: StreamSpec,
    pub sample_rate: u32,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveStreamFormat {
    pub input_sample_rate: u32,
    pub processed_destination_sample_rate: Option<u32>,
    pub local_monitor_sample_rate: Option<u32>,
    pub input_channels: u16,
    pub processed_destination_channels: Option<u16>,
    pub local_monitor_channels: Option<u16>,
    pub input_sample_format: String,
    pub processed_destination_sample_format: Option<String>,
    pub local_monitor_sample_format: Option<String>,
    pub input_buffer_frames: u32,
    pub processed_destination_buffer_frames: Option<u32>,
    pub local_monitor_buffer_frames: Option<u32>,
    pub dsp_block_frames: u32,
}

pub fn negotiate(
    input: &cpal::Device,
    output: &cpal::Device,
    target_buffer_frames: u32,
) -> Result<NegotiatedStreamConfig, AudioError> {
    let input_name = input
        .name()
        .map_err(|error| AudioError::DeviceName(error.to_string()))?;
    let output_name = output
        .name()
        .map_err(|error| AudioError::DeviceName(error.to_string()))?;
    let input_configs = supported_configs(input, DeviceDirection::Input, &input_name)?;
    let output_configs = supported_configs(output, DeviceDirection::Output, &output_name)?;

    let mut best: Option<CandidateChoice<'_>> = None;

    for input_config in &input_configs {
        for output_config in &output_configs {
            let min_rate = input_config
                .min_sample_rate()
                .0
                .max(output_config.min_sample_rate().0);
            let max_rate = input_config
                .max_sample_rate()
                .0
                .min(output_config.max_sample_rate().0);
            let Some(sample_rate) = preferred_rate(min_rate, max_rate) else {
                continue;
            };
            let score = (
                rate_score(sample_rate),
                format_score(input_config.sample_format())
                    + format_score(output_config.sample_format()),
                channel_score(input_config.channels()) + channel_score(output_config.channels()),
            );
            if best.as_ref().is_none_or(|current| score < current.0) {
                best = Some((score, input_config, output_config, sample_rate));
            }
        }
    }

    let Some((_, input_range, output_range, sample_rate)) = best else {
        return Err(AudioError::NoCommonSampleRate {
            input: input_name,
            output: output_name,
        });
    };

    Ok(NegotiatedStreamConfig {
        input: stream_spec(input_range, sample_rate, target_buffer_frames),
        output: stream_spec(output_range, sample_rate, target_buffer_frames),
        sample_rate,
    })
}

pub fn output_spec_at_rate(
    output: &cpal::Device,
    sample_rate: u32,
    target_buffer_frames: u32,
) -> Result<StreamSpec, AudioError> {
    let output_name = output
        .name()
        .map_err(|error| AudioError::DeviceName(error.to_string()))?;
    let output_configs = supported_configs(output, DeviceDirection::Output, &output_name)?;
    let best = output_configs
        .iter()
        .filter(|config| {
            (config.min_sample_rate().0..=config.max_sample_rate().0).contains(&sample_rate)
        })
        .min_by_key(|config| {
            (
                format_score(config.sample_format()),
                channel_score(config.channels()),
            )
        });
    best.map(|range| stream_spec(range, sample_rate, target_buffer_frames))
        .ok_or(AudioError::OutputSampleRateUnavailable {
            output: output_name,
            sample_rate,
        })
}

/// Selects a standalone input format for bounded, non-realtime capture.
///
/// This intentionally does not participate in live input/output negotiation.
pub fn input_spec(
    input: &cpal::Device,
    target_buffer_frames: u32,
) -> Result<StreamSpec, AudioError> {
    let input_name = input
        .name()
        .map_err(|error| AudioError::DeviceName(error.to_string()))?;
    let input_configs = supported_configs(input, DeviceDirection::Input, &input_name)?;
    for sample_rate in [48_000, 44_100] {
        let best = input_configs
            .iter()
            .filter(|config| {
                (config.min_sample_rate().0..=config.max_sample_rate().0).contains(&sample_rate)
            })
            .min_by_key(|config| {
                (
                    format_score(config.sample_format()),
                    channel_score(config.channels()),
                )
            });
        if let Some(range) = best {
            return Ok(stream_spec(range, sample_rate, target_buffer_frames));
        }
    }
    Err(AudioError::SupportedFormats {
        direction: DeviceDirection::Input.label(),
        name: input_name,
        details: "Voice Lab requires 44.1 kHz or 48 kHz input support.".to_owned(),
    })
}

fn supported_configs(
    device: &cpal::Device,
    direction: DeviceDirection,
    name: &str,
) -> Result<Vec<SupportedStreamConfigRange>, AudioError> {
    let configs: Vec<_> = match direction {
        DeviceDirection::Input => device
            .supported_input_configs()
            .map_err(|error| AudioError::SupportedFormats {
                direction: direction.label(),
                name: name.to_owned(),
                details: error.to_string(),
            })?
            .collect(),
        DeviceDirection::Output => device
            .supported_output_configs()
            .map_err(|error| AudioError::SupportedFormats {
                direction: direction.label(),
                name: name.to_owned(),
                details: error.to_string(),
            })?
            .collect(),
    };
    Ok(configs
        .into_iter()
        .filter(|config| sample_format::supported(config.sample_format()))
        .collect())
}

pub(crate) fn stream_spec(
    range: &SupportedStreamConfigRange,
    sample_rate: u32,
    target_buffer_frames: u32,
) -> StreamSpec {
    let supported = (*range).with_sample_rate(SampleRate(sample_rate));
    let (buffer_size, buffer_frames) =
        choose_buffer_size(range.buffer_size(), target_buffer_frames);
    let mut config = supported.config();
    config.buffer_size = buffer_size;
    StreamSpec {
        config,
        sample_format: supported.sample_format(),
        buffer_frames,
    }
}

fn choose_buffer_size(
    supported: &SupportedBufferSize,
    target_buffer_frames: u32,
) -> (BufferSize, u32) {
    match supported {
        SupportedBufferSize::Range { min, max } => {
            let frames = target_buffer_frames.clamp(*min, *max);
            (BufferSize::Fixed(frames), frames)
        }
        SupportedBufferSize::Unknown => (BufferSize::Default, target_buffer_frames),
    }
}

fn preferred_rate(min_rate: u32, max_rate: u32) -> Option<u32> {
    if min_rate > max_rate {
        return None;
    }
    for preferred in [48_000, 44_100] {
        if (min_rate..=max_rate).contains(&preferred) {
            return Some(preferred);
        }
    }
    Some(48_000_u32.clamp(min_rate, max_rate))
}

fn rate_score(rate: u32) -> u32 {
    match rate {
        48_000 => 0,
        44_100 => 1,
        _ => 2 + rate.abs_diff(48_000),
    }
}

fn format_score(format: SampleFormat) -> u8 {
    match format {
        SampleFormat::F32 => 0,
        SampleFormat::I16 => 1,
        SampleFormat::U16 => 2,
        _ => u8::MAX / 2,
    }
}

fn channel_score(channels: u16) -> u16 {
    match channels {
        2 => 0,
        1 => 1,
        other => 2 + other,
    }
}

#[cfg(test)]
mod tests {
    use super::preferred_rate;

    #[test]
    fn prefers_48khz_when_both_ranges_allow_it() {
        assert_eq!(preferred_rate(44_100, 96_000), Some(48_000));
    }

    #[test]
    fn falls_back_to_44_1khz() {
        assert_eq!(preferred_rate(44_100, 44_100), Some(44_100));
    }

    #[test]
    fn rejects_non_overlapping_ranges() {
        assert_eq!(preferred_rate(48_000, 44_100), None);
    }
}
