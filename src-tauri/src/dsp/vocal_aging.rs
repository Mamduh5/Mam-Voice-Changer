use std::f32::consts::TAU;

use super::{processor::AudioProcessor, smoothing::SmoothedValue};

pub const MAX_TREMOR_PITCH_CENTS: f32 = 18.0;
pub const MAX_JITTER_PITCH_CENTS: f32 = 9.0;
pub const MAX_TREMOR_GAIN_DEPTH: f32 = 0.035;
pub const MAX_SHIMMER_GAIN_DEPTH: f32 = 0.018;
pub const MAX_BREATH_NOISE_GAIN: f32 = 0.045;
pub const TREMOR_RATE_HZ: f32 = 4.8;
const JITTER_RATE_HZ: f32 = 7.0;
const SHIMMER_RATE_HZ: f32 = 5.2;
const PARAMETER_RAMP_MS: f32 = 30.0;
const ENVELOPE_ATTACK_MS: f32 = 8.0;
const ENVELOPE_RELEASE_MS: f32 = 90.0;
const PITCH_SEED: u32 = 0xA341_316C;
const SHIMMER_SEED: u32 = 0xC801_3EA4;
const BREATH_SEED: u32 = 0xAD90_777D;

/// Zero-latency, allocation-free-after-prepare vocal-aging effects.
///
/// Pitch tremor and jitter are returned as one bounded block-rate offset for the
/// existing pitch shifter. Amplitude movement, aspiration, and spectral aging
/// are then applied to the pitch-aligned wet signal with stereo-linked control.
pub struct VocalAgingProcessor {
    sample_rate: f32,
    channels: usize,
    pitch_age: SmoothedValue,
    pitch_tremor: SmoothedValue,
    audio_age: SmoothedValue,
    audio_breathiness: SmoothedValue,
    audio_tremor: SmoothedValue,
    pitch_tremor_phase: f32,
    amplitude_tremor_phase: f32,
    pitch_random: DeterministicRandom,
    pitch_jitter: RandomInterpolator,
    shimmer_random: DeterministicRandom,
    shimmer: RandomInterpolator,
    breath_random: DeterministicRandom,
    envelope: f32,
    envelope_attack: f32,
    envelope_release: f32,
    noise_input_previous: f32,
    noise_high_pass_previous: f32,
    noise_low_pass: f32,
    noise_high_pass_coefficient: f32,
    noise_low_pass_coefficient: f32,
    spectral_states: Vec<SpectralState>,
    spectral_low_coefficient: f32,
    spectral_presence_low_coefficient: f32,
    spectral_presence_high_coefficient: f32,
    spectral_high_coefficient: f32,
}

impl Default for VocalAgingProcessor {
    fn default() -> Self {
        Self {
            sample_rate: 48_000.0,
            channels: 1,
            pitch_age: SmoothedValue::new(0.0),
            pitch_tremor: SmoothedValue::new(0.0),
            audio_age: SmoothedValue::new(0.0),
            audio_breathiness: SmoothedValue::new(0.0),
            audio_tremor: SmoothedValue::new(0.0),
            pitch_tremor_phase: 0.0,
            amplitude_tremor_phase: 0.0,
            pitch_random: DeterministicRandom::new(PITCH_SEED),
            pitch_jitter: RandomInterpolator::default(),
            shimmer_random: DeterministicRandom::new(SHIMMER_SEED),
            shimmer: RandomInterpolator::default(),
            breath_random: DeterministicRandom::new(BREATH_SEED),
            envelope: 0.0,
            envelope_attack: 0.0,
            envelope_release: 0.0,
            noise_input_previous: 0.0,
            noise_high_pass_previous: 0.0,
            noise_low_pass: 0.0,
            noise_high_pass_coefficient: 0.0,
            noise_low_pass_coefficient: 0.0,
            spectral_states: Vec::new(),
            spectral_low_coefficient: 0.0,
            spectral_presence_low_coefficient: 0.0,
            spectral_presence_high_coefficient: 0.0,
            spectral_high_coefficient: 0.0,
        }
    }
}

impl VocalAgingProcessor {
    pub fn set_parameters(&mut self, age_character: f32, breathiness: f32, tremor: f32) {
        self.pitch_age.set_target(age_character);
        self.pitch_tremor.set_target(tremor);
        self.audio_age.set_target(age_character);
        self.audio_breathiness.set_target(breathiness);
        self.audio_tremor.set_target(tremor);
    }

