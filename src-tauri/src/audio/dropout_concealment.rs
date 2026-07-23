pub struct DropoutConcealer {
    channels: usize,
    maximum_gap_frames: usize,
    recovery_frames: usize,
    gap_frames: usize,
    recovery_remaining: usize,
    last_valid: Vec<f32>,
    last_output: Vec<f32>,
}

impl DropoutConcealer {
    pub fn new(channels: usize, sample_rate: u32, maximum_gap_milliseconds: u32) -> Self {
        let channels = channels.max(1);
        let maximum_gap_frames = ((sample_rate.max(1) as u64 * u64::from(maximum_gap_milliseconds))
            / 1_000)
            .max(1) as usize;
        let recovery_frames = ((sample_rate.max(1) as u64 * 2) / 1_000).max(1) as usize;
        Self {
            channels,
            maximum_gap_frames,
            recovery_frames,
            gap_frames: 0,
            recovery_remaining: 0,
            last_valid: vec![0.0; channels],
            last_output: vec![0.0; channels],
        }
    }

    /// Returns true when this frame was concealed rather than sourced from the ring.
    pub fn process_frame(&mut self, real: Option<&[f32]>, output: &mut [f32]) -> bool {
        if output.len() != self.channels {
            output.fill(0.0);
            return true;
        }
        match real.filter(|frame| frame.len() == self.channels) {
            Some(real) => {
                if self.gap_frames > 0 {
                    self.recovery_remaining = self.recovery_frames;
                }
                if self.recovery_remaining > 0 {
                    let real_mix =
                        1.0 - self.recovery_remaining as f32 / self.recovery_frames.max(1) as f32;
                    for (channel, output_sample) in output.iter_mut().enumerate() {
                        let value = finite(real[channel]);
                        *output_sample =
                            self.last_output[channel] * (1.0 - real_mix) + value * real_mix;
                        self.last_valid[channel] = value;
                        self.last_output[channel] = *output_sample;
                    }
                    self.recovery_remaining -= 1;
                } else {
                    for (channel, output_sample) in output.iter_mut().enumerate() {
                        let value = finite(real[channel]);
                        *output_sample = value;
                        self.last_valid[channel] = value;
                        self.last_output[channel] = value;
                    }
                }
                self.gap_frames = 0;
                false
            }
            None => {
                self.gap_frames = self.gap_frames.saturating_add(1);
                let decay = if self.gap_frames <= self.maximum_gap_frames {
                    1.0 - self.gap_frames as f32 / self.maximum_gap_frames as f32
                } else {
                    0.0
                };
                for (channel, output_sample) in output.iter_mut().enumerate() {
                    *output_sample = self.last_valid[channel] * decay;
                    self.last_output[channel] = *output_sample;
                }
                true
            }
        }
    }

    #[cfg(test)]
    const fn maximum_gap_frames(&self) -> usize {
        self.maximum_gap_frames
    }
}

fn finite(sample: f32) -> f32 {
    if sample.is_finite() {
        sample.clamp(-1.0, 1.0)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::DropoutConcealer;

    #[test]
    fn short_gap_decays_and_real_audio_returns_smoothly() {
        let mut concealer = DropoutConcealer::new(1, 1_000, 10);
        let mut output = [0.0];
        concealer.process_frame(Some(&[0.8]), &mut output);
        let mut concealed = Vec::new();
        for _ in 0..5 {
            assert!(concealer.process_frame(None, &mut output));
            concealed.push(output[0]);
        }
        assert!(concealed.windows(2).all(|pair| pair[1] < pair[0]));
        let before_recovery = output[0];
        assert!(!concealer.process_frame(Some(&[-0.8]), &mut output));
        assert!((output[0] - before_recovery).abs() < 0.1);
    }

    #[test]
    fn long_gap_is_strictly_bounded_and_becomes_silence() {
        let mut concealer = DropoutConcealer::new(2, 48_000, 3);
        let maximum = concealer.maximum_gap_frames();
        let mut output = [0.0; 2];
        concealer.process_frame(Some(&[0.5, -0.5]), &mut output);
        for _ in 0..=maximum {
            concealer.process_frame(None, &mut output);
            assert!(output
                .iter()
                .all(|sample| sample.is_finite() && sample.abs() <= 0.5));
        }
        assert_eq!(output, [0.0, 0.0]);
    }

    #[test]
    fn resetless_long_processing_cannot_enter_a_concealment_loop() {
        let mut concealer = DropoutConcealer::new(1, 48_000, 10);
        let mut output = [0.0];
        concealer.process_frame(Some(&[0.2]), &mut output);
        for _ in 0..1_000_000 {
            concealer.process_frame(None, &mut output);
        }
        assert_eq!(output, [0.0]);
    }
}
