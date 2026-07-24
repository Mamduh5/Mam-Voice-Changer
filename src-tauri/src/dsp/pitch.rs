use super::{processor::AudioProcessor, signalsmith::SignalsmithStretch, smoothing::SmoothedValue};

const STRETCH_BLOCK_FRAMES: usize = 2_048;
const STRETCH_INTERVAL_FRAMES: usize = 512;
const PARAMETER_UPDATE_FRAMES: usize = 64;
const PARAMETER_RAMP_MS: f32 = 20.0;

pub struct PitchShifter {
    backend: Option<SignalsmithBackend>,
    channel_count: usize,
    pitch_semitones: SmoothedValue,
    formant_shift_semitones: SmoothedValue,
    dynamic_pitch_offset_semitones: f32,
    input_scratch: Vec<f32>,
    latency_frames: usize,
}

impl Default for PitchShifter {
    fn default() -> Self {
        Self {
            backend: None,
            channel_count: 1,
            pitch_semitones: SmoothedValue::new(0.0),
            formant_shift_semitones: SmoothedValue::new(0.0),
            dynamic_pitch_offset_semitones: 0.0,
            input_scratch: Vec::new(),
            latency_frames: 0,
        }
    }
}

impl PitchShifter {
    pub fn set_pitch_semitones(&mut self, semitones: f32) {
        self.pitch_semitones.set_target(semitones);
    }

    pub fn set_formant_shift_semitones(&mut self, semitones: f32) {
        self.formant_shift_semitones.set_target(semitones);
    }

    pub fn set_dynamic_pitch_offset_semitones(&mut self, semitones: f32) {
        self.dynamic_pitch_offset_semitones = if semitones.is_finite() {
            semitones.clamp(-0.3, 0.3)
        } else {
            0.0
        };
    }

    pub const fn latency_frames(&self) -> usize {
        self.latency_frames
    }
}

impl AudioProcessor for PitchShifter {
    fn prepare(
        &mut self,
        sample_rate: u32,
        channels: usize,
        maximum_block_size: usize,
    ) -> Result<(), String> {
        if sample_rate == 0 {
            return Err("Pitch processing requires a nonzero sample rate.".to_owned());
        }
        if channels == 0 {
            return Err("Pitch processing requires at least one channel.".to_owned());
        }
        if maximum_block_size == 0 {
            return Err("Pitch processing requires a nonzero maximum block size.".to_owned());
        }
        let scratch_samples = PARAMETER_UPDATE_FRAMES
            .checked_mul(channels)
            .ok_or_else(|| "Pitch scratch-buffer size overflowed.".to_owned())?;
        let mut backend = SignalsmithBackend::new(channels)?;
        let input_scratch = vec![0.0; scratch_samples];

        self.channel_count = channels;
        self.pitch_semitones.prepare(sample_rate, PARAMETER_RAMP_MS);
        self.formant_shift_semitones
            .prepare(sample_rate, PARAMETER_RAMP_MS);
        self.pitch_semitones.reset_to_target();
        self.formant_shift_semitones.reset_to_target();

        backend.set_parameters(
            self.pitch_semitones.next(),
            self.formant_shift_semitones.next(),
        );
        self.latency_frames = backend.latency_frames();
        self.backend = Some(backend);
        self.input_scratch = input_scratch;
        Ok(())
    }

    fn process(&mut self, samples: &mut [f32]) {
        let Some(backend) = self.backend.as_mut() else {
            samples.fill(0.0);
            return;
        };

        let chunk_samples = PARAMETER_UPDATE_FRAMES * self.channel_count;
        for output in samples.chunks_mut(chunk_samples) {
            let frames = output.len() / self.channel_count;
            let mut pitch = self.pitch_semitones.next();
            let mut formant = self.formant_shift_semitones.next();
            for _ in 1..frames {
                pitch = self.pitch_semitones.next();
                formant = self.formant_shift_semitones.next();
            }

            backend.set_parameters(pitch + self.dynamic_pitch_offset_semitones, formant);
            for (input_sample, output_sample) in self.input_scratch[..output.len()]
                .iter_mut()
                .zip(output.iter())
            {
                *input_sample = if output_sample.is_finite() {
                    *output_sample
                } else {
                    0.0
                };
            }
            let _ = backend.process(&mut self.input_scratch[..output.len()], output);
            for sample in output {
                if !sample.is_finite() {
                    *sample = 0.0;
                }
            }
        }
    }

