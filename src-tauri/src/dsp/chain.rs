use serde::{Deserialize, Serialize};

use super::{
    dry_wet::{DelayLine, DryWetMixer},
    gain::Gain,
    high_pass::HighPass,
    master_limiter::{
        MasterLimiter, DEFAULT_MASTER_CEILING_DB, MAX_MASTER_CEILING_DB, MIN_MASTER_CEILING_DB,
    },
    noise_gate::{
        NoiseGate, DEFAULT_GATE_THRESHOLD_DB, DEFAULT_MINIMUM_GAIN_DB, MAX_GATE_THRESHOLD_DB,
        MIN_GATE_THRESHOLD_DB,
    },
    pitch::PitchShifter,
    processor::AudioProcessor,
    smoothing::SmoothedValue,
    tone::{ToneEq, MAX_TONE_DB, MIN_TONE_DB},
    vocal_aging::VocalAgingProcessor,
};

pub const MIN_GAIN_DB: f32 = -24.0;
pub const MAX_INPUT_GAIN_DB: f32 = 24.0;
pub const MAX_OUTPUT_GAIN_DB: f32 = 12.0;
pub const MIN_PITCH_SEMITONES: f32 = -12.0;
pub const MAX_PITCH_SEMITONES: f32 = 12.0;
pub const MIN_FORMANT_SHIFT_SEMITONES: f32 = -6.0;
pub const MAX_FORMANT_SHIFT_SEMITONES: f32 = 6.0;
pub const MIN_AGE_AMOUNT: f32 = 0.0;
pub const MAX_AGE_AMOUNT: f32 = 1.0;
const TRANSITION_RAMP_MS: f32 = 10.0;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DspParameters {
    pub pitch_semitones: f32,
    pub formant_shift_semitones: f32,
    pub dry_wet: f32,
    pub age_character: f32,
    pub breathiness: f32,
    pub tremor: f32,
    pub gate_enabled: bool,
    pub gate_threshold_db: f32,
    pub input_gain_db: f32,
    pub output_gain_db: f32,
    pub master_ceiling_db: f32,
    pub warmth_db: f32,
    pub brightness_db: f32,
    pub limiter_enabled: bool,
    pub bypass: bool,
    pub muted: bool,
}

