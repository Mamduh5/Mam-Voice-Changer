use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    time::Instant,
};

use super::{
    analysis::{audio_shape, compare_audio, AudioAnalysis, PerformanceMetrics, StructuralMetrics},
    manifest::{resolve_case_input, EvaluationCase, EvaluationManifest, EvaluationRenderer},
    report::{
        compare_baseline, evaluate_expectations, unavailable_metrics, write_reports, CaseReport,
        EvaluationReport,
    },
    world::{WorldReferenceProcessor, WorldRenderMetadata},
};
use crate::voice_lab::{
    offline::{ExistingDspOfflineProcessor, OfflineVoiceProcessor},
    wav,
};

#[derive(Clone, Debug)]
pub struct EvaluationOptions {
    pub output_directory: PathBuf,
    pub no_rendered_audio: bool,
    pub baseline_report: Option<PathBuf>,
}

pub fn evaluate_manifest_file(
    manifest_path: &Path,
    options: &EvaluationOptions,
) -> Result<EvaluationReport, String> {
    let manifest_contents = fs::read_to_string(manifest_path)
        .map_err(|error| format!("Cannot read evaluation manifest: {error}"))?;
    let manifest = EvaluationManifest::from_json(&manifest_contents)?;
    let corpus_root = manifest.resolve_corpus_root(manifest_path)?;
    let mut cases = Vec::with_capacity(manifest.cases.len());
    let mut source_analyses = BTreeMap::new();
    let mut ordered = manifest.cases.clone();
    ordered.sort_by(|left, right| left.id.cmp(&right.id));
    for case in &ordered {
        cases.push(evaluate_case(
            case,
            &corpus_root,
            &options.output_directory,
            options.no_rendered_audio,
            &mut source_analyses,
        )?);
    }

    let safe_manifest_label = manifest_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("evaluation-manifest.json")
        .to_owned();
    let mut report = EvaluationReport::new(safe_manifest_label, build_mode(), cases);
    if let Some(baseline_path) = &options.baseline_report {
        let contents = fs::read_to_string(baseline_path)
            .map_err(|error| format!("Cannot read baseline report: {error}"))?;
        let baseline = EvaluationReport::from_json(&contents)?;
        report.baseline_comparison = Some(compare_baseline(&report, &baseline));
    }
    write_reports(&report, &options.output_directory)?;
    Ok(report)
}

