use super::processor::AudioProcessor;

pub const DEFAULT_MASTER_CEILING_DB: f32 = -3.0;
pub const MIN_MASTER_CEILING_DB: f32 = -12.0;
pub const MAX_MASTER_CEILING_DB: f32 = -1.0;
const LOOKAHEAD_MS: f32 = 5.0;
const RELEASE_MS: f32 = 80.0;

pub struct MasterLimiter {
    enabled: bool,
    channels: usize,
    ceiling_linear: f32,
    delay: Vec<f32>,
    write_index: usize,
    lookahead_frames: usize,
    gain: f32,
    hold_frames: usize,
    release_coefficient: f32,
}

impl Default for MasterLimiter {
    fn default() -> Self {
        Self {
            enabled: true,
            channels: 1,
            ceiling_linear: db_to_linear(DEFAULT_MASTER_CEILING_DB),
            delay: Vec::new(),
            write_index: 0,
            lookahead_frames: 1,
            gain: 1.0,
            hold_frames: 0,
            release_coefficient: 0.0,
        }
    }
}

impl MasterLimiter {
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn set_ceiling_db(&mut self, ceiling_db: f32) {
        self.ceiling_linear = db_to_linear(ceiling_db);
    }

    pub const fn latency_frames(&self) -> usize {
        self.lookahead_frames
    }
}

impl AudioProcessor for MasterLimiter {
    fn prepare(&mut self, sample_rate: u32, channels: usize, _block_size: usize) {
        self.channels = channels.max(1);
        self.lookahead_frames =
            ((sample_rate.max(1) as f32 * LOOKAHEAD_MS / 1_000.0).round() as usize).max(1);
        self.delay = vec![0.0; self.lookahead_frames * self.channels];
        self.release_coefficient = time_coefficient(sample_rate, RELEASE_MS);
        self.reset();
    }

    fn process(&mut self, samples: &mut [f32]) {
        if self.delay.is_empty() {
            return;
        }

        for frame in samples.chunks_mut(self.channels) {
            let mut linked_peak = 0.0_f32;
            for sample in frame.iter_mut() {
                if !sample.is_finite() {
                    *sample = 0.0;
                }
                linked_peak = linked_peak.max(sample.abs());
            }

            let requested_gain = if self.enabled && linked_peak > self.ceiling_linear {
                self.ceiling_linear / linked_peak
            } else {
                1.0
            };

            if requested_gain < self.gain {
                self.gain = requested_gain;
                self.hold_frames = self.lookahead_frames;
            } else if self.hold_frames > 0 {
                self.hold_frames -= 1;
            } else {
                self.gain = self.release_coefficient * self.gain
                    + (1.0 - self.release_coefficient) * requested_gain;
            }

            for sample in frame {
                let delayed = self.delay[self.write_index];
                self.delay[self.write_index] = *sample;
                self.write_index = (self.write_index + 1) % self.delay.len();

                let limited = if delayed.is_finite() {
                    delayed * self.gain
                } else {
                    0.0
                };
                *sample = if self.enabled {
                    limited.clamp(-self.ceiling_linear, self.ceiling_linear)
                } else {
                    limited
                };
            }
        }
    }

    fn reset(&mut self) {
        self.delay.fill(0.0);
        self.write_index = 0;
        self.gain = 1.0;
        self.hold_frames = 0;
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
    use super::{AudioProcessor, MasterLimiter};

    #[test]
    fn links_channels_and_bounds_delayed_peaks() {
        let mut limiter = MasterLimiter::default();
        limiter.prepare(1_000, 2, 16);
        limiter.set_ceiling_db(-6.0);
        let mut samples = vec![0.0; 24];
        samples[2] = 2.0;
        samples[3] = 0.25;

        limiter.process(&mut samples);

        assert!(samples.iter().all(|sample| sample.is_finite()));
        assert!(samples.iter().all(|sample| sample.abs() <= 0.502));
    }
}
