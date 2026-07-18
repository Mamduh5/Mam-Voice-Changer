use std::{f32::consts::TAU, sync::Arc};

use rustfft::{num_complex::Complex, Fft, FftPlanner};

use super::{processor::AudioProcessor, smoothing::SmoothedValue};

pub const FFT_SIZE: usize = 2_048;
pub const OVERSAMPLING: usize = 4;
pub const HOP_SIZE: usize = FFT_SIZE / OVERSAMPLING;
pub const PITCH_LATENCY_FRAMES: usize = FFT_SIZE - HOP_SIZE;
const PITCH_RAMP_MS: f32 = 15.0;

pub struct PitchShifter {
    channels: Vec<PitchChannel>,
    channel_count: usize,
    pitch_semitones: SmoothedValue,
}

impl Default for PitchShifter {
    fn default() -> Self {
        Self {
            channels: Vec::new(),
            channel_count: 1,
            pitch_semitones: SmoothedValue::new(0.0),
        }
    }
}

impl PitchShifter {
    pub fn set_pitch_semitones(&mut self, semitones: f32) {
        self.pitch_semitones.set_target(semitones);
    }

    pub const fn latency_frames(&self) -> usize {
        PITCH_LATENCY_FRAMES
    }
}

impl AudioProcessor for PitchShifter {
    fn prepare(&mut self, sample_rate: u32, channels: usize, _block_size: usize) {
        self.channel_count = channels.max(1);
        self.pitch_semitones.prepare(sample_rate, PITCH_RAMP_MS);
        let mut planner = FftPlanner::<f32>::new();
        let forward = planner.plan_fft_forward(FFT_SIZE);
        let inverse = planner.plan_fft_inverse(FFT_SIZE);
        self.channels = (0..self.channel_count)
            .map(|_| PitchChannel::new(sample_rate, Arc::clone(&forward), Arc::clone(&inverse)))
            .collect();
    }

    fn process(&mut self, samples: &mut [f32]) {
        if self.channels.is_empty() {
            return;
        }

        for frame in samples.chunks_mut(self.channel_count) {
            let semitones = self.pitch_semitones.next();
            for (channel, sample) in frame.iter_mut().enumerate() {
                *sample = self.channels[channel].process_sample(*sample, semitones);
            }
        }
    }

    fn reset(&mut self) {
        for channel in &mut self.channels {
            channel.reset();
        }
        self.pitch_semitones.reset_to_target();
    }
}

struct PitchChannel {
    sample_rate: f32,
    forward: Arc<dyn Fft<f32>>,
    inverse: Arc<dyn Fft<f32>>,
    input_fifo: Vec<f32>,
    output_fifo: Vec<f32>,
    output_accumulator: Vec<f32>,
    last_phase: Vec<f32>,
    sum_phase: Vec<f32>,
    analysis_magnitude: Vec<f32>,
    analysis_frequency: Vec<f32>,
    synthesis_magnitude: Vec<f32>,
    synthesis_frequency: Vec<f32>,
    fft_buffer: Vec<Complex<f32>>,
    rover: usize,
}

impl PitchChannel {
    fn new(sample_rate: u32, forward: Arc<dyn Fft<f32>>, inverse: Arc<dyn Fft<f32>>) -> Self {
        let bins = FFT_SIZE / 2 + 1;
        Self {
            sample_rate: sample_rate as f32,
            forward,
            inverse,
            input_fifo: vec![0.0; FFT_SIZE],
            output_fifo: vec![0.0; FFT_SIZE],
            output_accumulator: vec![0.0; FFT_SIZE * 2],
            last_phase: vec![0.0; bins],
            sum_phase: vec![0.0; bins],
            analysis_magnitude: vec![0.0; bins],
            analysis_frequency: vec![0.0; bins],
            synthesis_magnitude: vec![0.0; bins],
            synthesis_frequency: vec![0.0; bins],
            fft_buffer: vec![Complex::new(0.0, 0.0); FFT_SIZE],
            rover: PITCH_LATENCY_FRAMES,
        }
    }

    fn process_sample(&mut self, input: f32, semitones: f32) -> f32 {
        self.input_fifo[self.rover] = input;
        let output = self.output_fifo[self.rover - PITCH_LATENCY_FRAMES];
        self.rover += 1;

        if self.rover >= FFT_SIZE {
            self.rover = PITCH_LATENCY_FRAMES;
            self.process_frame(2.0_f32.powf(semitones / 12.0));
        }

        output
    }

