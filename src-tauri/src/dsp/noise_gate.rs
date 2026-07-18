use super::processor::AudioProcessor;

pub const DEFAULT_GATE_THRESHOLD_DB: f32 = -50.0;
pub const MIN_GATE_THRESHOLD_DB: f32 = -80.0;
pub const MAX_GATE_THRESHOLD_DB: f32 = -10.0;
pub const ATTACK_MS: f32 = 10.0;
pub const RELEASE_MS: f32 = 120.0;
pub const ENVELOPE_ATTACK_MS: f32 = 5.0;
pub const ENVELOPE_RELEASE_MS: f32 = 50.0;
pub const HYSTERESIS_DB: f32 = 6.0;

pub struct NoiseGate {
    enabled: bool,
    channels: usize,
    threshold_linear: f32,
    close_threshold_linear: f32,
    envelope: f32,
    gain: f32,
    open: bool,
    attack_coefficient: f32,
    release_coefficient: f32,
    envelope_attack_coefficient: f32,
    envelope_release_coefficient: f32,
}

impl Default for NoiseGate {
    fn default() -> Self {
        let threshold_linear = db_to_linear(DEFAULT_GATE_THRESHOLD_DB);
        Self {
            enabled: true,
            channels: 1,
            threshold_linear,
            close_threshold_linear: threshold_linear * db_to_linear(-HYSTERESIS_DB),
            envelope: 0.0,
            gain: 0.0,
            open: false,
            attack_coefficient: 0.0,
            release_coefficient: 0.0,
            envelope_attack_coefficient: 0.0,
            envelope_release_coefficient: 0.0,
        }
    }
}

impl NoiseGate {
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn set_threshold_db(&mut self, threshold_db: f32) {
        self.threshold_linear = db_to_linear(threshold_db);
        self.close_threshold_linear = self.threshold_linear * db_to_linear(-HYSTERESIS_DB);
    }
}

impl AudioProcessor for NoiseGate {
    fn prepare(&mut self, sample_rate: u32, channels: usize, _block_size: usize) {
        self.channels = channels.max(1);
        self.attack_coefficient = time_coefficient(sample_rate, ATTACK_MS);
        self.release_coefficient = time_coefficient(sample_rate, RELEASE_MS);
        self.envelope_attack_coefficient = time_coefficient(sample_rate, ENVELOPE_ATTACK_MS);
        self.envelope_release_coefficient = time_coefficient(sample_rate, ENVELOPE_RELEASE_MS);
    }

    fn process(&mut self, samples: &mut [f32]) {
        for frame in samples.chunks_mut(self.channels) {
            let level = frame
                .iter()
                .fold(0.0_f32, |peak, sample| peak.max(sample.abs()));
            let envelope_coefficient = if level > self.envelope {
                self.envelope_attack_coefficient
            } else {
                self.envelope_release_coefficient
            };
            self.envelope =
                envelope_coefficient * self.envelope + (1.0 - envelope_coefficient) * level;

            if !self.enabled {
                self.open = true;
            } else if self.open {
                if self.envelope < self.close_threshold_linear {
                    self.open = false;
                }
            } else if self.envelope >= self.threshold_linear {
                self.open = true;
            }

            let target = if self.open { 1.0 } else { 0.0 };
            let gain_coefficient = if target > self.gain {
                self.attack_coefficient
            } else {
                self.release_coefficient
            };
            self.gain = gain_coefficient * self.gain + (1.0 - gain_coefficient) * target;
            for sample in frame {
                *sample *= self.gain;
            }
        }
    }

    fn reset(&mut self) {
        self.envelope = 0.0;
        self.open = !self.enabled;
        self.gain = if self.enabled { 0.0 } else { 1.0 };
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
    use super::{AudioProcessor, NoiseGate};

    fn gate() -> NoiseGate {
        let mut gate = NoiseGate::default();
        gate.prepare(48_000, 2, 256);
        gate.set_threshold_db(-40.0);
        gate.reset();
        gate
    }

    #[test]
    fn closes_below_threshold_with_coherent_channels() {
        let mut gate = gate();
        let mut samples = vec![0.001; 48_000 * 2];

        gate.process(&mut samples);

        assert!(samples[samples.len() - 2].abs() < 0.000_01);
        assert_eq!(samples[samples.len() - 2], samples[samples.len() - 1]);
    }

    #[test]
    fn opens_above_threshold() {
        let mut gate = gate();
        let mut samples = vec![0.1; 4_800 * 2];

        gate.process(&mut samples);

        assert!(samples[samples.len() - 2] > 0.09);
        assert_eq!(samples[samples.len() - 2], samples[samples.len() - 1]);
    }

    #[test]
    fn attack_and_release_do_not_hard_toggle() {
        let mut gate = gate();
        let mut loud = vec![0.1; 4_800 * 2];
        gate.process(&mut loud);
        let first_open_sample = loud.iter().copied().find(|sample| *sample > 0.0).unwrap();
        assert!(first_open_sample < 0.1);

        let mut quiet = vec![0.001; 48_000 * 2];
        gate.process(&mut quiet);
        assert!(quiet[0] > 0.0);
        assert!(quiet.last().copied().unwrap() < quiet[0]);
    }
}