impl Default for DspParameters {
    fn default() -> Self {
        Self {
            pitch_semitones: 0.0,
            formant_shift_semitones: 0.0,
            dry_wet: 0.35,
            age_character: 0.0,
            breathiness: 0.0,
            tremor: 0.0,
            gate_enabled: false,
            gate_threshold_db: DEFAULT_GATE_THRESHOLD_DB,
            input_gain_db: 0.0,
            output_gain_db: -6.0,
            master_ceiling_db: DEFAULT_MASTER_CEILING_DB,
            warmth_db: 0.0,
            brightness_db: 0.0,
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
        validate_range(
            "Formant shift",
            self.formant_shift_semitones,
            MIN_FORMANT_SHIFT_SEMITONES,
            MAX_FORMANT_SHIFT_SEMITONES,
            "semitones",
        )?;
        validate_range("Dry/wet", self.dry_wet, 0.0, 1.0, "")?;
        validate_range(
            "Age Character",
            self.age_character,
            MIN_AGE_AMOUNT,
            MAX_AGE_AMOUNT,
            "",
        )?;
        validate_range(
            "Breathiness",
            self.breathiness,
            MIN_AGE_AMOUNT,
            MAX_AGE_AMOUNT,
            "",
        )?;
        validate_range("Tremor", self.tremor, MIN_AGE_AMOUNT, MAX_AGE_AMOUNT, "")?;
        validate_range(
            "Gate threshold",
            self.gate_threshold_db,
            MIN_GATE_THRESHOLD_DB,
            MAX_GATE_THRESHOLD_DB,
            "dBFS",
        )?;
        validate_range(
            "Input gain",
            self.input_gain_db,
            MIN_GAIN_DB,
            MAX_INPUT_GAIN_DB,
            "dB",
        )?;
        validate_range(
            "Output gain",
            self.output_gain_db,
            MIN_GAIN_DB,
            MAX_OUTPUT_GAIN_DB,
            "dB",
        )?;
        validate_range(
            "Master ceiling",
            self.master_ceiling_db,
            MIN_MASTER_CEILING_DB,
            MAX_MASTER_CEILING_DB,
            "dBFS",
        )?;
        validate_range("Warmth", self.warmth_db, MIN_TONE_DB, MAX_TONE_DB, "dB")?;
        validate_range(
            "Brightness",
            self.brightness_db,
            MIN_TONE_DB,
            MAX_TONE_DB,
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
    noise_gate: NoiseGate,
    pitch: PitchShifter,
    dry_wet: DryWetMixer,
    vocal_aging: VocalAgingProcessor,
    tone: ToneEq,
    bypass_delay: DelayLine,
    bypass_mix: SmoothedValue,
    mute_gain: SmoothedValue,
    output_gain: Gain,
    limiter: MasterLimiter,
    dry_scratch: Vec<f32>,
    delayed_dry_scratch: Vec<f32>,
    bypass_scratch: Vec<f32>,
    delayed_bypass_scratch: Vec<f32>,
}

impl Default for DspChain {
    fn default() -> Self {
        let parameters = DspParameters::default();
        let pitch = PitchShifter::default();
        let latency_frames = pitch.latency_frames();
        Self {
            parameters,
            channels: 1,
            input_gain: Gain::new(parameters.input_gain_db),
            high_pass: HighPass::default(),
            noise_gate: NoiseGate::default(),
            dry_wet: DryWetMixer::new(parameters.dry_wet, latency_frames),
            vocal_aging: VocalAgingProcessor::default(),
            tone: ToneEq::default(),
            bypass_delay: DelayLine::new(latency_frames),
            pitch,
            bypass_mix: SmoothedValue::new(0.0),
            mute_gain: SmoothedValue::new(1.0),
            output_gain: Gain::new(parameters.output_gain_db),
            limiter: MasterLimiter::default(),
            dry_scratch: Vec::new(),
            delayed_dry_scratch: Vec::new(),
            bypass_scratch: Vec::new(),
            delayed_bypass_scratch: Vec::new(),
        }
    }
}

impl DspChain {
    pub fn set_parameters(&mut self, parameters: DspParameters) {
        self.parameters = parameters;
        self.input_gain.set_gain_db(parameters.input_gain_db);
        self.noise_gate.set_enabled(parameters.gate_enabled);
        self.noise_gate
            .set_threshold_db(parameters.gate_threshold_db);
        self.noise_gate.set_minimum_gain_db(DEFAULT_MINIMUM_GAIN_DB);
        self.pitch.set_pitch_semitones(parameters.pitch_semitones);
        self.pitch
            .set_formant_shift_semitones(parameters.formant_shift_semitones);
        self.dry_wet.set_mix(parameters.dry_wet);
        self.vocal_aging.set_parameters(
            parameters.age_character,
            parameters.breathiness,
            parameters.tremor,
        );
        self.tone.set_warmth_db(parameters.warmth_db);
        self.tone.set_brightness_db(parameters.brightness_db);
        self.bypass_mix
            .set_target(if parameters.bypass { 1.0 } else { 0.0 });
        self.mute_gain
            .set_target(if parameters.muted { 0.0 } else { 1.0 });
        self.output_gain.set_gain_db(parameters.output_gain_db);
        self.limiter.set_enabled(parameters.limiter_enabled);
        self.limiter.set_ceiling_db(parameters.master_ceiling_db);
    }

    pub fn latency_frames(&self) -> usize {
        self.dry_wet.latency_frames()
            + self.vocal_aging.latency_frames()
            + self.limiter.latency_frames()
    }

    pub fn take_expander_attenuated_frames(&mut self) -> usize {
        self.noise_gate.take_attenuated_frames()
    }
}

impl AudioProcessor for DspChain {
    fn prepare(&mut self, sample_rate: u32, channels: usize, block_size: usize) {
        self.channels = channels.max(1);
        self.input_gain
            .prepare(sample_rate, self.channels, block_size);
        self.high_pass
            .prepare(sample_rate, self.channels, block_size);
        self.noise_gate
            .prepare(sample_rate, self.channels, block_size);
        self.pitch.prepare(sample_rate, self.channels, block_size);
        let pitch_latency_frames = self.pitch.latency_frames();
        self.dry_wet.set_latency_frames(pitch_latency_frames);
        self.bypass_delay.set_latency_frames(pitch_latency_frames);
        self.dry_wet.prepare(sample_rate, self.channels);
        self.vocal_aging
            .prepare(sample_rate, self.channels, block_size);
        self.tone.prepare(sample_rate, self.channels, block_size);
        self.bypass_delay.prepare(self.channels);
        self.bypass_mix.prepare(sample_rate, TRANSITION_RAMP_MS);
        self.mute_gain.prepare(sample_rate, TRANSITION_RAMP_MS);
        self.output_gain
            .prepare(sample_rate, self.channels, block_size);
        self.limiter.prepare(sample_rate, self.channels, block_size);
        let block_samples = block_size.max(1) * self.channels;
        self.dry_scratch = vec![0.0; block_samples];
        self.delayed_dry_scratch = vec![0.0; block_samples];
        self.bypass_scratch = vec![0.0; block_samples];
        self.delayed_bypass_scratch = vec![0.0; block_samples];
    }

    fn process(&mut self, samples: &mut [f32]) {
        if samples.len() > self.dry_scratch.len() {
            return;
        }

        self.input_gain.process(samples);
        self.high_pass.process(samples);

        let len = samples.len();
        self.bypass_scratch[..len].copy_from_slice(samples);
        self.noise_gate.process(samples);
        self.dry_scratch[..len].copy_from_slice(samples);
        let frames = len / self.channels;
        let pitch_offset = self.vocal_aging.pitch_offset_semitones(frames);
        self.pitch.set_dynamic_pitch_offset_semitones(pitch_offset);
        self.pitch.process(samples);
        self.dry_wet.process(
            &self.dry_scratch[..len],
            samples,
            &mut self.delayed_dry_scratch[..len],
        );
        self.vocal_aging.process(samples);
        self.tone.process(samples);
        self.bypass_delay.process(
            &self.bypass_scratch[..len],
            &mut self.delayed_bypass_scratch[..len],
        );

        for (frame, bypass_frame) in samples
            .chunks_mut(self.channels)
            .zip(self.delayed_bypass_scratch[..len].chunks(self.channels))
        {
            let bypass = self.bypass_mix.next();
            for (sample, bypass_sample) in frame.iter_mut().zip(bypass_frame) {
                *sample = *sample * (1.0 - bypass) + *bypass_sample * bypass;
            }
        }

        self.output_gain.process(samples);
        self.limiter.process(samples);
        for frame in samples.chunks_mut(self.channels) {
            let mute = self.mute_gain.next();
            for sample in frame {
                *sample *= mute;
            }
        }
    }

    fn reset(&mut self) {
        self.input_gain.reset();
        self.high_pass.reset();
        self.noise_gate.reset();
        self.pitch.reset();
        self.dry_wet.reset();
        self.vocal_aging.reset();
        self.tone.reset();
        self.bypass_delay.reset();
        self.bypass_mix.reset_to_target();
        self.mute_gain.reset_to_target();
        self.output_gain.reset();
        self.limiter.reset();
        self.dry_scratch.fill(0.0);
        self.delayed_dry_scratch.fill(0.0);
        self.bypass_scratch.fill(0.0);
        self.delayed_bypass_scratch.fill(0.0);
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
            age_character: 1.0,
            breathiness: 1.0,
            tremor: 1.0,
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
        for invalid in [-0.1, 1.1, f32::NAN, f32::INFINITY] {
            assert!(DspParameters {
                age_character: invalid,
                ..DspParameters::default()
            }
            .validate()
            .is_err());
            assert!(DspParameters {
                breathiness: invalid,
                ..DspParameters::default()
            }
            .validate()
            .is_err());
            assert!(DspParameters {
                tremor: invalid,
                ..DspParameters::default()
            }
            .validate()
            .is_err());
        }
        assert!(DspParameters {
            age_character: 1.0,
            breathiness: 1.0,
            tremor: 1.0,
            ..DspParameters::default()
        }
        .validate()
        .is_ok());
        assert!(DspParameters {
            gate_threshold_db: -81.0,
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
            output_gain_db: 12.1,
            ..DspParameters::default()
        }
        .validate()
        .is_err());
    }

    #[test]
    fn limiter_contains_the_combined_aged_voice_and_aspiration() {
        let mut chain = prepared_chain(DspParameters {
            dry_wet: 0.0,
            age_character: 1.0,
            breathiness: 1.0,
            tremor: 1.0,
            output_gain_db: 0.0,
            master_ceiling_db: -12.0,
            limiter_enabled: true,
            ..DspParameters::default()
        });
        let mut maximum = 0.0_f32;
        for _ in 0..80 {
            let mut block = [1.0; 256];
            chain.process(&mut block);
            maximum = maximum.max(block.iter().map(|sample| sample.abs()).fold(0.0, f32::max));
        }
        let ceiling = 10.0_f32.powf(-12.0 / 20.0);
        assert!(maximum <= ceiling + 1.0e-4);
    }

    #[test]
    fn long_aged_stereo_processing_remains_finite_and_bounded() {
        let mut chain = DspChain::default();
        chain.prepare(44_100, 2, 127);
        chain.set_parameters(DspParameters {
            pitch_semitones: 3.5,
            formant_shift_semitones: 2.0,
            dry_wet: 0.9,
            age_character: 1.0,
            breathiness: 1.0,
            tremor: 1.0,
            output_gain_db: -6.0,
            limiter_enabled: true,
            ..DspParameters::default()
        });
        chain.reset();

        for block_index in 0..2_000 {
            let mut block = vec![0.0; 127 * 2];
            for (frame_index, frame) in block.chunks_exact_mut(2).enumerate() {
                let phase =
                    ((block_index * 127 + frame_index) as f32 * 220.0 * std::f32::consts::TAU)
                        / 44_100.0;
                frame.fill(phase.sin() * 0.4);
            }
            chain.process(&mut block);
            assert!(block
                .iter()
                .all(|sample| sample.is_finite() && sample.abs() <= 1.0));
        }
    }
}
