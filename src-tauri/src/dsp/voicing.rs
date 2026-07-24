use super::smoothing::SmoothedValue;

const ANALYSIS_WINDOW_MS: f32 = 25.0;
const ANALYSIS_HOP_MS: f32 = 10.0;
const MIN_F0_HZ: f32 = 60.0;
const MAX_F0_HZ: f32 = 500.0;
const DOWNSAMPLE_FACTOR: usize = 2;
const ATTACK_MS: f32 = 20.0;
const RELEASE_MS: f32 = 35.0;
const PRESERVATION_RAMP_MS: f32 = 20.0;
const VOICED_ON_THRESHOLD: f32 = 0.62;
const VOICED_OFF_THRESHOLD: f32 = 0.38;

/// Deterministic causal voiced/unvoiced analysis for one linked speech stream.
///
/// A trailing 25 ms window is downsampled by two and evaluated every 10 ms.
/// Normalized autocorrelation supplies periodicity, while RMS energy,
/// zero-crossing rate, and normalized first-difference energy suppress silence
/// and noise. The output is a continuous attack/release-smoothed probability.
pub struct VoicingDetector {
    sample_rate: u32,
    channels: usize,
    analysis: Vec<f32>,
    ordered: Vec<f32>,
    write_index: usize,
    filled: usize,
    downsample_sum: f32,
    downsample_count: usize,
    hop_frames: usize,
    frames_until_analysis: usize,
    minimum_lag: usize,
    maximum_lag: usize,
    attack_amount: f32,
    release_amount: f32,
    smoothed_probability: f32,
    current_probability: f32,
    probability_increment: f32,
    probability_ramp_remaining: usize,
    voiced: bool,
}

impl Default for VoicingDetector {
    fn default() -> Self {
        Self {
            sample_rate: 48_000,
            channels: 1,
            analysis: Vec::new(),
            ordered: Vec::new(),
            write_index: 0,
            filled: 0,
            downsample_sum: 0.0,
            downsample_count: 0,
            hop_frames: 480,
            frames_until_analysis: 480,
            minimum_lag: 48,
            maximum_lag: 400,
            attack_amount: 1.0,
            release_amount: 1.0,
            smoothed_probability: 0.0,
            current_probability: 0.0,
            probability_increment: 0.0,
            probability_ramp_remaining: 0,
            voiced: false,
        }
    }
}

impl VoicingDetector {
    pub fn prepare(
        &mut self,
        sample_rate: u32,
        channels: usize,
        maximum_block_size: usize,
    ) -> Result<(), String> {
        if sample_rate == 0 {
            return Err("Voicing analysis requires a nonzero sample rate.".to_owned());
        }
        if channels == 0 {
            return Err("Voicing analysis requires at least one channel.".to_owned());
        }
        if maximum_block_size == 0 {
            return Err("Voicing analysis requires a nonzero maximum block size.".to_owned());
        }

        let analysis_rate = sample_rate as f32 / DOWNSAMPLE_FACTOR as f32;
        let window_samples =
            ((analysis_rate * ANALYSIS_WINDOW_MS / 1_000.0).round() as usize).max(2);
        let minimum_lag = (analysis_rate / MAX_F0_HZ).floor().max(1.0) as usize;
        let maximum_lag = (analysis_rate / MIN_F0_HZ).ceil() as usize;
        if maximum_lag + 2 >= window_samples {
            return Err(
                "Voicing analysis window is too short for the configured F0 range.".to_owned(),
            );
        }

        self.sample_rate = sample_rate;
        self.channels = channels;
        self.analysis = vec![0.0; window_samples];
        self.ordered = vec![0.0; window_samples];
        self.hop_frames =
            ((sample_rate as f32 * ANALYSIS_HOP_MS / 1_000.0).round() as usize).max(1);
        self.minimum_lag = minimum_lag;
        self.maximum_lag = maximum_lag;
        self.attack_amount = smoothing_amount(ANALYSIS_HOP_MS, ATTACK_MS);
        self.release_amount = smoothing_amount(ANALYSIS_HOP_MS, RELEASE_MS);
        self.reset();
        Ok(())
    }