    /// Advances the pitch modulation by `frames` and returns one conservative
    /// block-average offset for the existing Signalsmith transpose control.
    pub fn pitch_offset_semitones(&mut self, frames: usize) -> f32 {
        if frames == 0 {
            return 0.0;
        }

        let phase_step = TREMOR_RATE_HZ / self.sample_rate;
        let jitter_interval = interval_frames(self.sample_rate, JITTER_RATE_HZ);
        let jitter_smoothing = one_pole_coefficient(self.sample_rate, 45.0);
        let mut sum = 0.0;

        for _ in 0..frames {
            let age = character_curve(self.pitch_age.next());
            let tremor = self.pitch_tremor.next();
            let periodic = (TAU * self.pitch_tremor_phase).sin()
                * (MAX_TREMOR_PITCH_CENTS / 100.0)
                * age
                * tremor;
            let irregular =
                self.pitch_jitter
                    .next(&mut self.pitch_random, jitter_interval, jitter_smoothing)
                    * (MAX_JITTER_PITCH_CENTS / 100.0)
                    * age;
            sum += periodic + irregular;
            self.pitch_tremor_phase = wrap_phase(self.pitch_tremor_phase + phase_step);
        }

        sum / frames as f32
    }

    pub const fn latency_frames(&self) -> usize {
        0
    }

    fn shaped_noise(&mut self) -> f32 {
        let input = self.breath_random.next_bipolar();
        let high_pass = self.noise_high_pass_coefficient
            * (self.noise_high_pass_previous + input - self.noise_input_previous);
        self.noise_input_previous = input;
        self.noise_high_pass_previous = high_pass;
        self.noise_low_pass += self.noise_low_pass_coefficient * (high_pass - self.noise_low_pass);
        self.noise_low_pass.clamp(-1.0, 1.0)
    }
}

impl AudioProcessor for VocalAgingProcessor {
    fn prepare(&mut self, sample_rate: u32, channels: usize, _block_size: usize) {
        self.sample_rate = sample_rate.max(1) as f32;
        self.channels = channels.max(1);
        for value in [
            &mut self.pitch_age,
            &mut self.pitch_tremor,
            &mut self.audio_age,
            &mut self.audio_breathiness,
            &mut self.audio_tremor,
        ] {
            value.prepare(sample_rate, PARAMETER_RAMP_MS);
        }
        self.envelope_attack = one_pole_coefficient(self.sample_rate, ENVELOPE_ATTACK_MS);
        self.envelope_release = one_pole_coefficient(self.sample_rate, ENVELOPE_RELEASE_MS);
        self.noise_high_pass_coefficient = high_pass_coefficient(self.sample_rate, 1_600.0);
        self.noise_low_pass_coefficient = low_pass_coefficient(self.sample_rate, 7_500.0);
        self.spectral_low_coefficient = low_pass_coefficient(self.sample_rate, 260.0);
        self.spectral_presence_low_coefficient = low_pass_coefficient(self.sample_rate, 900.0);
        self.spectral_presence_high_coefficient = low_pass_coefficient(self.sample_rate, 2_400.0);
        self.spectral_high_coefficient = low_pass_coefficient(self.sample_rate, 7_500.0);
        self.spectral_states = vec![SpectralState::default(); self.channels];
        self.reset();
    }

    fn process(&mut self, samples: &mut [f32]) {
        let phase_step = TREMOR_RATE_HZ / self.sample_rate;
        let shimmer_interval = interval_frames(self.sample_rate, SHIMMER_RATE_HZ);
        let shimmer_smoothing = one_pole_coefficient(self.sample_rate, 65.0);

        for frame in samples.chunks_mut(self.channels) {
            let age = character_curve(self.audio_age.next());
            let breathiness = self.audio_breathiness.next();
            let tremor = self.audio_tremor.next();
            let periodic = (TAU * self.amplitude_tremor_phase).sin();
            let shimmer = self.shimmer.next(
                &mut self.shimmer_random,
                shimmer_interval,
                shimmer_smoothing,
            );
            let gain = (1.0
                + periodic * MAX_TREMOR_GAIN_DEPTH * age * tremor
                + shimmer * MAX_SHIMMER_GAIN_DEPTH * age)
                .clamp(0.9, 1.1);

            let linked_peak = frame
                .iter()
                .filter(|sample| sample.is_finite())
                .fold(0.0_f32, |peak, sample| peak.max(sample.abs()));
            let envelope_coefficient = if linked_peak > self.envelope {
                self.envelope_attack
            } else {
                self.envelope_release
            };
            self.envelope += envelope_coefficient * (linked_peak - self.envelope);
            let activity = ((self.envelope - 0.0005).max(0.0) * 5.0).sqrt().min(1.0);
            let aspiration =
                self.shaped_noise() * MAX_BREATH_NOISE_GAIN * age * breathiness * activity;

            for (channel, sample) in frame.iter_mut().enumerate() {
                let input = if sample.is_finite() { *sample } else { 0.0 };
                let combined = input * gain + aspiration;
                let state = &mut self.spectral_states[channel];
                state.low += self.spectral_low_coefficient * (combined - state.low);
                state.presence_low +=
                    self.spectral_presence_low_coefficient * (combined - state.presence_low);
                state.presence_high +=
                    self.spectral_presence_high_coefficient * (combined - state.presence_high);
                state.high += self.spectral_high_coefficient * (combined - state.high);

                let thinned = combined - state.low * (0.24 * age);
                let presence = (state.presence_high - state.presence_low) * (0.16 * age);
                let colored = thinned + presence;
                let output = colored * (1.0 - 0.12 * age) + state.high * (0.12 * age);
                if output.is_finite() {
                    *sample = output;
                } else {
                    *state = SpectralState::default();
                    *sample = 0.0;
                }
            }

            self.amplitude_tremor_phase = wrap_phase(self.amplitude_tremor_phase + phase_step);
        }
    }

