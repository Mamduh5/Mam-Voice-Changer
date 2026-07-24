use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

use super::{
    analysis::{
        median_formant_error, median_formant_ratio, AnalysisConfiguration, ConsonantMetrics,
        FormantMetric, NumericalMetrics, PerformanceMetrics, PitchMetrics, SpectralMetrics,
        StructuralMetrics, VoicingMetrics,
    },
    manifest::MetricExpectations,
};

pub const REPORT_SCHEMA_VERSION: u32 = 1;
pub const TOOL_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EvaluationReport {
    pub schema_version: u32,
    pub tool_version: String,
    pub generated_at_unix_seconds: u64,
    pub build_mode: String,
    pub source_manifest: String,
    pub analysis_configuration: AnalysisConfiguration,
    pub cases: Vec<CaseReport>,
    pub summary: ReportSummary,
    pub baseline_comparison: Option<BaselineComparison>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaseReport {
    pub id: String,
    pub description: String,
    pub input: String,
    pub tags: Vec<String>,
    pub structural: StructuralMetrics,
    pub numerical: NumericalMetrics,
    pub pitch: PitchMetrics,
    pub voicing: VoicingMetrics,
    pub spectral: SpectralMetrics,
    pub consonant: ConsonantMetrics,
    pub formants: Vec<FormantMetric>,
    pub performance: PerformanceMetrics,
    pub expectations: Vec<ExpectationResult>,
    pub unavailable_metrics: Vec<String>,
    pub rendered_audio: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExpectationResult {
    pub metric: String,
    pub comparator: String,
    pub threshold: f64,
    pub measured: Option<f64>,
    pub passed: bool,
    pub explanation: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReportSummary {
    pub total_cases: usize,
    pub passed_expectations: usize,
    pub failed_expectations: usize,
    pub unavailable_metrics: usize,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BaselineComparison {
    pub schema_version: u32,
    pub cases: Vec<BaselineCaseComparison>,
    pub added_cases: Vec<String>,
    pub missing_cases: Vec<String>,
    pub improvements: usize,
    pub regressions: usize,
    pub unchanged: usize,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BaselineCaseComparison {
    pub case_id: String,
    pub metrics: Vec<MetricComparison>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MetricComparison {
    pub metric: String,
    pub baseline: f64,
    pub current: f64,
    pub delta: f64,
    pub classification: ChangeClassification,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ChangeClassification {
    Improvement,
    Regression,
    Unchanged,
}

impl EvaluationReport {
    pub fn new(source_manifest: String, build_mode: String, mut cases: Vec<CaseReport>) -> Self {
        cases.sort_by(|left, right| left.id.cmp(&right.id));
        let summary = summarize(&cases);
        Self {
            schema_version: REPORT_SCHEMA_VERSION,
            tool_version: TOOL_VERSION.to_owned(),
            generated_at_unix_seconds: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            build_mode,
            source_manifest,
            analysis_configuration: AnalysisConfiguration::default(),
            cases,
            summary,
            baseline_comparison: None,
            warnings: vec![
                "Objective metrics do not establish subjective naturalness or intelligibility."
                    .to_owned(),
                "Wall-clock performance is machine- and build-dependent.".to_owned(),
            ],
        }
    }

    pub fn from_json(contents: &str) -> Result<Self, String> {
        let report: Self = serde_json::from_str(contents)
            .map_err(|error| format!("Baseline report is not valid: {error}"))?;
        if report.schema_version != REPORT_SCHEMA_VERSION {
            return Err(format!(
                "Unsupported baseline report schema version {}. Expected {}.",
                report.schema_version, REPORT_SCHEMA_VERSION
            ));
        }
        let mut ids = BTreeSet::new();
        if report.cases.iter().any(|case| !ids.insert(&case.id)) {
            return Err("Baseline report contains duplicate case ids.".to_owned());
        }
        Ok(report)
    }

    pub fn failed_expectations(&self) -> usize {
        self.summary.failed_expectations
    }
}

pub fn evaluate_expectations(
    expected: &MetricExpectations,
    case: &CaseReport,
) -> Vec<ExpectationResult> {
    let mut results = Vec::new();
    maximum_u64(
        &mut results,
        "durationDeltaFrames",
        expected.maximum_duration_delta_frames,
        case.structural.duration_delta_frames.unsigned_abs(),
    );
    maximum_u64(
        &mut results,
        "nonFiniteSamples",
        expected.maximum_non_finite_samples,
        case.numerical.input_non_finite_samples + case.numerical.output_non_finite_samples,
    );
    maximum(
        &mut results,
        "outputClippingRatio",
        expected.maximum_output_clipping_ratio,
        Some(case.numerical.output_clipping_ratio),
        None,
    );
    maximum(
        &mut results,
        "pitchErrorCents",
        expected.maximum_pitch_error_cents,
        case.pitch.pitch_error_cents.map(f64::abs),
        case.pitch.unavailable_reason.clone(),
    );
    maximum(
        &mut results,
        "voicedUnvoicedDisagreement",
        expected.maximum_voiced_unvoiced_disagreement,
        Some(case.voicing.voiced_unvoiced_disagreement_ratio),
        None,
    );
    minimum(
        &mut results,
        "voicedFrameCoverage",
        expected.minimum_voiced_frame_coverage,
        Some(case.pitch.f0_estimation_coverage),
        None,
    );
    let neutral_drift = case
        .pitch
        .measured_pitch_ratio
        .map(|ratio| 1_200.0 * ratio.max(1.0e-12).log2())
        .map(f64::abs);
    maximum(
        &mut results,
        "neutralF0DriftCents",
        expected.maximum_neutral_f0_drift_cents,
        neutral_drift,
        case.pitch.unavailable_reason.clone(),
    );
    maximum(
        &mut results,
        "neutralRmsChangeDb",
        expected.maximum_neutral_rms_change_db,
        case.numerical.rms_change_db.map(f64::abs),
        Some("RMS change is unavailable for zero-energy input or output.".to_owned()),
    );
    minimum(
        &mut results,
        "formantRatio",
        expected.minimum_formant_ratio,
        median_formant_ratio(&case.formants),
        Some("No unambiguous configured formant-band peaks were available.".to_owned()),
    );
    maximum(
        &mut results,
        "formantRatio",
        expected.maximum_formant_ratio,
        median_formant_ratio(&case.formants),
        Some("No unambiguous configured formant-band peaks were available.".to_owned()),
    );
    minimum(
        &mut results,
        "unvoicedHighFrequencyEnergyRatio",
        expected.minimum_unvoiced_high_frequency_energy_ratio,
        case.consonant.unvoiced_high_frequency_energy_ratio,
        Some("No usable source-unvoiced high-frequency frames were available.".to_owned()),
    );
    maximum(
        &mut results,
        "unvoicedLogSpectralDistanceDb",
        expected.maximum_unvoiced_log_spectral_distance_db,
        case.spectral.unvoiced_log_spectral_distance_db,
        Some("No source-unvoiced frames were available.".to_owned()),
    );
    maximum(
        &mut results,
        "realTimeFactor",
        expected.maximum_real_time_factor,
        Some(case.performance.real_time_factor),
        None,
    );
    results
}

pub fn unavailable_metrics(case: &CaseReport) -> Vec<String> {
    let mut unavailable = Vec::new();
    if let Some(reason) = &case.pitch.unavailable_reason {
        unavailable.push(format!("pitch: {reason}"));
    }
    if case.spectral.unvoiced_log_spectral_distance_db.is_none() {
        unavailable.push("unvoicedLogSpectralDistanceDb: no unvoiced frames".to_owned());
    }
    if case
        .consonant
        .unvoiced_high_frequency_energy_ratio
        .is_none()
    {
        unavailable.push("unvoicedHighFrequencyEnergyRatio: no usable unvoiced energy".to_owned());
    }
    for formant in &case.formants {
        if let Some(reason) = &formant.unavailable_reason {
            unavailable.push(format!("formant {}: {reason}", formant.label));
        }
    }
    unavailable
}

pub fn compare_baseline(
    current: &EvaluationReport,
    baseline: &EvaluationReport,
) -> BaselineComparison {
    let current_cases = current
        .cases
        .iter()
        .map(|case| (case.id.as_str(), case))
        .collect::<BTreeMap<_, _>>();
    let baseline_cases = baseline
        .cases
        .iter()
        .map(|case| (case.id.as_str(), case))
        .collect::<BTreeMap<_, _>>();
    let added_cases = current_cases
        .keys()
        .filter(|id| !baseline_cases.contains_key(**id))
        .map(|id| (*id).to_owned())
        .collect::<Vec<_>>();
    let missing_cases = baseline_cases
        .keys()
        .filter(|id| !current_cases.contains_key(**id))
        .map(|id| (*id).to_owned())
        .collect::<Vec<_>>();
    let mut cases = Vec::new();
    let mut improvements = 0;
    let mut regressions = 0;
    let mut unchanged = 0;
    for (id, current_case) in &current_cases {
        let Some(baseline_case) = baseline_cases.get(id) else {
            continue;
        };
        let mut metrics = Vec::new();
        for (metric, baseline_value, current_value, lower_is_better) in
            comparable_metrics(baseline_case, current_case)
        {
            let delta = current_value - baseline_value;
            let tolerance = 1.0e-9_f64.max(baseline_value.abs() * 1.0e-9);
            let classification = if delta.abs() <= tolerance {
                unchanged += 1;
                ChangeClassification::Unchanged
            } else if (delta < 0.0) == lower_is_better {
                improvements += 1;
                ChangeClassification::Improvement
            } else {
                regressions += 1;
                ChangeClassification::Regression
            };
            metrics.push(MetricComparison {
                metric,
                baseline: baseline_value,
                current: current_value,
                delta,
                classification,
            });
        }
        cases.push(BaselineCaseComparison {
            case_id: (*id).to_owned(),
            metrics,
        });
    }
    BaselineComparison {
        schema_version: REPORT_SCHEMA_VERSION,
        cases,
        added_cases,
        missing_cases,
        improvements,
        regressions,
        unchanged,
    }
}

fn comparable_metrics(
    baseline: &CaseReport,
    current: &CaseReport,
) -> Vec<(String, f64, f64, bool)> {
    let mut metrics = Vec::new();
    push_optional(
        &mut metrics,
        "pitchErrorCents",
        baseline.pitch.pitch_error_cents.map(f64::abs),
        current.pitch.pitch_error_cents.map(f64::abs),
        true,
    );
    push_optional(
        &mut metrics,
        "voicedUnvoicedDisagreement",
        Some(baseline.voicing.voiced_unvoiced_disagreement_ratio),
        Some(current.voicing.voiced_unvoiced_disagreement_ratio),
        true,
    );
    push_optional(
        &mut metrics,
        "unvoicedLogSpectralDistanceDb",
        baseline.spectral.unvoiced_log_spectral_distance_db,
        current.spectral.unvoiced_log_spectral_distance_db,
        true,
    );
    let baseline_hf_error = baseline
        .consonant
        .unvoiced_high_frequency_energy_ratio
        .map(|value| (value - 1.0).abs());
    let current_hf_error = current
        .consonant
        .unvoiced_high_frequency_energy_ratio
        .map(|value| (value - 1.0).abs());
    push_optional(
        &mut metrics,
        "highFrequencyPreservationRatioError",
        baseline_hf_error,
        current_hf_error,
        true,
    );
    push_optional(
        &mut metrics,
        "formantRatioErrorCents",
        median_formant_error(&baseline.formants),
        median_formant_error(&current.formants),
        true,
    );
    push_optional(
        &mut metrics,
        "outputClippingRatio",
        Some(baseline.numerical.output_clipping_ratio),
        Some(current.numerical.output_clipping_ratio),
        true,
    );
    push_optional(
        &mut metrics,
        "outputNonFiniteSamples",
        Some(baseline.numerical.output_non_finite_samples as f64),
        Some(current.numerical.output_non_finite_samples as f64),
        true,
    );
    push_optional(
        &mut metrics,
        "durationDeltaFrames",
        Some(baseline.structural.duration_delta_frames.unsigned_abs() as f64),
        Some(current.structural.duration_delta_frames.unsigned_abs() as f64),
        true,
    );
    metrics
}

fn push_optional(
    target: &mut Vec<(String, f64, f64, bool)>,
    name: &str,
    baseline: Option<f64>,
    current: Option<f64>,
    lower_is_better: bool,
) {
    if let (Some(baseline), Some(current)) = (baseline, current) {
        target.push((name.to_owned(), baseline, current, lower_is_better));
    }
}

pub fn write_reports(report: &EvaluationReport, output: &Path) -> Result<(), String> {
    fs::create_dir_all(output)
        .map_err(|error| format!("Cannot create report directory: {error}"))?;
    let json = serde_json::to_string_pretty(report)
        .map_err(|error| format!("Cannot serialize report JSON: {error}"))?;
    fs::write(output.join("report.json"), format!("{json}\n"))
        .map_err(|error| format!("Cannot write report.json: {error}"))?;
    fs::write(output.join("cases.csv"), csv(report))
        .map_err(|error| format!("Cannot write cases.csv: {error}"))?;
    fs::write(output.join("report.md"), markdown(report))
        .map_err(|error| format!("Cannot write report.md: {error}"))?;
    Ok(())
}

fn csv(report: &EvaluationReport) -> String {
    let mut output = String::from(
        "id,description,passedExpectations,failedExpectations,pitchErrorCents,voicingDisagreement,unvoicedLsdDb,hfEnergyRatio,formantRatioErrorCents,outputClippingRatio,outputNonFinite,durationDeltaFrames,realTimeFactor\n",
    );
    for case in &report.cases {
        let passed = case
            .expectations
            .iter()
            .filter(|result| result.passed)
            .count();
        let failed = case.expectations.len() - passed;
        let values = [
            csv_field(&case.id),
            csv_field(&case.description),
            passed.to_string(),
            failed.to_string(),
            optional_number(case.pitch.pitch_error_cents),
            format_number(case.voicing.voiced_unvoiced_disagreement_ratio),
            optional_number(case.spectral.unvoiced_log_spectral_distance_db),
            optional_number(case.consonant.unvoiced_high_frequency_energy_ratio),
            optional_number(median_formant_error(&case.formants)),
            format_number(case.numerical.output_clipping_ratio),
            case.numerical.output_non_finite_samples.to_string(),
            case.structural.duration_delta_frames.to_string(),
            format_number(case.performance.real_time_factor),
        ];
        output.push_str(&values.join(","));
        output.push('\n');
    }
    output
}

fn markdown(report: &EvaluationReport) -> String {
    let mut output = format!(
        "# Voice evaluation report\n\n- Cases: {}\n- Passed expectations: {}\n- Failed expectations: {}\n- Unavailable metrics: {}\n- Build mode: {}\n\n",
        report.summary.total_cases,
        report.summary.passed_expectations,
        report.summary.failed_expectations,
        report.summary.unavailable_metrics,
        report.build_mode
    );
    output.push_str("| Case | Pitch error (cents) | V/UV disagreement | HF ratio | Formant error (cents) | Non-finite | RTF | Result |\n");
    output.push_str("| --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |\n");
    for case in &report.cases {
        let failed = case
            .expectations
            .iter()
            .filter(|result| !result.passed)
            .count();
        output.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} |\n",
            case.id,
            optional_number(case.pitch.pitch_error_cents),
            format_number(case.voicing.voiced_unvoiced_disagreement_ratio),
            optional_number(case.consonant.unvoiced_high_frequency_energy_ratio),
            optional_number(median_formant_error(&case.formants)),
            case.numerical.output_non_finite_samples,
            format_number(case.performance.real_time_factor),
            if failed == 0 { "pass" } else { "fail" }
        ));
    }
    output.push_str("\n## Metric limitations\n\n");
    output
        .push_str("- Objective metrics do not replace listening tests or establish naturalness.\n");
    output.push_str(
        "- Spectral distance is descriptive; lower is not universally better for intentional transformations.\n",
    );
    output.push_str(
        "- Formant peaks are synthetic-fixture envelope estimates, not clinical measurements.\n",
    );
    output.push_str("- Real-time factor varies by machine and build mode.\n");
    if let Some(baseline) = &report.baseline_comparison {
        output.push_str(&format!(
            "\n## Baseline comparison\n\n- Improvements: {}\n- Regressions: {}\n- Unchanged: {}\n- Added cases: {}\n- Missing cases: {}\n",
            baseline.improvements,
            baseline.regressions,
            baseline.unchanged,
            baseline.added_cases.len(),
            baseline.missing_cases.len()
        ));
    }
    output
}

fn summarize(cases: &[CaseReport]) -> ReportSummary {
    ReportSummary {
        total_cases: cases.len(),
        passed_expectations: cases
            .iter()
            .flat_map(|case| &case.expectations)
            .filter(|expectation| expectation.passed)
            .count(),
        failed_expectations: cases
            .iter()
            .flat_map(|case| &case.expectations)
            .filter(|expectation| !expectation.passed)
            .count(),
        unavailable_metrics: cases
            .iter()
            .map(|case| case.unavailable_metrics.len())
            .sum(),
    }
}

fn maximum_u64(
    target: &mut Vec<ExpectationResult>,
    metric: &str,
    threshold: Option<u64>,
    measured: u64,
) {
    maximum(
        target,
        metric,
        threshold.map(|value| value as f64),
        Some(measured as f64),
        None,
    );
}

fn maximum(
    target: &mut Vec<ExpectationResult>,
    metric: &str,
    threshold: Option<f64>,
    measured: Option<f64>,
    unavailable: Option<String>,
) {
    if let Some(threshold) = threshold {
        expectation(target, metric, "<=", threshold, measured, unavailable);
    }
}

fn minimum(
    target: &mut Vec<ExpectationResult>,
    metric: &str,
    threshold: Option<f64>,
    measured: Option<f64>,
    unavailable: Option<String>,
) {
    if let Some(threshold) = threshold {
        expectation(target, metric, ">=", threshold, measured, unavailable);
    }
}

fn expectation(
    target: &mut Vec<ExpectationResult>,
    metric: &str,
    comparator: &str,
    threshold: f64,
    measured: Option<f64>,
    unavailable: Option<String>,
) {
    let passed = measured.is_some_and(|value| {
        if comparator == "<=" {
            value <= threshold
        } else {
            value >= threshold
        }
    });
    target.push(ExpectationResult {
        metric: metric.to_owned(),
        comparator: comparator.to_owned(),
        threshold,
        measured,
        passed,
        explanation: measured
            .is_none()
            .then(|| unavailable.unwrap_or_else(|| "Metric was unavailable.".to_owned())),
    });
}

fn csv_field(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_owned()
    }
}

fn optional_number(value: Option<f64>) -> String {
    value.map(format_number).unwrap_or_default()
}

fn format_number(value: f64) -> String {
    format!("{value:.8}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::voice_evaluation::analysis::{
        ConsonantMetrics, NumericalMetrics, PerformanceMetrics, PitchMetrics, SpectralMetrics,
        StructuralMetrics, VoicingMetrics,
    };

    fn case(id: &str) -> CaseReport {
        CaseReport {
            id: id.to_owned(),
            description: "comma, quote \" and private D:\\secret".to_owned(),
            input: "fixtures/test.wav".to_owned(),
            tags: Vec::new(),
            structural: StructuralMetrics {
                input_sample_rate: 48_000,
                output_sample_rate: 48_000,
                input_channels: 1,
                output_channels: 1,
                input_frames: 48_000,
                output_frames: 48_000,
                duration_delta_frames: 0,
                reported_dsp_latency_frames: 2_288,
                reported_dsp_latency_ms: 47.666,
                input_sanitized: false,
            },
            numerical: NumericalMetrics {
                input_non_finite_samples: 0,
                output_non_finite_samples: 0,
                input_peak: 0.2,
                output_peak: 0.2,
                input_rms: 0.1,
                output_rms: 0.1,
                rms_change_db: Some(0.0),
                input_dc_offset: 0.0,
                output_dc_offset: 0.0,
                input_clipping_ratio: 0.0,
                output_clipping_ratio: 0.0,
            },
            pitch: PitchMetrics {
                median_input_f0_hz: Some(220.0),
                median_output_f0_hz: Some(440.0),
                measured_pitch_ratio: Some(2.0),
                expected_pitch_ratio: Some(2.0),
                pitch_error_cents: Some(0.0),
                voiced_frame_count: 50,
                f0_estimation_coverage: 1.0,
                unavailable_reason: None,
            },
            voicing: VoicingMetrics {
                source_voiced_frame_ratio: 1.0,
                output_voiced_frame_ratio: 1.0,
                voiced_unvoiced_disagreement_ratio: 0.0,
                voiced_to_unvoiced_errors: 0,
                unvoiced_to_voiced_errors: 0,
                compared_frames: 50,
            },
            spectral: SpectralMetrics {
                mean_log_spectral_distance_db: Some(2.0),
                median_log_spectral_distance_db: Some(2.0),
                voiced_log_spectral_distance_db: Some(2.0),
                unvoiced_log_spectral_distance_db: None,
                high_frequency_unvoiced_log_spectral_distance_db: None,
            },
            consonant: ConsonantMetrics {
                source_unvoiced_high_frequency_energy: None,
                output_unvoiced_high_frequency_energy: None,
                unvoiced_high_frequency_energy_ratio: None,
                unvoiced_waveform_correlation: None,
            },
            formants: Vec::new(),
            performance: PerformanceMetrics {
                render_wall_time_ms: 1.0,
                rendered_audio_duration_seconds: 1.0,
                real_time_factor: 0.001,
                processing_ms_per_audio_second: 1.0,
                build_mode: "debug".to_owned(),
            },
            expectations: Vec::new(),
            unavailable_metrics: Vec::new(),
            rendered_audio: None,
        }
    }

    #[test]
    fn json_csv_markdown_and_summary_are_stable_and_safe() {
        let mut report = EvaluationReport::new(
            "manifest.json".to_owned(),
            "debug".to_owned(),
            vec![case("z"), case("a")],
        );
        report.cases[0].expectations.push(ExpectationResult {
            metric: "pitch".to_owned(),
            comparator: "<=".to_owned(),
            threshold: 1.0,
            measured: Some(0.0),
            passed: true,
            explanation: None,
        });
        report.summary = summarize(&report.cases);
        assert_eq!(report.cases[0].id, "a");
        assert_eq!(report.summary.passed_expectations, 1);
        let json = serde_json::to_string(&report).unwrap();
        assert_eq!(EvaluationReport::from_json(&json).unwrap(), report);
        let csv = csv(&report);
        assert!(csv.contains("\"comma, quote \"\" and private D:\\secret\""));
        let markdown = markdown(&report);
        assert!(markdown.contains("Voice evaluation report"));
        assert!(!json.contains("C:\\\\Users\\\\"));
    }

    #[test]
    fn baseline_classifies_quality_changes_and_ignores_timing() {
        let baseline = EvaluationReport::new(
            "manifest.json".to_owned(),
            "debug".to_owned(),
            vec![case("existing"), case("missing")],
        );
        let mut improved = case("existing");
        improved.voicing.voiced_unvoiced_disagreement_ratio = -0.01;
        improved.numerical.output_clipping_ratio = 0.1;
        improved.performance.real_time_factor = 99.0;
        let current = EvaluationReport::new(
            "manifest.json".to_owned(),
            "debug".to_owned(),
            vec![improved, case("added")],
        );
        let comparison = compare_baseline(&current, &baseline);
        assert_eq!(comparison.added_cases, ["added"]);
        assert_eq!(comparison.missing_cases, ["missing"]);
        assert!(comparison.improvements > 0);
        assert!(comparison.regressions > 0);
        assert!(comparison
            .cases
            .iter()
            .flat_map(|case| &case.metrics)
            .all(|metric| metric.metric != "realTimeFactor"));
    }
}
