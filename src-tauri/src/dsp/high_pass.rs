use std::f32::consts::TAU;

use super::processor::AudioProcessor;

const CUTOFF_HZ: f32 = 20.0;

pub struct HighPass {
    coefficient: f32,
    previous_input: Vec<f32>,
    previous_output: Vec<f32>,
    channel_cursor: usize,
}

impl Default for HighPass {
    fn default() -> Self {
        Self {
            coefficient: 0.0,
            previous_input: Vec::new(),
            previous_output: Vec::new(),
            channel_cursor: 0,
        }
    }
}

impl AudioProcessor for HighPass {
    fn prepare(&mut self, sample_rate: u32, channels: usize, _block_size: usize) {
        self.coefficient = (-TAU * CUTOFF_HZ / sample_rate.max(1) as f32).exp();
        self.previous_input = vec![0.0; channels.max(1)];
        self.previous_output = vec![0.0; channels.max(1)];
        self.channel_cursor = 0;
    }

    fn process(&mut self, samples: &mut [f32]) {
        let channels = self.previous_input.len();
        if channels == 0 {
            return;
        }

        for sample in samples {
            let channel = self.channel_cursor;
            let input = *sample;
            let output = input - self.previous_input[channel]
                + self.coefficient * self.previous_output[channel];
            self.previous_input[channel] = input;
            self.previous_output[channel] = output;
            *sample = output;
            self.channel_cursor = (self.channel_cursor + 1) % channels;
        }
    }

    fn reset(&mut self) {
        self.previous_input.fill(0.0);
        self.previous_output.fill(0.0);
        self.channel_cursor = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::{AudioProcessor, HighPass};

    #[test]
    fn rejects_a_dc_signal_without_becoming_unstable() {
        let mut filter = HighPass::default();
        filter.prepare(48_000, 1, 256);
        let mut samples = vec![1.0; 48_000];

        filter.process(&mut samples);

        assert!(samples.iter().all(|sample| sample.is_finite()));
        assert!(samples.last().copied().unwrap_or_default().abs() < 0.000_1);
    }

    #[test]
    fn keeps_channel_history_independent() {
        let mut filter = HighPass::default();
        filter.prepare(48_000, 2, 4);
        let mut samples = [1.0, 0.0, 1.0, 0.0];

        filter.process(&mut samples);

        assert_eq!(samples[1], 0.0);
        assert_eq!(samples[3], 0.0);
    }
}