    fn reset(&mut self) {
        self.pitch_semitones.reset_to_target();
        self.formant_shift_semitones.reset_to_target();
        self.dynamic_pitch_offset_semitones = 0.0;
        self.input_scratch.fill(0.0);
        if let Some(backend) = self.backend.as_mut() {
            backend.reset();
            backend.set_parameters(
                self.pitch_semitones.next(),
                self.formant_shift_semitones.next(),
            );
        }
    }
}

struct SignalsmithBackend {
    stretch: SignalsmithStretch,
    latency_frames: usize,
    pitch_semitones: f32,
    formant_shift_semitones: f32,
}

impl SignalsmithBackend {
    fn new(channels: usize) -> Result<Self, String> {
        let stretch =
            SignalsmithStretch::new(channels, STRETCH_BLOCK_FRAMES, STRETCH_INTERVAL_FRAMES)?;
        let latency_frames = stretch.input_latency() + stretch.output_latency();
        Ok(Self {
            stretch,
            latency_frames,
            pitch_semitones: f32::NAN,
            formant_shift_semitones: f32::NAN,
        })
    }

    fn set_parameters(&mut self, pitch_semitones: f32, formant_shift_semitones: f32) {
        if pitch_semitones.to_bits() != self.pitch_semitones.to_bits() {
            self.stretch.set_pitch_semitones(pitch_semitones);
            self.pitch_semitones = pitch_semitones;
        }
        if formant_shift_semitones.to_bits() != self.formant_shift_semitones.to_bits() {
            self.stretch
                .set_formant_semitones(formant_shift_semitones, true);
            self.formant_shift_semitones = formant_shift_semitones;
        }
    }

    fn process(&mut self, input: &mut [f32], output: &mut [f32]) -> Result<(), &'static str> {
        self.stretch.process(input, output)
    }

    fn reset(&mut self) {
        self.stretch.reset();
        self.pitch_semitones = f32::NAN;
        self.formant_shift_semitones = f32::NAN;
    }

    const fn latency_frames(&self) -> usize {
        self.latency_frames
    }
}

#[cfg(test)]
mod tests {
    use std::f32::consts::TAU;

    use super::{AudioProcessor, PitchShifter};
    use crate::dsp::chain::DspParameters;

    const SAMPLE_RATE: u32 = 48_000;
    const STREAM_BLOCK_FRAMES: usize = 256;
    const SOURCE_FRAMES: usize = SAMPLE_RATE as usize * 2;
    const ANALYSIS_START: usize = SAMPLE_RATE as usize / 2;
    const ANALYSIS_FRAMES: usize = SAMPLE_RATE as usize;

    fn sine(frequency_hz: f32, frames: usize, amplitude: f32) -> Vec<f32> {
        (0..frames)
            .map(|frame| (TAU * frequency_hz * frame as f32 / SAMPLE_RATE as f32).sin() * amplitude)
            .collect()
    }

    fn vowel_like(fundamental_hz: f32, frames: usize) -> Vec<f32> {
        let harmonics: Vec<_> = (1..=40)
            .filter_map(|harmonic| {
                let frequency = fundamental_hz * harmonic as f32;
                (frequency < SAMPLE_RATE as f32 * 0.45).then(|| {
                    let first_formant = (-0.5 * ((frequency - 700.0) / 180.0).powi(2)).exp();
                    let second_formant =
                        0.7 * (-0.5 * ((frequency - 1_300.0) / 260.0).powi(2)).exp();
                    let floor = 0.02 / harmonic as f32;
                    (frequency, first_formant + second_formant + floor)
                })
            })
            .collect();
        let normalization = harmonics.iter().map(|(_, weight)| weight).sum::<f32>();
        (0..frames)
            .map(|frame| {
                harmonics
                    .iter()
                    .map(|(frequency, weight)| {
                        weight * (TAU * frequency * frame as f32 / SAMPLE_RATE as f32).sin()
                    })
                    .sum::<f32>()
                    * (0.35 / normalization)
            })
            .collect()
    }

    fn render(source: &[f32], pitch: f32, formant: f32) -> (Vec<f32>, usize) {
        let mut processor = PitchShifter::default();
        processor.set_pitch_semitones(pitch);
        processor.set_formant_shift_semitones(formant);
        processor
            .prepare(SAMPLE_RATE, 1, STREAM_BLOCK_FRAMES)
            .unwrap();
        processor.reset();
        let latency = processor.latency_frames();
        let render_frames =
            (source.len() + latency).div_ceil(STREAM_BLOCK_FRAMES) * STREAM_BLOCK_FRAMES;
        let mut stream = vec![0.0; render_frames];
        stream[..source.len()].copy_from_slice(source);
        for block in stream.chunks_mut(STREAM_BLOCK_FRAMES) {
            processor.process(block);
        }
        (stream[latency..latency + source.len()].to_vec(), latency)
    }

