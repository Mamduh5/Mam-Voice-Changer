use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use serde_json::json;

use super::{
    manifest::ListeningManifest,
    package::{
        append_csv_row_owned, common_safety_gain, prepare_package, read_key, PrepareOptions,
        Renderer,
    },
    ratings::{validate_ratings_file, RATINGS_HEADER},
    summary::summarize_ratings,
};
use crate::voice_lab::{clip::AudioClip, wav};

static SEQUENCE: AtomicU64 = AtomicU64::new(0);

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new(label: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "mam-voice-listening-{label}-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn fixture(path: &Path, frequency: f32, peak: f32) {
    let sample_rate = 48_000;
    let samples = (0..12_000)
        .map(|frame| {
            (std::f32::consts::TAU * frequency * frame as f32 / sample_rate as f32).sin() * peak
        })
        .collect();
    let clip = AudioClip::new("synthetic-test", sample_rate, 1, samples).unwrap();
    wav::export(path, &clip).unwrap();
}

fn manifest_value(clips: usize) -> serde_json::Value {
    json!({
        "schemaVersion": 1,
        "corpusRoot": ".",
        "study": {
            "id": "local-synthetic-study",
            "title": "Local synthetic study",
            "seed": 20260727
        },
        "clips": (0..clips).map(|index| json!({
            "id": format!("synthetic-{index:02}"),
            "input": format!("clip-{index:02}.wav"),
            "description": format!("Synthetic fixture {index}"),
            "tags": if index % 2 == 0 { vec!["even"] } else { vec!["odd"] },
            "transform": {
                "pitchSemitones": 0.0,
                "formantShiftSemitones": 0.0,
                "consonantPreservation": 1.0,
                "dryWet": 1.0
            }
        })).collect::<Vec<_>>()
    })
}

fn study(clips: usize) -> (TestDirectory, PathBuf) {
    let root = TestDirectory::new("study");
    for index in 0..clips {
        fixture(
            &root.0.join(format!("clip-{index:02}.wav")),
            160.0 + index as f32 * 30.0,
            0.12,
        );
    }
    let manifest = root.0.join("listening-manifest.json");
    fs::write(
        &manifest,
        serde_json::to_string_pretty(&manifest_value(clips)).unwrap(),
    )
    .unwrap();
    (root, manifest)
}

fn prepare(manifest: &Path, output: &Path, seed: u64) {
    prepare_package(&PrepareOptions {
        manifest_path: manifest.to_owned(),
        output_directory: output.to_owned(),
        seed_override: Some(seed),
    })
    .unwrap();
}

#[test]
fn manifest_is_strict_versioned_bounded_and_world_compatible() {
    assert!(
        ListeningManifest::from_json(&serde_json::to_string(&manifest_value(1)).unwrap()).is_ok()
    );

    let mut unsupported = manifest_value(1);
    unsupported["schemaVersion"] = json!(2);
    assert!(ListeningManifest::from_json(&unsupported.to_string())
        .unwrap_err()
        .contains("Unsupported listening manifest schema"));

    let mut unknown = manifest_value(1);
    unknown["unknown"] = json!(true);
    assert!(ListeningManifest::from_json(&unknown.to_string())
        .unwrap_err()
        .contains("unknown field"));

    let mut duplicate = manifest_value(2);
    duplicate["clips"][1]["id"] = duplicate["clips"][0]["id"].clone();
    assert!(ListeningManifest::from_json(&duplicate.to_string())
        .unwrap_err()
        .contains("duplicated"));

    let mut empty = manifest_value(0);
    assert!(ListeningManifest::from_json(&empty.to_string())
        .unwrap_err()
        .contains("at least one"));

    empty = manifest_value(1);
    empty["clips"][0]["input"] = json!("../private.wav");
    assert!(ListeningManifest::from_json(&empty.to_string())
        .unwrap_err()
        .contains("normalized relative path"));
    for unsafe_path in ["C:/private.wav", "speech\\private.wav"] {
        let mut unsafe_manifest = manifest_value(1);
        unsafe_manifest["clips"][0]["input"] = json!(unsafe_path);
        assert!(ListeningManifest::from_json(&unsafe_manifest.to_string())
            .unwrap_err()
            .contains("normalized relative path"));
    }

    let mut empty_study = manifest_value(1);
    empty_study["study"]["title"] = json!("");
    assert!(ListeningManifest::from_json(&empty_study.to_string())
        .unwrap_err()
        .contains("study title"));

    let mut invalid = manifest_value(1);
    invalid["clips"][0]["transform"]["pitchSemitones"] = json!(99);
    assert!(ListeningManifest::from_json(&invalid.to_string())
        .unwrap_err()
        .contains("Pitch must"));

    let mut unsupported_world = manifest_value(1);
    unsupported_world["clips"][0]["transform"]["brightnessDb"] = json!(1.0);
    assert!(ListeningManifest::from_json(&unsupported_world.to_string())
        .unwrap_err()
        .contains("WORLD reference renderer does not support"));
}

