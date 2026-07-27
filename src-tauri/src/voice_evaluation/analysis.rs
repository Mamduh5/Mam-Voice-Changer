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
const YIN_THRESHOLD: f64 = 0.20;
const YIN_FALLBACK_THRESHOLD: f64 = 0.35;
const YIN_MINIMUM_CONFIDENCE: f64 = 0.65;
const MAXIMUM_VOICED_ZERO_CROSSING_RATE: f64 = 0.35;
const OCTAVE_TOLERANCE_CENTS: f64 = 100.0;
const LARGE_ERROR_CENTS: f64 = 100.0;
const TEMPORAL_TRANSITION_WEIGHT: f64 = 0.12;

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

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PitchMetrics {
    pub median_input_f0_hz: Option<f64>,
    pub median_output_f0_hz: Option<f64>,
    pub measured_pitch_ratio: Option<f64>,
    pub expected_pitch_ratio: Option<f64>,
    pub pitch_error_cents: Option<f64>,
    pub voiced_frame_count: usize,
    pub f0_estimation_coverage: f64,
    #[serde(default)]
    pub total_source_frames: usize,
    #[serde(default)]
    pub reliable_source_voiced_frames: usize,
    #[serde(default)]
    pub paired_reliable_frames: usize,
    #[serde(default)]
    pub unpaired_source_voiced_frames: usize,
    #[serde(default)]
    pub low_confidence_exclusions: usize,
    #[serde(default)]
    pub mean_absolute_pitch_error_cents: Option<f64>,
    #[serde(default)]
    pub median_absolute_pitch_error_cents: Option<f64>,
    #[serde(default)]
    pub p10_pitch_error_cents: Option<f64>,
    #[serde(default)]
    pub p90_pitch_error_cents: Option<f64>,
    #[serde(default)]
    pub median_source_confidence: Option<f64>,
    #[serde(default)]
    pub median_output_confidence: Option<f64>,
    #[serde(default)]
    pub median_paired_confidence: Option<f64>,
    #[serde(default)]
    pub octave_doubling_count: usize,
    #[serde(default)]
    pub octave_halving_count: usize,
    #[serde(default)]
    pub large_non_octave_error_count: usize,
    #[serde(default)]
    pub octave_ambiguity_count: usize,
    #[serde(default)]
    pub source_track_fingerprint: String,
    #[serde(default)]
    pub legacy_pitch_error_cents: Option<f64>,
    #[serde(default)]
    pub legacy_source_median_f0_hz: Option<f64>,
    #[serde(default)]
    pub legacy_output_median_f0_hz: Option<f64>,
    #[serde(default)]
    pub legacy_paired_frames: usize,
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
    confidence: f64,
    voiced: bool,
    low_confidence_candidate: bool,
    octave_ambiguous: bool,
    candidates: Vec<PitchCandidate>,
    legacy_f0_hz: Option<f64>,
    legacy_voiced: bool,
    spectrum: Vec<f64>,
}