    pub fn process(&mut self, input: &[f32], probabilities: &mut [f32]) {
        let frames = input.len() / self.channels;
        if self.analysis.is_empty()
            || input.len() != frames * self.channels
            || probabilities.len() != frames
        {
            probabilities.fill(0.0);
            return;
        }

        for (frame, probability) in input.chunks_exact(self.channels).zip(probabilities) {
            if self.probability_ramp_remaining > 0 {
                self.current_probability += self.probability_increment;
                self.probability_ramp_remaining -= 1;
                if self.probability_ramp_remaining == 0 {
                    self.current_probability = self.smoothed_probability;
                }
            }
            self.current_probability = finite_probability(self.current_probability);
            *probability = self.current_probability;

            let linked_sample = frame
                .iter()
                .map(|sample| if sample.is_finite() { *sample } else { 0.0 })
                .sum::<f32>()
                / self.channels as f32;
            self.downsample_sum += linked_sample;
            self.downsample_count += 1;
            if self.downsample_count == DOWNSAMPLE_FACTOR {
                self.analysis[self.write_index] = self.downsample_sum / DOWNSAMPLE_FACTOR as f32;
                self.write_index = (self.write_index + 1) % self.analysis.len();
                self.filled = (self.filled + 1).min(self.analysis.len());
                self.downsample_sum = 0.0;
                self.downsample_count = 0;
            }

            self.frames_until_analysis -= 1;
            if self.frames_until_analysis == 0 {
                self.frames_until_analysis = self.hop_frames;
                let raw = self.analyse();
                self.voiced = if self.voiced {
                    raw >= VOICED_OFF_THRESHOLD
                } else {
                    raw >= VOICED_ON_THRESHOLD
                };
                let stabilized = if self.voiced {
                    raw.max(0.55)
                } else {
                    raw.min(0.45)
                };
                let amount = if stabilized > self.smoothed_probability {
                    self.attack_amount
                } else {
                    self.release_amount
                };
                self.smoothed_probability += amount * (stabilized - self.smoothed_probability);
                self.smoothed_probability = finite_probability(self.smoothed_probability);
                self.probability_ramp_remaining = self.hop_frames;
                self.probability_increment =
                    (self.smoothed_probability - self.current_probability) / self.hop_frames as f32;
            }
        }
    }

    pub fn reset(&mut self) {
        self.analysis.fill(0.0);
        self.ordered.fill(0.0);
        self.write_index = 0;
        self.filled = 0;
        self.downsample_sum = 0.0;
        self.downsample_count = 0;
        self.frames_until_analysis = self.hop_frames;
        self.smoothed_probability = 0.0;
        self.current_probability = 0.0;
        self.probability_increment = 0.0;
        self.probability_ramp_remaining = 0;
        self.voiced = false;
    }

    pub const fn added_latency_frames(&self) -> usize {
        0
    }

