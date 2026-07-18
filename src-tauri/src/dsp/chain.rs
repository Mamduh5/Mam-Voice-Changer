use serde::{Deserialize, Serialize};

use super::{
    dry_wet::DryWetMixer, gain::Gain, high_pass::HighPass, limiter::SoftLimiter,
    pitch::PitchShifter, processor::AudioProcessor, smoothing::SmoothedValue,
};

pub const MIN_GAIN_DB: f32 = -24.0;
pub const MAX_GAIN_DB: f32 = 24.0;
pub const MIN_PITCH_SEMITONES: f32 = -12.0;
pub const MAX_PITCH_SEMITONES: f32 = 12.0;
const TRANSITION_RAMP_MS: f32 = 10.0;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DspParameters {
    pub pitch_semitones: f32,
    pub dry_wet: f32,
    pub input_gain_db: f32,
    pub output_gain_db: f32,
    pub limiter_enabled: bool,
    pub bypass: bool,
    pub muted: bool,
}

impl Default for DspParameters {
    fn default() -> Self {
        Self {
            pitch_semitones: 0.0,
            dry_wet: 1.0,
            input_gain_db: 0.0,
            output_gain_db: 0.0,
            limiter_enabled: true,
            bypass: false,
            muted: false,
        }
    }
}

impl DspParameters {
    pub fn validate(self) -> Result<Self, String> {
        validate_range(
            "Pitch",
            self.pitch_semitones,
            MIN_PITCH_SEMITONES,
            MAX_PITCH_SEMITONES,
            "semitones",
        )?;
        validate_range("Dry/wet", self.dry_wet, 0.0, 1.0, "")?;
        validate_range(
            "Input gain",
            self.input_gain_db,
            MIN_GAIN_DB,
            MAX_GAIN_DB,
            "dB",
        )?;
        validate_range(
            "Output gain",
            self.output_gain_db,
            MIN_GAIN_DB,
            MAX_GAIN_DB,
            "dB",
        )?;
        Ok(self)
    }
}

fn validate_range(label: &str, value: f32, min: f32, max: f32, unit: &str) -> Result<(), String> {
    if value.is_finite() && (min..=max).contains(&value) {
        return Ok(());
    }
    let suffix = if unit.is_empty() {
        String::new()
    } else {
        format!(" {unit}")
    };
    Err(format!(
        "{label} must be a finite value between {min}{suffix} and {max}{suffix}."
    ))
}

pub struct DspChain {
    parameters: DspParameters,
    channels: usize,
    input_gain: Gain,
    high_pass: HighPass,
    pitch: PitchShifter,
    dry_wet: DryWetMixer,
    bypass_mix: SmoothedValue,
    mute_gain: SmoothedValue,
    output_gain: Gain,
    limiter: SoftLimiter,
    dry_scratch: Vec<f32>,
    delayed_dry_scratch: Vec<f32>,
}

impl Default for DspChain {
    fn default() -> Self {
        let parameters = DspParameters::default();
        let pitch = PitchShifter::default();
        Self {
            parameters,
            channels: 1,
            input_gain: Gain::new(parameters.input_gain_db),
            high_pass: HighPass::default(),
            dry_wet: DryWetMixer::new(parameters.dry_wet, pitch.latency_frames()),
            pitch,
            bypass_mix: SmoothedValue::new(0.0),
            mute_gain: SmoothedValue::new(1.0),
            output_gain: Gain::new(parameters.output_gain_db),
            limiter: SoftLimiter,
            dry_scratch: Vec::new(),
            delayed_dry_scratch: Vec::new(),
        }
    }
}

impl DspChain {
    pub fn set_parameters(&mut self, parameters: DspParameters) {
        self.parameters = parameters;
        self.input_gain.set_gain_db(parameters.input_gain_db);
        self.pitch.set_pitch_semitones(parameters.pitch_semitones);
        self.dry_wet.set_mix(parameters.dry_wet);
        self.bypass_mix
            .set_target(if parameters.bypass { 1.0 } else { 0.0 });
        self.mute_gain
            .set_target(if parameters.muted { 0.0 } else { 1.0 });
        self.output_gain.set_gain_db(parameters.output_gain_db);
    }

    pub fn latency_frames(&self) -> usize {
        self.dry_wet.latency_frames()
    }
}