#[test]
fn prepare_rejects_missing_and_malformed_wav() {
    let root = TestDirectory::new("invalid-wav");
    let manifest = root.0.join("manifest.json");
    fs::write(&manifest, manifest_value(1).to_string()).unwrap();
    let missing = root.0.join("missing-output");
    assert!(prepare_package(&PrepareOptions {
        manifest_path: manifest.clone(),
        output_directory: missing,
        seed_override: None,
    })
    .unwrap_err()
    .contains("Cannot read input WAV"));

    fs::write(root.0.join("clip-00.wav"), b"not a wav").unwrap();
    let malformed = root.0.join("malformed-output");
    assert!(prepare_package(&PrepareOptions {
        manifest_path: manifest,
        output_directory: malformed,
        seed_override: None,
    })
    .unwrap_err()
    .contains("Cannot open WAV"));
}

#[test]
fn deterministic_balanced_package_is_blinded_shape_safe_and_private() {
    let (root, manifest) = study(4);
    let first = root.0.join("package-one");
    let second = root.0.join("package-two");
    let different = root.0.join("package-different");
    prepare(&manifest, &first, 77);
    prepare(&manifest, &second, 77);
    prepare(&manifest, &different, 78);

    let first_key = read_key(&first).unwrap();
    let second_key = read_key(&second).unwrap();
    let different_key = read_key(&different).unwrap();
    assert_eq!(first_key, second_key);
    assert_ne!(
        first_key
            .trials
            .iter()
            .map(|trial| (&trial.source_clip_id, trial.a_renderer))
            .collect::<Vec<_>>(),
        different_key
            .trials
            .iter()
            .map(|trial| (&trial.source_clip_id, trial.a_renderer))
            .collect::<Vec<_>>()
    );
    let existing_a = first_key
        .trials
        .iter()
        .filter(|trial| trial.a_renderer == Renderer::ExistingDsp)
        .count();
    assert!(existing_a.abs_diff(first_key.trials.len() - existing_a) <= 1);
    assert_eq!(
        first_key
            .trials
            .iter()
            .map(|trial| &trial.source_clip_id)
            .collect::<std::collections::HashSet<_>>()
            .len(),
        4
    );

    for trial in &first_key.trials {
        let audio = first.join("participant/audio");
        let reference =
            wav::import(&audio.join(format!("{}-reference.wav", trial.trial_id))).unwrap();
        let a = wav::import(&audio.join(format!("{}-a.wav", trial.trial_id))).unwrap();
        let b = wav::import(&audio.join(format!("{}-b.wav", trial.trial_id))).unwrap();
        assert_eq!(
            (a.sample_rate, a.channels, a.frames()),
            (
                reference.sample_rate,
                reference.channels,
                reference.frames()
            )
        );
        assert_eq!(
            (b.sample_rate, b.channels, b.frames()),
            (
                reference.sample_rate,
                reference.channels,
                reference.frames()
            )
        );
        assert!(a
            .samples
            .iter()
            .chain(&b.samples)
            .all(|value| value.is_finite()));
        assert_eq!(trial.common_safety_gain, 1.0);
        assert_eq!(trial.rendered_hashes.len(), 2);
        assert_eq!(trial.participant_hashes.len(), 3);
    }

    let participant = collect_text(&first.join("participant"));
    let normalized = participant
        .to_ascii_lowercase()
        .replace(['-', '_', ' '], "");
    assert!(!normalized.contains("existingdsp"));
    assert!(!normalized.contains("worldreference"));
    assert!(!normalized.contains("signalsmith"));
    assert!(!participant.contains("D:\\"));
    assert!(!participant.contains("C:\\Users\\"));
    assert!(!first.join("participant/key.json").exists());
    assert!(first.join("administrator/key.json").is_file());
    let package_summary = fs::read_to_string(first.join("package-summary.json")).unwrap();
    assert!(!package_summary.contains("\"seed\""));
    assert!(!package_summary.to_ascii_lowercase().contains("world"));
    assert!(!package_summary.to_ascii_lowercase().contains("signalsmith"));
    let resolved = fs::read_to_string(first.join("administrator/manifest-resolved.json")).unwrap();
    assert!(!resolved.contains("D:\\"));
    assert!(!resolved.contains("C:\\Users\\"));
}