fn evaluate_case(
    case: &EvaluationCase,
    corpus_root: &Path,
    output_directory: &Path,
    no_rendered_audio: bool,
    source_analyses: &mut BTreeMap<PathBuf, AudioAnalysis>,
) -> Result<CaseReport, String> {
    let input_path = resolve_case_input(corpus_root, &case.input)?;
    let input = wav::import(&input_path)
        .map_err(|error| format!("Case '{}' input failed: {error}", case.id))?;
    validate_segments(case, input.frames(), input.sample_rate)?;

    let started = Instant::now();
    let (rendered_clip, reported_latency_frames, mut world_metadata): (
        _,
        usize,
        Option<WorldRenderMetadata>,
    ) = match case.renderer {
        EvaluationRenderer::ExistingDsp => {
            let mut processor = ExistingDspOfflineProcessor;
            let rendered = processor
                .render(&input, case.parameters)
                .map_err(|error| format!("Case '{}' rendering failed: {error}", case.id))?;
            (rendered.clip, rendered.metadata.latency_frames, None)
        }
        EvaluationRenderer::WorldReference => {
            let mut processor = WorldReferenceProcessor::default();
            let rendered = processor
                .render(&input, case.parameters)
                .map_err(|error| format!("Case '{}' rendering failed: {error}", case.id))?;
            (rendered.clip, 0, Some(rendered.metadata))
        }
    };
    let elapsed = started.elapsed();

    if !source_analyses.contains_key(&input_path) {
        source_analyses.insert(
            input_path.clone(),
            AudioAnalysis::new(&input.samples, input.sample_rate, input.channels)?,
        );
    }
    let input_analysis = source_analyses
        .get(&input_path)
        .ok_or_else(|| "Source analysis cache did not retain the input track.".to_owned())?;
    let output_analysis = AudioAnalysis::new(
        &rendered_clip.samples,
        rendered_clip.sample_rate,
        rendered_clip.channels,
    )?;
    let (numerical, pitch, voicing, spectral, consonant, formants) = compare_audio(
        input_analysis,
        &output_analysis,
        &case.segments,
        case.expected_pitch_ratio,
        &case.formant_bands,
        case.parameters.formant_shift_semitones,
    );
    let (input_rate, input_channels, input_frames, _) = audio_shape(input_analysis);
    let (output_rate, output_channels, output_frames, _) = audio_shape(&output_analysis);
    let duration_seconds = input_frames as f64 / f64::from(input_rate);
    let render_seconds = elapsed.as_secs_f64();
    let structural = StructuralMetrics {
        input_sample_rate: input_rate,
        output_sample_rate: output_rate,
        input_channels,
        output_channels,
        input_frames,
        output_frames,
        duration_delta_frames: output_frames as i64 - input_frames as i64,
        reported_dsp_latency_frames: reported_latency_frames,
        reported_dsp_latency_ms: reported_latency_frames as f64 * 1_000.0 / f64::from(input_rate),
        input_sanitized: false,
    };
    let performance = PerformanceMetrics {
        render_wall_time_ms: render_seconds * 1_000.0,
        rendered_audio_duration_seconds: duration_seconds,
        real_time_factor: render_seconds / duration_seconds.max(f64::EPSILON),
        processing_ms_per_audio_second: render_seconds * 1_000.0
            / duration_seconds.max(f64::EPSILON),
        build_mode: build_mode(),
    };
    if let Some(world) = &mut world_metadata {
        world.render_wall_time_ms = performance.render_wall_time_ms;
        world.real_time_factor = performance.real_time_factor;
    }
    let rendered_audio = if no_rendered_audio {
        None
    } else {
        let relative = format!("rendered/{}.wav", case.id);
        let target = output_directory.join(&relative);
        let parent = target
            .parent()
            .ok_or_else(|| "Rendered output path has no parent.".to_owned())?;
        fs::create_dir_all(parent)
            .map_err(|error| format!("Cannot create rendered output directory: {error}"))?;
        wav::export(&target, &rendered_clip)
            .map_err(|error| format!("Case '{}' output failed: {error}", case.id))?;
        Some(relative)
    };
    let mut report = CaseReport {
        id: case.id.clone(),
        description: case.description.clone(),
        input: case.input.clone(),
        renderer: case.renderer,
        comparison_group: case.comparison_group.clone(),
        parameters: case.parameters,
        tags: case.tags.clone(),
        structural,
        numerical,
        pitch,
        voicing,
        spectral,
        consonant,
        formants,
        performance,
        expectations: Vec::new(),
        unavailable_metrics: Vec::new(),
        rendered_audio,
        world: world_metadata,
    };
    report.expectations = evaluate_expectations(&case.expectations, &report);
    report.unavailable_metrics = unavailable_metrics(&report);
    Ok(report)
}

fn validate_segments(case: &EvaluationCase, frames: usize, sample_rate: u32) -> Result<(), String> {
    let duration_ms = frames as u64 * 1_000 / u64::from(sample_rate);
    for segment in &case.segments {
        if segment.end_ms > duration_ms {
            return Err(format!(
                "Case '{}' segment '{}' ends at {} ms, beyond the {} ms input.",
                case.id, segment.label, segment.end_ms, duration_ms
            ));
        }
    }
    for pair in case.segments.windows(2) {
        if pair[1].start_ms < pair[0].start_ms {
            return Err(format!(
                "Case '{}' segments must use deterministic start-time ordering.",
                case.id
            ));
        }
    }
    Ok(())
}

