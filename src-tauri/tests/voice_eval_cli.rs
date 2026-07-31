use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};

static SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn directory(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "mam-voice-eval-cli-{label}-{}-{}",
        std::process::id(),
        SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ))
}

fn run(arguments: &[&Path]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_voice-eval"));
    for argument in arguments {
        command.arg(argument);
    }
    command.output().unwrap()
}

fn flag(value: &'static str) -> &'static Path {
    Path::new(value)
}

#[test]
fn cli_generation_reports_exit_codes_and_baseline_are_end_to_end() {
    let root = directory("end-to-end");
    let generated = run(&[flag("--generate-example"), &root]);
    assert!(
        generated.status.success(),
        "{}",
        String::from_utf8_lossy(&generated.stderr)
    );
    let manifest = root.join("evaluation-manifest.json");
    let mut value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&manifest).unwrap()).unwrap();
    value["cases"]
        .as_array_mut()
        .unwrap()
        .retain(|case| matches!(case["id"].as_str(), Some("neutral" | "neutral-world")));
    fs::write(&manifest, serde_json::to_string_pretty(&value).unwrap()).unwrap();

    let baseline_output = root.join("baseline");
    let passing = run(&[
        flag("--manifest"),
        &manifest,
        flag("--output"),
        &baseline_output,
        flag("--no-rendered-audio"),
        flag("--fail-on-expectation"),
    ]);
    assert!(
        passing.status.success(),
        "{}",
        String::from_utf8_lossy(&passing.stderr)
    );
    assert!(String::from_utf8_lossy(&passing.stdout).contains("neutral: PASS"));
    assert!(String::from_utf8_lossy(&passing.stdout).contains("neutral-world: PASS"));
    for report in ["report.json", "cases.csv", "report.md"] {
        assert!(baseline_output.join(report).is_file());
    }
    assert!(!baseline_output.join("rendered").exists());
    let report_contents = fs::read_to_string(baseline_output.join("report.json")).unwrap();
    assert!(!report_contents.contains(&root.to_string_lossy().to_string()));
    assert!(report_contents.contains("\"renderer\": \"worldReference\""));
    assert!(report_contents.contains("\"crossRendererComparisons\""));

    let comparison_output = root.join("comparison");
    let comparison = run(&[
        flag("--manifest"),
        &manifest,
        flag("--output"),
        &comparison_output,
        flag("--baseline"),
        &baseline_output.join("report.json"),
        flag("--no-rendered-audio"),
    ]);
    assert!(comparison.status.success());
    let comparison_json: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(comparison_output.join("report.json")).unwrap())
            .unwrap();
    assert!(comparison_json["baselineComparison"].is_object());

    value["cases"][0]["expectations"]["maximumRealTimeFactor"] = serde_json::json!(0.0);
    fs::write(&manifest, serde_json::to_string_pretty(&value).unwrap()).unwrap();
    let allowed_failure = run(&[
        flag("--manifest"),
        &manifest,
        flag("--output"),
        &root.join("allowed-failure"),
        flag("--no-rendered-audio"),
    ]);
    assert!(allowed_failure.status.success());
    let enforced_failure = run(&[
        flag("--manifest"),
        &manifest,
        flag("--output"),
        &root.join("enforced-failure"),
        flag("--no-rendered-audio"),
        flag("--fail-on-expectation"),
    ]);
    assert_eq!(enforced_failure.status.code(), Some(1));

    let malformed = root.join("malformed.json");
    fs::write(&malformed, "{").unwrap();
    let malformed_result = run(&[
        flag("--manifest"),
        &malformed,
        flag("--output"),
        &root.join("malformed-report"),
    ]);
    assert_eq!(malformed_result.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&malformed_result.stderr).contains("not valid JSON"));
    let _ = fs::remove_dir_all(root);
}
