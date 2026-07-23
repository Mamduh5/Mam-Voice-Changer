use std::sync::atomic::{AtomicU64, Ordering};

use serde::Serialize;

pub const MAX_CLIP_SECONDS: usize = 15;
pub const SUPPORTED_SAMPLE_RATES: [u32; 2] = [44_100, 48_000];
const WAVEFORM_BUCKETS: usize = 96;
static CLIP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug)]
pub struct AudioClip {
    pub id: String,
    pub source_name: String,
    pub sample_rate: u32,
    pub channels: usize,
    pub samples: Vec<f32>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipSummary {
    pub source_name: String,
    pub duration_ms: u64,
    pub sample_rate: u32,
    pub channels: usize,
    pub frames: usize,
    pub peak: f32,
    pub waveform: Vec<f32>,
}

impl AudioClip {
    pub fn new(
        source_name: impl Into<String>,
        sample_rate: u32,
        channels: usize,
        samples: Vec<f32>,
    ) -> Result<Self, String> {
        if !SUPPORTED_SAMPLE_RATES.contains(&sample_rate) {
            return Err("Voice Lab supports only 44.1 kHz and 48 kHz audio.".to_owned());
        }
        if !(1..=2).contains(&channels) {
            return Err("Voice Lab supports only mono or stereo audio.".to_owned());
        }
        if samples.is_empty() || !samples.len().is_multiple_of(channels) {
            return Err("The clip must contain complete audio frames.".to_owned());
        }
        let maximum_samples = sample_rate as usize * channels * MAX_CLIP_SECONDS;
        if samples.len() > maximum_samples {
            return Err(format!(
                "Voice Lab clips cannot exceed {MAX_CLIP_SECONDS} seconds."
            ));
        }
        if samples.iter().any(|sample| !sample.is_finite()) {
            return Err("The clip contains invalid audio samples.".to_owned());
        }
        Ok(Self {
            id: format!(
                "lab-clip-{:016x}",
                CLIP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
            ),
            source_name: source_name.into(),
            sample_rate,
            channels,
            samples: samples
                .into_iter()
                .map(|sample| sample.clamp(-1.0, 1.0))
                .collect(),
        })
    }

    pub fn frames(&self) -> usize {
        self.samples.len() / self.channels
    }

    pub fn summary(&self) -> ClipSummary {
        let frames = self.frames();
        let bucket_frames = frames.div_ceil(WAVEFORM_BUCKETS).max(1);
        let waveform = self
            .samples
            .chunks(bucket_frames * self.channels)
            .take(WAVEFORM_BUCKETS)
            .map(|bucket| {
                bucket
                    .iter()
                    .fold(0.0_f32, |peak, sample| peak.max(sample.abs()))
            })
            .collect();
        ClipSummary {
            source_name: self.source_name.clone(),
            duration_ms: (frames as u64 * 1_000) / u64::from(self.sample_rate),
            sample_rate: self.sample_rate,
            channels: self.channels,
            frames,
            peak: self
                .samples
                .iter()
                .fold(0.0_f32, |peak, sample| peak.max(sample.abs())),
            waveform,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AudioClip, MAX_CLIP_SECONDS};

    #[test]
    fn validates_bounded_supported_clips() {
        assert!(AudioClip::new("mono", 48_000, 1, vec![0.0; 48_000]).is_ok());
        assert!(AudioClip::new("rate", 32_000, 1, vec![0.0]).is_err());
        assert!(AudioClip::new("channels", 48_000, 3, vec![0.0; 3]).is_err());
        assert!(AudioClip::new("frames", 48_000, 2, vec![0.0; 3]).is_err());
        assert!(
            AudioClip::new("long", 48_000, 1, vec![0.0; 48_000 * MAX_CLIP_SECONDS + 1]).is_err()
        );
    }

    #[test]
    fn summarizes_without_exposing_audio_samples() {
        let clip = AudioClip::new("sample", 48_000, 1, vec![0.0, -0.75, 0.5]).unwrap();
        let summary = clip.summary();
        assert_eq!(summary.source_name, "sample");
        assert_eq!(summary.frames, 3);
        assert_eq!(summary.peak, 0.75);
        assert!(!summary.waveform.is_empty());
    }
}
