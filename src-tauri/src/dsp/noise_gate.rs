use super::processor::AudioProcessor;

pub const DEFAULT_GATE_THRESHOLD_DB: f32 = -48.0;
pub const MIN_GATE_THRESHOLD_DB: f32 = -80.0;
pub const MAX_GATE_THRESHOLD_DB: f32 = -10.0;
pub const HYSTERESIS_DB: f32 = 6.0;
pub const ATTACK_MS: f32 = 10.0;
pub const HOLD_MS: f32 = 120.0;
pub const RELEASE_MS: f32 = 180.0;
pub const ENVELOPE_ATTACK_MS: f32 = 5.0;
pub const ENVELOPE_RELEASE_MS: f32 = 50.0;
pub const DEFAULT_MINIMUM_GAIN_DB: f32 = -18.0;
pub const MIN_MINIMUM_GAIN_DB: f32 = -36.0;
pub const MAX_MINIMUM_GAIN_DB: f32 = 0.0;

/// Stereo-linked speech expander retained under the existing Gate control name.
/// Quiet speech approaches a configurable attenuation floor rather than hard zero.
pub struct NoiseGate {
    enabled: bool,
    channels: usize,
    open_threshold_linear: f32,
    close_threshold_linear: f32,
    minimum_gain: f32,
    envelope: f32,
    gain: f32,
    open: bool,
    hold_frames: usize,
    hold_remaining: usize,
    attack_coefficient: f32,
    release_coefficient: f32,
    envelope_attack_coefficient: f32,
    envelope_release_coefficient: f32,
    attenuated_frames: usize,
}

impl Default for NoiseGate {
    fn default() -> Self {
        let open_threshold_linear = db_to_linear(DEFAULT_GATE_THRESHOLD_DB);
        Self {
            enabled: true,
            channels: 1,
            open_threshold_linear,
            close_threshold_linear: open_threshold_linear * db_to_linear(-HYSTERESIS_DB),
            minimum_gain: db_to_linear(DEFAULT_MINIMUM_GAIN_DB),
            envelope: 0.0,
            gain: db_to_linear(DEFAULT_MINIMUM_GAIN_DB),
            open: false,
            hold_frames: 1,
            hold_remaining: 0,
            attack_coefficient: 0.0,
            release_coefficient: 0.0,
            envelope_attack_coefficient: 0.0,
            envelope_release_coefficient: 0.0,
            attenuated_frames: 0,
        }
    }
}

impl NoiseGate {
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.open = true;
            self.gain = 1.0;
            self.hold_remaining = self.hold_frames;
        }
    }

    pub fn set_threshold_db(&mut self, threshold_db: f32) {
        self.open_threshold_linear = db_to_linear(threshold_db);
        self.close_threshold_linear = self.open_threshold_linear * db_to_linear(-HYSTERESIS_DB);
    }

    pub fn set_minimum_gain_db(&mut self, gain_db: f32) {
        self.minimum_gain = db_to_linear(gain_db.clamp(MIN_MINIMUM_GAIN_DB, MAX_MINIMUM_GAIN_DB));
    }

    pub fn take_attenuated_frames(&mut self) -> usize {
        std::mem::take(&mut self.attenuated_frames)
    }
}

impl AudioProcessor for NoiseGate {
    fn prepare(&mut self, sample_rate: u32, channels: usize, _block_size: usize) {
        self.channels = channels.max(1);
        self.hold_frames =
            ((sample_rate.max(1) as f32 * HOLD_MS / 1_000.0).round() as usize).max(1);
        self.attack_coefficient = time_coefficient(sample_rate, ATTACK_MS);
        self.release_coefficient = time_coefficient(sample_rate, RELEASE_MS);
        self.envelope_attack_coefficient = time_coefficient(sample_rate, ENVELOPE_ATTACK_MS);
        self.envelope_release_coefficient = time_coefficient(sample_rate, ENVELOPE_RELEASE_MS);
    }

    fn process(&mut self, samples: &mut [f32]) {
        if !self.enabled {
            self.gain = 1.0;
            self.open = true;
            return;
        }

        for frame in samples.chunks_mut(self.channels) {
            let level = frame.iter().fold(0.0_f32, |peak, sample| {
                peak.max(if sample.is_finite() {
                    sample.abs()
                } else {
                    0.0
                })
            });
            let envelope_coefficient = if level > self.envelope {
                self.envelope_attack_coefficient
            } else {
                self.envelope_release_coefficient
            };
            self.envelope =
                envelope_coefficient * self.envelope + (1.0 - envelope_coefficient) * level;

            if self.open {
                if self.envelope >= self.close_threshold_linear {
                    self.hold_remaining = self.hold_frames;
                } else if self.hold_remaining > 0 {
                    self.hold_remaining -= 1;
                } else {
                    self.open = false;
                }
            } else if self.envelope >= self.open_threshold_linear {
                self.open = true;
                self.hold_remaining = self.hold_frames;
            }

            let target = if self.open { 1.0 } else { self.minimum_gain };
            let coefficient = if target > self.gain {
                self.attack_coefficient
            } else {
                self.release_coefficient
            };
            self.gain = coefficient * self.gain + (1.0 - coefficient) * target;
            if self.gain < 0.999 {
                self.attenuated_frames += 1;
            }
            for sample in frame {
                *sample = if sample.is_finite() {
                    *sample * self.gain
                } else {
                    0.0
                };
            }
        }
    }

