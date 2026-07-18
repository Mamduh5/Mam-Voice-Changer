#[derive(Clone, Copy)]
pub struct SmoothedValue {
    current: f32,
    target: f32,
    increment: f32,
    remaining: usize,
    ramp_samples: usize,
}

impl SmoothedValue {
    pub fn new(value: f32) -> Self {
        Self {
            current: value,
            target: value,
            increment: 0.0,
            remaining: 0,
            ramp_samples: 1,
        }
    }

    pub fn prepare(&mut self, sample_rate: u32, ramp_ms: f32) {
        self.ramp_samples = ((sample_rate as f32 * ramp_ms / 1_000.0).round() as usize).max(1);
    }

    pub fn set_target(&mut self, target: f32) {
        if (target - self.target).abs() <= f32::EPSILON {
            return;
        }
        self.target = target;
        self.remaining = self.ramp_samples;
        self.increment = (self.target - self.current) / self.remaining as f32;
    }

    pub fn next(&mut self) -> f32 {
        if self.remaining > 0 {
            self.current += self.increment;
            self.remaining -= 1;
            if self.remaining == 0 {
                self.current = self.target;
            }
        }
        self.current
    }

    pub fn reset_to_target(&mut self) {
        self.current = self.target;
        self.increment = 0.0;
        self.remaining = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::SmoothedValue;

    #[test]
    fn reaches_target_over_the_configured_ramp() {
        let mut value = SmoothedValue::new(0.0);
        value.prepare(1_000, 10.0);
        value.set_target(1.0);
        let values: Vec<_> = (0..10).map(|_| value.next()).collect();

        assert!(values.windows(2).all(|pair| pair[1] > pair[0]));
        assert!((values[9] - 1.0).abs() < f32::EPSILON);
    }
}