    fn process_frame(&mut self, pitch_factor: f32) {
        let frequency_per_bin = self.sample_rate / FFT_SIZE as f32;
        let expected_phase = TAU * HOP_SIZE as f32 / FFT_SIZE as f32;

        for index in 0..FFT_SIZE {
            let window = hann(index);
            self.fft_buffer[index] = Complex::new(self.input_fifo[index] * window, 0.0);
        }
        self.forward.process(&mut self.fft_buffer);

        for bin in 0..=FFT_SIZE / 2 {
            let value = self.fft_buffer[bin];
            let magnitude = 2.0 * value.norm();
            let phase = value.arg();
            let mut phase_delta = phase - self.last_phase[bin];
            self.last_phase[bin] = phase;
            phase_delta -= bin as f32 * expected_phase;
            phase_delta -= TAU * (phase_delta / TAU).round();
            let bin_deviation = OVERSAMPLING as f32 * phase_delta / TAU;

            self.analysis_magnitude[bin] = magnitude;
            self.analysis_frequency[bin] = (bin as f32 + bin_deviation) * frequency_per_bin;
        }

        self.synthesis_magnitude.fill(0.0);
        self.synthesis_frequency.fill(0.0);
        for bin in 0..=FFT_SIZE / 2 {
            let shifted_bin = (bin as f32 * pitch_factor).floor() as usize;
            if shifted_bin <= FFT_SIZE / 2 {
                self.synthesis_magnitude[shifted_bin] += self.analysis_magnitude[bin];
                self.synthesis_frequency[shifted_bin] = self.analysis_frequency[bin] * pitch_factor;
            }
        }

        self.fft_buffer.fill(Complex::new(0.0, 0.0));
        for bin in 0..=FFT_SIZE / 2 {
            let bin_frequency = bin as f32 * frequency_per_bin;
            let bin_deviation = (self.synthesis_frequency[bin] - bin_frequency) / frequency_per_bin;
            let phase_delta =
                bin as f32 * expected_phase + TAU * bin_deviation / OVERSAMPLING as f32;
            self.sum_phase[bin] += phase_delta;
            self.fft_buffer[bin] =
                Complex::from_polar(self.synthesis_magnitude[bin], self.sum_phase[bin]);
            if bin > 0 && bin < FFT_SIZE / 2 {
                self.fft_buffer[FFT_SIZE - bin] = self.fft_buffer[bin].conj();
            }
        }
        self.inverse.process(&mut self.fft_buffer);

        let scale = 4.0 / (FFT_SIZE as f32 * OVERSAMPLING as f32);
        for index in 0..FFT_SIZE {
            self.output_accumulator[index] += scale * hann(index) * self.fft_buffer[index].re;
        }
        self.output_fifo[..HOP_SIZE].copy_from_slice(&self.output_accumulator[..HOP_SIZE]);
        self.output_accumulator.copy_within(HOP_SIZE.., 0);
        self.output_accumulator[FFT_SIZE * 2 - HOP_SIZE..].fill(0.0);
        self.input_fifo.copy_within(HOP_SIZE.., 0);
    }

    fn reset(&mut self) {
        self.input_fifo.fill(0.0);
        self.output_fifo.fill(0.0);
        self.output_accumulator.fill(0.0);
        self.last_phase.fill(0.0);
        self.sum_phase.fill(0.0);
        self.analysis_magnitude.fill(0.0);
        self.analysis_frequency.fill(0.0);
        self.synthesis_magnitude.fill(0.0);
        self.synthesis_frequency.fill(0.0);
        self.fft_buffer.fill(Complex::new(0.0, 0.0));
        self.rover = PITCH_LATENCY_FRAMES;
    }
}

fn hann(index: usize) -> f32 {
    0.5 - 0.5 * (TAU * index as f32 / FFT_SIZE as f32).cos()
}

#[cfg(test)]
mod tests {
    use std::f32::consts::TAU;

    use super::{AudioProcessor, PitchShifter, FFT_SIZE, PITCH_LATENCY_FRAMES};

    const SAMPLE_RATE: u32 = 48_000;
    const BLOCK_SIZE: usize = 256;

    fn shifted_sine(semitones: f32) -> Vec<f32> {
        let frames = SAMPLE_RATE as usize * 2;
        let mut samples: Vec<f32> = (0..frames)
            .map(|index| (TAU * 440.0 * index as f32 / SAMPLE_RATE as f32).sin() * 0.5)
            .collect();
        let mut shifter = PitchShifter::default();
        shifter.prepare(SAMPLE_RATE, 1, BLOCK_SIZE);
        shifter.set_pitch_semitones(semitones);
        shifter.pitch_semitones.reset_to_target();
        for block in samples.chunks_mut(BLOCK_SIZE) {
            shifter.process(block);
        }
        samples
    }

    fn estimate_frequency(samples: &[f32]) -> f32 {
        let start = PITCH_LATENCY_FRAMES + FFT_SIZE * 2;
        let segment = &samples[start..];
        let crossings = segment
            .windows(2)
            .filter(|pair| pair[0] <= 0.0 && pair[1] > 0.0)
            .count();
        crossings as f32 * SAMPLE_RATE as f32 / segment.len() as f32
    }

    #[test]
    fn shifts_440_hz_up_one_octave() {
        let output = shifted_sine(12.0);
        assert!((estimate_frequency(&output) - 880.0).abs() < 20.0);
    }

    #[test]
    fn shifts_440_hz_down_one_octave() {
        let output = shifted_sine(-12.0);
        assert!((estimate_frequency(&output) - 220.0).abs() < 12.0);
    }

    #[test]
    fn zero_semitones_retains_unity_frequency() {
        let output = shifted_sine(0.0);
        assert!((estimate_frequency(&output) - 440.0).abs() < 10.0);
    }

    #[test]
    fn output_is_finite_and_block_processing_is_continuous() {
        let input: Vec<f32> = (0..SAMPLE_RATE as usize)
            .map(|index| (TAU * 440.0 * index as f32 / SAMPLE_RATE as f32).sin() * 0.5)
            .collect();
        let mut whole = input.clone();
        let mut blocked = input;
        let mut whole_shifter = PitchShifter::default();
        whole_shifter.prepare(SAMPLE_RATE, 1, BLOCK_SIZE);
        whole_shifter.set_pitch_semitones(5.0);
        whole_shifter.pitch_semitones.reset_to_target();
        let mut blocked_shifter = PitchShifter::default();
        blocked_shifter.prepare(SAMPLE_RATE, 1, BLOCK_SIZE);
        blocked_shifter.set_pitch_semitones(5.0);
        blocked_shifter.pitch_semitones.reset_to_target();

        whole_shifter.process(&mut whole);
        for block in blocked.chunks_mut(BLOCK_SIZE) {
            blocked_shifter.process(block);
        }

        assert!(blocked.iter().all(|sample| sample.is_finite()));
        assert!(whole
            .iter()
            .zip(&blocked)
            .all(|(left, right)| (left - right).abs() < 0.000_001));
    }
}