    fn analysis_window(samples: &[f32]) -> &[f32] {
        &samples[ANALYSIS_START..ANALYSIS_START + ANALYSIS_FRAMES]
    }

    /// Normalized autocorrelation is searched only around the known test-note
    /// period. This deliberately avoids octave errors on harmonic material and
    /// is not intended to be a general-purpose speech pitch tracker.
    fn estimate_fundamental(samples: &[f32], expected_hz: f32) -> f32 {
        let minimum_hz = expected_hz * 0.8;
        let maximum_hz = expected_hz * 1.2;
        let minimum_lag = (SAMPLE_RATE as f32 / maximum_hz).floor() as usize;
        let maximum_lag = (SAMPLE_RATE as f32 / minimum_hz).ceil() as usize;
        let mean = samples.iter().copied().sum::<f32>() / samples.len() as f32;
        let correlation = |lag: usize| {
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
            cross / (left_energy * right_energy).sqrt().max(f64::EPSILON)
        };
        let correlations: Vec<_> = (minimum_lag..=maximum_lag).map(correlation).collect();
        let best_offset = correlations
            .iter()
            .enumerate()
            .max_by(|left, right| left.1.total_cmp(right.1))
            .map(|(index, _)| index)
            .unwrap();
        let best_lag = minimum_lag + best_offset;
        let interpolated_lag = if best_offset > 0 && best_offset + 1 < correlations.len() {
            let left = correlations[best_offset - 1];
            let center = correlations[best_offset];
            let right = correlations[best_offset + 1];
            let denominator = left - 2.0 * center + right;
            if denominator.abs() > f64::EPSILON {
                best_lag as f64 + 0.5 * (left - right) / denominator
            } else {
                best_lag as f64
            }
        } else {
            best_lag as f64
        };
        SAMPLE_RATE as f32 / interpolated_lag as f32
    }

    /// The formant test samples energy only at the unchanged harmonic
    /// frequencies, then compares their power-weighted spectral centroid.
    fn harmonic_envelope_centroid(samples: &[f32], fundamental_hz: f32) -> f32 {
        let mut weighted_frequency = 0.0_f64;
        let mut total_power = 0.0_f64;
        for harmonic in 1..=40 {
            let frequency = fundamental_hz * harmonic as f32;
            if frequency >= SAMPLE_RATE as f32 * 0.45 {
                break;
            }
            let mut real = 0.0_f64;
            let mut imaginary = 0.0_f64;
            for (index, sample) in samples.iter().enumerate() {
                let phase = f64::from(TAU * frequency) * index as f64 / f64::from(SAMPLE_RATE);
                let window = 0.5 - 0.5 * (TAU * index as f32 / (samples.len() - 1) as f32).cos();
                real += f64::from(*sample * window) * phase.cos();
                imaginary -= f64::from(*sample * window) * phase.sin();
            }
            let power = real * real + imaginary * imaginary;
            weighted_frequency += f64::from(frequency) * power;
            total_power += power;
        }
        (weighted_frequency / total_power.max(f64::EPSILON)) as f32
    }

    fn rms(samples: &[f32]) -> f32 {
        (samples.iter().map(|sample| sample * sample).sum::<f32>() / samples.len() as f32).sqrt()
    }

    fn assert_frequency(actual: f32, expected: f32, tolerance: f32) {
        let relative_error = (actual - expected).abs() / expected;
        assert!(
            relative_error <= tolerance,
            "measured {actual:.3} Hz, expected {expected:.3} Hz (relative error {relative_error:.4})"
        );
    }

    #[test]
    fn positive_octave_transposes_220_hz_to_440_hz() {
        let source = sine(220.0, SOURCE_FRAMES, 0.2);
        let (output, _) = render(&source, 12.0, 0.0);
        let measured = estimate_fundamental(analysis_window(&output), 440.0);
        assert_frequency(measured, 440.0, 0.02);
    }