    fn analyse(&mut self) -> f32 {
        if self.filled < self.analysis.len() {
            return 0.0;
        }

        for index in 0..self.analysis.len() {
            let source = (self.write_index + index) % self.analysis.len();
            self.ordered[index] = self.analysis[source];
        }

        let mean = self.ordered.iter().copied().sum::<f32>() / self.ordered.len() as f32;
        let mut energy = 0.0_f64;
        let mut difference_energy = 0.0_f64;
        let mut zero_crossings = 0_usize;
        let mut previous = 0.0_f32;
        for (index, sample) in self.ordered.iter_mut().enumerate() {
            *sample = if sample.is_finite() {
                *sample - mean
            } else {
                0.0
            };
            let value = f64::from(*sample);
            energy += value * value;
            if index > 0 {
                let difference = f64::from(*sample - previous);
                difference_energy += difference * difference;
                if (*sample >= 0.0) != (previous >= 0.0) {
                    zero_crossings += 1;
                }
            }
            previous = *sample;
        }

        let rms = (energy / self.ordered.len() as f64).sqrt() as f32;
        if !rms.is_finite() || rms < 1.0e-5 || energy <= f64::EPSILON {
            return 0.0;
        }

        let mut periodicity = 0.0_f64;
        for lag in self.minimum_lag..=self.maximum_lag {
            let mut cross = 0.0_f64;
            let mut left_energy = 0.0_f64;
            let mut right_energy = 0.0_f64;
            for index in 0..self.ordered.len() - lag {
                let left = f64::from(self.ordered[index]);
                let right = f64::from(self.ordered[index + lag]);
                cross += left * right;
                left_energy += left * left;
                right_energy += right * right;
            }
            let normalized = cross / (left_energy * right_energy).sqrt().max(f64::EPSILON);
            periodicity = periodicity.max(normalized);
        }

        let zero_crossing_rate = zero_crossings as f32 / (self.ordered.len() - 1) as f32;
        let high_frequency_ratio = (difference_energy / (4.0 * energy).max(f64::EPSILON)) as f32;
        let energy_score = smoothstep(0.0005, 0.006, rms);
        let periodicity_score = smoothstep(0.32, 0.72, periodicity as f32);
        let zero_crossing_score = 1.0 - smoothstep(0.14, 0.35, zero_crossing_rate);
        let high_frequency_score = 1.0 - smoothstep(0.16, 0.42, high_frequency_ratio);
        finite_probability(
            energy_score * periodicity_score * zero_crossing_score.min(high_frequency_score),
        )
    }
}

pub struct ConsonantPreserver {
    channels: usize,
    probability_delay: Vec<f32>,
    delay_index: usize,
    preservation: SmoothedValue,
}

impl Default for ConsonantPreserver {
    fn default() -> Self {
        Self {
            channels: 1,
            probability_delay: Vec::new(),
            delay_index: 0,
            preservation: SmoothedValue::new(1.0),
        }
    }
}

impl ConsonantPreserver {
    pub fn prepare(
        &mut self,
        sample_rate: u32,
        channels: usize,
        latency_frames: usize,
    ) -> Result<(), String> {
        if sample_rate == 0 || channels == 0 {
            return Err(
                "Consonant preservation requires a nonzero sample rate and channel count."
                    .to_owned(),
            );
        }
        self.channels = channels;
        self.probability_delay = vec![0.0; latency_frames.max(1)];
        self.delay_index = 0;
        self.preservation.prepare(sample_rate, PRESERVATION_RAMP_MS);
        self.preservation.reset_to_target();
        Ok(())
    }

    pub fn set_amount(&mut self, amount: f32) {
        self.preservation.set_target(amount);
    }

    pub fn process(
        &mut self,
        current_probabilities: &[f32],
        aligned_preserved: &[f32],
        transformed: &mut [f32],
    ) {
        let frames = transformed.len() / self.channels;
        if current_probabilities.len() != frames
            || aligned_preserved.len() != transformed.len()
            || transformed.len() != frames * self.channels
            || self.probability_delay.is_empty()
        {
            transformed.fill(0.0);
            return;
        }

        for ((probability, preserved_frame), transformed_frame) in current_probabilities
            .iter()
            .zip(aligned_preserved.chunks_exact(self.channels))
            .zip(transformed.chunks_exact_mut(self.channels))
        {
            let aligned_probability = self.probability_delay[self.delay_index];
            self.probability_delay[self.delay_index] = finite_probability(*probability);
            self.delay_index = (self.delay_index + 1) % self.probability_delay.len();
            let preserved_amount =
                self.preservation.next() * (1.0 - finite_probability(aligned_probability));
            for (wet, dry) in transformed_frame.iter_mut().zip(preserved_frame) {
                let transformed_sample = if wet.is_finite() { *wet } else { 0.0 };
                let preserved_sample = if dry.is_finite() { *dry } else { 0.0 };
                *wet = transformed_sample * (1.0 - preserved_amount)
                    + preserved_sample * preserved_amount;
            }
        }
    }

    pub fn reset(&mut self) {
        self.probability_delay.fill(0.0);
        self.delay_index = 0;
        self.preservation.reset_to_target();
    }
}

