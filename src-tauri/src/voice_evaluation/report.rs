use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

use crate::dsp::chain::DspParameters;

use super::{
    analysis::{
        median_formant_error, median_formant_ratio, AnalysisConfiguration, ConsonantMetrics,
        FormantMetric, NumericalMetrics, PerformanceMetrics, PitchMetrics, SpectralMetrics,
        StructuralMetrics, VoicingMetrics,
    },
    manifest::{EvaluationRenderer, MetricExpectations},
    world::WorldRenderMetadata,
};

pub const REPORT_SCHEMA_VERSION: u32 = 2;
pub const PREVIOUS_REPORT_SCHEMA_VERSION: u32 = 1;
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
    #[serde(default)]
    pub renderer_summaries: Vec<RendererSummary>,
    #[serde(default)]
    pub cross_renderer_comparisons: Vec<CrossRendererComparison>,
    #[serde(default)]
    pub relative_expectations: Vec<RelativeExpectationResult>,
    pub baseline_comparison: Option<BaselineComparison>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaseReport {
    pub id: String,
    pub description: String,
    pub input: String,
    #[serde(default)]
    pub renderer: EvaluationRenderer,
    #[serde(default)]
    pub comparison_group: Option<String>,
    #[serde(default)]
    pub parameters: DspParameters,
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
    #[serde(default)]
    pub world: Option<WorldRenderMetadata>,
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

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RendererSummary {
    pub renderer: EvaluationRenderer,
    pub total_cases: usize,
    pub passed_expectations: usize,
    pub failed_expectations: usize,
    pub unavailable_metrics: usize,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CrossRendererComparison {
    pub comparison_group: String,
    pub cases: Vec<CrossRendererCaseMetrics>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CrossRendererCaseMetrics {
    pub case_id: String,
    pub renderer: EvaluationRenderer,
    pub pitch_error_cents: Option<f64>,
    pub formant_ratio_error_cents: Option<f64>,
    pub voiced_unvoiced_disagreement: f64,
    pub unvoiced_high_frequency_lsd_db: Option<f64>,
    pub unvoiced_correlation: Option<f64>,
    pub clipping_ratio: f64,
    pub rms_change_db: Option<f64>,
    pub duration_delta_frames: i64,
    pub real_time_factor: f64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RelativeExpectationResult {
    pub renderer: EvaluationRenderer,
    pub metric: String,
    pub case_ids: Vec<String>,
    pub passed: bool,
    pub explanation: String,
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
    #[serde(default)]
    pub renderer: EvaluationRenderer,
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
        let relative_expectations = evaluate_relative_expectations(&cases);
        let summary = summarize(&cases, &relative_expectations);
        let renderer_summaries = summarize_renderers(&cases, &relative_expectations);
        let cross_renderer_comparisons = cross_renderer_comparisons(&cases);
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
            renderer_summaries,
            cross_renderer_comparisons,
            relative_expectations,
            baseline_comparison: None,
            warnings: vec![
                "Objective metrics do not establish subjective naturalness or intelligibility."
                    .to_owned(),
                "Wall-clock performance is machine- and build-dependent.".to_owned(),
            ],
        }
    }

    pub fn from_json(contents: &str) -> Result<Self, String> {
        let mut report: Self = serde_json::from_str(contents)
            .map_err(|error| format!("Baseline report is not valid: {error}"))?;
        if !matches!(
            report.schema_version,
            PREVIOUS_REPORT_SCHEMA_VERSION | REPORT_SCHEMA_VERSION
        ) {
            return Err(format!(
                "Unsupported baseline report schema version {}. Supported versions are {} and {}.",
                report.schema_version, PREVIOUS_REPORT_SCHEMA_VERSION, REPORT_SCHEMA_VERSION
            ));
        }
        let mut ids = BTreeSet::new();
        if report
            .cases
            .iter()
            .any(|case| !ids.insert((case.id.as_str(), case.renderer)))
        {
            return Err(
                "Baseline report contains duplicate case ids for the same renderer.".to_owned(),
            );
        }
        if report.schema_version == PREVIOUS_REPORT_SCHEMA_VERSION {
            report.schema_version = REPORT_SCHEMA_VERSION;
            report.renderer_summaries =
                summarize_renderers(&report.cases, &report.relative_expectations);
            report.cross_renderer_comparisons = cross_renderer_comparisons(&report.cases);
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
        .map(|case| ((case.id.as_str(), case.renderer), case))
        .collect::<BTreeMap<_, _>>();
    let baseline_cases = baseline
        .cases
        .iter()
        .map(|case| ((case.id.as_str(), case.renderer), case))
        .collect::<BTreeMap<_, _>>();
    let added_cases = current_cases
        .keys()
        .filter(|id| !baseline_cases.contains_key(*id))
        .map(|(id, renderer)| format!("{id}@{}", renderer.as_str()))
        .collect::<Vec<_>>();
    let missing_cases = baseline_cases
        .keys()
        .filter(|id| !current_cases.contains_key(*id))
        .map(|(id, renderer)| format!("{id}@{}", renderer.as_str()))
        .collect::<Vec<_>>();
    let mut cases = Vec::new();
    let mut improvements = 0;
    let mut regressions = 0;
    let mut unchanged = 0;
    for ((id, renderer), current_case) in &current_cases {
        let Some(baseline_case) = baseline_cases.get(&(*id, *renderer)) else {
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
            renderer: *renderer,
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
        "id,renderer,comparisonGroup,description,passedExpectations,failedExpectations,pitchErrorCents,voicingDisagreement,unvoicedLsdDb,unvoicedCorrelation,hfEnergyRatio,formantRatioErrorCents,outputClippingRatio,outputNonFinite,durationDeltaFrames,realTimeFactor,worldRevision,worldRawSynthesisFrames,worldDurationAdjustment,worldChannelPolicy\n",
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
            case.renderer.as_str().to_owned(),
            case.comparison_group
                .as_deref()
                .map(csv_field)
                .unwrap_or_default(),
            csv_field(&case.description),
            passed.to_string(),
            failed.to_string(),
            optional_number(case.pitch.pitch_error_cents),
            format_number(case.voicing.voiced_unvoiced_disagreement_ratio),
            optional_number(case.spectral.unvoiced_log_spectral_distance_db),
            optional_number(case.consonant.unvoiced_waveform_correlation),
            optional_number(case.consonant.unvoiced_high_frequency_energy_ratio),
            optional_number(median_formant_error(&case.formants)),
            format_number(case.numerical.output_clipping_ratio),
            case.numerical.output_non_finite_samples.to_string(),
            case.structural.duration_delta_frames.to_string(),
            format_number(case.performance.real_time_factor),
            case.world
                .as_ref()
                .map(|world| world.revision.clone())
                .unwrap_or_default(),
            case.world
                .as_ref()
                .map(|world| world.raw_synthesis_frames.to_string())
                .unwrap_or_default(),
            case.world
                .as_ref()
                .map(|world| world.duration_adjustment.as_str().to_owned())
                .unwrap_or_default(),
            case.world
                .as_ref()
                .map(|world| world.channel_policy.as_str().to_owned())
                .unwrap_or_default(),
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
    output.push_str("## Cases\n\n");
    output.push_str("| Case | Renderer | Group | Pitch error (cents) | V/UV disagreement | HF ratio | Formant error (cents) | Non-finite | RTF | Result |\n");
    output.push_str("| --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |\n");
    for case in &report.cases {
        let failed = case
            .expectations
            .iter()
            .filter(|result| !result.passed)
            .count();
        output.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |\n",
            case.id,
            case.renderer.as_str(),
            case.comparison_group.as_deref().unwrap_or(""),
            optional_number(case.pitch.pitch_error_cents),
            format_number(case.voicing.voiced_unvoiced_disagreement_ratio),
            optional_number(case.consonant.unvoiced_high_frequency_energy_ratio),
            optional_number(median_formant_error(&case.formants)),
            case.numerical.output_non_finite_samples,
            format_number(case.performance.real_time_factor),
            if failed == 0 { "pass" } else { "fail" }
        ));
    }
    output.push_str("\n## Renderer summaries\n\n");
    output.push_str(
        "| Renderer | Cases | Passed expectations | Failed expectations | Unavailable metrics |\n",
    );
    output.push_str("| --- | ---: | ---: | ---: | ---: |\n");
    for summary in &report.renderer_summaries {
        output.push_str(&format!(
            "| {} | {} | {} | {} | {} |\n",
            summary.renderer.as_str(),
            summary.total_cases,
            summary.passed_expectations,
            summary.failed_expectations,
            summary.unavailable_metrics
        ));
    }
    output.push_str("\n## Relative expectations\n\n");
    for expectation in &report.relative_expectations {
        output.push_str(&format!(
            "- `{}` / `{}`: {} — {}\n",
            expectation.renderer.as_str(),
            expectation.metric,
            if expectation.passed { "pass" } else { "fail" },
            expectation.explanation
        ));
    }
    output.push_str("\n## Cross-renderer comparisons\n\n");
    output.push_str("These paired measurements are descriptive and do not declare a winner.\n\n");
    output.push_str("| Group | Case | Renderer | Pitch error | Formant error | V/UV | Unvoiced HF LSD | Unvoiced correlation | Clipping | RMS change | Duration delta | RTF |\n");
    output.push_str(
        "| --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |\n",
    );
    for comparison in &report.cross_renderer_comparisons {
        for case in &comparison.cases {
            output.push_str(&format!(
                "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |\n",
                comparison.comparison_group,
                case.case_id,
                case.renderer.as_str(),
                optional_number(case.pitch_error_cents),
                optional_number(case.formant_ratio_error_cents),
                format_number(case.voiced_unvoiced_disagreement),
                optional_number(case.unvoiced_high_frequency_lsd_db),
                optional_number(case.unvoiced_correlation),
                format_number(case.clipping_ratio),
                optional_number(case.rms_change_db),
                case.duration_delta_frames,
                format_number(case.real_time_factor),
            ));
        }
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

fn summarize(
    cases: &[CaseReport],
    relative_expectations: &[RelativeExpectationResult],
) -> ReportSummary {
    ReportSummary {
        total_cases: cases.len(),
        passed_expectations: cases
            .iter()
            .flat_map(|case| &case.expectations)
            .filter(|expectation| expectation.passed)
            .count()
            + relative_expectations
                .iter()
                .filter(|expectation| expectation.passed)
                .count(),
        failed_expectations: cases
            .iter()
            .flat_map(|case| &case.expectations)
            .filter(|expectation| !expectation.passed)
            .count()
            + relative_expectations
                .iter()
                .filter(|expectation| !expectation.passed)
                .count(),
        unavailable_metrics: cases
            .iter()
            .map(|case| case.unavailable_metrics.len())
            .sum(),
    }
}

fn summarize_renderers(
    cases: &[CaseReport],
    relative_expectations: &[RelativeExpectationResult],
) -> Vec<RendererSummary> {
    let mut groups = BTreeMap::<EvaluationRenderer, Vec<&CaseReport>>::new();
    for case in cases {
        groups.entry(case.renderer).or_default().push(case);
    }
    groups
        .into_iter()
        .map(|(renderer, cases)| RendererSummary {
            renderer,
            total_cases: cases.len(),
            passed_expectations: cases
                .iter()
                .flat_map(|case| &case.expectations)
                .filter(|expectation| expectation.passed)
                .count()
                + relative_expectations
                    .iter()
                    .filter(|expectation| expectation.renderer == renderer && expectation.passed)
                    .count(),
            failed_expectations: cases
                .iter()
                .flat_map(|case| &case.expectations)
                .filter(|expectation| !expectation.passed)
                .count()
                + relative_expectations
                    .iter()
                    .filter(|expectation| expectation.renderer == renderer && !expectation.passed)
                    .count(),
            unavailable_metrics: cases
                .iter()
                .map(|case| case.unavailable_metrics.len())
                .sum(),
        })
        .collect()
}

fn evaluate_relative_expectations(cases: &[CaseReport]) -> Vec<RelativeExpectationResult> {
    let preservation = |amount: f32| {
        cases.iter().find(|case| {
            case.renderer == EvaluationRenderer::WorldReference
                && case.tags.iter().any(|tag| tag == "preservation")
                && (case.parameters.consonant_preservation - amount).abs() <= f32::EPSILON
        })
    };
    let (Some(none), Some(half), Some(full)) =
        (preservation(0.0), preservation(0.5), preservation(1.0))
    else {
        return Vec::new();
    };
    let case_ids = vec![none.id.clone(), half.id.clone(), full.id.clone()];
    let correlation_improves = match (
        none.consonant.unvoiced_waveform_correlation,
        full.consonant.unvoiced_waveform_correlation,
    ) {
        (Some(none), Some(full)) => full > none,
        _ => false,
    };
    let spectral_improves = match (
        none.spectral
            .high_frequency_unvoiced_log_spectral_distance_db,
        full.spectral
            .high_frequency_unvoiced_log_spectral_distance_db,
    ) {
        (Some(none), Some(full)) => full < none,
        _ => false,
    };
    let correlation_between = match (
        none.consonant.unvoiced_waveform_correlation,
        half.consonant.unvoiced_waveform_correlation,
        full.consonant.unvoiced_waveform_correlation,
    ) {
        (Some(none), Some(half), Some(full)) => half > none.min(full) && half < none.max(full),
        _ => false,
    };
    let spectral_between = match (
        none.spectral
            .high_frequency_unvoiced_log_spectral_distance_db,
        half.spectral
            .high_frequency_unvoiced_log_spectral_distance_db,
        full.spectral
            .high_frequency_unvoiced_log_spectral_distance_db,
    ) {
        (Some(none), Some(half), Some(full)) => half > none.min(full) && half < none.max(full),
        _ => false,
    };
    vec![
        RelativeExpectationResult {
            renderer: EvaluationRenderer::WorldReference,
            metric: "preservationImprovesUnvoicedCorrelation".to_owned(),
            case_ids: case_ids.clone(),
            passed: correlation_improves,
            explanation:
                "Preservation 1.0 must have higher aligned unvoiced correlation than 0.0."
                    .to_owned(),
        },
        RelativeExpectationResult {
            renderer: EvaluationRenderer::WorldReference,
            metric: "preservationImprovesUnvoicedSpectralSimilarity".to_owned(),
            case_ids: case_ids.clone(),
            passed: spectral_improves,
            explanation:
                "Preservation 1.0 must have lower high-frequency unvoiced LSD than 0.0."
                    .to_owned(),
        },
        RelativeExpectationResult {
            renderer: EvaluationRenderer::WorldReference,
            metric: "preservationHalfIsIntermediate".to_owned(),
            case_ids,
            passed: correlation_between || spectral_between,
            explanation:
                "Preservation 0.5 must fall strictly between 0.0 and 1.0 for correlation or high-frequency LSD."
                    .to_owned(),
        },
    ]
}

fn cross_renderer_comparisons(cases: &[CaseReport]) -> Vec<CrossRendererComparison> {
    let mut groups = BTreeMap::<&str, Vec<&CaseReport>>::new();
    for case in cases {
        if let Some(group) = &case.comparison_group {
            groups.entry(group).or_default().push(case);
        }
    }
    groups
        .into_iter()
        .filter(|(_, cases)| {
            cases
                .iter()
                .map(|case| case.renderer)
                .collect::<BTreeSet<_>>()
                .len()
                > 1
        })
        .map(|(comparison_group, mut cases)| {
            cases.sort_by_key(|case| (case.renderer, case.id.as_str()));
            CrossRendererComparison {
                comparison_group: comparison_group.to_owned(),
                cases: cases
                    .into_iter()
                    .map(|case| CrossRendererCaseMetrics {
                        case_id: case.id.clone(),
                        renderer: case.renderer,
                        pitch_error_cents: case.pitch.pitch_error_cents.map(f64::abs),
                        formant_ratio_error_cents: median_formant_error(&case.formants),
                        voiced_unvoiced_disagreement: case
                            .voicing
                            .voiced_unvoiced_disagreement_ratio,
                        unvoiced_high_frequency_lsd_db: case
                            .spectral
                            .high_frequency_unvoiced_log_spectral_distance_db,
                        unvoiced_correlation: case.consonant.unvoiced_waveform_correlation,
                        clipping_ratio: case.numerical.output_clipping_ratio,
                        rms_change_db: case.numerical.rms_change_db,
                        duration_delta_frames: case.structural.duration_delta_frames,
                        real_time_factor: case.performance.real_time_factor,
                    })
                    .collect(),
            }
        })
        .collect()
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
            renderer: EvaluationRenderer::ExistingDsp,
            comparison_group: None,
            parameters: DspParameters::default(),
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
            world: None,
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
        report.summary = summarize(&report.cases, &report.relative_expectations);
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
        assert_eq!(comparison.added_cases, ["added@existingDsp"]);
        assert_eq!(comparison.missing_cases, ["missing@existingDsp"]);
        assert!(comparison.improvements > 0);
        assert!(comparison.regressions > 0);
        assert!(comparison
            .cases
            .iter()
            .flat_map(|case| &case.metrics)
            .all(|metric| metric.metric != "realTimeFactor"));
    }

    #[test]
    fn renderer_identity_cross_groups_and_legacy_reports_are_safe() {
        let mut existing = case("neutral-existing");
        existing.comparison_group = Some("neutral".to_owned());
        let mut world = case("neutral-world");
        world.renderer = EvaluationRenderer::WorldReference;
        world.comparison_group = Some("neutral".to_owned());
        world.world = Some(WorldRenderMetadata::default());
        let report = EvaluationReport::new(
            "manifest.json".to_owned(),
            "release".to_owned(),
            vec![existing.clone(), world],
        );
        assert_eq!(report.renderer_summaries.len(), 2);
        assert_eq!(report.cross_renderer_comparisons.len(), 1);
        assert_eq!(report.cross_renderer_comparisons[0].cases.len(), 2);
        let json = serde_json::to_string(&report).unwrap();
        assert!(json.contains("\"renderer\":\"worldReference\""));
        assert!(json.contains("\"world\":"));
        assert!(csv(&report).contains("worldReference"));
        assert!(markdown(&report).contains("Cross-renderer comparisons"));

        let baseline = EvaluationReport::new(
            "manifest.json".to_owned(),
            "release".to_owned(),
            vec![existing],
        );
        let comparison = compare_baseline(
            &EvaluationReport::new(
                "manifest.json".to_owned(),
                "release".to_owned(),
                vec![case("neutral-existing")],
            ),
            &baseline,
        );
        assert!(comparison
            .cases
            .iter()
            .all(|case| { case.renderer == EvaluationRenderer::ExistingDsp }));
        let mut renderer_changed = case("stable-id");
        renderer_changed.renderer = EvaluationRenderer::WorldReference;
        let identity_comparison = compare_baseline(
            &EvaluationReport::new(
                "manifest.json".to_owned(),
                "release".to_owned(),
                vec![renderer_changed],
            ),
            &EvaluationReport::new(
                "manifest.json".to_owned(),
                "release".to_owned(),
                vec![case("stable-id")],
            ),
        );
        assert!(identity_comparison.cases.is_empty());
        assert_eq!(
            identity_comparison.added_cases,
            ["stable-id@worldReference"]
        );
        assert_eq!(identity_comparison.missing_cases, ["stable-id@existingDsp"]);

        let mut legacy = serde_json::to_value(EvaluationReport::new(
            "manifest.json".to_owned(),
            "debug".to_owned(),
            vec![case("legacy")],
        ))
        .unwrap();
        legacy["schemaVersion"] = serde_json::Value::from(1);
        let root = legacy.as_object_mut().unwrap();
        root.remove("rendererSummaries");
        root.remove("crossRendererComparisons");
        root.remove("relativeExpectations");
        let case = root["cases"][0].as_object_mut().unwrap();
        case.remove("renderer");
        case.remove("comparisonGroup");
        case.remove("parameters");
        case.remove("world");
        let migrated =
            EvaluationReport::from_json(&serde_json::to_string(&legacy).unwrap()).unwrap();
        assert_eq!(migrated.schema_version, REPORT_SCHEMA_VERSION);
        assert_eq!(migrated.cases[0].renderer, EvaluationRenderer::ExistingDsp);
    }
}