fn build_mode() -> String {
    if cfg!(debug_assertions) {
        "debug".to_owned()
    } else {
        "release".to_owned()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use hound::{SampleFormat, WavSpec, WavWriter};

    use super::*;
    use crate::{
        dsp::chain::DspParameters,
        voice_evaluation::manifest::{
            AnalysisSegment, MetricExpectations, SegmentKind, MANIFEST_SCHEMA_VERSION,
        },
        voice_evaluation::{fixtures::generate_example, report::CaseReport},
    };

    static SEQUENCE: AtomicU64 = AtomicU64::new(0);

    fn directory(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "mam-voice-eval-{label}-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn reports_missing_malformed_and_unsupported_audio_clearly() {
        let root = directory("input-errors");
        fs::create_dir_all(&root).unwrap();
        let missing = root.join("manifest.json");
        let manifest = EvaluationManifest {
            schema_version: MANIFEST_SCHEMA_VERSION,
            corpus_root: ".".to_owned(),
            cases: vec![EvaluationCase {
                id: "missing".to_owned(),
                description: "missing".to_owned(),
                input: "missing.wav".to_owned(),
                renderer: EvaluationRenderer::ExistingDsp,
                comparison_group: None,
                parameters: DspParameters::default(),
                expected_pitch_ratio: None,
                segments: Vec::new(),
                formant_bands: Vec::new(),
                expectations: MetricExpectations::default(),
                tags: Vec::new(),
            }],
        };
        fs::write(&missing, serde_json::to_string(&manifest).unwrap()).unwrap();
        let options = EvaluationOptions {
            output_directory: root.join("report"),
            no_rendered_audio: true,
            baseline_report: None,
        };
        assert!(evaluate_manifest_file(&missing, &options)
            .unwrap_err()
            .contains("Cannot read input WAV"));

        fs::write(root.join("missing.wav"), b"not a wave").unwrap();
        assert!(evaluate_manifest_file(&missing, &options)
            .unwrap_err()
            .contains("Cannot open WAV"));

        let unsupported = root.join("missing.wav");
        let mut writer = WavWriter::create(
            &unsupported,
            WavSpec {
                channels: 1,
                sample_rate: 32_000,
                bits_per_sample: 16,
                sample_format: SampleFormat::Int,
            },
        )
        .unwrap();
        writer.write_sample::<i16>(0).unwrap();
        writer.finalize().unwrap();
        assert!(evaluate_manifest_file(&missing, &options)
            .unwrap_err()
            .contains("44.1 kHz and 48 kHz"));

        let mut writer = WavWriter::create(
            &unsupported,
            WavSpec {
                channels: 1,
                sample_rate: 48_000,
                bits_per_sample: 32,
                sample_format: SampleFormat::Float,
            },
        )
        .unwrap();
        writer.write_sample::<f32>(f32::NAN).unwrap();
        writer.finalize().unwrap();
        assert!(evaluate_manifest_file(&missing, &options)
            .unwrap_err()
            .contains("invalid audio samples"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn invalid_out_of_bounds_segment_is_rejected_after_wav_validation() {
        let root = directory("segment");
        fs::create_dir_all(&root).unwrap();
        let wav_path = root.join("short.wav");
        let mut writer = WavWriter::create(
            &wav_path,
            WavSpec {
                channels: 1,
                sample_rate: 48_000,
                bits_per_sample: 16,
                sample_format: SampleFormat::Int,
            },
        )
        .unwrap();
        for _ in 0..4_800 {
            writer.write_sample::<i16>(0).unwrap();
        }
        writer.finalize().unwrap();
        let case = EvaluationCase {
            id: "segment".to_owned(),
            description: "segment".to_owned(),
            input: "short.wav".to_owned(),
            renderer: EvaluationRenderer::ExistingDsp,
            comparison_group: None,
            parameters: DspParameters::default(),
            expected_pitch_ratio: None,
            segments: vec![AnalysisSegment {
                label: "too-long".to_owned(),
                start_ms: 0,
                end_ms: 200,
                kind: SegmentKind::All,
            }],
            formant_bands: Vec::new(),
            expectations: MetricExpectations::default(),
            tags: Vec::new(),
        };
        assert!(validate_segments(&case, 4_800, 48_000).is_err());
        let _ = fs::remove_dir_all(root);
    }

    fn find<'a>(report: &'a EvaluationReport, id: &str) -> &'a CaseReport {
        report.cases.iter().find(|case| case.id == id).unwrap()
    }

    #[test]
    fn generated_corpus_proves_pitch_preservation_formant_and_structural_metrics() {
        let root = directory("generated-evaluation");
        let manifest = generate_example(&root).unwrap();
        let report = evaluate_manifest_file(
            &manifest,
            &EvaluationOptions {
                output_directory: root.join("report"),
                no_rendered_audio: true,
                baseline_report: None,
            },
        )
        .unwrap();
        let existing_summary = report
            .renderer_summaries
            .iter()
            .find(|summary| summary.renderer == EvaluationRenderer::ExistingDsp)
            .unwrap();
        assert_eq!(existing_summary.failed_expectations, 0);
        assert_eq!(report.cross_renderer_comparisons.len(), 13);
        assert_eq!(report.pitch_analysis.pitch_estimator, "yinCmndf");
        assert_eq!(
            find(&report, "pitch-up-twelve")
                .pitch
                .source_track_fingerprint,
            find(&report, "pitch-up-twelve-world")
                .pitch
                .source_track_fingerprint
        );
        for id in ["pitch-up-twelve", "pitch-down-twelve", "pitch-up-seven"] {
            assert!(find(&report, id).pitch.pitch_error_cents.unwrap().abs() < 45.0);
            assert!(
                find(&report, &format!("{id}-world"))
                    .pitch
                    .pitch_error_cents
                    .unwrap()
                    .abs()
                    <= 35.0
            );
        }
        assert_eq!(
            find(&report, "silence-safety")
                .pitch
                .unavailable_reason
                .as_deref(),
            Some("notEnoughVoicedFrames")
        );

        let none = find(&report, "preservation-0");
        let half = find(&report, "preservation-5");
        let full = find(&report, "preservation-10");
        assert!(
            none.consonant.unvoiced_waveform_correlation.unwrap()
                < half.consonant.unvoiced_waveform_correlation.unwrap()
        );
        assert!(
            half.consonant.unvoiced_waveform_correlation.unwrap()
                < full.consonant.unvoiced_waveform_correlation.unwrap()
        );
        assert!(
            none.spectral
                .high_frequency_unvoiced_log_spectral_distance_db
                .unwrap()
                > half
                    .spectral
                    .high_frequency_unvoiced_log_spectral_distance_db
                    .unwrap()
        );
        assert!(
            half.spectral
                .high_frequency_unvoiced_log_spectral_distance_db
                .unwrap()
                > full
                    .spectral
                    .high_frequency_unvoiced_log_spectral_distance_db
                    .unwrap()
        );
        assert!(full.pitch.pitch_error_cents.unwrap().abs() < 45.0);

        let formant_up = find(&report, "formant-up-four");
        let formant_down = find(&report, "formant-down-four");
        let formant_up_world = find(&report, "formant-up-four-world");
        let formant_down_world = find(&report, "formant-down-four-world");
        assert!(super::super::analysis::median_formant_ratio(&formant_up.formants).unwrap() > 1.0);
        assert!(
            super::super::analysis::median_formant_ratio(&formant_down.formants).unwrap() < 1.0
        );
        assert!(formant_up.pitch.pitch_error_cents.unwrap().abs() < 10.0);
        assert!(formant_down.pitch.pitch_error_cents.unwrap().abs() < 10.0);
        assert!(formant_up_world.pitch.pitch_error_cents.unwrap().abs() <= 15.0);
        assert!(formant_down_world.pitch.pitch_error_cents.unwrap().abs() <= 15.0);
        for formant in &formant_up_world.formants {
            assert!(formant.output_peak_hz.unwrap() > formant.input_peak_hz.unwrap());
            assert!(formant.ratio_error_cents.unwrap().abs() <= 100.0);
        }
        for formant in &formant_down_world.formants {
            assert!(formant.output_peak_hz.unwrap() < formant.input_peak_hz.unwrap());
            assert!(formant.ratio_error_cents.unwrap().abs() <= 100.0);
        }
        assert!(
            super::super::analysis::median_formant_error(&formant_up_world.formants).unwrap()
                <= 100.0
        );
        assert!(
            super::super::analysis::median_formant_error(&formant_down_world.formants).unwrap()
                <= 100.0
        );
        for formant_case in [formant_up_world, formant_down_world] {
            assert_eq!(formant_case.structural.duration_delta_frames, 0);
            assert_eq!(formant_case.numerical.output_clipping_ratio, 0.0);
            assert_eq!(formant_case.numerical.output_non_finite_samples, 0);
        }

        let mono_44 = find(&report, "neutral-44100-mono");
        assert_eq!(mono_44.structural.input_sample_rate, 44_100);
        assert_eq!(mono_44.structural.output_sample_rate, 44_100);
        assert_eq!(mono_44.structural.input_channels, 1);
        let stereo_48 = find(&report, "neutral-48000-stereo");
        assert_eq!(stereo_48.structural.input_sample_rate, 48_000);
        assert_eq!(stereo_48.structural.output_channels, 2);
        assert!(report.cases.iter().all(|case| {
            case.structural.duration_delta_frames == 0
                && case.numerical.output_non_finite_samples == 0
        }));
        assert!(report
            .relative_expectations
            .iter()
            .all(|expectation| expectation.passed));
        let silence_world = find(&report, "silence-safety-world");
        assert!(silence_world.numerical.output_rms <= 1.0e-8);
        assert!(report
            .cases
            .iter()
            .filter(|case| case.renderer == EvaluationRenderer::WorldReference)
            .all(|case| {
                case.world.is_some() && case.voicing.voiced_unvoiced_disagreement_ratio <= 0.10
            }));
        let _ = fs::remove_dir_all(root);
    }
}