    fn reset(&mut self) {
        self.pitch_age.reset_to_target();
        self.pitch_tremor.reset_to_target();
        self.audio_age.reset_to_target();
        self.audio_breathiness.reset_to_target();
        self.audio_tremor.reset_to_target();
        self.pitch_tremor_phase = 0.0;
        self.amplitude_tremor_phase = 0.0;
        self.pitch_random.reset(PITCH_SEED);
        self.pitch_jitter = RandomInterpolator::default();
        self.shimmer_random.reset(SHIMMER_SEED);
        self.shimmer = RandomInterpolator::default();
        self.breath_random.reset(BREATH_SEED);
        self.envelope = 0.0;
        self.noise_input_previous = 0.0;
        self.noise_high_pass_previous = 0.0;
        self.noise_low_pass = 0.0;
        self.spectral_states.fill(SpectralState::default());
    }
}

#[derive(Clone, Copy, Default)]
struct SpectralState {
    low: f32,
    presence_low: f32,
    presence_high: f32,
    high: f32,
}

#[derive(Clone, Copy, Default)]
struct RandomInterpolator {
    current: f32,
    target: f32,
    frames_until_target: usize,
}

impl RandomInterpolator {
    fn next(&mut self, random: &mut DeterministicRandom, interval: usize, smoothing: f32) -> f32 {
        if self.frames_until_target == 0 {
            self.target = random.next_bipolar();
            self.frames_until_target = interval;
        }
        self.frames_until_target -= 1;
        self.current += smoothing * (self.target - self.current);
        self.current.clamp(-1.0, 1.0)
    }
}

#[derive(Clone, Copy)]
struct DeterministicRandom {
    state: u32,
}

impl DeterministicRandom {
    const fn new(seed: u32) -> Self {
        Self { state: seed }
    }

    fn reset(&mut self, seed: u32) {
        self.state = seed;
    }

    fn next_bipolar(&mut self) -> f32 {
        let mut value = self.state;
        value ^= value << 13;
        value ^= value >> 17;
        value ^= value << 5;
        self.state = value;
        value as f32 * (2.0 / u32::MAX as f32) - 1.0
    }
}

fn character_curve(value: f32) -> f32 {
    let value = value.clamp(0.0, 1.0);
    value * value * (3.0 - 2.0 * value)
}

fn interval_frames(sample_rate: f32, rate_hz: f32) -> usize {
    (sample_rate / rate_hz).round().max(1.0) as usize
}

fn wrap_phase(phase: f32) -> f32 {
    if phase >= 1.0 {
        phase - 1.0
    } else {
        phase
    }
}

fn one_pole_coefficient(sample_rate: f32, time_ms: f32) -> f32 {
    1.0 - (-1.0 / (sample_rate * time_ms / 1_000.0)).exp()
}

fn low_pass_coefficient(sample_rate: f32, frequency_hz: f32) -> f32 {
    1.0 - (-TAU * frequency_hz.min(sample_rate * 0.45) / sample_rate).exp()
}

fn high_pass_coefficient(sample_rate: f32, frequency_hz: f32) -> f32 {
    (-TAU * frequency_hz.min(sample_rate * 0.45) / sample_rate).exp()
}

#[cfg(test)]
mod tests {
    use super::{
        AudioProcessor, VocalAgingProcessor, MAX_JITTER_PITCH_CENTS, MAX_TREMOR_PITCH_CENTS,
    };

    fn prepared(
        sample_rate: u32,
        channels: usize,
        age: f32,
        breath: f32,
        tremor: f32,
    ) -> VocalAgingProcessor {
        let mut processor = VocalAgingProcessor::default();
        processor.set_parameters(age, breath, tremor);
        processor.prepare(sample_rate, channels, 256);
        processor
    }

