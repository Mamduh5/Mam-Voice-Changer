use std::{env, path::PathBuf};

use super::{
    fixtures::generate_example,
    runner::{evaluate_manifest_file, EvaluationOptions},
};

#[derive(Clone, Debug, Default, PartialEq)]
struct CliArguments {
    manifest: Option<PathBuf>,
    output: Option<PathBuf>,
    fail_on_expectation: bool,
    no_rendered_audio: bool,
    baseline: Option<PathBuf>,
    generate_example: Option<PathBuf>,
}

pub fn cli_main() -> u8 {
    match run(env::args().skip(1)) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("voice-eval: {error}");
            eprintln!(
                "Usage: voice-eval --manifest <path> --output <directory> [--fail-on-expectation] [--no-rendered-audio] [--baseline <report.json>]\n       voice-eval --generate-example <directory>"
            );
            2
        }
    }
}

fn run(arguments: impl IntoIterator<Item = String>) -> Result<u8, String> {
    let arguments = parse(arguments)?;
    if let Some(directory) = arguments.generate_example {
        let manifest = generate_example(&directory)?;
        println!(
            "Generated deterministic example corpus and {}",
            manifest
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("evaluation-manifest.json")
        );
        return Ok(0);
    }
    let manifest = arguments
        .manifest
        .ok_or_else(|| "--manifest is required.".to_owned())?;
    let output_directory = arguments
        .output
        .ok_or_else(|| "--output is required.".to_owned())?;
    let report = evaluate_manifest_file(
        &manifest,
        &EvaluationOptions {
            output_directory,
            no_rendered_audio: arguments.no_rendered_audio,
            baseline_report: arguments.baseline,
        },
    )?;
    for case in &report.cases {
        let failed = case
            .expectations
            .iter()
            .filter(|expectation| !expectation.passed)
            .count();
        println!(
            "{}: {} pitch={} cents, V/UV={:.4}, HF={}, RTF={:.5}",
            case.id,
            if failed == 0 { "PASS" } else { "FAIL" },
            display(case.pitch.pitch_error_cents),
            case.voicing.voiced_unvoiced_disagreement_ratio,
            display(case.consonant.unvoiced_high_frequency_energy_ratio),
            case.performance.real_time_factor
        );
    }
    println!(
        "{} cases, {} passed expectations, {} failed expectations",
        report.summary.total_cases,
        report.summary.passed_expectations,
        report.summary.failed_expectations
    );
    Ok(u8::from(
        arguments.fail_on_expectation && report.failed_expectations() > 0,
    ))
}

fn parse(arguments: impl IntoIterator<Item = String>) -> Result<CliArguments, String> {
    let mut parsed = CliArguments::default();
    let mut arguments = arguments.into_iter();
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--manifest" => {
                set_path(&mut parsed.manifest, arguments.next(), "--manifest")?;
            }
            "--output" => {
                set_path(&mut parsed.output, arguments.next(), "--output")?;
            }
            "--baseline" => {
                set_path(&mut parsed.baseline, arguments.next(), "--baseline")?;
            }
            "--generate-example" => {
                set_path(
                    &mut parsed.generate_example,
                    arguments.next(),
                    "--generate-example",
                )?;
            }
            "--fail-on-expectation" => {
                set_switch(&mut parsed.fail_on_expectation, "--fail-on-expectation")?
            }
            "--no-rendered-audio" => {
                set_switch(&mut parsed.no_rendered_audio, "--no-rendered-audio")?
            }
            _ => return Err(format!("Unknown argument '{argument}'.")),
        }
    }
    if parsed.generate_example.is_some()
        && (parsed.manifest.is_some()
            || parsed.output.is_some()
            || parsed.baseline.is_some()
            || parsed.fail_on_expectation
            || parsed.no_rendered_audio)
    {
        return Err("--generate-example cannot be combined with evaluation arguments.".to_owned());
    }
    Ok(parsed)
}

fn set_path(target: &mut Option<PathBuf>, value: Option<String>, flag: &str) -> Result<(), String> {
    if target.is_some() {
        return Err(format!("{flag} may be supplied only once."));
    }
    let value = value.ok_or_else(|| format!("{flag} requires a value."))?;
    if value.starts_with("--") || value.is_empty() {
        return Err(format!("{flag} requires a path value."));
    }
    *target = Some(PathBuf::from(value));
    Ok(())
}

fn set_switch(target: &mut bool, flag: &str) -> Result<(), String> {
    if *target {
        Err(format!("{flag} may be supplied only once."))
    } else {
        *target = true;
        Ok(())
    }
}

fn display(value: Option<f64>) -> String {
    value
        .map(|value| format!("{value:.3}"))
        .unwrap_or_else(|| "unavailable".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strict_parser_accepts_documented_shape_and_rejects_ambiguity() {
        let parsed = parse([
            "--manifest".to_owned(),
            "manifest.json".to_owned(),
            "--output".to_owned(),
            "report".to_owned(),
            "--fail-on-expectation".to_owned(),
            "--no-rendered-audio".to_owned(),
            "--baseline".to_owned(),
            "old.json".to_owned(),
        ])
        .unwrap();
        assert_eq!(parsed.manifest, Some(PathBuf::from("manifest.json")));
        assert!(parsed.fail_on_expectation);
        assert!(parse(["--unknown".to_owned()]).is_err());
        assert!(parse(["--manifest".to_owned()]).is_err());
        assert!(parse([
            "--generate-example".to_owned(),
            "example".to_owned(),
            "--output".to_owned(),
            "report".to_owned(),
        ])
        .is_err());
    }
}