#[derive(Clone, Copy, Debug)]
struct PitchCandidate {
    f0_hz: f64,
    confidence: f64,
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
        stabilize_pitch_track(&mut frames);
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
    let mut measured_ratios = Vec::new();
    let mut pitch_errors = Vec::new();
    let mut source_confidences = Vec::new();
    let mut output_confidences = Vec::new();
    let mut paired_confidences = Vec::new();
    let mut octave_ambiguity_count = 0_usize;
    for (source, transformed) in &voiced_candidates {
        if let (Some(source_f0), Some(output_f0_value)) = (source.f0_hz, transformed.f0_hz) {
            input_f0.push(source_f0);
            output_f0.push(output_f0_value);
            source_confidences.push(source.confidence);
            output_confidences.push(transformed.confidence);
            paired_confidences.push(source.confidence.min(transformed.confidence));
            let ratio = output_f0_value / source_f0.max(SPECTRAL_EPSILON);
            measured_ratios.push(ratio);
            if let Some(expected) = expected_pitch_ratio {
                pitch_errors.push(cents(ratio / expected));
            }
            octave_ambiguity_count +=
                usize::from(source.octave_ambiguous || transformed.octave_ambiguous);
        }
    }
    let voiced_frame_count = input_f0.len();
    let coverage = voiced_frame_count as f64 / voiced_candidates.len().max(1) as f64;
    let sufficient = voiced_frame_count >= MINIMUM_PITCH_FRAMES;
    let median_input_f0_hz = sufficient.then(|| median(&input_f0));
    let median_output_f0_hz = sufficient.then(|| median(&output_f0));
    let measured_pitch_ratio = sufficient.then(|| median(&measured_ratios));
    let pitch_error_cents =
        (sufficient && expected_pitch_ratio.is_some()).then(|| median(&pitch_errors));
    let absolute_errors = pitch_errors
        .iter()
        .map(|value| value.abs())
        .collect::<Vec<_>>();
    let octave_doubling_count = pitch_errors
        .iter()
        .filter(|error| (**error - 1_200.0).abs() <= OCTAVE_TOLERANCE_CENTS)
        .count();
    let octave_halving_count = pitch_errors
        .iter()
        .filter(|error| (**error + 1_200.0).abs() <= OCTAVE_TOLERANCE_CENTS)
        .count();
    let large_non_octave_error_count = pitch_errors
        .iter()
        .filter(|error| {
            error.abs() > LARGE_ERROR_CENTS
                && (**error - 1_200.0).abs() > OCTAVE_TOLERANCE_CENTS
                && (**error + 1_200.0).abs() > OCTAVE_TOLERANCE_CENTS
        })
        .count();
    let low_confidence_exclusions = paired
        .iter()
        .filter(|(source, transformed)| {
            source.low_confidence_candidate
                || (source.voiced && transformed.low_confidence_candidate)
        })
        .count();

