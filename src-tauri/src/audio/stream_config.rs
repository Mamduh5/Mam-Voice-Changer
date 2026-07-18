use cpal::{
    traits::DeviceTrait, BufferSize, SampleFormat, SampleRate, StreamConfig, SupportedBufferSize,
    SupportedStreamConfigRange,
};
use serde::Serialize;

use crate::{
    audio::{device::DeviceDirection, sample_format},
    error::AudioError,
};

const TARGET_BUFFER_FRAMES: u32 = 256;

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
    pub sample_rate: u32,
    pub input_channels: u16,
    pub output_channels: u16,
    pub input_sample_format: String,
    pub output_sample_format: String,
    pub input_buffer_frames: u32,
    pub output_buffer_frames: u32,
}

impl NegotiatedStreamConfig {
    pub fn active_format(&self) -> ActiveStreamFormat {
        ActiveStreamFormat {
            sample_rate: self.sample_rate,
            input_channels: self.input.config.channels,
            output_channels: self.output.config.channels,
            input_sample_format: format!("{:?}", self.input.sample_format),
            output_sample_format: format!("{:?}", self.output.sample_format),
            input_buffer_frames: self.input.buffer_frames,
            output_buffer_frames: self.output.buffer_frames,
        }
    }

    pub fn estimated_latency_ms(&self, prefill_frames: u32) -> f32 {
        let total_frames = self.input.buffer_frames + self.output.buffer_frames + prefill_frames;
        total_frames as f32 * 1_000.0 / self.sample_rate as f32
    }
}

pub fn negotiate(
    input: &cpal::Device,
    output: &cpal::Device,
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
        input: stream_spec(input_range, sample_rate),
        output: stream_spec(output_range, sample_rate),
        sample_rate,
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

fn stream_spec(range: &SupportedStreamConfigRange, sample_rate: u32) -> StreamSpec {
    let supported = (*range).with_sample_rate(SampleRate(sample_rate));
    let (buffer_size, buffer_frames) = choose_buffer_size(range.buffer_size());
    let mut config = supported.config();
    config.buffer_size = buffer_size;
    StreamSpec {
        config,
        sample_format: supported.sample_format(),
        buffer_frames,
    }
}

fn choose_buffer_size(supported: &SupportedBufferSize) -> (BufferSize, u32) {
    match supported {
        SupportedBufferSize::Range { min, max } => {
            let frames = TARGET_BUFFER_FRAMES.clamp(*min, *max);
            (BufferSize::Fixed(frames), frames)
        }
        SupportedBufferSize::Unknown => (BufferSize::Default, TARGET_BUFFER_FRAMES),
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
