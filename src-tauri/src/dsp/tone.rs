use std::f32::consts::PI;

use super::processor::AudioProcessor;

pub const MIN_TONE_DB: f32 = -6.0;
pub const MAX_TONE_DB: f32 = 6.0;
const WARMTH_FREQUENCY_HZ: f32 = 200.0;
const BRIGHTNESS_FREQUENCY_HZ: f32 = 4_000.0;
const COEFFICIENT_RAMP_MS: f32 = 20.0;

pub struct ToneEq {
    warmth: ShelfFilter,
    brightness: ShelfFilter,
}

impl Default for ToneEq {
    fn default() -> Self {
        Self {
            warmth: ShelfFilter::new(ShelfKind::Low, WARMTH_FREQUENCY_HZ),
            brightness: ShelfFilter::new(ShelfKind::High, BRIGHTNESS_FREQUENCY_HZ),
        }
    }
}

impl ToneEq {
    pub fn set_warmth_db(&mut self, gain_db: f32) {
        self.warmth.set_gain_db(gain_db);
    }

    pub fn set_brightness_db(&mut self, gain_db: f32) {
        self.brightness.set_gain_db(gain_db);
    }
}

impl AudioProcessor for ToneEq {
    fn prepare(&mut self, sample_rate: u32, channels: usize, block_size: usize) {
        self.warmth.prepare(sample_rate, channels, block_size);
        self.brightness.prepare(sample_rate, channels, block_size);
    }

    fn process(&mut self, samples: &mut [f32]) {
        self.warmth.process(samples);
        self.brightness.process(samples);
    }

    fn reset(&mut self) {
        self.warmth.reset();
        self.brightness.reset();
    }
}

#[derive(Clone, Copy)]
enum ShelfKind {
    Low,
    High,
}

struct ShelfFilter {
    kind: ShelfKind,
    frequency_hz: f32,
    sample_rate: f32,
    channels: usize,
    gain_db: f32,
    target_gain_db: f32,
    coefficients: Coefficients,
    target_coefficients: Coefficients,
    coefficient_step: Coefficients,
    ramp_frames: usize,
    remaining_frames: usize,
    states: Vec<FilterState>,
}

impl ShelfFilter {
    fn new(kind: ShelfKind, frequency_hz: f32) -> Self {
        Self {
            kind,
            frequency_hz,
            sample_rate: 48_000.0,
            channels: 1,
            gain_db: 0.0,
            target_gain_db: 0.0,
            coefficients: Coefficients::UNITY,
            target_coefficients: Coefficients::UNITY,
            coefficient_step: Coefficients::ZERO,
            ramp_frames: 1,
            remaining_frames: 0,
            states: Vec::new(),
        }
    }

    fn set_gain_db(&mut self, gain_db: f32) {
        if gain_db.to_bits() == self.target_gain_db.to_bits() {
            return;
        }
        self.target_gain_db = gain_db;
        self.target_coefficients =
            shelf_coefficients(self.kind, self.sample_rate, self.frequency_hz, gain_db);
        self.coefficient_step = self
            .target_coefficients
            .subtract(self.coefficients)
            .scale(1.0 / self.ramp_frames as f32);
        self.remaining_frames = self.ramp_frames;
    }

    fn next_coefficients(&mut self) -> Coefficients {
        if self.remaining_frames > 0 {
            self.coefficients = self.coefficients.add(self.coefficient_step);
            self.remaining_frames -= 1;
            if self.remaining_frames == 0 {
                self.coefficients = self.target_coefficients;
                self.gain_db = self.target_gain_db;
            }
        }
        self.coefficients
    }
}

impl AudioProcessor for ShelfFilter {
    fn prepare(&mut self, sample_rate: u32, channels: usize, _block_size: usize) {
        self.sample_rate = sample_rate.max(1) as f32;
        self.channels = channels.max(1);
        self.ramp_frames =
            ((self.sample_rate * COEFFICIENT_RAMP_MS / 1_000.0).round() as usize).max(1);
        self.coefficients =
            shelf_coefficients(self.kind, self.sample_rate, self.frequency_hz, self.gain_db);
        self.target_coefficients = shelf_coefficients(
            self.kind,
            self.sample_rate,
            self.frequency_hz,
            self.target_gain_db,
        );
        self.coefficient_step = self
            .target_coefficients
            .subtract(self.coefficients)
            .scale(1.0 / self.ramp_frames as f32);
        self.remaining_frames = self.ramp_frames;
        self.states = vec![FilterState::default(); self.channels];
    }