    let legacy_pairs = paired
        .iter()
        .filter_map(|(source, transformed)| {
            (source.legacy_voiced && transformed.legacy_voiced)
                .then(|| source.legacy_f0_hz.zip(transformed.legacy_f0_hz))
                .flatten()
        })
        .collect::<Vec<_>>();
    let legacy_input = legacy_pairs.iter().map(|pair| pair.0).collect::<Vec<_>>();
    let legacy_output = legacy_pairs.iter().map(|pair| pair.1).collect::<Vec<_>>();
    let legacy_source_median_f0_hz = (!legacy_input.is_empty()).then(|| median(&legacy_input));
    let legacy_output_median_f0_hz = (!legacy_output.is_empty()).then(|| median(&legacy_output));
    let legacy_pitch_error_cents = legacy_source_median_f0_hz
        .zip(legacy_output_median_f0_hz)
        .zip(expected_pitch_ratio)
        .map(|((source, output), expected)| {
            cents((output / source.max(SPECTRAL_EPSILON)) / expected)
        });
    let pitch = PitchMetrics {
        median_input_f0_hz,
        median_output_f0_hz,
        measured_pitch_ratio,
        expected_pitch_ratio,
        pitch_error_cents,
        voiced_frame_count,
        f0_estimation_coverage: coverage,
        total_source_frames: paired.len(),
        reliable_source_voiced_frames: voiced_candidates.len(),
        paired_reliable_frames: voiced_frame_count,
        unpaired_source_voiced_frames: voiced_candidates.len().saturating_sub(voiced_frame_count),
        low_confidence_exclusions,
        mean_absolute_pitch_error_cents: (sufficient && !absolute_errors.is_empty())
            .then(|| absolute_errors.iter().sum::<f64>() / absolute_errors.len() as f64),
        median_absolute_pitch_error_cents: (sufficient && !absolute_errors.is_empty())
            .then(|| median(&absolute_errors)),
        p10_pitch_error_cents: (sufficient && !pitch_errors.is_empty())
            .then(|| percentile(&pitch_errors, 0.10)),
        p90_pitch_error_cents: (sufficient && !pitch_errors.is_empty())
            .then(|| percentile(&pitch_errors, 0.90)),
        median_source_confidence: median_option(&source_confidences),
        median_output_confidence: median_option(&output_confidences),
        median_paired_confidence: median_option(&paired_confidences),
        octave_doubling_count,
        octave_halving_count,
        large_non_octave_error_count,
        octave_ambiguity_count,
        source_track_fingerprint: source_track_fingerprint(input),
        legacy_pitch_error_cents,
        legacy_source_median_f0_hz,
        legacy_output_median_f0_hz,
        legacy_paired_frames: legacy_pairs.len(),
        unavailable_reason: (!sufficient).then(|| "notEnoughVoicedFrames".to_owned()),
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
    let candidates = yin_candidates(&windowed, sample_rate, MIN_F0_HZ, MAX_F0_HZ);
    let selected = select_yin_candidate(&candidates);
    let confidence = selected.map_or(0.0, |candidate| candidate.confidence);
    let voiced = rms >= VOICING_RMS_THRESHOLD
        && zero_crossing_rate < MAXIMUM_VOICED_ZERO_CROSSING_RATE
        && confidence >= YIN_MINIMUM_CONFIDENCE;
    let octave_ambiguous = selected.is_some_and(|selected| {
        candidates.iter().any(|candidate| {
            candidate.f0_hz != selected.f0_hz
                && (cents(candidate.f0_hz / selected.f0_hz).abs() - 1_200.0).abs()
                    <= OCTAVE_TOLERANCE_CENTS
                && candidate.confidence + 0.10 >= selected.confidence
        })
    });
    let (legacy_f0, periodicity) = estimate_f0_legacy(&windowed, sample_rate);
    let legacy_voiced = rms >= VOICING_RMS_THRESHOLD
        && periodicity >= VOICING_PERIODICITY_THRESHOLD
        && zero_crossing_rate < MAXIMUM_VOICED_ZERO_CROSSING_RATE;
    let spectrum = magnitude_spectrum(&windowed);
    FrameAnalysis {
        center_frame,
        f0_hz: voiced.then(|| selected.unwrap().f0_hz),
        confidence,
        voiced,
        low_confidence_candidate: !candidates.is_empty() && !voiced,
        octave_ambiguous,
        candidates,
        legacy_f0_hz: legacy_voiced.then_some(legacy_f0),
        legacy_voiced,
        spectrum,
    }
}

fn yin_candidates(
    samples: &[f64],
    sample_rate: u32,
    minimum_f0_hz: f64,
    maximum_f0_hz: f64,
) -> Vec<PitchCandidate> {
    if samples.len() < 4
        || !minimum_f0_hz.is_finite()
        || !maximum_f0_hz.is_finite()
        || minimum_f0_hz <= 0.0
        || maximum_f0_hz <= minimum_f0_hz
    {
        return Vec::new();
    }
    let minimum_lag = (f64::from(sample_rate) / maximum_f0_hz).floor().max(1.0) as usize;
    let maximum_lag = ((f64::from(sample_rate) / minimum_f0_hz).ceil() as usize)
        .min(samples.len().saturating_sub(2));
    if maximum_lag <= minimum_lag {
        return Vec::new();
    }

    let mut difference = vec![0.0; maximum_lag + 1];
    for lag in 1..=maximum_lag {
        let mut sum = 0.0;
        for index in 0..samples.len() - lag {
            let delta = samples[index] - samples[index + lag];
            sum += delta * delta;
        }
        difference[lag] = if sum.is_finite() {
            sum.max(0.0)
        } else {
            f64::INFINITY
        };
    }
    let mut cumulative = 0.0;
    let mut cmndf = vec![1.0; maximum_lag + 1];
    for lag in 1..=maximum_lag {
        cumulative += difference[lag];
        cmndf[lag] = if cumulative > f64::EPSILON && cumulative.is_finite() {
            (difference[lag] * lag as f64 / cumulative).clamp(0.0, 1.0)
        } else {
            1.0
        };
    }

    let mut candidates = Vec::new();
    for lag in minimum_lag..=maximum_lag {
        let left = lag
            .checked_sub(1)
            .filter(|value| *value >= minimum_lag)
            .map_or(f64::INFINITY, |value| cmndf[value]);
        let right = if lag < maximum_lag {
            cmndf[lag + 1]
        } else {
            f64::INFINITY
        };
        if cmndf[lag] <= left && cmndf[lag] <= right && (cmndf[lag] < left || cmndf[lag] < right) {
            let interpolated_lag = parabolic_minimum(lag, &cmndf);
            let confidence = (1.0 - cmndf[lag]).clamp(0.0, 1.0);
            let f0_hz = f64::from(sample_rate) / interpolated_lag;
            if f0_hz.is_finite()
                && (minimum_f0_hz..=maximum_f0_hz).contains(&f0_hz)
                && confidence.is_finite()
            {
                candidates.push(PitchCandidate { f0_hz, confidence });
            }
        }
    }
    candidates
}

fn select_yin_candidate(candidates: &[PitchCandidate]) -> Option<PitchCandidate> {
    let initial = candidates
        .iter()
        .copied()
        .find(|candidate| 1.0 - candidate.confidence <= YIN_THRESHOLD)
        .or_else(|| {
            candidates
                .iter()
                .copied()
                .filter(|candidate| 1.0 - candidate.confidence <= YIN_FALLBACK_THRESHOLD)
                .max_by(|left, right| left.confidence.total_cmp(&right.confidence))
        })?;
    Some(
        candidates
            .iter()
            .copied()
            .filter(|candidate| {
                let harmonic_order = initial.f0_hz / candidate.f0_hz;
                (2.0..=4.0).contains(&harmonic_order)
                    && (harmonic_order - harmonic_order.round()).abs() <= 0.08
                    && candidate.confidence >= initial.confidence + 0.05
            })
            .max_by(|left, right| left.confidence.total_cmp(&right.confidence))
            .unwrap_or(initial),
    )
}

fn parabolic_minimum(lag: usize, values: &[f64]) -> f64 {
    if lag == 0 || lag + 1 >= values.len() {
        return lag as f64;
    }
    let left = values[lag - 1];
    let center = values[lag];
    let right = values[lag + 1];
    let denominator = left - 2.0 * center + right;
    if !denominator.is_finite() || denominator.abs() <= f64::EPSILON {
        lag as f64
    } else {
        (lag as f64 + 0.5 * (left - right) / denominator).clamp(lag as f64 - 1.0, lag as f64 + 1.0)
    }
}

fn stabilize_pitch_track(frames: &mut [FrameAnalysis]) {
    for _ in 0..2 {
        let snapshot = frames.iter().map(|frame| frame.f0_hz).collect::<Vec<_>>();
        for index in 1..frames.len().saturating_sub(1) {
            let (Some(previous), Some(next), Some(current)) =
                (snapshot[index - 1], snapshot[index + 1], snapshot[index])
            else {
                continue;
            };
            let transition_cost = |candidate: PitchCandidate| {
                let previous_octaves = (candidate.f0_hz / previous).log2().abs();
                let next_octaves = (candidate.f0_hz / next).log2().abs();
                1.0 - candidate.confidence
                    + TEMPORAL_TRANSITION_WEIGHT * (previous_octaves + next_octaves)
            };
            let current_cost = transition_cost(PitchCandidate {
                f0_hz: current,
                confidence: frames[index].confidence,
            });
            if let Some(best) = frames[index]
                .candidates
                .iter()
                .copied()
                .filter(|candidate| candidate.confidence >= YIN_MINIMUM_CONFIDENCE)
                .min_by(|left, right| transition_cost(*left).total_cmp(&transition_cost(*right)))
            {
                if transition_cost(best) + 0.02 < current_cost {
                    frames[index].f0_hz = Some(best.f0_hz);
                    frames[index].confidence = best.confidence;
                }
            }
        }
    }
}

fn estimate_f0_legacy(samples: &[f64], sample_rate: u32) -> (f64, f64) {
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

fn percentile(values: &[f64], quantile: f64) -> f64 {
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let position = quantile.clamp(0.0, 1.0) * sorted.len().saturating_sub(1) as f64;
    let lower = position.floor() as usize;
    let upper = position.ceil() as usize;
    let fraction = position - lower as f64;
    sorted[lower] * (1.0 - fraction) + sorted[upper] * fraction
}

fn source_track_fingerprint(analysis: &AudioAnalysis) -> String {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    let mut update = |value: u64| {
        for byte in value.to_le_bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    };
    update(u64::from(analysis.sample_rate));
    update(analysis.frames.len() as u64);
    for frame in &analysis.frames {
        update(frame.center_frame as u64);
        update(frame.f0_hz.map(f64::to_bits).unwrap_or_default());
        update(frame.confidence.to_bits());
        update(u64::from(frame.voiced));
    }
    format!("{hash:016x}")
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

    fn harmonic_at(frequency: f32, frames: usize, sample_rate: u32) -> Vec<f32> {
        (0..frames)
            .map(|index| {
                (1..=6)
                    .map(|harmonic| {
                        (TAU_F32 * frequency * harmonic as f32 * index as f32 / sample_rate as f32)
                            .sin()
                            / harmonic as f32
                    })
                    .sum::<f32>()
                    * 0.1
            })
            .collect()
    }

    fn harmonic(frequency: f32, frames: usize) -> Vec<f32> {
        harmonic_at(frequency, frames, 48_000)
    }

    fn median_track(analysis: &AudioAnalysis) -> f64 {
        median(
            &analysis
                .frames
                .iter()
                .filter_map(|frame| frame.f0_hz)
                .collect::<Vec<_>>(),
        )
    }

    fn weighted_harmonic(
        frequency: f32,
        frames: usize,
        sample_rate: u32,
        weights: &[f32],
    ) -> Vec<f32> {
        (0..frames)
            .map(|index| {
                weights
                    .iter()
                    .enumerate()
                    .map(|(offset, weight)| {
                        let harmonic = offset + 1;
                        weight
                            * (TAU_F32 * frequency * harmonic as f32 * index as f32
                                / sample_rate as f32)
                                .sin()
                    })
                    .sum::<f32>()
                    * 0.1
            })
            .collect()
    }

    fn track_frame(center_frame: usize, f0_hz: f64) -> FrameAnalysis {
        FrameAnalysis {
            center_frame,
            f0_hz: Some(f0_hz),
            confidence: 0.95,
            voiced: true,
            low_confidence_candidate: false,
            octave_ambiguous: false,
            candidates: Vec::new(),
            legacy_f0_hz: Some(f0_hz),
            legacy_voiced: true,
            spectrum: vec![0.0; FFT_SIZE / 2 + 1],
        }
    }

    fn analysis_from_track(track: &[f64]) -> AudioAnalysis {
        AudioAnalysis {
            samples: vec![0.0; 48_000],
            mono: vec![0.0; 48_000],
            frames: track
                .iter()
                .enumerate()
                .map(|(index, f0)| track_frame(index * 480 + 960, *f0))
                .collect(),
            sample_rate: 48_000,
            channels: 1,
            sample_count: 48_000,
            non_finite_samples: 0,
        }
    }

    #[test]
    fn yin_tracks_harmonic_fundamentals_at_supported_rates_and_frequencies() {
        for sample_rate in [44_100, 48_000] {
            for expected in [90.0_f64, 220.0, 400.0, 800.0] {
                let samples = harmonic_at(expected as f32, sample_rate as usize / 2, sample_rate);
                let analysis = AudioAnalysis::new(&samples, sample_rate, 1).unwrap();
                let measured = median_track(&analysis);
                assert!(
                    cents(measured / expected).abs() < 25.0,
                    "{sample_rate} Hz / {expected} Hz measured {measured} Hz"
                );
            }
        }
    }

    #[test]
    fn yin_prefers_present_fundamental_over_stronger_second_or_third_harmonic() {
        for weights in [[0.30, 1.0, 0.20, 0.10], [0.30, 0.20, 1.0, 0.10]] {
            let samples = weighted_harmonic(180.0, 24_000, 48_000, &weights);
            let analysis = AudioAnalysis::new(&samples, 48_000, 1).unwrap();
            let measured = median_track(&analysis);
            assert!(
                cents(measured / 180.0).abs() < 35.0,
                "measured {measured} Hz for weights {weights:?}"
            );
        }
    }

    #[test]
    fn yin_tracks_slow_contour_without_systematic_octave_jumps() {
        let sample_rate = 48_000_u32;
        let mut phase = 0.0_f32;
        let samples = (0..sample_rate as usize)
            .map(|index| {
                let progress = index as f32 / sample_rate as f32;
                let frequency = 160.0 + 80.0 * progress;
                phase += TAU_F32 * frequency / sample_rate as f32;
                (phase.sin() + 0.4 * (2.0 * phase).sin() + 0.2 * (3.0 * phase).sin()) * 0.08
            })
            .collect::<Vec<_>>();
        let analysis = AudioAnalysis::new(&samples, sample_rate, 1).unwrap();
        let voiced = analysis
            .frames
            .iter()
            .filter_map(|frame| frame.f0_hz)
            .collect::<Vec<_>>();
        assert!(voiced.len() > 80);
        assert!(voiced
            .windows(2)
            .all(|pair| cents(pair[1] / pair[0]).abs() < 150.0));
    }

    #[test]
    fn yin_rejects_noise_between_voiced_regions() {
        let sample_rate = 48_000_u32;
        let voiced = harmonic_at(180.0, 12_000, sample_rate);
        let mut state = 0x1234_5678_u32;
        let noise = (0..12_000)
            .map(|_| {
                state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                ((state >> 8) as f32 / 16_777_215.0 - 0.5) * 0.2
            })
            .collect::<Vec<_>>();
        let samples = [voiced.as_slice(), noise.as_slice(), voiced.as_slice()].concat();
        let analysis = AudioAnalysis::new(&samples, sample_rate, 1).unwrap();
        let middle_voiced = analysis
            .frames
            .iter()
            .filter(|frame| (12_000..24_000).contains(&frame.center_frame) && frame.voiced)
            .count();
        let outer_voiced = analysis
            .frames
            .iter()
            .filter(|frame| !(12_000..24_000).contains(&frame.center_frame) && frame.voiced)
            .count();
        assert!(middle_voiced <= 2);
        assert!(outer_voiced > 35);
    }

    #[test]
    fn paired_frame_metric_avoids_ratio_of_medians_failure() {
        let source = analysis_from_track(&[10.0, 100.0, 1_000.0, 1_001.0, 1_002.0]);
        let output = analysis_from_track(&[20.0, 200.0, 2_000.0, 10.01, 10.02]);
        let (_, pitch, _, _, _, _) = compare_audio(&source, &output, &[], Some(2.0), &[], 0.0);
        assert!(pitch.pitch_error_cents.unwrap().abs() < f64::EPSILON);
        assert_eq!(pitch.measured_pitch_ratio, Some(2.0));
        assert!(pitch.legacy_pitch_error_cents.unwrap().abs() > 1_000.0);
    }

    #[test]
    fn source_track_is_shared_by_fingerprint_and_analysis_is_deterministic() {
        let samples = harmonic(220.0, 24_000);
        let source_a = AudioAnalysis::new(&samples, 48_000, 1).unwrap();
        let source_b = AudioAnalysis::new(&samples, 48_000, 1).unwrap();
        let output_a = AudioAnalysis::new(&harmonic(330.0, 24_000), 48_000, 1).unwrap();
        let output_b = AudioAnalysis::new(&harmonic(440.0, 24_000), 48_000, 1).unwrap();
        let (_, pitch_a, _, _, _, _) =
            compare_audio(&source_a, &output_a, &[], Some(1.5), &[], 0.0);
        let (_, pitch_b, _, _, _, _) =
            compare_audio(&source_b, &output_b, &[], Some(2.0), &[], 0.0);
        assert_eq!(
            pitch_a.source_track_fingerprint,
            pitch_b.source_track_fingerprint
        );
        assert_eq!(
            source_a
                .frames
                .iter()
                .map(|frame| frame.f0_hz)
                .collect::<Vec<_>>(),
            source_b
                .frames
                .iter()
                .map(|frame| frame.f0_hz)
                .collect::<Vec<_>>()
        );
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