#[test]
fn common_safety_gain_is_shared_and_cannot_hide_invalid_output() {
    let valid = AudioClip::new("valid", 48_000, 1, vec![0.5; 100]).unwrap();
    let quiet = AudioClip::new("quiet", 48_000, 1, vec![0.2; 100]).unwrap();
    assert_eq!(common_safety_gain("valid", &valid, &quiet).unwrap(), 1.0);

    let hot = AudioClip::new("hot", 48_000, 1, vec![0.96; 100]).unwrap();
    let gain = common_safety_gain("hot", &hot, &quiet).unwrap();
    assert!((gain - 0.95 / 0.96).abs() < f32::EPSILON);

    let full_scale = AudioClip::new("full", 48_000, 1, vec![1.0; 100]).unwrap();
    assert!(common_safety_gain("full", &full_scale, &quiet)
        .unwrap_err()
        .contains("clipping cannot be hidden"));

    let invalid = AudioClip {
        id: "invalid".to_owned(),
        source_name: "invalid".to_owned(),
        sample_rate: 48_000,
        channels: 1,
        samples: vec![f32::NAN],
    };
    assert!(common_safety_gain("invalid", &invalid, &quiet).is_err());
}

#[test]
fn ratings_validation_and_summary_unblind_without_inverting_results() {
    let (root, manifest) = study(4);
    let package = root.0.join("package");
    prepare(&manifest, &package, 1234);
    let key = read_key(&package).unwrap();
    let ratings = root.0.join("completed.csv");
    let completed = completed_ratings(&key.trials);
    fs::write(&ratings, edit_rating(&completed, 15, "tie")).unwrap();
    let key_path = package.join("administrator/key.json");
    let hidden_key = package.join("administrator/key.hidden");
    fs::rename(&key_path, &hidden_key).unwrap();
    let validation = validate_ratings_file(&package, &ratings).unwrap();
    fs::rename(&hidden_key, &key_path).unwrap();
    assert_eq!(validation.completed_trials, 4);

    let output = root.0.join("results");
    let summary = summarize_ratings(&package, &ratings, &output).unwrap();
    let world = &summary.renderers["worldReference"];
    let existing = &summary.renderers["existingDsp"];
    assert_eq!(world.dimensions["naturalness"].mean, Some(7.0));
    assert_eq!(world.dimensions["naturalness"].median, Some(7.0));
    assert_eq!(existing.dimensions["naturalness"].mean, Some(3.0));
    assert_eq!(world.preference_wins, 3);
    assert_eq!(existing.preference_losses, 3);
    assert_eq!(world.ties, 1);
    assert_eq!(existing.ties, 1);
    assert_eq!(world.artifact_flag_counts["metallic"], 4);
    assert_eq!(
        summary.paired_differences_world_minus_existing_dsp["naturalness"],
        Some(4.0)
    );
    assert_eq!(summary.by_tag["even"].rated_trials, 2);
    assert_eq!(summary.by_tag["odd"].rated_trials, 2);
    assert_eq!(summary.by_transformation_type.len(), 1);
    assert!(output.join("summary.json").is_file());
    assert!(output.join("summary.csv").is_file());
    assert!(output.join("summary.md").is_file());
    assert!(output.join("trial-results.csv").is_file());
}

