use super::processor::AudioProcessor;
use super::smoothing::SmoothedValue;

const GAIN_RAMP_MS: f32 = 10.0;

pub struct Gain {
    linear_gain: SmoothedValue,
    channels: usize,
}

impl Gain {
    pub fn new(gain_db: f32) -> Self {
        Self {
            linear_gain: SmoothedValue::new(db_to_linear(gain_db)),
            channels: 1,
        }
    }

    pub fn set_gain_db(&mut self, gain_db: f32) {
        self.linear_gain.set_target(db_to_linear(gain_db));
    }
}

impl AudioProcessor for Gain {
    fn prepare(&mut self, sample_rate: u32, channels: usize, _block_size: usize) {
        self.channels = channels.max(1);
        self.linear_gain.prepare(sample_rate, GAIN_RAMP_MS);
    }

    fn process(&mut self, samples: &mut [f32]) {
        for frame in samples.chunks_mut(self.channels) {
            let gain = self.linear_gain.next();
            for sample in frame {
                *sample *= gain;
            }
        }
    }

    fn reset(&mut self) {
        self.linear_gain.reset_to_target();
    }
}

fn db_to_linear(gain_db: f32) -> f32 {
    10.0_f32.powf(gain_db / 20.0)
}

#[cfg(test)]
mod tests {
    use super::{AudioProcessor, Gain};

    #[test]
    fn applies_decibel_gain() {
        let mut gain = Gain::new(6.020_6);
        let mut samples = [0.25, -0.25];

        gain.process(&mut samples);

        assert!((samples[0] - 0.5).abs() < 0.000_1);
        assert!((samples[1] + 0.5).abs() < 0.000_1);
    }
}
