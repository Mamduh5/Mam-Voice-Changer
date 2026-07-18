use super::processor::AudioProcessor;

pub struct Gain {
    linear_gain: f32,
}

impl Gain {
    pub fn new(gain_db: f32) -> Self {
        Self {
            linear_gain: db_to_linear(gain_db),
        }
    }

    pub fn set_gain_db(&mut self, gain_db: f32) {
        self.linear_gain = db_to_linear(gain_db);
    }
}

impl AudioProcessor for Gain {
    fn prepare(&mut self, _sample_rate: u32, _channels: usize, _block_size: usize) {}

    fn process(&mut self, samples: &mut [f32]) {
        for sample in samples {
            *sample *= self.linear_gain;
        }
    }

    fn reset(&mut self) {}
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