impl AudioProcessor for DspChain {
    fn prepare(&mut self, sample_rate: u32, channels: usize, block_size: usize) {
        self.channels = channels.max(1);
        self.input_gain
            .prepare(sample_rate, self.channels, block_size);
        self.high_pass
            .prepare(sample_rate, self.channels, block_size);
        self.pitch.prepare(sample_rate, self.channels, block_size);
        self.dry_wet.prepare(sample_rate, self.channels);
        self.bypass_mix.prepare(sample_rate, TRANSITION_RAMP_MS);
        self.mute_gain.prepare(sample_rate, TRANSITION_RAMP_MS);
        self.output_gain
            .prepare(sample_rate, self.channels, block_size);
        self.limiter.prepare(sample_rate, self.channels, block_size);
        let block_samples = block_size.max(1) * self.channels;
        self.dry_scratch = vec![0.0; block_samples];
        self.delayed_dry_scratch = vec![0.0; block_samples];
    }

    fn process(&mut self, samples: &mut [f32]) {
        if samples.len() > self.dry_scratch.len() {
            return;
        }

        self.input_gain.process(samples);
        self.high_pass.process(samples);

        let len = samples.len();
        self.dry_scratch[..len].copy_from_slice(samples);
        self.pitch.process(samples);
        self.dry_wet.process(
            &self.dry_scratch[..len],
            samples,
            &mut self.delayed_dry_scratch[..len],
        );

        for (frame, delayed_frame) in samples
            .chunks_mut(self.channels)
            .zip(self.delayed_dry_scratch[..len].chunks(self.channels))
        {
            let bypass = self.bypass_mix.next();
            let mute = self.mute_gain.next();
            for (sample, delayed) in frame.iter_mut().zip(delayed_frame) {
                *sample = (*sample * (1.0 - bypass) + *delayed * bypass) * mute;
            }
        }

        self.output_gain.process(samples);
        if self.parameters.limiter_enabled {
            self.limiter.process(samples);
        }
    }

    fn reset(&mut self) {
        self.input_gain.reset();
        self.high_pass.reset();
        self.pitch.reset();
        self.dry_wet.reset();
        self.bypass_mix.reset_to_target();
        self.mute_gain.reset_to_target();
        self.output_gain.reset();
        self.limiter.reset();
        self.dry_scratch.fill(0.0);
        self.delayed_dry_scratch.fill(0.0);
    }
}

#[cfg(test)]
mod tests {
    use super::{AudioProcessor, DspChain, DspParameters};

    fn prepared_chain(parameters: DspParameters) -> DspChain {
        let mut chain = DspChain::default();
        chain.prepare(48_000, 1, 256);
        chain.set_parameters(parameters);
        chain.reset();
        chain
    }

    #[test]
    fn mute_wins_over_bypass() {
        let mut chain = prepared_chain(DspParameters {
            bypass: true,
            muted: true,
            ..DspParameters::default()
        });
        let mut samples = [0.25, -0.5];

        chain.process(&mut samples);

        assert_eq!(samples, [0.0, 0.0]);
    }

    #[test]
    fn bypass_output_does_not_depend_on_pitch_or_dry_wet() {
        let mut pitched = prepared_chain(DspParameters {
            pitch_semitones: 12.0,
            dry_wet: 1.0,
            bypass: true,
            limiter_enabled: false,
            ..DspParameters::default()
        });
        let mut dry = prepared_chain(DspParameters {
            pitch_semitones: -12.0,
            dry_wet: 0.0,
            bypass: true,
            limiter_enabled: false,
            ..DspParameters::default()
        });
        let mut left = [0.25; 256];
        let mut right = left;

        pitched.process(&mut left);
        dry.process(&mut right);

        assert_eq!(left, right);
    }

    #[test]
    fn validates_pitch_dry_wet_and_gain_ranges() {
        assert!(DspParameters::default().validate().is_ok());
        assert!(DspParameters {
            pitch_semitones: 12.1,
            ..DspParameters::default()
        }
        .validate()
        .is_err());
        assert!(DspParameters {
            dry_wet: -0.1,
            ..DspParameters::default()
        }
        .validate()
        .is_err());
        assert!(DspParameters {
            input_gain_db: f32::NAN,
            ..DspParameters::default()
        }
        .validate()
        .is_err());
        assert!(DspParameters {
            output_gain_db: 24.1,
            ..DspParameters::default()
        }
        .validate()
        .is_err());
    }
}