fn smoothing_amount(interval_ms: f32, time_ms: f32) -> f32 {
    1.0 - (-interval_ms / time_ms).exp()
}

fn smoothstep(edge0: f32, edge1: f32, value: f32) -> f32 {
    let normalized = ((value - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    normalized * normalized * (3.0 - 2.0 * normalized)
}

fn finite_probability(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use std::f32::consts::TAU;

    use super::{ConsonantPreserver, VoicingDetector};
    use crate::dsp::{dry_wet::DelayLine, pitch::PitchShifter, processor::AudioProcessor};

    const SAMPLE_RATE: u32 = 48_000;
    const BLOCK_SIZE: usize = 127;

    fn harmonic(frequency: f32, sample_rate: u32, frames: usize) -> Vec<f32> {
        (0..frames)
            .map(|frame| {
                (1..=8)
                    .map(|harmonic| {
                        (TAU * frequency * harmonic as f32 * frame as f32 / sample_rate as f32)
                            .sin()
                            / harmonic as f32
                    })
                    .sum::<f32>()
                    * 0.12
            })
            .collect()
    }

    fn deterministic_noise(frames: usize, amplitude: f32) -> Vec<f32> {
        let mut state = 0xA341_316C_u32;
        (0..frames)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 17;
                state ^= state << 5;
                (state as f32 * (2.0 / u32::MAX as f32) - 1.0) * amplitude
            })
            .collect()
    }

    fn fricative(frames: usize, amplitude: f32) -> Vec<f32> {
        let noise = deterministic_noise(frames, amplitude);
        let mut previous = 0.0;
        noise
            .into_iter()
            .map(|sample| {
                let high_pass = sample - previous;
                previous = sample;
                high_pass * 0.5
            })
            .collect()
    }

    fn vowel_fricative_vowel() -> Vec<f32> {
        let segment_frames = SAMPLE_RATE as usize / 2;
        let mut signal = harmonic(220.0, SAMPLE_RATE, segment_frames);
        signal.extend(fricative(segment_frames, 0.1));
        signal.extend(harmonic(180.0, SAMPLE_RATE, segment_frames));
        signal
    }

    fn probabilities(signal: &[f32], sample_rate: u32, block_size: usize) -> Vec<f32> {
        let mut detector = VoicingDetector::default();
        detector.prepare(sample_rate, 1, block_size).unwrap();
        let mut output = vec![0.0; signal.len()];
        for (input, probabilities) in signal.chunks(block_size).zip(output.chunks_mut(block_size)) {
            detector.process(input, probabilities);
        }
        output
    }

    fn steady_median(values: &[f32]) -> f32 {
        let start = values.len() * 3 / 4;
        let mut tail = values[start..].to_vec();
        tail.sort_by(f32::total_cmp);
        tail[tail.len() / 2]
    }

    fn render_preserved(
        source: &[f32],
        pitch_semitones: f32,
        formant_semitones: f32,
        preservation: f32,
    ) -> (Vec<f32>, Vec<f32>, usize) {
        let mut detector = VoicingDetector::default();
        detector.prepare(SAMPLE_RATE, 1, BLOCK_SIZE).unwrap();
        let mut pitch = PitchShifter::default();
        pitch.set_pitch_semitones(pitch_semitones);
        pitch.set_formant_shift_semitones(formant_semitones);
        pitch.prepare(SAMPLE_RATE, 1, BLOCK_SIZE).unwrap();
        let latency = pitch.latency_frames();
        let mut delay = DelayLine::new(latency);
        delay.prepare(1);
        let mut preserver = ConsonantPreserver::default();
        preserver.set_amount(preservation);
        preserver.prepare(SAMPLE_RATE, 1, latency).unwrap();

        detector.reset();
        pitch.reset();
        delay.reset();
        preserver.reset();
        let render_frames = (source.len() + latency).div_ceil(BLOCK_SIZE) * BLOCK_SIZE;
        let mut stream = vec![0.0; render_frames];
        stream[..source.len()].copy_from_slice(source);
        let mut aligned = vec![0.0; render_frames];
        let mut probabilities = vec![0.0; BLOCK_SIZE];
        for (block, aligned_block) in stream
            .chunks_mut(BLOCK_SIZE)
            .zip(aligned.chunks_mut(BLOCK_SIZE))
        {
            detector.process(block, &mut probabilities[..block.len()]);
            let dry = block.to_vec();
            pitch.process(block);
            delay.process(&dry, aligned_block);
            preserver.process(&probabilities[..block.len()], aligned_block, block);
        }
        (
            stream[latency..latency + source.len()].to_vec(),
            aligned[latency..latency + source.len()].to_vec(),
            latency,
        )
    }

    fn estimate_fundamental(samples: &[f32], expected_hz: f32) -> f32 {
        let minimum_lag = (SAMPLE_RATE as f32 / (expected_hz * 1.2)).floor() as usize;
        let maximum_lag = (SAMPLE_RATE as f32 / (expected_hz * 0.8)).ceil() as usize;
        let mean = samples.iter().sum::<f32>() / samples.len() as f32;
        let best_lag = (minimum_lag..=maximum_lag)
            .max_by(|left_lag, right_lag| {
                normalized_correlation(samples, mean, *left_lag)
                    .total_cmp(&normalized_correlation(samples, mean, *right_lag))
            })
            .unwrap();
        SAMPLE_RATE as f32 / best_lag as f32
    }

    fn normalized_correlation(samples: &[f32], mean: f32, lag: usize) -> f32 {
        let mut cross = 0.0_f64;
        let mut left_energy = 0.0_f64;
        let mut right_energy = 0.0_f64;
        for index in 0..samples.len() - lag {
            let left = f64::from(samples[index] - mean);
            let right = f64::from(samples[index + lag] - mean);
            cross += left * right;
            left_energy += left * left;
            right_energy += right * right;
        }
        (cross / (left_energy * right_energy).sqrt().max(f64::EPSILON)) as f32
    }

    fn rms_error(left: &[f32], right: &[f32]) -> f32 {
        (left
            .iter()
            .zip(right)
            .map(|(left, right)| (left - right).powi(2))
            .sum::<f32>()
            / left.len() as f32)
            .sqrt()
    }

    fn difference_energy(samples: &[f32]) -> f32 {
        samples
            .windows(2)
            .map(|pair| (pair[1] - pair[0]).powi(2))
            .sum::<f32>()
            / (samples.len() - 1) as f32
    }

    fn harmonic_envelope_centroid(samples: &[f32], fundamental_hz: f32) -> f32 {
        let mut weighted_frequency = 0.0_f64;
        let mut total_power = 0.0_f64;
        for harmonic_index in 1..=40 {
            let frequency = fundamental_hz * harmonic_index as f32;
            if frequency >= SAMPLE_RATE as f32 * 0.45 {
                break;
            }
            let mut real = 0.0_f64;
            let mut imaginary = 0.0_f64;
            for (index, sample) in samples.iter().enumerate() {
                let phase = f64::from(TAU * frequency) * index as f64 / f64::from(SAMPLE_RATE);
                real += f64::from(*sample) * phase.cos();
                imaginary -= f64::from(*sample) * phase.sin();
            }
            let power = real * real + imaginary * imaginary;
            weighted_frequency += f64::from(frequency) * power;
            total_power += power;
        }
        (weighted_frequency / total_power.max(f64::EPSILON)) as f32
    }

    #[test]
    fn harmonic_speech_range_signals_become_strongly_voiced() {
        for frequency in [90.0, 220.0, 400.0] {
            let signal = harmonic(frequency, 48_000, 48_000);
            let probability = probabilities(&signal, 48_000, 127);
            assert!(
                steady_median(&probability) > 0.75,
                "{frequency} Hz probability was {}",
                steady_median(&probability)
            );
        }
    }

    #[test]
    fn white_fricative_and_low_level_noise_remain_unvoiced() {
        for signal in [
            deterministic_noise(48_000, 0.15),
            fricative(48_000, 0.15),
            deterministic_noise(48_000, 0.000_1),
        ] {
            let probability = probabilities(&signal, 48_000, 256);
            assert!(steady_median(&probability) < 0.25);
        }
    }

    #[test]
    fn silence_nonfinite_startup_and_reset_are_stable() {
        let mut detector = VoicingDetector::default();
        detector.prepare(48_000, 1, 256).unwrap();
        let mut input = vec![0.0; 48_000];
        input[0] = f32::NAN;
        input[1] = f32::INFINITY;
        input[2] = f32::NEG_INFINITY;
        let mut first = vec![0.0; input.len()];
        detector.process(&input, &mut first);
        assert!(first.iter().all(|value| value.is_finite() && *value == 0.0));
        detector.reset();
        let mut second = vec![0.0; input.len()];
        detector.process(&input, &mut second);
        assert_eq!(first, second);
    }

    #[test]
    fn sample_rates_and_block_boundaries_do_not_change_classification() {
        for (sample_rate, block_size) in [(44_100, 73), (48_000, 511)] {
            let signal = harmonic(220.0, sample_rate, sample_rate as usize);
            let probability = probabilities(&signal, sample_rate, block_size);
            assert!(steady_median(&probability) > 0.75);
        }
    }

    #[test]
    fn repeated_voiced_unvoiced_classification_does_not_chatter() {
        let segment_frames = SAMPLE_RATE as usize * 3 / 10;
        let mut signal = Vec::with_capacity(segment_frames * 8);
        for _ in 0..4 {
            signal.extend(harmonic(220.0, SAMPLE_RATE, segment_frames));
            signal.extend(fricative(segment_frames, 0.1));
        }
        let probability = probabilities(&signal, SAMPLE_RATE, 73);
        let crossings = probability
            .windows(2)
            .filter(|pair| (pair[0] < 0.5) != (pair[1] < 0.5))
            .count();
        assert!(
            (7..=9).contains(&crossings),
            "expected one stabilized decision per segment transition, got {crossings}"
        );
        let maximum_step = probability
            .windows(2)
            .map(|pair| (pair[1] - pair[0]).abs())
            .fold(0.0_f32, f32::max);
        assert!(
            maximum_step < 0.002,
            "probability smoothing produced a {maximum_step} per-sample step"
        );
    }

    #[test]
    fn vowel_fricative_vowel_preserves_unvoiced_audio_and_transforms_voicing() {
        let source = vowel_fricative_vowel();
        let (unpreserved, reference, latency) = render_preserved(&source, 7.0, 0.0, 0.0);
        let (preserved, preserved_reference, _) = render_preserved(&source, 7.0, 0.0, 1.0);
        assert_eq!(reference, preserved_reference);
        assert!(latency > 0);

        for (voiced_window, source_hz) in [
            (&preserved[12_000..20_000], 220.0),
            (&preserved[58_000..68_000], 180.0),
        ] {
            let expected_hz = source_hz * 2.0_f32.powf(7.0 / 12.0);
            let measured_hz = estimate_fundamental(voiced_window, expected_hz);
            assert!(
                (measured_hz - expected_hz).abs() / expected_hz < 0.03,
                "preserved voiced F0 was {measured_hz:.2} Hz, expected {expected_hz:.2} Hz"
            );
        }

        let fricative_range = 30_000..42_000;
        let preserved_error = rms_error(
            &preserved[fricative_range.clone()],
            &reference[fricative_range.clone()],
        );
        let unpreserved_error = rms_error(
            &unpreserved[fricative_range.clone()],
            &reference[fricative_range.clone()],
        );
        assert!(
            preserved_error < unpreserved_error * 0.25,
            "preserved error {preserved_error:.5} was not substantially below unpreserved {unpreserved_error:.5}"
        );
        let reference_hf = difference_energy(&reference[fricative_range.clone()]);
        let preserved_hf = difference_energy(&preserved[fricative_range.clone()]);
        let unpreserved_hf = difference_energy(&unpreserved[fricative_range]);
        assert!(
            (preserved_hf - reference_hf).abs() < (unpreserved_hf - reference_hf).abs() * 0.25,
            "preserved high-frequency energy {preserved_hf:.6} was not closer to reference {reference_hf:.6} than unpreserved {unpreserved_hf:.6}"
        );

        let maximum_step = preserved
            .windows(2)
            .map(|pair| (pair[1] - pair[0]).abs())
            .fold(0.0_f32, f32::max);
        assert!(
            maximum_step < 0.5,
            "maximum transition step was {maximum_step}"
        );
        assert!(preserved
            .iter()
            .all(|sample| sample.is_finite() && sample.abs() < 1.0));
    }

    #[test]
    fn preservation_control_moves_monotonically_toward_unvoiced_reference() {
        let source = vowel_fricative_vowel();
        let (none, reference, _) = render_preserved(&source, 12.0, 0.0, 0.0);
        let (half, _, _) = render_preserved(&source, 12.0, 0.0, 0.5);
        let (full, _, _) = render_preserved(&source, 12.0, 0.0, 1.0);
        let range = 30_000..42_000;
        let errors = [
            rms_error(&none[range.clone()], &reference[range.clone()]),
            rms_error(&half[range.clone()], &reference[range.clone()]),
            rms_error(&full[range.clone()], &reference[range]),
        ];
        assert!(
            errors[0] > errors[1] && errors[1] > errors[2],
            "preservation errors were not strictly monotonic: {errors:?}"
        );
        assert!(
            errors[1] > errors[0] * 0.35 && errors[1] < errors[0] * 0.65,
            "half-preservation error {} was not proportional to no-preservation {}",
            errors[1],
            errors[0]
        );
    }

    #[test]
    fn consonant_preservation_keeps_independent_formant_behavior_on_voicing() {
        let source = harmonic(120.0, SAMPLE_RATE, SAMPLE_RATE as usize);
        let (neutral, _, _) = render_preserved(&source, 0.0, 0.0, 1.0);
        let (raised, _, _) = render_preserved(&source, 0.0, 4.0, 1.0);
        let range = 24_000..42_000;
        let neutral_centroid = harmonic_envelope_centroid(&neutral[range.clone()], 120.0);
        let raised_centroid = harmonic_envelope_centroid(&raised[range], 120.0);
        assert!(
            raised_centroid > neutral_centroid * 1.03,
            "raised formant centroid {raised_centroid:.2} did not exceed neutral {neutral_centroid:.2}"
        );
    }

    #[test]
    fn linked_stereo_probability_and_output_are_synchronized() {
        let mono = harmonic(220.0, SAMPLE_RATE, SAMPLE_RATE as usize / 2);
        let stereo: Vec<_> = mono.iter().flat_map(|sample| [*sample, *sample]).collect();
        let mut detector = VoicingDetector::default();
        detector.prepare(SAMPLE_RATE, 2, 73).unwrap();
        let mut output = vec![0.0; mono.len()];
        for (input, probability) in stereo.chunks(73 * 2).zip(output.chunks_mut(73)) {
            detector.process(input, probability);
        }
        assert!(steady_median(&output) > 0.75);

        let mut preserver = ConsonantPreserver::default();
        preserver.set_amount(1.0);
        preserver.prepare(SAMPLE_RATE, 2, 17).unwrap();
        let aligned: Vec<_> = mono.iter().flat_map(|sample| [*sample, *sample]).collect();
        let mut transformed = aligned.clone();
        preserver.process(&output, &aligned, &mut transformed);
        assert!(transformed
            .chunks_exact(2)
            .all(|frame| (frame[0] - frame[1]).abs() <= f32::EPSILON));
    }

    #[test]
    fn repeated_transitions_and_live_amount_changes_stay_bounded_and_smooth() {
        let mut preserver = ConsonantPreserver::default();
        preserver.prepare(SAMPLE_RATE, 1, 31).unwrap();
        let frames = SAMPLE_RATE as usize;
        let mut probability = vec![0.0; frames];
        for (index, value) in probability.iter_mut().enumerate() {
            *value = 0.5 - 0.5 * (TAU * index as f32 / 2_400.0).cos();
        }
        let aligned = vec![0.2; frames];
        let mut transformed = vec![-0.2; frames];
        for (block_index, ((probability, aligned), transformed)) in probability
            .chunks(BLOCK_SIZE)
            .zip(aligned.chunks(BLOCK_SIZE))
            .zip(transformed.chunks_mut(BLOCK_SIZE))
            .enumerate()
        {
            preserver.set_amount(match block_index % 3 {
                0 => 0.0,
                1 => 0.5,
                _ => 1.0,
            });
            preserver.process(probability, aligned, transformed);
        }
        assert!(transformed
            .iter()
            .all(|sample| sample.is_finite() && sample.abs() <= 0.2));
        let maximum_step = transformed
            .windows(2)
            .map(|pair| (pair[1] - pair[0]).abs())
            .fold(0.0_f32, f32::max);
        assert!(
            maximum_step < 0.01,
            "smoothed transition step was {maximum_step}"
        );
    }

    #[test]
    fn reset_repeats_detector_and_preservation_state_exactly() {
        let source = vowel_fricative_vowel();
        let mut detector = VoicingDetector::default();
        detector.prepare(SAMPLE_RATE, 1, BLOCK_SIZE).unwrap();
        let run = |detector: &mut VoicingDetector| {
            let mut result = vec![0.0; source.len()];
            for (input, output) in source.chunks(BLOCK_SIZE).zip(result.chunks_mut(BLOCK_SIZE)) {
                detector.process(input, output);
            }
            result
        };
        let first = run(&mut detector);
        detector.reset();
        let second = run(&mut detector);
        assert_eq!(first, second);

        let mut preserver = ConsonantPreserver::default();
        preserver.set_amount(0.8);
        preserver.prepare(SAMPLE_RATE, 1, 37).unwrap();
        let aligned = source.clone();
        let process_once = |preserver: &mut ConsonantPreserver| {
            let mut transformed: Vec<_> = source.iter().map(|sample| -*sample).collect();
            for ((probability, aligned), transformed) in first
                .chunks(BLOCK_SIZE)
                .zip(aligned.chunks(BLOCK_SIZE))
                .zip(transformed.chunks_mut(BLOCK_SIZE))
            {
                preserver.process(probability, aligned, transformed);
            }
            transformed
        };
        let first_mix = process_once(&mut preserver);
        preserver.reset();
        let second_mix = process_once(&mut preserver);
        assert_eq!(first_mix, second_mix);
    }

    #[test]
    fn reported_pitch_latency_aligns_preserved_transient_without_extra_delay() {
        let mut pitch = PitchShifter::default();
        pitch.prepare(SAMPLE_RATE, 1, BLOCK_SIZE).unwrap();
        let latency = pitch.latency_frames();
        assert!(latency > 0);
        let mut dry_delay = DelayLine::new(latency);
        dry_delay.prepare(1);
        let mut input = vec![0.0; latency + 32];
        input[0] = 1.0;
        let mut aligned = vec![0.0; input.len()];
        dry_delay.process(&input, &mut aligned);

        let probabilities = vec![0.0; input.len()];
        let mut transformed = aligned.clone();
        let mut preserver = ConsonantPreserver::default();
        preserver.set_amount(1.0);
        preserver.prepare(SAMPLE_RATE, 1, latency).unwrap();
        preserver.process(&probabilities, &aligned, &mut transformed);

        assert_eq!(
            transformed.iter().position(|sample| *sample == 1.0),
            Some(latency)
        );
        assert_eq!(transformed, aligned);
    }

    #[test]
    fn recombination_sanitizes_nonfinite_audio_and_stays_finite() {
        let mut preserver = ConsonantPreserver::default();
        preserver.set_amount(1.0);
        preserver.prepare(SAMPLE_RATE, 1, 3).unwrap();
        let probabilities = [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 0.0];
        let aligned = [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 0.2];
        let mut transformed = [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -0.2];
        preserver.process(&probabilities, &aligned, &mut transformed);
        assert!(transformed.iter().all(|sample| sample.is_finite()));
    }
}