#[test]
fn ratings_errors_are_field_specific_and_incomplete_summary_is_explicit() {
    let (root, manifest) = study(2);
    let package = root.0.join("package");
    prepare(&manifest, &package, 44);
    let key = read_key(&package).unwrap();
    let valid = completed_ratings(&key.trials);

    let invalid_score = edit_rating(&valid, 1, "8");
    let invalid_preference = edit_rating(&valid, 15, "renderer");
    let unknown_artifact = edit_rating(&valid, 17, "unknown artifact");
    let unknown_trial = edit_rating(&valid, 0, "trial-999");
    for (label, edited, expected) in [
        (
            "duplicate",
            format!("{valid}{}", valid.lines().nth(1).unwrap()) + "\n",
            "duplicates trial",
        ),
        (
            "invalid-score",
            invalid_score,
            "must be an integer from 1 to 7",
        ),
        (
            "invalid-preference",
            invalid_preference,
            "must be A, B, or tie",
        ),
        (
            "unknown-artifact",
            unknown_artifact,
            "unknown artifact flag",
        ),
        ("unknown-trial", unknown_trial, "unknown trial"),
    ] {
        let path = root.0.join(format!("{label}.csv"));
        fs::write(&path, edited).unwrap();
        assert!(
            validate_ratings_file(&package, &path)
                .unwrap_err()
                .contains(expected),
            "{label}"
        );
    }

    let excessive = edit_rating(&valid, 19, &"n".repeat(513));
    let excessive_path = root.0.join("excessive.csv");
    fs::write(&excessive_path, excessive).unwrap();
    assert!(validate_ratings_file(&package, &excessive_path)
        .unwrap_err()
        .contains("exceeds 512"));

    let missing = format!(
        "{}\n{}\n",
        valid.lines().next().unwrap(),
        valid.lines().nth(1).unwrap()
    );
    let missing_path = root.0.join("missing.csv");
    fs::write(&missing_path, missing).unwrap();
    assert!(validate_ratings_file(&package, &missing_path)
        .unwrap_err()
        .contains("missing expected trial"));
    let incomplete_output = root.0.join("incomplete-results");
    let incomplete = summarize_ratings(&package, &missing_path, &incomplete_output).unwrap();
    assert_eq!(incomplete.rated_trials, 1);
    assert_eq!(incomplete.missing_trials.len(), 1);
    assert!(incomplete
        .warnings
        .iter()
        .any(|warning| warning.contains("incomplete")));
}

fn completed_ratings(trials: &[super::package::TrialKey]) -> String {
    let mut output = format!("{}\n", RATINGS_HEADER.join(","));
    for trial in trials {
        let world_is_a = trial.a_renderer == Renderer::WorldReference;
        let mut fields = vec![trial.trial_id.clone()];
        let (a, b) = if world_is_a { ("7", "3") } else { ("3", "7") };
        fields.extend(std::iter::repeat_n(a.to_owned(), 7));
        fields.extend(std::iter::repeat_n(b.to_owned(), 7));
        fields.push(if world_is_a { "A" } else { "B" }.to_owned());
        fields.push("5".to_owned());
        fields.push(if world_is_a { "metallic" } else { "" }.to_owned());
        fields.push(if world_is_a { "" } else { "metallic" }.to_owned());
        fields.push("synthetic note, with comma".to_owned());
        append_csv_row_owned(&mut output, &fields);
    }
    output
}

fn edit_rating(csv: &str, field: usize, value: &str) -> String {
    let mut rows = super::ratings::parse_csv(csv).unwrap();
    rows[1][field] = value.to_owned();
    let mut output = String::new();
    for row in rows {
        append_csv_row_owned(&mut output, &row);
    }
    output
}

fn collect_text(root: &Path) -> String {
    let mut output = String::new();
    for entry in fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        output.push_str(&path.file_name().unwrap().to_string_lossy());
        if path.is_dir() {
            output.push_str(&collect_text(&path));
        } else if path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|value| value == "md" || value == "csv")
        {
            output.push_str(&fs::read_to_string(path).unwrap());
        }
    }
    output
}