    #[test]
    fn negative_octave_transposes_220_hz_to_110_hz() {
        let source = sine(220.0, SOURCE_FRAMES, 0.2);
        let (output, _) = render(&source, -12.0, 0.0);
        let measured = estimate_fundamental(analysis_window(&output), 110.0);
        assert_frequency(measured, 110.0, 0.02);
    }

    #[test]
    fn seven_semitones_matches_the_equal_tempered_frequency_ratio() {
        let source = sine(220.0, SOURCE_FRAMES, 0.2);
        let (output, _) = render(&source, 7.0, 0.0);
        let expected = 220.0 * 2.0_f32.powf(7.0 / 12.0);
        let measured = estimate_fundamental(analysis_window(&output), expected);
        assert_frequency(measured, expected, 0.02);
    }

    #[test]
    fn live_stream_and_aligned_render_preserve_duration() {
        let source = sine(220.0, SOURCE_FRAMES + 137, 0.2);
        let (output, latency) = render(&source, 5.0, -2.0);
        assert_eq!(output.len(), source.len());
        assert!(latency > 0);
        println!("Signalsmith algorithmic latency: {latency} frames");
    }

    #[test]
    fn formant_shift_moves_the_envelope_without_moving_fundamental() {
        let source = vowel_like(120.0, SOURCE_FRAMES);
        let (neutral, _) = render(&source, 0.0, 0.0);
        let (raised, _) = render(&source, 0.0, 4.0);
        let neutral_window = analysis_window(&neutral);
        let raised_window = analysis_window(&raised);
        let neutral_f0 = estimate_fundamental(neutral_window, 120.0);
        let raised_f0 = estimate_fundamental(raised_window, 120.0);
        assert_frequency(neutral_f0, 120.0, 0.01);
        assert_frequency(raised_f0, neutral_f0, 0.01);
        let neutral_centroid = harmonic_envelope_centroid(neutral_window, 120.0);
        let raised_centroid = harmonic_envelope_centroid(raised_window, 120.0);
        assert!(
            raised_centroid > neutral_centroid * 1.03,
            "raised centroid {raised_centroid:.2} Hz did not exceed neutral {neutral_centroid:.2} Hz"
        );
    }

    #[test]
    fn neutral_configuration_is_finite_pitch_stable_and_level_bounded() {
        let source = sine(220.0, SOURCE_FRAMES, 0.2);
        let (output, _) = render(&source, 0.0, 0.0);
        assert_eq!(output.len(), source.len());
        assert!(output.iter().all(|sample| sample.is_finite()));
        assert_frequency(
            estimate_fundamental(analysis_window(&output), 220.0),
            220.0,
            0.01,
        );
        let level_ratio = rms(analysis_window(&output)) / rms(analysis_window(&source));
        assert!(
            (0.8..=1.2).contains(&level_ratio),
            "neutral level ratio was {level_ratio:.3}"
        );
    }

    #[test]
    fn live_parameter_transitions_stay_finite_bounded_and_continuous() {
        let mut processor = PitchShifter::default();
        processor
            .prepare(SAMPLE_RATE, 1, STREAM_BLOCK_FRAMES)
            .unwrap();
        processor.reset();
        let mut output = Vec::with_capacity(STREAM_BLOCK_FRAMES * 240);
        for block_index in 0..240 {
            if block_index == 80 {
                processor.set_pitch_semitones(7.0);
                processor.set_formant_shift_semitones(4.0);
            } else if block_index == 160 {
                processor.set_pitch_semitones(-4.0);
                processor.set_formant_shift_semitones(-3.0);
            }
            let first_frame = block_index * STREAM_BLOCK_FRAMES;
            let mut block: Vec<_> = (0..STREAM_BLOCK_FRAMES)
                .map(|offset| {
                    (TAU * 220.0 * (first_frame + offset) as f32 / SAMPLE_RATE as f32).sin() * 0.1
                })
                .collect();
            processor.process(&mut block);
            output.extend(block);
        }
        assert!(output
            .iter()
            .all(|sample| sample.is_finite() && sample.abs() < 1.0));
        let stable_output = &output[processor.latency_frames().min(output.len() - 1)..];
        let maximum_step = stable_output
            .windows(2)
            .map(|pair| (pair[1] - pair[0]).abs())
            .fold(0.0_f32, f32::max);
        assert!(
            maximum_step < 0.25,
            "transition produced a {maximum_step:.4} single-sample step"
        );
    }

