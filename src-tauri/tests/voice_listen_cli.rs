use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};

use hound::{SampleFormat, WavSpec, WavWriter};

static SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn directory(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "mam-voice-listen-cli-{label}-{}-{}",
        std::process::id(),
        SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ))
}

fn run(arguments: &[&Path]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_voice-listen"));
    for argument in arguments {
        command.arg(argument);
    }
    command.output().unwrap()
}

fn flag(value: &'static str) -> &'static Path {
    Path::new(value)
}

#[test]
fn prepare_validate_and_summarize_are_end_to_end_and_local() {
    let root = directory("end-to-end");
    fs::create_dir_all(&root).unwrap();
    let source = root.join("synthetic.wav");
    let mut writer = WavWriter::create(
        &source,
        WavSpec {
            channels: 1,
            sample_rate: 48_000,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        },
    )
    .unwrap();
    for frame in 0..12_000 {
        let sample = (std::f32::consts::TAU * 180.0 * frame as f32 / 48_000.0).sin() * 0.1;
        writer
            .write_sample((sample * f32::from(i16::MAX)) as i16)
            .unwrap();
    }
    writer.finalize().unwrap();

    let manifest = root.join("manifest.json");
    fs::write(
        &manifest,
        serde_json::to_string_pretty(&serde_json::json!({
            "schemaVersion": 1,
            "corpusRoot": ".",
            "study": {
                "id": "synthetic-cli",
                "title": "Synthetic CLI",
                "seed": 7
            },
            "clips": [{
                "id": "synthetic",
                "input": "synthetic.wav",
                "description": "Generated sine fixture",
                "tags": ["synthetic"],
                "transform": {
                    "pitchSemitones": 0.0,
                    "formantShiftSemitones": 0.0,
                    "consonantPreservation": 1.0,
                    "dryWet": 1.0
                }
            }]
        }))
        .unwrap(),
    )
    .unwrap();

    let package = root.join("package");
    let prepared = run(&[
        flag("prepare"),
        flag("--manifest"),
        &manifest,
        flag("--output"),
        &package,
        flag("--seed"),
        flag("99"),
    ]);
    assert!(
        prepared.status.success(),
        "{}",
        String::from_utf8_lossy(&prepared.stderr)
    );
    assert!(String::from_utf8_lossy(&prepared.stdout).contains("No listening or ratings occurred"));

    let template = fs::read_to_string(package.join("participant/ratings.csv")).unwrap();
    let header = template.lines().next().unwrap();
    let ratings = root.join("ratings.csv");
    fs::write(
        &ratings,
        format!(
            "{header}\ntrial-001,5,5,5,5,5,5,5,4,4,4,4,4,4,4,tie,3,,,synthetic automated rating\n"
        ),
    )
    .unwrap();

    let validation = run(&[
        flag("validate-ratings"),
        flag("--package"),
        &package,
        flag("--ratings"),
        &ratings,
    ]);
    assert!(
        validation.status.success(),
        "{}",
        String::from_utf8_lossy(&validation.stderr)
    );

    let results = root.join("results");
    let summarized = run(&[
        flag("summarize"),
        flag("--package"),
        &package,
        flag("--ratings"),
        &ratings,
        flag("--output"),
        &results,
    ]);
    assert!(
        summarized.status.success(),
        "{}",
        String::from_utf8_lossy(&summarized.stderr)
    );
    for file in [
        "summary.json",
        "summary.csv",
        "summary.md",
        "trial-results.csv",
    ] {
        assert!(results.join(file).is_file());
    }
    let summary = fs::read_to_string(results.join("summary.json")).unwrap();
    assert!(summary.contains("\"ratedTrials\": 1"));
    assert!(summary.contains("not standardized MOS"));

    let invalid = run(&[flag("prepare"), flag("--network"), flag("yes")]);
    assert_eq!(invalid.status.code(), Some(2));
    let _ = fs::remove_dir_all(root);
}
