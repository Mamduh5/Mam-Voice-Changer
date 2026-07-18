use super::processor::AudioProcessor;

const THRESHOLD: f32 = 0.95;
const HEADROOM: f32 = 1.0 - THRESHOLD;

pub struct SoftLimiter;

impl AudioProcessor for SoftLimiter {
    fn prepare(&mut self, _sample_rate: u32, _channels: usize, _block_size: usize) {}

    fn process(&mut self, samples: &mut [f32]) {
        for sample in samples {
            let magnitude = sample.abs();
            if magnitude.is_nan() {
                *sample = 0.0;
            } else if magnitude > THRESHOLD {
                let over_threshold = (magnitude - THRESHOLD) / HEADROOM;
                let limited = THRESHOLD + HEADROOM * (1.0 - (-over_threshold).exp());
                *sample = sample.signum() * limited.min(1.0);
            }
        }
    }

    fn reset(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::{AudioProcessor, SoftLimiter};

    #[test]
    fn leaves_safe_samples_unchanged_and_bounds_peaks() {
        let mut limiter = SoftLimiter;
        let mut samples = [0.5, -0.5, 1.5, -4.0, f32::INFINITY, f32::NAN];

        limiter.process(&mut samples);

        assert_eq!(samples[0], 0.5);
        assert_eq!(samples[1], -0.5);
        assert!(samples.iter().all(|sample| sample.is_finite()));
        assert!(samples.iter().all(|sample| sample.abs() <= 1.0));
    }
}