    #[test]
    fn reset_repeats_the_same_signal_and_parameter_sequence() {
        let source = sine(220.0, SOURCE_FRAMES, 0.2);
        let mut processor = PitchShifter::default();
        processor.set_pitch_semitones(3.0);
        processor.set_formant_shift_semitones(-2.0);
        processor
            .prepare(SAMPLE_RATE, 1, STREAM_BLOCK_FRAMES)
            .unwrap();
        let latency = processor.latency_frames();
        let render_frames =
            (source.len() + latency).div_ceil(STREAM_BLOCK_FRAMES) * STREAM_BLOCK_FRAMES;
        let process_once = |processor: &mut PitchShifter| {
            let mut stream = vec![0.0; render_frames];
            stream[..source.len()].copy_from_slice(&source);
            for block in stream.chunks_mut(STREAM_BLOCK_FRAMES) {
                processor.process(block);
            }
            stream
        };
        processor.reset();
        let first = process_once(&mut processor);
        processor.reset();
        let second = process_once(&mut processor);
        let maximum_difference = first
            .iter()
            .zip(second)
            .map(|(left, right)| (left - right).abs())
            .fold(0.0_f32, f32::max);
        assert!(maximum_difference <= 1.0e-6);
    }

    #[test]
    fn mono_and_stereo_processing_remain_channel_synchronized() {
        let mono = sine(220.0, SOURCE_FRAMES, 0.2);
        let mut stereo: Vec<_> = mono.iter().flat_map(|sample| [*sample, *sample]).collect();
        let mut processor = PitchShifter::default();
        processor.set_pitch_semitones(5.0);
        processor.set_formant_shift_semitones(2.0);
        processor
            .prepare(SAMPLE_RATE, 2, STREAM_BLOCK_FRAMES)
            .unwrap();
        processor.reset();
        for block in stereo.chunks_mut(STREAM_BLOCK_FRAMES * 2) {
            processor.process(block);
        }
        let maximum_channel_difference = stereo
            .chunks_exact(2)
            .map(|frame| (frame[0] - frame[1]).abs())
            .fold(0.0_f32, f32::max);
        println!("Maximum identical-input stereo deviation: {maximum_channel_difference:.8}");
        assert!(
            maximum_channel_difference <= 1.0e-3,
            "identical stereo channels diverged by {maximum_channel_difference}"
        );
    }

    #[test]
    fn preparation_rejects_unsupported_configuration_without_panicking() {
        let mut processor = PitchShifter::default();
        assert!(processor.prepare(0, 1, 256).is_err());
        assert!(processor.prepare(SAMPLE_RATE, 0, 256).is_err());
        assert!(processor.prepare(SAMPLE_RATE, 1, 0).is_err());
    }

    #[test]
    fn repreparation_replaces_state_safely_and_failed_reprepare_keeps_the_old_backend() {
        let mut processor = PitchShifter::default();
        processor
            .prepare(SAMPLE_RATE, 1, STREAM_BLOCK_FRAMES)
            .unwrap();
        let mono_latency = processor.latency_frames();
        processor.prepare(44_100, 2, 127).unwrap();
        assert_eq!(processor.channel_count, 2);
        assert_eq!(processor.latency_frames(), mono_latency);
        assert!(processor.prepare(44_100, 0, 127).is_err());
        let mut stereo = vec![0.1; 127 * 2];
        processor.process(&mut stereo);
        assert!(stereo.iter().all(|sample| sample.is_finite()));
    }

    #[test]
    fn invalid_input_samples_are_converted_to_safe_finite_output() {
        let mut processor = PitchShifter::default();
        processor
            .prepare(SAMPLE_RATE, 1, STREAM_BLOCK_FRAMES)
            .unwrap();
        let mut samples = vec![0.0; STREAM_BLOCK_FRAMES];
        samples[0] = f32::NAN;
        samples[1] = f32::INFINITY;
        samples[2] = f32::NEG_INFINITY;
        processor.process(&mut samples);
        assert!(samples.iter().all(|sample| sample.is_finite()));
    }

    #[test]
    fn parameter_validation_rejects_every_nonfinite_and_out_of_range_boundary() {
        for invalid_pitch in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -12.01, 12.01] {
            assert!(DspParameters {
                pitch_semitones: invalid_pitch,
                ..DspParameters::default()
            }
            .validate()
            .is_err());
        }
        for invalid_formant in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -6.01, 6.01] {
            assert!(DspParameters {
                formant_shift_semitones: invalid_formant,
                ..DspParameters::default()
            }
            .validate()
            .is_err());
        }
    }
}