    fn process(&mut self, samples: &mut [f32]) {
        for frame in samples.chunks_mut(self.channels) {
            let coefficients = self.next_coefficients();
            for (channel, sample) in frame.iter_mut().enumerate() {
                let input = if sample.is_finite() { *sample } else { 0.0 };
                let state = &mut self.states[channel];
                let output = coefficients.b0 * input
                    + coefficients.b1 * state.x1
                    + coefficients.b2 * state.x2
                    - coefficients.a1 * state.y1
                    - coefficients.a2 * state.y2;
                if output.is_finite() {
                    state.x2 = state.x1;
                    state.x1 = input;
                    state.y2 = state.y1;
                    state.y1 = output;
                    *sample = output;
                } else {
                    *state = FilterState::default();
                    *sample = 0.0;
                }
            }
        }
    }

    fn reset(&mut self) {
        self.states.fill(FilterState::default());
        self.coefficients = self.target_coefficients;
        self.coefficient_step = Coefficients::ZERO;
        self.remaining_frames = 0;
        self.gain_db = self.target_gain_db;
    }
}

#[derive(Clone, Copy, Default)]
struct FilterState {
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
}

#[derive(Clone, Copy)]
struct Coefficients {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
}

impl Coefficients {
    const UNITY: Self = Self {
        b0: 1.0,
        b1: 0.0,
        b2: 0.0,
        a1: 0.0,
        a2: 0.0,
    };
    const ZERO: Self = Self {
        b0: 0.0,
        b1: 0.0,
        b2: 0.0,
        a1: 0.0,
        a2: 0.0,
    };

    fn add(self, other: Self) -> Self {
        Self {
            b0: self.b0 + other.b0,
            b1: self.b1 + other.b1,
            b2: self.b2 + other.b2,
            a1: self.a1 + other.a1,
            a2: self.a2 + other.a2,
        }
    }

    fn subtract(self, other: Self) -> Self {
        Self {
            b0: self.b0 - other.b0,
            b1: self.b1 - other.b1,
            b2: self.b2 - other.b2,
            a1: self.a1 - other.a1,
            a2: self.a2 - other.a2,
        }
    }

    fn scale(self, factor: f32) -> Self {
        Self {
            b0: self.b0 * factor,
            b1: self.b1 * factor,
            b2: self.b2 * factor,
            a1: self.a1 * factor,
            a2: self.a2 * factor,
        }
    }
}

fn shelf_coefficients(
    kind: ShelfKind,
    sample_rate: f32,
    frequency_hz: f32,
    gain_db: f32,
) -> Coefficients {
    if gain_db.abs() <= f32::EPSILON {
        return Coefficients::UNITY;
    }

    let amplitude = 10.0_f32.powf(gain_db / 40.0);
    let omega = 2.0 * PI * frequency_hz.min(sample_rate * 0.45) / sample_rate;
    let cosine = omega.cos();
    let alpha = omega.sin() * 0.5 * 2.0_f32.sqrt();
    let beta = 2.0 * amplitude.sqrt() * alpha;
    let common_plus = amplitude + 1.0;
    let common_minus = amplitude - 1.0;

    let (b0, b1, b2, a0, a1, a2) = match kind {
        ShelfKind::Low => (
            amplitude * (common_plus - common_minus * cosine + beta),
            2.0 * amplitude * (common_minus - common_plus * cosine),
            amplitude * (common_plus - common_minus * cosine - beta),
            common_plus + common_minus * cosine + beta,
            -2.0 * (common_minus + common_plus * cosine),
            common_plus + common_minus * cosine - beta,
        ),
        ShelfKind::High => (
            amplitude * (common_plus + common_minus * cosine + beta),
            -2.0 * amplitude * (common_minus + common_plus * cosine),
            amplitude * (common_plus + common_minus * cosine - beta),
            common_plus - common_minus * cosine + beta,
            2.0 * (common_minus - common_plus * cosine),
            common_plus - common_minus * cosine - beta,
        ),
    };

    Coefficients {
        b0: b0 / a0,
        b1: b1 / a0,
        b2: b2 / a0,
        a1: a1 / a0,
        a2: a2 / a0,
    }
}
