use super::smoothing::SmoothedValue;

const MIX_RAMP_MS: f32 = 10.0;

pub struct DryWetMixer {
    delay: DelayLine,
    channels: usize,
    mix: SmoothedValue,
    latency_frames: usize,
}

impl DryWetMixer {
    pub fn new(mix: f32, latency_frames: usize) -> Self {
        Self {
            delay: DelayLine::new(latency_frames),
            channels: 1,
            mix: SmoothedValue::new(mix),
            latency_frames,
        }
    }

    pub fn prepare(&mut self, sample_rate: u32, channels: usize) {
        self.channels = channels.max(1);
        self.delay.prepare(self.channels);
        self.mix.prepare(sample_rate, MIX_RAMP_MS);
    }

    pub fn set_mix(&mut self, mix: f32) {
        self.mix.set_target(mix);
    }

    pub fn process(&mut self, dry: &[f32], wet: &mut [f32], delayed_dry: &mut [f32]) {
        self.delay.process(dry, delayed_dry);
        for ((dry_frame, wet_frame), delayed_frame) in dry
            .chunks(self.channels)
            .zip(wet.chunks_mut(self.channels))
            .zip(delayed_dry.chunks_mut(self.channels))
        {
            let mix = self.mix.next();
            for channel in 0..dry_frame.len() {
                wet_frame[channel] =
                    delayed_frame[channel] * (1.0 - mix) + wet_frame[channel] * mix;
            }
        }
    }

    pub fn reset(&mut self) {
        self.delay.reset();
        self.mix.reset_to_target();
    }

    pub const fn latency_frames(&self) -> usize {
        self.latency_frames
    }
}

pub struct DelayLine {
    buffer: Vec<f32>,
    write_index: usize,
    latency_frames: usize,
}

impl DelayLine {
    pub fn new(latency_frames: usize) -> Self {
        Self {
            buffer: Vec::new(),
            write_index: 0,
            latency_frames,
        }
    }

    pub fn prepare(&mut self, channels: usize) {
        self.buffer = vec![0.0; self.latency_frames.max(1) * channels.max(1)];
        self.write_index = 0;
    }

    pub fn process(&mut self, input: &[f32], output: &mut [f32]) {
        for (input_sample, output_sample) in input.iter().zip(output) {
            *output_sample = self.buffer[self.write_index];
            self.buffer[self.write_index] = *input_sample;
            self.write_index = (self.write_index + 1) % self.buffer.len();
        }
    }

    pub fn reset(&mut self) {
        self.buffer.fill(0.0);
        self.write_index = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::DryWetMixer;

    fn processed(mix: f32) -> (Vec<f32>, Vec<f32>, usize) {
        let mut mixer = DryWetMixer::new(mix, 2);
        mixer.prepare(48_000, 1);
        let dry = [1.0, 2.0, 3.0, 4.0];
        let mut wet = vec![10.0, 20.0, 30.0, 40.0];
        let mut delayed = vec![0.0; wet.len()];
        mixer.process(&dry, &mut wet, &mut delayed);
        (wet, delayed, mixer.latency_frames())
    }

    #[test]
    fn zero_mix_returns_latency_aligned_dry_audio() {
        let (output, delayed, latency) = processed(0.0);
        assert_eq!(latency, 2);
        assert_eq!(delayed, vec![0.0, 0.0, 1.0, 2.0]);
        assert_eq!(output, delayed);
    }

    #[test]
    fn full_mix_returns_wet_audio() {
        let (output, _, _) = processed(1.0);
        assert_eq!(output, vec![10.0, 20.0, 30.0, 40.0]);
    }

    #[test]
    fn intermediate_mix_blends_aligned_paths() {
        let (output, _, _) = processed(0.5);
        assert_eq!(output, vec![5.0, 10.0, 15.5, 21.0]);
    }
}