    fn reset(&mut self) {
        self.envelope = 0.0;
        self.open = !self.enabled;
        self.gain = if self.enabled { self.minimum_gain } else { 1.0 };
        self.hold_remaining = 0;
        self.attenuated_frames = 0;
    }
}

fn db_to_linear(value_db: f32) -> f32 {
    10.0_f32.powf(value_db / 20.0)
}

fn time_coefficient(sample_rate: u32, time_ms: f32) -> f32 {
    (-1.0 / (sample_rate.max(1) as f32 * time_ms / 1_000.0)).exp()
}

#[cfg(test)]
mod tests {
    use super::{AudioProcessor, NoiseGate, DEFAULT_MINIMUM_GAIN_DB, HOLD_MS};

    fn expander(sample_rate: u32, channels: usize) -> NoiseGate {
        let mut expander = NoiseGate::default();
        expander.prepare(sample_rate, channels, 256);
        expander.set_threshold_db(-40.0);
        expander.reset();
        expander
    }

    #[test]
    fn disabled_state_is_exactly_neutral() {
        let mut expander = expander(48_000, 1);
        expander.set_enabled(false);
        let mut samples = [0.001, -0.25, 0.75];
        let expected = samples;
        expander.process(&mut samples);
        assert_eq!(samples, expected);
    }

    #[test]
    fn quiet_speech_reaches_a_nonzero_minimum_gain_instead_of_being_chopped() {
        let mut expander = expander(48_000, 1);
        let mut quiet = vec![0.001; 48_000];
        expander.process(&mut quiet);
        let expected_floor = 10.0_f32.powf(DEFAULT_MINIMUM_GAIN_DB / 20.0);
        let final_gain = quiet.last().copied().unwrap() / 0.001;
        assert!(final_gain > 0.0);
        assert!((final_gain - expected_floor).abs() < 0.01);
        assert!(quiet
            .windows(2)
            .all(|pair| (pair[1] - pair[0]).abs() < 0.0001));
    }

    #[test]
    fn open_close_hysteresis_and_hold_prevent_threshold_chatter() {
        let mut expander = expander(48_000, 1);
        let mut loud = vec![0.1; 4_800];
        expander.process(&mut loud);
        assert!(loud.last().copied().unwrap() > 0.09);

        let hold_frames = (48_000.0 * HOLD_MS / 1_000.0) as usize;
        let mut between_thresholds = vec![0.007; hold_frames / 2];
        expander.process(&mut between_thresholds);
        assert!(between_thresholds.last().copied().unwrap() > 0.006);

        let mut below_close = vec![0.0001; hold_frames + 48_000];
        expander.process(&mut below_close);
        assert!(below_close[hold_frames / 2] > below_close[below_close.len() - 1]);
    }

    #[test]
    fn attack_release_and_stereo_gain_are_smooth_and_linked() {
        let mut expander = expander(48_000, 2);
        let mut loud = vec![0.1; 4_800 * 2];
        expander.process(&mut loud);
        assert!(loud
            .chunks_exact(2)
            .all(|frame| (frame[0] - frame[1]).abs() < f32::EPSILON));
        assert!(loud
            .chunks_exact(2)
            .map(|frame| frame[0])
            .collect::<Vec<_>>()
            .windows(2)
            .all(|pair| (pair[1] - pair[0]).abs() < 0.01));
    }

    #[test]
    fn timing_is_sample_rate_independent_and_output_stays_finite() {
        for sample_rate in [32_000, 44_100, 48_000, 96_000] {
            let mut expander = expander(sample_rate, 1);
            let mut loud = vec![0.1; sample_rate as usize / 5];
            expander.process(&mut loud);
            let mut quiet = vec![0.001; sample_rate as usize];
            expander.process(&mut quiet);
            assert!(quiet.iter().all(|sample| sample.is_finite()));
            let gain = quiet.last().copied().unwrap() / 0.001;
            assert!((gain - 10.0_f32.powf(DEFAULT_MINIMUM_GAIN_DB / 20.0)).abs() < 0.02);
        }
    }

    #[test]
    fn enabled_expander_recovers_non_finite_input() {
        let mut expander = expander(48_000, 2);
        let mut samples = [f32::NAN, f32::INFINITY, 0.1, -0.1];
        expander.process(&mut samples);
        assert!(samples.iter().all(|sample| sample.is_finite()));
    }

    #[test]
    fn minimum_gain_is_configurable() {
        let mut expander = expander(48_000, 1);
        expander.set_minimum_gain_db(-6.0);
        expander.reset();
        let mut quiet = vec![0.001; 48_000];
        expander.process(&mut quiet);
        let gain = quiet.last().copied().unwrap() / 0.001;
        assert!((gain - 10.0_f32.powf(-6.0 / 20.0)).abs() < 0.01);
    }
}
