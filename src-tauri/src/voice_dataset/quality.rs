use serde::{Deserialize, Serialize};

pub const SILENCE_THRESHOLD: f32 = 0.012;
pub const CLIPPING_THRESHOLD: f32 = 0.995;
pub const MIN_TAKE_MS: u64 = 1_000;
pub const MAX_TAKE_MS: u64 = 20_000;
const MAX_EDGE_SILENCE_MS: u64 = 1_500;
const FAIL_EDGE_SILENCE_MS: u64 = 4_000;
const LOW_RMS: f32 = 0.012;
const HIGH_RMS: f32 = 0.55;

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureMetrics {
    pub callback_gaps: u64,
    pub queue_overflow_count: u64,
    pub dropped_frames: u64,
    pub maximum_observed_level: f32,
    pub non_finite_input_count: u64,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum QualityClassification {
    Pass,
    Warning,
    Fail,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum QualityReasonCode {
    TooShort,
    TooLong,
    Clipping,
    LevelTooLow,
    LevelTooHigh,
    ExcessiveLeadingSilence,
    ExcessiveTrailingSilence,
    ExcessiveSilence,
    LowEstimatedSnr,
    PossibleDropout,
    CaptureOverflow,
    NonFiniteInput,
    UnsupportedFormat,
    ManualReviewRequired,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QualityReason {
    pub code: QualityReasonCode,
    pub guidance: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TakeQualityReport {
    pub classification: QualityClassification,
    pub reasons: Vec<QualityReason>,
    pub duration_ms: u64,
    pub peak_amplitude: f32,
    pub rms_level: f32,
    pub clipped_sample_count: u64,
    pub clipped_sample_ratio: f32,
    pub dc_offset: f32,
    pub leading_silence_ms: u64,
    pub trailing_silence_ms: u64,
    pub total_low_energy_ratio: f32,
    pub estimated_active_speech_ratio: f32,
    pub estimated_background_noise_floor: f32,
    pub heuristic_signal_to_noise_db: f32,
    pub consecutive_zero_regions: u64,
    pub recording_queue_overflow_count: u64,
    pub dropped_frames: u64,
    pub callback_gaps: u64,
    pub non_finite_sample_count_before_sanitization: u64,
    pub sample_rate: u32,
    pub channels: u16,
}

pub fn analyze_take(
    samples: &[f32],
    sample_rate: u32,
    capture: CaptureMetrics,
) -> TakeQualityReport {
    let frames = samples.len() as u64;
    let duration_ms = frames.saturating_mul(1_000) / u64::from(sample_rate.max(1));
    let mut peak = 0.0_f32;
    let mut sum = 0.0_f64;
    let mut sum_squares = 0.0_f64;
    let mut clipped = 0_u64;
    let mut low_energy = 0_u64;
    let mut zero_regions = 0_u64;
    let mut in_zero_region = false;
    let mut quiet_energy = Vec::new();

    for sample in samples.iter().copied() {
        let finite = if sample.is_finite() { sample } else { 0.0 };
        let absolute = finite.abs();
        peak = peak.max(absolute);
        sum += f64::from(finite);
        sum_squares += f64::from(finite) * f64::from(finite);
        clipped += u64::from(absolute >= CLIPPING_THRESHOLD);
        if absolute < SILENCE_THRESHOLD {
            low_energy += 1;
            quiet_energy.push(absolute);
        }
        let zero = absolute <= f32::EPSILON;
        if zero && !in_zero_region {
            zero_regions += 1;
        }
        in_zero_region = zero;
    }

    let count = samples.len().max(1) as f64;
    let rms = (sum_squares / count).sqrt() as f32;
    let dc_offset = (sum / count) as f32;
    let clipped_ratio = clipped as f32 / samples.len().max(1) as f32;
    let low_ratio = low_energy as f32 / samples.len().max(1) as f32;
    quiet_energy.sort_by(f32::total_cmp);
    let noise_floor = quiet_energy
        .get(quiet_energy.len().saturating_mul(8) / 10)
        .copied()
        .unwrap_or(0.000_001)
        .max(0.000_001);
    let estimated_snr = 20.0 * (rms.max(0.000_001) / noise_floor).log10();
    let leading_frames = samples
        .iter()
        .position(|sample| sample.abs() >= SILENCE_THRESHOLD)
        .unwrap_or(samples.len()) as u64;
    let trailing_frames = samples
        .iter()
        .rposition(|sample| sample.abs() >= SILENCE_THRESHOLD)
        .map_or(samples.len() as u64, |index| {
            (samples.len() - index - 1) as u64
        });
    let leading_ms = leading_frames.saturating_mul(1_000) / u64::from(sample_rate.max(1));
    let trailing_ms = trailing_frames.saturating_mul(1_000) / u64::from(sample_rate.max(1));

    let mut reasons = Vec::new();
    let mut classification = QualityClassification::Pass;
    let mut add = |code, guidance: &str, severity: QualityClassification| {
        reasons.push(QualityReason {
            code,
            guidance: guidance.to_owned(),
        });
        classification = worse(classification, severity);
    };
    if duration_ms < MIN_TAKE_MS {
        add(
            QualityReasonCode::TooShort,
            "Record the complete phrase for at least one second.",
            QualityClassification::Fail,
        );
    }
    if duration_ms > MAX_TAKE_MS {
        add(
            QualityReasonCode::TooLong,
            "Use a shorter phrase and keep prompted takes under twenty seconds.",
            QualityClassification::Fail,
        );
    }
    if clipped > 0 {
        add(
            QualityReasonCode::Clipping,
            "Lower the Windows microphone level or move slightly farther away.",
            if clipped_ratio > 0.005 {
                QualityClassification::Fail
            } else {
                QualityClassification::Warning
            },
        );
    }
    if rms < LOW_RMS {
        add(
            QualityReasonCode::LevelTooLow,
            "Move closer to the microphone and speak naturally.",
            QualityClassification::Warning,
        );
    }
    if rms > HIGH_RMS {
        add(
            QualityReasonCode::LevelTooHigh,
            "Lower the microphone level and redo the take.",
            QualityClassification::Warning,
        );
    }
    for (duration, code, guidance) in [
        (
            leading_ms,
            QualityReasonCode::ExcessiveLeadingSilence,
            "Wait only briefly after pressing Record before speaking.",
        ),
        (
            trailing_ms,
            QualityReasonCode::ExcessiveTrailingSilence,
            "Avoid leaving a long pause after the phrase.",
        ),
    ] {
        if duration > MAX_EDGE_SILENCE_MS {
            add(
                code,
                guidance,
                if duration > FAIL_EDGE_SILENCE_MS {
                    QualityClassification::Fail
                } else {
                    QualityClassification::Warning
                },
            );
        }
    }
    if low_ratio > 0.75 {
        add(
            QualityReasonCode::ExcessiveSilence,
            "Redo the take with the full phrase clearly audible.",
            QualityClassification::Fail,
        );
    }
    if estimated_snr < 12.0 && rms >= LOW_RMS {
        add(
            QualityReasonCode::LowEstimatedSnr,
            "Reduce background noise and avoid touching the microphone.",
            QualityClassification::Warning,
        );
    }
    if zero_regions > 4 && low_ratio < 0.98 {
        add(
            QualityReasonCode::PossibleDropout,
            "Redo this take and check the microphone connection.",
            QualityClassification::Warning,
        );
    }
    if capture.queue_overflow_count > 0 || capture.dropped_frames > 0 {
        add(
            QualityReasonCode::CaptureOverflow,
            "The recording queue overflowed; redo this take before accepting it.",
            QualityClassification::Fail,
        );
    }
    if capture.non_finite_input_count > 0 {
        add(
            QualityReasonCode::NonFiniteInput,
            "Invalid input samples were sanitized; redo the take.",
            QualityClassification::Fail,
        );
    }
    if reasons.is_empty() {
        reasons.push(QualityReason {
            code: QualityReasonCode::ManualReviewRequired,
            guidance:
                "Listen through headphones and confirm the phrase is complete before accepting."
                    .to_owned(),
        });
    }

    TakeQualityReport {
        classification,
        reasons,
        duration_ms,
        peak_amplitude: peak,
        rms_level: rms,
        clipped_sample_count: clipped,
        clipped_sample_ratio: clipped_ratio,
        dc_offset,
        leading_silence_ms: leading_ms,
        trailing_silence_ms: trailing_ms,
        total_low_energy_ratio: low_ratio,
        estimated_active_speech_ratio: 1.0 - low_ratio,
        estimated_background_noise_floor: noise_floor,
        heuristic_signal_to_noise_db: estimated_snr,
        consecutive_zero_regions: zero_regions,
        recording_queue_overflow_count: capture.queue_overflow_count,
        dropped_frames: capture.dropped_frames,
        callback_gaps: capture.callback_gaps,
        non_finite_sample_count_before_sanitization: capture.non_finite_input_count,
        sample_rate,
        channels: 1,
    }
}

fn worse(left: QualityClassification, right: QualityClassification) -> QualityClassification {
    use QualityClassification::{Fail, Pass, Warning};
    match (left, right) {
        (Fail, _) | (_, Fail) => Fail,
        (Warning, _) | (_, Warning) => Warning,
        _ => Pass,
    }
}

#[cfg(test)]
mod tests {
    use super::{analyze_take, CaptureMetrics, QualityClassification, QualityReasonCode};

    #[test]
    fn measures_silence_peak_rms_clipping_dc_and_overflow() {
        let mut samples = vec![0.0; 48_000];
        samples.extend(vec![0.25; 48_000]);
        samples[50_000] = 1.0;
        let report = analyze_take(
            &samples,
            48_000,
            CaptureMetrics {
                queue_overflow_count: 1,
                ..CaptureMetrics::default()
            },
        );
        assert_eq!(report.duration_ms, 2_000);
        assert_eq!(report.leading_silence_ms, 1_000);
        assert_eq!(report.peak_amplitude, 1.0);
        assert!(report.rms_level > 0.1);
        assert!(report.dc_offset > 0.1);
        assert_eq!(report.classification, QualityClassification::Fail);
        assert!(report
            .reasons
            .iter()
            .any(|reason| reason.code == QualityReasonCode::CaptureOverflow));
    }

    #[test]
    fn distinguishes_pass_warning_and_fail() {
        let clean: Vec<f32> = (0..96_000)
            .map(|index| ((index as f32 * 0.03).sin()) * 0.2)
            .collect();
        assert_ne!(
            analyze_take(&clean, 48_000, CaptureMetrics::default()).classification,
            QualityClassification::Fail
        );
        assert_eq!(
            analyze_take(&[0.0; 100], 48_000, CaptureMetrics::default()).classification,
            QualityClassification::Fail
        );
    }
}
