use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    time::Instant,
};

use serde::{Deserialize, Serialize};

use super::{
    manifest::{ListeningClip, ListeningManifest, ListeningStudy, ListeningTransform},
    PACKAGE_SCHEMA_VERSION,
};
use crate::{
    voice_dataset::hash::sha256_file,
    voice_evaluation::{
        analysis::{compare_audio, AudioAnalysis},
        world::WorldReferenceProcessor,
        PitchAnalysisMetadata,
    },
    voice_lab::{
        clip::AudioClip,
        offline::{ExistingDspOfflineProcessor, OfflineVoiceProcessor},
        wav,
    },
};

const CONSERVATIVE_PEAK: f32 = 0.95;

#[derive(Clone, Debug)]
pub struct PrepareOptions {
    pub manifest_path: PathBuf,
    pub output_directory: PathBuf,
    pub seed_override: Option<u64>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum Renderer {
    ExistingDsp,
    WorldReference,
}

impl Renderer {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::ExistingDsp => "existingDsp",
            Self::WorldReference => "worldReference",
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct AdministratorKey {
    pub schema_version: u32,
    pub study: ListeningStudy,
    pub seed: u64,
    #[serde(default)]
    pub pitch_analysis: PitchAnalysisMetadata,
    pub trials: Vec<TrialKey>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct TrialKey {
    pub trial_id: String,
    pub source_clip_id: String,
    pub a_renderer: Renderer,
    pub b_renderer: Renderer,
    pub transformation: ListeningTransform,
    pub transformation_type: String,
    pub tags: Vec<String>,
    pub source_hash: String,
    pub rendered_hashes: BTreeMap<String, String>,
    pub participant_hashes: BTreeMap<String, String>,
    pub common_safety_gain: f32,
    pub presentation_order: Vec<String>,
    pub seed: u64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackageSummary {
    pub schema_version: u32,
    #[serde(skip)]
    pub seed: u64,
    pub trial_count: usize,
    pub participant_directory: String,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug)]
struct RenderedTrial {
    key: TrialKey,
    metrics: Vec<(Renderer, ObjectiveMetrics)>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ObjectiveMetrics {
    pub pitch_error_cents: Option<f64>,
    pub median_source_f0_hz: Option<f64>,
    pub median_output_f0_hz: Option<f64>,
    pub measured_pitch_ratio: Option<f64>,
    pub mean_absolute_pitch_error_cents: Option<f64>,
    pub median_absolute_pitch_error_cents: Option<f64>,
    pub p10_pitch_error_cents: Option<f64>,
    pub p90_pitch_error_cents: Option<f64>,
    pub total_source_frames: usize,
    pub reliable_source_voiced_frames: usize,
    pub paired_reliable_frames: usize,
    pub paired_coverage: f64,
    pub unpaired_source_voiced_frames: usize,
    pub low_confidence_exclusions: usize,
    pub octave_doubling_count: usize,
    pub octave_halving_count: usize,
    pub large_non_octave_error_count: usize,
    pub octave_ambiguity_count: usize,
    pub source_track_fingerprint: String,
    pub legacy_pitch_error_cents: Option<f64>,
    pub legacy_source_median_f0_hz: Option<f64>,
    pub legacy_output_median_f0_hz: Option<f64>,
    pub legacy_paired_frames: usize,
    pub formant_error_cents: Option<f64>,
    pub voiced_unvoiced_disagreement: f64,
    pub unvoiced_high_frequency_lsd_db: Option<f64>,
    pub waveform_correlation: Option<f64>,
    pub clipping_ratio: f64,
    pub non_finite_count: u64,
    pub duration_delta_frames: i64,
    pub real_time_factor: f64,
}

pub fn prepare_package(options: &PrepareOptions) -> Result<PackageSummary, String> {
    require_empty_output(&options.output_directory)?;
    let manifest = ListeningManifest::from_file(&options.manifest_path)?;
    let resolved = manifest.resolve_inputs(&options.manifest_path)?;
    let seed = options.seed_override.unwrap_or(manifest.study.seed);
    let assignments = assignments(&resolved, seed);

    let participant = options.output_directory.join("participant");
    let participant_audio = participant.join("audio");
    let administrator = options.output_directory.join("administrator");
    let raw_rendered = administrator.join("raw-rendered");
    for directory in [&participant_audio, &raw_rendered] {
        fs::create_dir_all(directory)
            .map_err(|error| format!("Cannot create listening package directory: {error}"))?;
    }

    let mut trials = Vec::with_capacity(assignments.len());
    for assignment in assignments {
        trials.push(render_trial(
            assignment,
            seed,
            &participant_audio,
            &raw_rendered,
        )?);
    }

    write_participant_files(&participant, &manifest.study, &trials)?;
    write_administrator_files(&administrator, &manifest, seed, &trials)?;
    let summary = PackageSummary {
        schema_version: PACKAGE_SCHEMA_VERSION,
        seed,
        trial_count: trials.len(),
        participant_directory: "participant".to_owned(),
        warnings: vec![
            "No listening or rating occurred during package preparation.".to_owned(),
            "Administrator files unblind renderer identity and must not be shared before ratings are complete."
                .to_owned(),
            "Deleting this package does not delete the original local corpus.".to_owned(),
        ],
    };
    write_json(
        &options.output_directory.join("package-summary.json"),
        &summary,
    )?;
    Ok(summary)
}

pub(crate) fn require_empty_output(output: &Path) -> Result<(), String> {
    if !output.exists() {
        return Ok(());
    }
    if !output.is_dir() {
        return Err("Listening package output exists and is not a directory.".to_owned());
    }
    let mut entries = fs::read_dir(output)
        .map_err(|error| format!("Cannot inspect listening package output: {error}"))?;
    if entries
        .next()
        .transpose()
        .map_err(|error| format!("Cannot inspect listening package output directory: {error}"))?
        .is_some()
    {
        return Err(
            "Listening package output must not exist or must be an empty directory.".to_owned(),
        );
    }
    Ok(())
}

#[derive(Clone)]
struct Assignment {
    trial_id: String,
    clip: ListeningClip,
    input_path: PathBuf,
    a_renderer: Renderer,
    first_output: &'static str,
}

fn assignments(resolved: &[(ListeningClip, PathBuf)], seed: u64) -> Vec<Assignment> {
    let mut indexes = (0..resolved.len()).collect::<Vec<_>>();
    let mut random = DeterministicRandom::new(seed);
    random.shuffle(&mut indexes);
    let existing_first = random.next_u64() & 1 == 0;
    indexes
        .into_iter()
        .enumerate()
        .map(|(position, index)| {
            let (clip, path) = &resolved[index];
            let a_renderer = if (position % 2 == 0) == existing_first {
                Renderer::ExistingDsp
            } else {
                Renderer::WorldReference
            };
            Assignment {
                trial_id: format!("trial-{:03}", position + 1),
                clip: clip.clone(),
                input_path: path.clone(),
                a_renderer,
                first_output: if random.next_u64() & 1 == 0 { "A" } else { "B" },
            }
        })
        .collect()
}

fn render_trial(
    assignment: Assignment,
    seed: u64,
    participant_audio: &Path,
    raw_rendered: &Path,
) -> Result<RenderedTrial, String> {
    let input = wav::import(&assignment.input_path)
        .map_err(|error| format!("Clip '{}' input failed: {error}", assignment.clip.id))?;
    let parameters = assignment.clip.transform.parameters()?;

    let existing_started = Instant::now();
    let existing = ExistingDspOfflineProcessor
        .render(&input, parameters)
        .map_err(|error| {
            format!(
                "Clip '{}' existingDsp rendering failed: {error}",
                assignment.clip.id
            )
        })?
        .clip;
    let existing_elapsed = existing_started.elapsed().as_secs_f64();

    let world_started = Instant::now();
    let world = WorldReferenceProcessor::default()
        .render(&input, parameters)
        .map_err(|error| {
            format!(
                "Clip '{}' worldReference rendering failed: {error}",
                assignment.clip.id
            )
        })?
        .clip;
    let world_elapsed = world_started.elapsed().as_secs_f64();

    validate_render_shape(&assignment.clip.id, &input, &existing, "existingDsp")?;
    validate_render_shape(&assignment.clip.id, &input, &world, "worldReference")?;
    let common_gain = common_safety_gain(&assignment.clip.id, &existing, &world)?;
    let existing_listening = scaled_clip(&existing, common_gain)?;
    let world_listening = scaled_clip(&world, common_gain)?;

    let reference_path = participant_audio.join(format!("{}-reference.wav", assignment.trial_id));
    let a_path = participant_audio.join(format!("{}-a.wav", assignment.trial_id));
    let b_path = participant_audio.join(format!("{}-b.wav", assignment.trial_id));
    let existing_raw_path = raw_rendered.join(format!("{}-existing-dsp.wav", assignment.trial_id));
    let world_raw_path = raw_rendered.join(format!("{}-world-reference.wav", assignment.trial_id));
    wav::export(&reference_path, &input)?;
    wav::export(&existing_raw_path, &existing)?;
    wav::export(&world_raw_path, &world)?;
    let (a, b) = if assignment.a_renderer == Renderer::ExistingDsp {
        (&existing_listening, &world_listening)
    } else {
        (&world_listening, &existing_listening)
    };
    wav::export(&a_path, a)?;
    wav::export(&b_path, b)?;

    let source_hash = sha256_file(&assignment.input_path)
        .map_err(|error| format!("Cannot hash source clip '{}': {error}", assignment.clip.id))?;
    let rendered_hashes = BTreeMap::from([
        (
            Renderer::ExistingDsp.as_str().to_owned(),
            hash_file(&existing_raw_path)?,
        ),
        (
            Renderer::WorldReference.as_str().to_owned(),
            hash_file(&world_raw_path)?,
        ),
    ]);
    let participant_hashes = BTreeMap::from([
        ("reference".to_owned(), hash_file(&reference_path)?),
        ("a".to_owned(), hash_file(&a_path)?),
        ("b".to_owned(), hash_file(&b_path)?),
    ]);
    let duration_seconds = input.frames() as f64 / f64::from(input.sample_rate);
    let input_analysis = AudioAnalysis::new(&input.samples, input.sample_rate, input.channels)?;
    let metrics = vec![
        (
            Renderer::ExistingDsp,
            objective_metrics(
                &input_analysis,
                input.frames(),
                &existing,
                parameters.pitch_semitones,
                existing_elapsed / duration_seconds.max(f64::EPSILON),
            )?,
        ),
        (
            Renderer::WorldReference,
            objective_metrics(
                &input_analysis,
                input.frames(),
                &world,
                parameters.pitch_semitones,
                world_elapsed / duration_seconds.max(f64::EPSILON),
            )?,
        ),
    ];
    let b_renderer = if assignment.a_renderer == Renderer::ExistingDsp {
        Renderer::WorldReference
    } else {
        Renderer::ExistingDsp
    };
    Ok(RenderedTrial {
        key: TrialKey {
            trial_id: assignment.trial_id,
            source_clip_id: assignment.clip.id,
            a_renderer: assignment.a_renderer,
            b_renderer,
            transformation: assignment.clip.transform,
            transformation_type: assignment.clip.transform.grouping_key(),
            tags: assignment.clip.tags,
            source_hash,
            rendered_hashes,
            participant_hashes,
            common_safety_gain: common_gain,
            presentation_order: if assignment.first_output == "A" {
                vec!["A".to_owned(), "B".to_owned()]
            } else {
                vec!["B".to_owned(), "A".to_owned()]
            },
            seed,
        },
        metrics,
    })
}

fn validate_render_shape(
    clip_id: &str,
    input: &AudioClip,
    output: &AudioClip,
    renderer: &str,
) -> Result<(), String> {
    if input.sample_rate != output.sample_rate
        || input.channels != output.channels
        || input.frames() != output.frames()
    {
        return Err(format!(
            "Clip '{clip_id}' {renderer} output changed sample rate, channels, or frame count."
        ));
    }
    if output.samples.iter().any(|sample| !sample.is_finite()) {
        return Err(format!(
            "Clip '{clip_id}' {renderer} output contains non-finite samples."
        ));
    }
    Ok(())
}

pub(super) fn common_safety_gain(
    clip_id: &str,
    existing: &AudioClip,
    world: &AudioClip,
) -> Result<f32, String> {
    if existing
        .samples
        .iter()
        .chain(&world.samples)
        .any(|sample| !sample.is_finite())
    {
        return Err(format!(
            "Clip '{clip_id}' rendered output contains non-finite samples."
        ));
    }
    let peak = existing
        .samples
        .iter()
        .chain(&world.samples)
        .fold(0.0_f32, |peak, sample| peak.max(sample.abs()));
    if peak >= 1.0 {
        return Err(format!(
            "Clip '{clip_id}' rendered output reaches digital full scale; clipping cannot be hidden by package scaling."
        ));
    }
    Ok(if peak > CONSERVATIVE_PEAK {
        CONSERVATIVE_PEAK / peak
    } else {
        1.0
    })
}

fn scaled_clip(clip: &AudioClip, gain: f32) -> Result<AudioClip, String> {
    AudioClip::new(
        clip.source_name.clone(),
        clip.sample_rate,
        clip.channels,
        clip.samples.iter().map(|sample| sample * gain).collect(),
    )
}

fn objective_metrics(
    input_analysis: &AudioAnalysis,
    input_frames: usize,
    output: &AudioClip,
    pitch_semitones: f32,
    real_time_factor: f64,
) -> Result<ObjectiveMetrics, String> {
    let output_analysis = AudioAnalysis::new(&output.samples, output.sample_rate, output.channels)?;
    let expected_pitch_ratio = 2.0_f64.powf(f64::from(pitch_semitones) / 12.0);
    let (numerical, pitch, voicing, spectral, consonant, formants) = compare_audio(
        input_analysis,
        &output_analysis,
        &[],
        Some(expected_pitch_ratio),
        &[],
        0.0,
    );
    Ok(ObjectiveMetrics {
        pitch_error_cents: pitch.pitch_error_cents,
        median_source_f0_hz: pitch.median_input_f0_hz,
        median_output_f0_hz: pitch.median_output_f0_hz,
        measured_pitch_ratio: pitch.measured_pitch_ratio,
        mean_absolute_pitch_error_cents: pitch.mean_absolute_pitch_error_cents,
        median_absolute_pitch_error_cents: pitch.median_absolute_pitch_error_cents,
        p10_pitch_error_cents: pitch.p10_pitch_error_cents,
        p90_pitch_error_cents: pitch.p90_pitch_error_cents,
        total_source_frames: pitch.total_source_frames,
        reliable_source_voiced_frames: pitch.reliable_source_voiced_frames,
        paired_reliable_frames: pitch.paired_reliable_frames,
        paired_coverage: pitch.f0_estimation_coverage,
        unpaired_source_voiced_frames: pitch.unpaired_source_voiced_frames,
        low_confidence_exclusions: pitch.low_confidence_exclusions,
        octave_doubling_count: pitch.octave_doubling_count,
        octave_halving_count: pitch.octave_halving_count,
        large_non_octave_error_count: pitch.large_non_octave_error_count,
        octave_ambiguity_count: pitch.octave_ambiguity_count,
        source_track_fingerprint: pitch.source_track_fingerprint,
        legacy_pitch_error_cents: pitch.legacy_pitch_error_cents,
        legacy_source_median_f0_hz: pitch.legacy_source_median_f0_hz,
        legacy_output_median_f0_hz: pitch.legacy_output_median_f0_hz,
        legacy_paired_frames: pitch.legacy_paired_frames,
        formant_error_cents: crate::voice_evaluation::analysis::median_formant_error(&formants),
        voiced_unvoiced_disagreement: voicing.voiced_unvoiced_disagreement_ratio,
        unvoiced_high_frequency_lsd_db: spectral.high_frequency_unvoiced_log_spectral_distance_db,
        waveform_correlation: consonant.unvoiced_waveform_correlation,
        clipping_ratio: numerical.output_clipping_ratio,
        non_finite_count: numerical.output_non_finite_samples,
        duration_delta_frames: output.frames() as i64 - input_frames as i64,
        real_time_factor,
    })
}

fn write_participant_files(
    participant: &Path,
    _study: &ListeningStudy,
    trials: &[RenderedTrial],
) -> Result<(), String> {
    let instructions = "\
# Blinded local listening study\n\n\
This is a local, blinded listening study. It does not test speaker identity or ask you to infer personal attributes.\n\n\
For every trial:\n\n\
1. Listen to the original reference.\n\
2. Listen to A and B in the order listed in `trials.csv`.\n\
3. Replay as needed using the same headphones, safe volume, and environment.\n\
4. Rate A and B independently from 1 (lowest) to 7 (highest).\n\
5. Choose A, B, or tie, then provide confidence from 1 to 5.\n\n\
Rate naturalness, intelligibility, consonant clarity, pitch plausibility, vocal-character plausibility, absence of metallic/buzzy/phasey/robotic artifacts, and overall quality. Artifact flags use semicolon-separated values from the template. Notes are optional.\n\n\
Ratings are subjective. Do not inspect the administrator directory or key before completing ratings. Keep the same playback setup for all trials and use a safe listening volume.\n";
    fs::write(participant.join("instructions.md"), instructions)
        .map_err(|error| format!("Cannot write participant instructions: {error}"))?;

    let mut trial_csv = "trial_id,reference,a,b,first_output\n".to_owned();
    for trial in trials {
        append_csv_row(
            &mut trial_csv,
            &[
                &trial.key.trial_id,
                &format!("audio/{}-reference.wav", trial.key.trial_id),
                &format!("audio/{}-a.wav", trial.key.trial_id),
                &format!("audio/{}-b.wav", trial.key.trial_id),
                &trial.key.presentation_order[0],
            ],
        );
    }
    fs::write(participant.join("trials.csv"), trial_csv)
        .map_err(|error| format!("Cannot write participant trials.csv: {error}"))?;

    let mut ratings = format!("{}\n", super::ratings::RATINGS_HEADER.join(","));
    for trial in trials {
        let mut fields = vec![trial.key.trial_id.clone()];
        fields.resize(super::ratings::RATINGS_HEADER.len(), String::new());
        append_csv_row_owned(&mut ratings, &fields);
    }
    fs::write(participant.join("ratings.csv"), ratings)
        .map_err(|error| format!("Cannot write participant ratings.csv: {error}"))
}

fn write_administrator_files(
    administrator: &Path,
    manifest: &ListeningManifest,
    seed: u64,
    trials: &[RenderedTrial],
) -> Result<(), String> {
    let key = AdministratorKey {
        schema_version: PACKAGE_SCHEMA_VERSION,
        study: manifest.study.clone(),
        seed,
        pitch_analysis: PitchAnalysisMetadata::current(),
        trials: trials.iter().map(|trial| trial.key.clone()).collect(),
    };
    write_json(&administrator.join("key.json"), &key)?;

    let mut resolved = manifest.clone();
    resolved.corpus_root = ".".to_owned();
    write_json(&administrator.join("manifest-resolved.json"), &resolved)?;

    let mut metrics = "trial_id,source_clip_id,renderer,pitch_estimator_version,pitch_metric_version,pitch_error_cents,median_source_f0_hz,median_output_f0_hz,measured_pitch_ratio,mean_absolute_pitch_error_cents,median_absolute_pitch_error_cents,p10_pitch_error_cents,p90_pitch_error_cents,total_source_frames,reliable_source_voiced_frames,paired_reliable_frames,paired_coverage,unpaired_source_voiced_frames,low_confidence_exclusions,octave_doubling_count,octave_halving_count,large_non_octave_error_count,octave_ambiguity_count,source_track_fingerprint,legacy_pitch_error_cents,legacy_source_median_f0_hz,legacy_output_median_f0_hz,legacy_paired_frames,formant_error_cents,voiced_unvoiced_disagreement,unvoiced_hf_lsd_db,waveform_correlation,clipping_ratio,non_finite_count,duration_delta_frames,render_rtf\n".to_owned();
    for trial in trials {
        for (renderer, values) in &trial.metrics {
            append_csv_row(
                &mut metrics,
                &[
                    &trial.key.trial_id,
                    &trial.key.source_clip_id,
                    renderer.as_str(),
                    &PitchAnalysisMetadata::current()
                        .pitch_estimator_version
                        .to_string(),
                    &PitchAnalysisMetadata::current()
                        .pitch_metric_version
                        .to_string(),
                    &optional_number(values.pitch_error_cents),
                    &optional_number(values.median_source_f0_hz),
                    &optional_number(values.median_output_f0_hz),
                    &optional_number(values.measured_pitch_ratio),
                    &optional_number(values.mean_absolute_pitch_error_cents),
                    &optional_number(values.median_absolute_pitch_error_cents),
                    &optional_number(values.p10_pitch_error_cents),
                    &optional_number(values.p90_pitch_error_cents),
                    &values.total_source_frames.to_string(),
                    &values.reliable_source_voiced_frames.to_string(),
                    &values.paired_reliable_frames.to_string(),
                    &format_number(values.paired_coverage),
                    &values.unpaired_source_voiced_frames.to_string(),
                    &values.low_confidence_exclusions.to_string(),
                    &values.octave_doubling_count.to_string(),
                    &values.octave_halving_count.to_string(),
                    &values.large_non_octave_error_count.to_string(),
                    &values.octave_ambiguity_count.to_string(),
                    &values.source_track_fingerprint,
                    &optional_number(values.legacy_pitch_error_cents),
                    &optional_number(values.legacy_source_median_f0_hz),
                    &optional_number(values.legacy_output_median_f0_hz),
                    &values.legacy_paired_frames.to_string(),
                    &optional_number(values.formant_error_cents),
                    &format_number(values.voiced_unvoiced_disagreement),
                    &optional_number(values.unvoiced_high_frequency_lsd_db),
                    &optional_number(values.waveform_correlation),
                    &format_number(values.clipping_ratio),
                    &values.non_finite_count.to_string(),
                    &values.duration_delta_frames.to_string(),
                    &format_number(values.real_time_factor),
                ],
            );
        }
    }
    fs::write(administrator.join("render-metrics.csv"), metrics)
        .map_err(|error| format!("Cannot write administrator render metrics: {error}"))?;

    let hashes = trials
        .iter()
        .map(|trial| {
            serde_json::json!({
                "trialId": trial.key.trial_id,
                "sourceHash": trial.key.source_hash,
                "renderedHashes": trial.key.rendered_hashes,
                "participantHashes": trial.key.participant_hashes,
            })
        })
        .collect::<Vec<_>>();
    write_json(
        &administrator.join("hashes.json"),
        &serde_json::json!({
            "schemaVersion": PACKAGE_SCHEMA_VERSION,
            "algorithm": "SHA-256",
            "trials": hashes,
        }),
    )
}

pub(crate) fn read_key(package: &Path) -> Result<AdministratorKey, String> {
    let path = package.join("administrator/key.json");
    let contents = fs::read_to_string(&path)
        .map_err(|error| format!("Cannot read administrator key: {error}"))?;
    let key: AdministratorKey = serde_json::from_str(&contents)
        .map_err(|error| format!("Administrator key is invalid: {error}"))?;
    if key.schema_version != PACKAGE_SCHEMA_VERSION {
        return Err(format!(
            "Unsupported listening package schema version {}.",
            key.schema_version
        ));
    }
    Ok(key)
}

fn hash_file(path: &Path) -> Result<String, String> {
    sha256_file(path).map_err(|error| format!("Cannot hash '{}': {error}", path.display()))
}

pub(crate) fn write_json(path: &Path, value: &impl Serialize) -> Result<(), String> {
    let mut json = serde_json::to_string_pretty(value)
        .map_err(|error| format!("Cannot serialize '{}': {error}", path.display()))?;
    json.push('\n');
    fs::write(path, json).map_err(|error| format!("Cannot write '{}': {error}", path.display()))
}

pub(crate) fn append_csv_row(output: &mut String, fields: &[&str]) {
    for (index, field) in fields.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str(&csv_field(field));
    }
    output.push('\n');
}

pub(crate) fn append_csv_row_owned(output: &mut String, fields: &[String]) {
    let borrowed = fields.iter().map(String::as_str).collect::<Vec<_>>();
    append_csv_row(output, &borrowed);
}

pub(crate) fn csv_field(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_owned()
    }
}

pub(crate) fn optional_number(value: Option<f64>) -> String {
    value.map(format_number).unwrap_or_default()
}

pub(crate) fn format_number(value: f64) -> String {
    format!("{value:.8}")
}

struct DeterministicRandom {
    state: u64,
}

impl DeterministicRandom {
    fn new(seed: u64) -> Self {
        Self {
            state: seed ^ 0x9e37_79b9_7f4a_7c15,
        }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        value ^ (value >> 31)
    }

    fn shuffle<T>(&mut self, values: &mut [T]) {
        for index in (1..values.len()).rev() {
            let target = (self.next_u64() % (index as u64 + 1)) as usize;
            values.swap(index, target);
        }
    }
}
