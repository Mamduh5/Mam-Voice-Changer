use std::f64::consts::TAU;

use serde::{Deserialize, Serialize};

use super::manifest::{AnalysisSegment, FormantBand, SegmentKind};

pub const ANALYSIS_WINDOW_MS: f64 = 40.0;
pub const ANALYSIS_HOP_MS: f64 = 10.0;
pub const MIN_F0_HZ: f64 = 50.0;
pub const MAX_F0_HZ: f64 = 1_000.0;
pub const VOICING_PERIODICITY_THRESHOLD: f64 = 0.55;
pub const VOICING_RMS_THRESHOLD: f64 = 0.000_1;
pub const CLIPPING_THRESHOLD: f64 = 0.995;
pub const SPECTRAL_EPSILON: f64 = 1.0e-9;
pub const SPECTRAL_MINIMUM_HZ: f64 = 50.0;
pub const SPECTRAL_MAXIMUM_HZ: f64 = 10_000.0;
pub const HIGH_FREQUENCY_MINIMUM_HZ: f64 = 3_000.0;
const FFT_SIZE: usize = 2_048;
const MINIMUM_PITCH_FRAMES: usize = 3;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AnalysisConfiguration {
    pub window_ms: f64,
    pub hop_ms: f64,
    pub fft_size: usize,
    pub minimum_f0_hz: f64,
    pub maximum_f0_hz: f64,
    pub voicing_periodicity_threshold: f64,
    pub voicing_rms_threshold: f64,
    pub clipping_threshold: f64,
    pub spectral_epsilon: f64,
    pub spectral_minimum_hz: f64,
    pub spectral_maximum_hz: f64,
    pub high_frequency_minimum_hz: f64,
}

