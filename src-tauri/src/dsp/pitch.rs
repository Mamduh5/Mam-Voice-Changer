use super::{processor::AudioProcessor, signalsmith::SignalsmithStretch, smoothing::SmoothedValue};

const STRETCH_BLOCK_FRAMES: usize = 2_048;
const STRETCH_INTERVAL_FRAMES: usize = 512;
const PROCESS_CHUNK_FRAMES: usize = STRETCH_INTERVAL_FRAMES;
const PARAMETER_RAMP_MS: f32 = 20.0;

pub struct PitchShifter {
    backend: Option<SignalsmithBackend>,
    channel_count: usize,
    pitch_semitones: SmoothedValue,
    formant_shift_semitones: SmoothedValue,
    dynamic_pitch_offset_semitones: f32,
    input_scratch: Vec<f32>,
    latency_frames: usize,
}

impl Default for PitchShifter {
    fn default() -> Self {
        Self {
            backend: None,
            channel_count: 1,
            pitch_semitones: SmoothedValue::new(0.0),
            formant_shift_semitones: SmoothedValue::new(0.0),
            dynamic_pitch_offset_semitones: 0.0,
            input_scratch: Vec::new(),
            latency_frames: 0,
        }
    }
}

impl PitchShifter {
    pub fn set_pitch_semitones(&mut self, semitones: f32) {
        self.pitch_semitones.set_target(semitones);
    }

    pub fn set_formant_shift_semitones(&mut self, semitones: f32) {
        self.formant_shift_semitones.set_target(semitones);
    }

    pub fn set_dynamic_pitch_offset_semitones(&mut self, semitones: f32) {
        self.dynamic_pitch_offset_semitones = if semitones.is_finite() {
            semitones.clamp(-0.3, 0.3)
        } else {
            0.0
        };
    }

    pub const fn latency_frames(&self) -> usize {
        self.latency_frames
    }
}

impl AudioProcessor for PitchShifter {
    fn prepare(&mut self, sample_rate: u32, channels: usize, _block_size: usize) {
        self.channel_count = channels.max(1);
        self.pitch_semitones.prepare(sample_rate, PARAMETER_RAMP_MS);
        self.formant_shift_semitones
            .prepare(sample_rate, PARAMETER_RAMP_MS);
        self.pitch_semitones.reset_to_target();
        self.formant_shift_semitones.reset_to_target();

        let mut backend = SignalsmithBackend::new(self.channel_count);
        backend.set_parameters(
            self.pitch_semitones.next(),
            self.formant_shift_semitones.next(),
        );
        self.latency_frames = backend.latency_frames();
        self.backend = Some(backend);
        self.input_scratch = vec![0.0; PROCESS_CHUNK_FRAMES * self.channel_count];
    }

    fn process(&mut self, samples: &mut [f32]) {
        let Some(backend) = self.backend.as_mut() else {
            samples.fill(0.0);
            return;
        };

        let chunk_samples = PROCESS_CHUNK_FRAMES * self.channel_count;
        for output in samples.chunks_mut(chunk_samples) {
            let frames = output.len() / self.channel_count;
            let mut pitch = self.pitch_semitones.next();
            let mut formant = self.formant_shift_semitones.next();
            for _ in 1..frames {
                pitch = self.pitch_semitones.next();
                formant = self.formant_shift_semitones.next();
            }

            backend.set_parameters(pitch + self.dynamic_pitch_offset_semitones, formant);
            self.input_scratch[..output.len()].copy_from_slice(output);
            backend.process(&mut self.input_scratch[..output.len()], output);
            for sample in output {
                if !sample.is_finite() {
                    *sample = 0.0;
                }
            }
        }
    }

    fn reset(&mut self) {
        self.pitch_semitones.reset_to_target();
        self.formant_shift_semitones.reset_to_target();
        self.dynamic_pitch_offset_semitones = 0.0;
        self.input_scratch.fill(0.0);
        if let Some(backend) = self.backend.as_mut() {
            backend.reset();
            backend.set_parameters(
                self.pitch_semitones.next(),
                self.formant_shift_semitones.next(),
            );
        }
    }
}

struct SignalsmithBackend {
    stretch: SignalsmithStretch,
    latency_frames: usize,
    pitch_semitones: f32,
    formant_shift_semitones: f32,
}

impl SignalsmithBackend {
    fn new(channels: usize) -> Self {
        let stretch = SignalsmithStretch::new(
            channels.max(1),
            STRETCH_BLOCK_FRAMES,
            STRETCH_INTERVAL_FRAMES,
        )
        .expect("Signalsmith backend allocation failed");
        let latency_frames = stretch.input_latency() + stretch.output_latency();
        Self {
            stretch,
            latency_frames,
            pitch_semitones: f32::NAN,
            formant_shift_semitones: f32::NAN,
        }
    }

    fn set_parameters(&mut self, pitch_semitones: f32, formant_shift_semitones: f32) {
        if pitch_semitones.to_bits() != self.pitch_semitones.to_bits() {
            self.stretch.set_pitch_semitones(pitch_semitones);
            self.pitch_semitones = pitch_semitones;
        }
        if formant_shift_semitones.to_bits() != self.formant_shift_semitones.to_bits() {
            self.stretch
                .set_formant_semitones(formant_shift_semitones, true);
            self.formant_shift_semitones = formant_shift_semitones;
        }
    }

    fn process(&mut self, input: &mut [f32], output: &mut [f32]) {
        self.stretch.process(input, output);
    }

    fn reset(&mut self) {
        self.stretch.reset();
        self.pitch_semitones = f32::NAN;
        self.formant_shift_semitones = f32::NAN;
    }

    const fn latency_frames(&self) -> usize {
        self.latency_frames
    }
}