    #[test]
    fn zero_age_is_sample_neutral_and_silence_stays_silent() {
        let mut processor = prepared(48_000, 1, 0.0, 1.0, 1.0);
        let mut samples = vec![0.25; 1_024];
        let original = samples.clone();
        processor.process(&mut samples);
        assert_eq!(samples, original);

        let mut silence_processor = prepared(48_000, 1, 1.0, 1.0, 1.0);
        let mut silence = vec![0.0; 48_000];
        silence_processor.process(&mut silence);
        assert!(silence.iter().all(|sample| sample.abs() < 1.0e-7));
    }

    #[test]
    fn pitch_modulation_is_smooth_bounded_and_resettable() {
        let mut processor = prepared(48_000, 1, 1.0, 1.0, 1.0);
        let sequence: Vec<_> = (0..2_000)
            .map(|_| processor.pitch_offset_semitones(24))
            .collect();
        let bound = (MAX_TREMOR_PITCH_CENTS + MAX_JITTER_PITCH_CENTS) / 100.0;
        assert!(sequence.iter().all(|value| value.abs() <= bound));
        assert!(sequence
            .windows(2)
            .all(|pair| (pair[1] - pair[0]).abs() < 0.08));

        processor.reset();
        let repeated: Vec<_> = (0..2_000)
            .map(|_| processor.pitch_offset_semitones(24))
            .collect();
        assert_eq!(sequence, repeated);
    }

    #[test]
    fn modulation_rate_is_sample_rate_independent() {
        let mut at_44k = prepared(44_100, 1, 1.0, 0.0, 1.0);
        let mut at_48k = prepared(48_000, 1, 1.0, 0.0, 1.0);
        for _ in 0..441 {
            at_44k.pitch_offset_semitones(100);
        }
        for _ in 0..480 {
            at_48k.pitch_offset_semitones(100);
        }
        assert!((at_44k.pitch_tremor_phase - at_48k.pitch_tremor_phase).abs() < 1.0e-3);
    }

    #[test]
    fn stereo_modulation_is_linked_and_output_remains_bounded() {
        let mut processor = prepared(96_000, 2, 1.0, 1.0, 1.0);
        let mut samples = vec![0.2; 96_000 * 2];
        processor.process(&mut samples);
        assert!(samples
            .iter()
            .all(|sample| sample.is_finite() && sample.abs() < 0.5));
        assert!(samples
            .chunks_exact(2)
            .all(|frame| (frame[0] - frame[1]).abs() < f32::EPSILON));
    }

    #[test]
    fn breathiness_follows_speech_and_spectral_aging_changes_the_signal() {
        let mut neutral = prepared(48_000, 1, 0.0, 0.0, 0.0);
        let mut aged = prepared(48_000, 1, 1.0, 1.0, 0.0);
        let source: Vec<_> = (0..48_000)
            .map(|index| ((index as f32 * 440.0 * std::f32::consts::TAU) / 48_000.0).sin() * 0.2)
            .collect();
        let mut unchanged = source.clone();
        let mut changed = source;
        neutral.process(&mut unchanged);
        aged.process(&mut changed);
        assert!(changed.iter().all(|sample| sample.is_finite()));
        assert!(changed
            .iter()
            .zip(&unchanged)
            .any(|(left, right)| (left - right).abs() > 1.0e-4));
    }

    #[test]
    fn repeated_enable_disable_and_varied_blocks_stay_finite() {
        for sample_rate in [32_000, 44_100, 48_000, 96_000] {
            let mut processor = prepared(sample_rate, 1, 0.0, 0.0, 0.0);
            for block_len in [1, 17, 128, 511, 2_048] {
                let mut block = vec![0.1; block_len];
                processor.set_parameters(1.0, 1.0, 1.0);
                processor.pitch_offset_semitones(block_len);
                processor.process(&mut block);
                processor.set_parameters(0.0, 0.0, 0.0);
                processor.process(&mut block);
                assert!(block.iter().all(|sample| sample.is_finite()));
            }
        }
    }

    #[test]
    fn breath_noise_is_bounded_and_enable_transition_is_smooth() {
        let mut dry = prepared(48_000, 1, 1.0, 0.0, 0.0);
        let mut breathy = prepared(48_000, 1, 1.0, 1.0, 0.0);
        let mut dry_signal = vec![0.2; 48_000];
        let mut breathy_signal = dry_signal.clone();
        dry.process(&mut dry_signal);
        breathy.process(&mut breathy_signal);
        let maximum_added = dry_signal
            .iter()
            .zip(&breathy_signal)
            .map(|(left, right)| (left - right).abs())
            .fold(0.0_f32, f32::max);
        assert!(maximum_added > 1.0e-5);
        assert!(maximum_added < 0.07);

        let mut transition = prepared(48_000, 1, 0.0, 0.0, 0.0);
        let mut before = [0.2];
        transition.process(&mut before);
        transition.set_parameters(1.0, 1.0, 1.0);
        let mut after = [0.2];
        transition.process(&mut after);
        assert!((after[0] - before[0]).abs() < 0.01);
    }
}