impl Default for AnalysisConfiguration {
    fn default() -> Self {
        Self {
            window_ms: ANALYSIS_WINDOW_MS,
            hop_ms: ANALYSIS_HOP_MS,
            fft_size: FFT_SIZE,
            minimum_f0_hz: MIN_F0_HZ,
            maximum_f0_hz: MAX_F0_HZ,
            voicing_periodicity_threshold: VOICING_PERIODICITY_THRESHOLD,
            voicing_rms_threshold: VOICING_RMS_THRESHOLD,
            clipping_threshold: CLIPPING_THRESHOLD,
            spectral_epsilon: SPECTRAL_EPSILON,
            spectral_minimum_hz: SPECTRAL_MINIMUM_HZ,
            spectral_maximum_hz: SPECTRAL_MAXIMUM_HZ,
            high_frequency_minimum_hz: HIGH_FREQUENCY_MINIMUM_HZ,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StructuralMetrics {
    pub input_sample_rate: u32,
    pub output_sample_rate: u32,
    pub input_channels: usize,
    pub output_channels: usize,
    pub input_frames: usize,
    pub output_frames: usize,
    pub duration_delta_frames: i64,
    pub reported_dsp_latency_frames: usize,
    pub reported_dsp_latency_ms: f64,
    pub input_sanitized: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NumericalMetrics {
    pub input_non_finite_samples: u64,
    pub output_non_finite_samples: u64,
    pub input_peak: f64,
    pub output_peak: f64,
    pub input_rms: f64,
    pub output_rms: f64,
    pub rms_change_db: Option<f64>,
    pub input_dc_offset: f64,
    pub output_dc_offset: f64,
    pub input_clipping_ratio: f64,
    pub output_clipping_ratio: f64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PitchMetrics {
    pub median_input_f0_hz: Option<f64>,
    pub median_output_f0_hz: Option<f64>,
    pub measured_pitch_ratio: Option<f64>,
    pub expected_pitch_ratio: Option<f64>,
    pub pitch_error_cents: Option<f64>,
    pub voiced_frame_count: usize,
    pub f0_estimation_coverage: f64,
    pub unavailable_reason: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VoicingMetrics {
    pub source_voiced_frame_ratio: f64,
    pub output_voiced_frame_ratio: f64,
    pub voiced_unvoiced_disagreement_ratio: f64,
    pub voiced_to_unvoiced_errors: usize,
    pub unvoiced_to_voiced_errors: usize,
    pub compared_frames: usize,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SpectralMetrics {
    pub mean_log_spectral_distance_db: Option<f64>,
    pub median_log_spectral_distance_db: Option<f64>,
    pub voiced_log_spectral_distance_db: Option<f64>,
    pub unvoiced_log_spectral_distance_db: Option<f64>,
    pub high_frequency_unvoiced_log_spectral_distance_db: Option<f64>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConsonantMetrics {
    pub source_unvoiced_high_frequency_energy: Option<f64>,
    pub output_unvoiced_high_frequency_energy: Option<f64>,
    pub unvoiced_high_frequency_energy_ratio: Option<f64>,
    pub unvoiced_waveform_correlation: Option<f64>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FormantMetric {
    pub label: String,
    pub minimum_hz: f64,
    pub maximum_hz: f64,
    pub input_peak_hz: Option<f64>,
    pub output_peak_hz: Option<f64>,
    pub measured_ratio: Option<f64>,
    pub expected_ratio: f64,
    pub ratio_error_cents: Option<f64>,
    pub unavailable_reason: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PerformanceMetrics {
    pub render_wall_time_ms: f64,
    pub rendered_audio_duration_seconds: f64,
    pub real_time_factor: f64,
    pub processing_ms_per_audio_second: f64,
    pub build_mode: String,
}

#[derive(Clone, Debug)]
pub struct AudioAnalysis {
    samples: Vec<f64>,
    mono: Vec<f64>,
    frames: Vec<FrameAnalysis>,
    sample_rate: u32,
    channels: usize,
    sample_count: usize,
    non_finite_samples: u64,
}

#[derive(Clone, Debug)]
struct FrameAnalysis {
    center_frame: usize,
    f0_hz: Option<f64>,
    voiced: bool,
    spectrum: Vec<f64>,
}

#[derive(Clone, Copy, Debug, Default)]
struct Complex {
    re: f64,
    im: f64,
}

impl AudioAnalysis {
    pub fn new(samples: &[f32], sample_rate: u32, channels: usize) -> Result<Self, String> {
        if sample_rate == 0 || channels == 0 {
            return Err("Analysis requires a nonzero sample rate and channel count.".to_owned());
        }
        if samples.is_empty() || !samples.len().is_multiple_of(channels) {
            return Err("Analysis requires non-empty complete audio frames.".to_owned());
        }
        let mut non_finite_samples = 0_u64;
        let samples = samples
            .iter()
            .map(|sample| {
                if sample.is_finite() {
                    f64::from(*sample)
                } else {
                    non_finite_samples += 1;
                    0.0
                }
            })
            .collect::<Vec<_>>();
        let mono = samples
            .chunks_exact(channels)
            .map(|frame| frame.iter().sum::<f64>() / channels as f64)
            .collect::<Vec<_>>();
        let window_frames = ((f64::from(sample_rate) * ANALYSIS_WINDOW_MS / 1_000.0).round()
            as usize)
            .min(FFT_SIZE);
        let hop_frames =
            ((f64::from(sample_rate) * ANALYSIS_HOP_MS / 1_000.0).round() as usize).max(1);
        let mut frames = Vec::new();
        if mono.len() >= window_frames {
            for start in (0..=mono.len() - window_frames).step_by(hop_frames) {
                frames.push(analyze_frame(
                    &mono[start..start + window_frames],
                    sample_rate,
                    start + window_frames / 2,
                ));
            }
        }
        let sample_count = samples.len();
        Ok(Self {
            samples,
            mono,
            frames,
            sample_rate,
            channels,
            sample_count,
            non_finite_samples,
        })
    }

    pub fn basic_metrics(&self) -> BasicMetrics {
        let mut peak = 0.0_f64;
        let mut sum = 0.0_f64;
        let mut sum_squares = 0.0_f64;
        let mut clipped = 0_u64;
        for sample in &self.samples {
            peak = peak.max(sample.abs());
            sum += sample;
            sum_squares += sample * sample;
            clipped += u64::from(sample.abs() >= CLIPPING_THRESHOLD);
        }
        let count = self.samples.len().max(1) as f64;
        BasicMetrics {
            non_finite_samples: self.non_finite_samples,
            peak,
            rms: (sum_squares / count).sqrt(),
            dc_offset: sum / count,
            clipping_ratio: clipped as f64 / count,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct BasicMetrics {
    pub non_finite_samples: u64,
    pub peak: f64,
    pub rms: f64,
    pub dc_offset: f64,
    pub clipping_ratio: f64,
}

pub fn compare_audio(
    input: &AudioAnalysis,
    output: &AudioAnalysis,
    segments: &[AnalysisSegment],
    expected_pitch_ratio: Option<f64>,
    formant_bands: &[FormantBand],
    formant_semitones: f32,
) -> (
    NumericalMetrics,
    PitchMetrics,
    VoicingMetrics,
    SpectralMetrics,
    ConsonantMetrics,
    Vec<FormantMetric>,
) {
    let input_basic = input.basic_metrics();
    let output_basic = output.basic_metrics();
    let rms_change_db = if input_basic.rms > SPECTRAL_EPSILON && output_basic.rms > SPECTRAL_EPSILON
    {
        Some(20.0 * (output_basic.rms / input_basic.rms).log10())
    } else {
        None
    };
    let numerical = NumericalMetrics {
        input_non_finite_samples: input_basic.non_finite_samples,
        output_non_finite_samples: output_basic.non_finite_samples,
        input_peak: input_basic.peak,
        output_peak: output_basic.peak,
        input_rms: input_basic.rms,
        output_rms: output_basic.rms,
        rms_change_db,
        input_dc_offset: input_basic.dc_offset,
        output_dc_offset: output_basic.dc_offset,
        input_clipping_ratio: input_basic.clipping_ratio,
        output_clipping_ratio: output_basic.clipping_ratio,
    };

    let frame_count = input.frames.len().min(output.frames.len());
    let paired = input.frames[..frame_count]
        .iter()
        .zip(&output.frames[..frame_count])
        .filter(|(source, _)| frame_selected(source.center_frame, input.sample_rate, segments))
        .collect::<Vec<_>>();
    let voiced_candidates = paired
        .iter()
        .filter(|(source, _)| {
            declared_kind(source.center_frame, input.sample_rate, segments)
                .is_none_or(|kind| matches!(kind, SegmentKind::Voiced | SegmentKind::All))
                && source.voiced
        })
        .collect::<Vec<_>>();
    let mut input_f0 = Vec::new();
    let mut output_f0 = Vec::new();
    for (source, transformed) in &voiced_candidates {
        if let (Some(source_f0), Some(output_f0_value)) = (source.f0_hz, transformed.f0_hz) {
            input_f0.push(source_f0);
            output_f0.push(output_f0_value);
        }
    }
    let voiced_frame_count = input_f0.len();
    let coverage = voiced_frame_count as f64 / voiced_candidates.len().max(1) as f64;
    let (median_input_f0_hz, median_output_f0_hz, measured_pitch_ratio, pitch_error_cents) =
        if voiced_frame_count >= MINIMUM_PITCH_FRAMES {
            let input_median = median(&input_f0);
            let output_median = median(&output_f0);
            let ratio = output_median / input_median.max(SPECTRAL_EPSILON);
            let error = expected_pitch_ratio.map(|expected| cents(ratio / expected));
            (Some(input_median), Some(output_median), Some(ratio), error)
        } else {
            (None, None, None, None)
        };
    let pitch = PitchMetrics {
        median_input_f0_hz,
        median_output_f0_hz,
        measured_pitch_ratio,
        expected_pitch_ratio,
        pitch_error_cents,
        voiced_frame_count,
        f0_estimation_coverage: coverage,
        unavailable_reason: (voiced_frame_count < MINIMUM_PITCH_FRAMES)
            .then(|| "notEnoughVoicedFrames".to_owned()),
    };

    let compared_frames = paired.len();
    let source_voiced = paired.iter().filter(|(source, _)| source.voiced).count();
    let output_voiced = paired
        .iter()
        .filter(|(_, transformed)| transformed.voiced)
        .count();
    let voiced_to_unvoiced_errors = paired
        .iter()
        .filter(|(source, transformed)| source.voiced && !transformed.voiced)
        .count();
    let unvoiced_to_voiced_errors = paired
        .iter()
        .filter(|(source, transformed)| !source.voiced && transformed.voiced)
        .count();
    let voicing = VoicingMetrics {
        source_voiced_frame_ratio: source_voiced as f64 / compared_frames.max(1) as f64,
        output_voiced_frame_ratio: output_voiced as f64 / compared_frames.max(1) as f64,
        voiced_unvoiced_disagreement_ratio: (voiced_to_unvoiced_errors + unvoiced_to_voiced_errors)
            as f64
            / compared_frames.max(1) as f64,
        voiced_to_unvoiced_errors,
        unvoiced_to_voiced_errors,
        compared_frames,
    };

    let mut all_lsd = Vec::new();
    let mut voiced_lsd = Vec::new();
    let mut unvoiced_lsd = Vec::new();
    let mut unvoiced_hf_lsd = Vec::new();
    let mut source_hf = 0.0;
    let mut output_hf = 0.0;
    let mut unvoiced_frames = 0_usize;
    for (source, transformed) in &paired {
        let distance = log_spectral_distance(
            &source.spectrum,
            &transformed.spectrum,
            input.sample_rate,
            SPECTRAL_MINIMUM_HZ,
            SPECTRAL_MAXIMUM_HZ,
        );
        all_lsd.push(distance);
        if source.voiced {
            voiced_lsd.push(distance);
        } else {
            unvoiced_lsd.push(distance);
            unvoiced_hf_lsd.push(log_spectral_distance(
                &source.spectrum,
                &transformed.spectrum,
                input.sample_rate,
                HIGH_FREQUENCY_MINIMUM_HZ,
                SPECTRAL_MAXIMUM_HZ,
            ));
            source_hf += band_energy(
                &source.spectrum,
                input.sample_rate,
                HIGH_FREQUENCY_MINIMUM_HZ,
                SPECTRAL_MAXIMUM_HZ,
            );
            output_hf += band_energy(
                &transformed.spectrum,
                input.sample_rate,
                HIGH_FREQUENCY_MINIMUM_HZ,
                SPECTRAL_MAXIMUM_HZ,
            );
            unvoiced_frames += 1;
        }
    }
    let spectral = SpectralMetrics {
        mean_log_spectral_distance_db: mean_option(&all_lsd),
        median_log_spectral_distance_db: median_option(&all_lsd),
        voiced_log_spectral_distance_db: mean_option(&voiced_lsd),
        unvoiced_log_spectral_distance_db: mean_option(&unvoiced_lsd),
        high_frequency_unvoiced_log_spectral_distance_db: mean_option(&unvoiced_hf_lsd),
    };
    let source_hf_average = (unvoiced_frames > 0).then(|| source_hf / unvoiced_frames as f64);
    let output_hf_average = (unvoiced_frames > 0).then(|| output_hf / unvoiced_frames as f64);
    let consonant = ConsonantMetrics {
        source_unvoiced_high_frequency_energy: source_hf_average,
        output_unvoiced_high_frequency_energy: output_hf_average,
        unvoiced_high_frequency_energy_ratio: source_hf_average
            .zip(output_hf_average)
            .and_then(|(source, output)| (source > SPECTRAL_EPSILON).then_some(output / source)),
        unvoiced_waveform_correlation: waveform_correlation(input, output, segments),
    };

    let expected_formant_ratio = 2.0_f64.powf(f64::from(formant_semitones) / 12.0);
    let input_formant_peaks = formant_bands
        .iter()
        .map(|band| envelope_peak(input, segments, band, None))
        .collect::<Vec<_>>();
    let formants = formant_bands
        .iter()
        .zip(input_formant_peaks)
        .map(|band| {
            let (band, input_peak) = band;
            let expected_output_peak = input_peak.map(|peak| peak * expected_formant_ratio);
            let output_peak = envelope_peak(output, segments, band, expected_output_peak);
            let measured_ratio = input_peak
                .zip(output_peak)
                .map(|(source, transformed)| transformed / source);
            FormantMetric {
                label: band.label.clone(),
                minimum_hz: band.minimum_hz,
                maximum_hz: band.maximum_hz,
                input_peak_hz: input_peak,
                output_peak_hz: output_peak,
                measured_ratio,
                expected_ratio: expected_formant_ratio,
                ratio_error_cents: measured_ratio
                    .map(|ratio| cents(ratio / expected_formant_ratio)),
                unavailable_reason: measured_ratio
                    .is_none()
                    .then(|| "ambiguousOrMissingEnvelopePeak".to_owned()),
            }
        })
        .collect();

    (numerical, pitch, voicing, spectral, consonant, formants)
}

fn analyze_frame(samples: &[f64], sample_rate: u32, center_frame: usize) -> FrameAnalysis {
    let mut windowed = vec![0.0; samples.len()];
    let mean = samples.iter().sum::<f64>() / samples.len() as f64;
    let mut energy = 0.0;
    let mut zero_crossings = 0_usize;
    for index in 0..samples.len() {
        let hann = 0.5 - 0.5 * (TAU * index as f64 / (samples.len() - 1) as f64).cos();
        windowed[index] = (samples[index] - mean) * hann;
        energy += windowed[index] * windowed[index];
        if index > 0 && (windowed[index] >= 0.0) != (windowed[index - 1] >= 0.0) {
            zero_crossings += 1;
        }
    }
    let rms = (energy / samples.len() as f64).sqrt();
    let zero_crossing_rate = zero_crossings as f64 / (samples.len() - 1) as f64;
    let (f0_hz, periodicity) = estimate_f0(&windowed, sample_rate);
    let voiced = rms >= VOICING_RMS_THRESHOLD
        && periodicity >= VOICING_PERIODICITY_THRESHOLD
        && zero_crossing_rate < 0.35;
    let spectrum = magnitude_spectrum(&windowed);
    FrameAnalysis {
        center_frame,
        f0_hz: voiced.then_some(f0_hz),
        voiced,
        spectrum,
    }
}

fn estimate_f0(samples: &[f64], sample_rate: u32) -> (f64, f64) {
    let minimum_lag = (f64::from(sample_rate) / MAX_F0_HZ).floor().max(1.0) as usize;
    let maximum_lag =
        ((f64::from(sample_rate) / MIN_F0_HZ).ceil() as usize).min(samples.len().saturating_sub(2));
    let correlation = |lag: usize| normalized_correlation(samples, lag);
    let correlations = (minimum_lag..=maximum_lag)
        .map(correlation)
        .collect::<Vec<_>>();
    let (best_offset, best) = correlations
        .iter()
        .copied()
        .enumerate()
        .max_by(|left, right| left.1.total_cmp(&right.1))
        .unwrap_or((0, 0.0));
    let strong = (best * 0.95).max(0.72);
    let selected_offset = (1..correlations.len().saturating_sub(1))
        .find(|index| {
            correlations[*index] >= strong
                && correlations[*index] >= correlations[*index - 1]
                && correlations[*index] >= correlations[*index + 1]
        })
        .unwrap_or(best_offset);
    let lag = minimum_lag + selected_offset;
    let interpolated = if selected_offset > 0 && selected_offset + 1 < correlations.len() {
        let left = correlations[selected_offset - 1];
        let center = correlations[selected_offset];
        let right = correlations[selected_offset + 1];
        let denominator = left - 2.0 * center + right;
        if denominator.abs() > f64::EPSILON {
            lag as f64 + 0.5 * (left - right) / denominator
        } else {
            lag as f64
        }
    } else {
        lag as f64
    };
    (f64::from(sample_rate) / interpolated, best.clamp(0.0, 1.0))
}

fn normalized_correlation(samples: &[f64], lag: usize) -> f64 {
    let mut cross = 0.0;
    let mut left_energy = 0.0;
    let mut right_energy = 0.0;
    for index in 0..samples.len() - lag {
        let left = samples[index];
        let right = samples[index + lag];
        cross += left * right;
        left_energy += left * left;
        right_energy += right * right;
    }
    cross / (left_energy * right_energy).sqrt().max(f64::EPSILON)
}

fn magnitude_spectrum(samples: &[f64]) -> Vec<f64> {
    let mut values = vec![Complex::default(); FFT_SIZE];
    for (target, sample) in values.iter_mut().zip(samples) {
        target.re = *sample;
    }
    fft(&mut values);
    values[..=FFT_SIZE / 2]
        .iter()
        .map(|value| (value.re * value.re + value.im * value.im).sqrt() / FFT_SIZE as f64)
        .collect()
}

fn fft(values: &mut [Complex]) {
    let length = values.len();
    let mut j = 0_usize;
    for index in 1..length {
        let mut bit = length >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j ^= bit;
        if index < j {
            values.swap(index, j);
        }
    }
    let mut width = 2;
    while width <= length {
        let angle = -TAU / width as f64;
        let step = Complex {
            re: angle.cos(),
            im: angle.sin(),
        };
        for start in (0..length).step_by(width) {
            let mut twiddle = Complex { re: 1.0, im: 0.0 };
            for offset in 0..width / 2 {
                let even = values[start + offset];
                let odd = multiply(values[start + offset + width / 2], twiddle);
                values[start + offset] = Complex {
                    re: even.re + odd.re,
                    im: even.im + odd.im,
                };
                values[start + offset + width / 2] = Complex {
                    re: even.re - odd.re,
                    im: even.im - odd.im,
                };
                twiddle = multiply(twiddle, step);
            }
        }
        width *= 2;
    }
}

fn multiply(left: Complex, right: Complex) -> Complex {
    Complex {
        re: left.re * right.re - left.im * right.im,
        im: left.re * right.im + left.im * right.re,
    }
}

fn frame_selected(center: usize, sample_rate: u32, segments: &[AnalysisSegment]) -> bool {
    segments.is_empty()
        || segments.iter().any(|segment| {
            frame_in_segment(center, sample_rate, segment)
                && !matches!(segment.kind, SegmentKind::Silence)
        })
}

fn declared_kind(
    center: usize,
    sample_rate: u32,
    segments: &[AnalysisSegment],
) -> Option<SegmentKind> {
    segments
        .iter()
        .find(|segment| frame_in_segment(center, sample_rate, segment))
        .map(|segment| segment.kind)
}

fn frame_in_segment(center: usize, sample_rate: u32, segment: &AnalysisSegment) -> bool {
    let time_ms = center as u64 * 1_000 / u64::from(sample_rate);
    (segment.start_ms..segment.end_ms).contains(&time_ms)
}

fn log_spectral_distance(
    source: &[f64],
    output: &[f64],
    sample_rate: u32,
    minimum_hz: f64,
    maximum_hz: f64,
) -> f64 {
    let maximum_hz = maximum_hz.min(f64::from(sample_rate) / 2.0);
    let minimum_bin = frequency_bin(minimum_hz, sample_rate).max(1);
    let maximum_bin = frequency_bin(maximum_hz, sample_rate)
        .min(source.len().min(output.len()).saturating_sub(1));
    if maximum_bin < minimum_bin {
        return 0.0;
    }
    let mut squared = 0.0;
    let mut count = 0_usize;
    for bin in minimum_bin..=maximum_bin {
        let source_db = 20.0 * (source[bin] + SPECTRAL_EPSILON).log10();
        let output_db = 20.0 * (output[bin] + SPECTRAL_EPSILON).log10();
        squared += (source_db - output_db).powi(2);
        count += 1;
    }
    (squared / count.max(1) as f64).sqrt()
}

fn band_energy(spectrum: &[f64], sample_rate: u32, minimum_hz: f64, maximum_hz: f64) -> f64 {
    let minimum_bin = frequency_bin(minimum_hz, sample_rate).max(1);
    let maximum_bin = frequency_bin(maximum_hz.min(f64::from(sample_rate) / 2.0), sample_rate)
        .min(spectrum.len().saturating_sub(1));
    if maximum_bin < minimum_bin {
        return 0.0;
    }
    spectrum[minimum_bin..=maximum_bin]
        .iter()
        .map(|magnitude| magnitude * magnitude)
        .sum::<f64>()
        / (maximum_bin - minimum_bin + 1) as f64
}

fn frequency_bin(frequency: f64, sample_rate: u32) -> usize {
    (frequency * FFT_SIZE as f64 / f64::from(sample_rate)).round() as usize
}

fn waveform_correlation(
    input: &AudioAnalysis,
    output: &AudioAnalysis,
    segments: &[AnalysisSegment],
) -> Option<f64> {
    let mut source_values = Vec::new();
    let mut output_values = Vec::new();
    for index in 0..input.mono.len().min(output.mono.len()) {
        let explicitly_unvoiced = segments.iter().any(|segment| {
            segment.kind == SegmentKind::Unvoiced
                && frame_in_segment(index, input.sample_rate, segment)
        });
        if explicitly_unvoiced {
            source_values.push(input.mono[index]);
            output_values.push(output.mono[index]);
        }
    }
    if source_values.len() < 2 {
        return None;
    }
    Some(correlation(&source_values, &output_values))
}

fn correlation(left: &[f64], right: &[f64]) -> f64 {
    let left_mean = left.iter().sum::<f64>() / left.len() as f64;
    let right_mean = right.iter().sum::<f64>() / right.len() as f64;
    let mut cross = 0.0;
    let mut left_energy = 0.0;
    let mut right_energy = 0.0;
    for (left, right) in left.iter().zip(right) {
        let left = *left - left_mean;
        let right = *right - right_mean;
        cross += left * right;
        left_energy += left * left;
        right_energy += right * right;
    }
    cross / (left_energy * right_energy).sqrt().max(f64::EPSILON)
}

fn envelope_peak(
    analysis: &AudioAnalysis,
    segments: &[AnalysisSegment],
    band: &FormantBand,
    preferred_hz: Option<f64>,
) -> Option<f64> {
    let selected = analysis
        .frames
        .iter()
        .filter(|frame| {
            frame.voiced
                && (segments.is_empty()
                    || segments.iter().any(|segment| {
                        matches!(segment.kind, SegmentKind::Voiced | SegmentKind::All)
                            && frame_in_segment(frame.center_frame, analysis.sample_rate, segment)
                    }))
        })
        .collect::<Vec<_>>();
    if selected.is_empty() || band.maximum_hz >= f64::from(analysis.sample_rate) / 2.0 {
        return None;
    }
    let mut average = vec![0.0; FFT_SIZE / 2 + 1];
    for frame in &selected {
        for (sum, magnitude) in average.iter_mut().zip(&frame.spectrum) {
            *sum += *magnitude;
        }
    }
    for value in &mut average {
        *value /= selected.len() as f64;
    }
    let smoothed = (0..average.len())
        .map(|index| {
            let start = index.saturating_sub(6);
            let end = (index + 6).min(average.len() - 1);
            average[start..=end].iter().sum::<f64>() / (end - start + 1) as f64
        })
        .collect::<Vec<_>>();
    envelope_peak_from_spectrum(&smoothed, analysis.sample_rate, band, preferred_hz)
}

fn envelope_peak_from_spectrum(
    smoothed: &[f64],
    sample_rate: u32,
    band: &FormantBand,
    preferred_hz: Option<f64>,
) -> Option<f64> {
    let minimum_bin = frequency_bin(band.minimum_hz, sample_rate);
    let maximum_bin =
        frequency_bin(band.maximum_hz, sample_rate).min(smoothed.len().saturating_sub(1));
    let (global_offset, global_peak) = smoothed[minimum_bin..=maximum_bin]
        .iter()
        .copied()
        .enumerate()
        .max_by(|left, right| left.1.total_cmp(&right.1))?;
    let floor = smoothed[minimum_bin..=maximum_bin].iter().sum::<f64>()
        / (maximum_bin - minimum_bin + 1) as f64;
    if global_peak <= SPECTRAL_EPSILON || global_peak < floor * 1.05 {
        return None;
    }
    let global_bin = minimum_bin + global_offset;
    let selected_bin = preferred_hz
        .filter(|preferred| preferred.is_finite() && *preferred > 0.0)
        .and_then(|preferred| {
            (minimum_bin..=maximum_bin)
                .filter(|bin| {
                    let value = smoothed[*bin];
                    let left = bin
                        .checked_sub(1)
                        .filter(|left| *left >= minimum_bin)
                        .map_or(f64::NEG_INFINITY, |left| smoothed[left]);
                    let right = if *bin < maximum_bin {
                        smoothed[*bin + 1]
                    } else {
                        f64::NEG_INFINITY
                    };
                    value >= floor * 1.05
                        && value >= left
                        && value >= right
                        && (value > left || value > right)
                })
                .min_by(|left, right| {
                    let frequency =
                        |bin: usize| bin as f64 * f64::from(sample_rate) / FFT_SIZE as f64;
                    let left_distance = (frequency(*left) / preferred).ln().abs();
                    let right_distance = (frequency(*right) / preferred).ln().abs();
                    left_distance.total_cmp(&right_distance)
                })
        })
        .unwrap_or(global_bin);
    Some(selected_bin as f64 * f64::from(sample_rate) / FFT_SIZE as f64)
}

fn mean_option(values: &[f64]) -> Option<f64> {
    (!values.is_empty()).then(|| values.iter().sum::<f64>() / values.len() as f64)
}

fn median_option(values: &[f64]) -> Option<f64> {
    (!values.is_empty()).then(|| median(values))
}

fn median(values: &[f64]) -> f64 {
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    if sorted.len().is_multiple_of(2) {
        (sorted[sorted.len() / 2 - 1] + sorted[sorted.len() / 2]) * 0.5
    } else {
        sorted[sorted.len() / 2]
    }
}

fn cents(ratio: f64) -> f64 {
    1_200.0 * ratio.max(SPECTRAL_EPSILON).log2()
}

pub fn median_formant_ratio(formants: &[FormantMetric]) -> Option<f64> {
    let ratios = formants
        .iter()
        .filter_map(|formant| formant.measured_ratio)
        .collect::<Vec<_>>();
    median_option(&ratios)
}

pub fn median_formant_error(formants: &[FormantMetric]) -> Option<f64> {
    let errors = formants
        .iter()
        .filter_map(|formant| formant.ratio_error_cents)
        .map(f64::abs)
        .collect::<Vec<_>>();
    median_option(&errors)
}

pub fn audio_shape(analysis: &AudioAnalysis) -> (u32, usize, usize, usize) {
    (
        analysis.sample_rate,
        analysis.channels,
        analysis.mono.len(),
        analysis.sample_count,
    )
}

#[cfg(test)]
mod tests {
    use std::f32::consts::TAU as TAU_F32;

    use super::*;

    fn harmonic(frequency: f32, frames: usize) -> Vec<f32> {
        (0..frames)
            .map(|index| {
                (1..=6)
                    .map(|harmonic| {
                        (TAU_F32 * frequency * harmonic as f32 * index as f32 / 48_000.0).sin()
                            / harmonic as f32
                    })
                    .sum::<f32>()
                    * 0.1
            })
            .collect()
    }

    #[test]
    fn pitch_voicing_silence_and_epsilon_spectral_metrics_are_finite() {
        assert!(AudioAnalysis::new(&[], 48_000, 1).is_err());
        let defensive =
            AudioAnalysis::new(&[f32::NAN, f32::INFINITY, 0.5, -0.5], 48_000, 2).unwrap();
        let defensive_metrics = defensive.basic_metrics();
        assert_eq!(defensive_metrics.non_finite_samples, 2);
        assert_eq!(defensive_metrics.peak, 0.5);
        assert_eq!(defensive_metrics.clipping_ratio, 0.0);
        let voiced = AudioAnalysis::new(&harmonic(220.0, 48_000), 48_000, 1).unwrap();
        assert!(voiced.frames.iter().filter(|frame| frame.voiced).count() > 80);
        let f0 = median(
            &voiced
                .frames
                .iter()
                .filter_map(|frame| frame.f0_hz)
                .collect::<Vec<_>>(),
        );
        assert!((f0 - 220.0).abs() / 220.0 < 0.02);

        let silence = AudioAnalysis::new(&vec![0.0; 48_000], 48_000, 1).unwrap();
        assert!(silence.frames.iter().all(|frame| !frame.voiced));
        let (_, pitch, voicing, spectral, _, _) =
            compare_audio(&silence, &silence, &[], Some(1.0), &[], 0.0);
        assert_eq!(
            pitch.unavailable_reason.as_deref(),
            Some("notEnoughVoicedFrames")
        );
        assert_eq!(voicing.voiced_unvoiced_disagreement_ratio, 0.0);
        assert!(spectral.mean_log_spectral_distance_db.unwrap().is_finite());
    }

    #[test]
    fn ambiguous_silent_formant_band_is_unavailable() {
        let silence = AudioAnalysis::new(&vec![0.0; 48_000], 48_000, 1).unwrap();
        let band = FormantBand {
            label: "F1-like".to_owned(),
            minimum_hz: 400.0,
            maximum_hz: 1_000.0,
        };
        let (_, _, _, _, _, formants) =
            compare_audio(&silence, &silence, &[], Some(1.0), &[band], 4.0);
        assert!(formants[0].measured_ratio.is_none());
        assert_eq!(
            formants[0].unavailable_reason.as_deref(),
            Some("ambiguousOrMissingEnvelopePeak")
        );
    }

    #[test]
    fn target_aware_formant_peak_ignores_stronger_overlapping_resonance() {
        let sample_rate = 48_000;
        let mut spectrum = vec![1.0; FFT_SIZE / 2 + 1];
        let bin = |frequency: f64| frequency_bin(frequency, sample_rate);
        let contaminating_f1 = bin(960.0);
        let expected_f2 = bin(1_650.0);
        for (center, height) in [(contaminating_f1, 10.0), (expected_f2, 7.0)] {
            spectrum[center - 1] = height * 0.8;
            spectrum[center] = height;
            spectrum[center + 1] = height * 0.8;
        }
        let band = FormantBand {
            label: "F2-like".to_owned(),
            minimum_hz: 900.0,
            maximum_hz: 1_900.0,
        };
        let global = envelope_peak_from_spectrum(&spectrum, sample_rate, &band, None).unwrap();
        let tracked =
            envelope_peak_from_spectrum(&spectrum, sample_rate, &band, Some(1_650.0)).unwrap();
        assert!((global - 960.0).abs() <= f64::from(sample_rate) / FFT_SIZE as f64);
        assert!((tracked - 1_650.0).abs() <= f64::from(sample_rate) / FFT_SIZE as f64);
    }

    #[test]
    fn source_output_mask_disagreement_is_computed() {
        let voiced = AudioAnalysis::new(&harmonic(220.0, 48_000), 48_000, 1).unwrap();
        let silence = AudioAnalysis::new(&vec![0.0; 48_000], 48_000, 1).unwrap();
        let (_, _, metrics, _, _, _) = compare_audio(&voiced, &silence, &[], None, &[], 0.0);
        assert!(metrics.voiced_to_unvoiced_errors > 0);
        assert!(metrics.voiced_unvoiced_disagreement_ratio > 0.9);
    }
}
