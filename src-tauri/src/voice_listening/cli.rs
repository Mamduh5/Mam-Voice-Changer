use std::{env, path::PathBuf};

use super::{
    package::{prepare_package, PrepareOptions},
    ratings::validate_ratings_file,
    summary::summarize_ratings,
};

const USAGE: &str = "\
voice-listen prepare --manifest <listening-manifest.json> --output <directory> [--seed <u64>]
voice-listen validate-ratings --package <directory> --ratings <ratings.csv>
voice-listen summarize --package <directory> --ratings <ratings.csv> --output <directory>";

pub fn cli_main() -> u8 {
    match run(env::args().skip(1).collect()) {
        Ok(message) => {
            println!("{message}");
            0
        }
        Err(error) => {
            eprintln!("voice-listen: {error}\n\n{USAGE}");
            2
        }
    }
}

fn run(arguments: Vec<String>) -> Result<String, String> {
    let Some(operation) = arguments.first().map(String::as_str) else {
        return Err("Choose an operation.".to_owned());
    };
    match operation {
        "prepare" => {
            let options = parse_options(&arguments[1..], &["--manifest", "--output"], &["--seed"])?;
            let seed_override = options
                .get("--seed")
                .map(|value| {
                    value
                        .parse::<u64>()
                        .map_err(|_| "--seed must be an unsigned 64-bit integer.".to_owned())
                })
                .transpose()?;
            let summary = prepare_package(&PrepareOptions {
                manifest_path: PathBuf::from(required(&options, "--manifest")?),
                output_directory: PathBuf::from(required(&options, "--output")?),
                seed_override,
            })?;
            Ok(format!(
                "Prepared {} blinded trial(s) locally with seed {}. No listening or ratings occurred.",
                summary.trial_count, summary.seed
            ))
        }
        "validate-ratings" => {
            let options = parse_options(&arguments[1..], &["--package", "--ratings"], &[])?;
            let validation = validate_ratings_file(
                &PathBuf::from(required(&options, "--package")?),
                &PathBuf::from(required(&options, "--ratings")?),
            )?;
            Ok(format!(
                "Ratings are valid and complete: {}/{} trial(s). Renderer identities were not read during row validation.",
                validation.completed_trials, validation.expected_trials
            ))
        }
        "summarize" => {
            let options = parse_options(
                &arguments[1..],
                &["--package", "--ratings", "--output"],
                &[],
            )?;
            let summary = summarize_ratings(
                &PathBuf::from(required(&options, "--package")?),
                &PathBuf::from(required(&options, "--ratings")?),
                &PathBuf::from(required(&options, "--output")?),
            )?;
            Ok(format!(
                "Wrote descriptive local results for {}/{} rated trial(s). No universal winner or standardized MOS was declared.",
                summary.rated_trials, summary.expected_trials
            ))
        }
        other => Err(format!("Unknown operation '{other}'.")),
    }
}

fn parse_options(
    arguments: &[String],
    required_names: &[&str],
    optional_names: &[&str],
) -> Result<std::collections::BTreeMap<String, String>, String> {
    if !arguments.len().is_multiple_of(2) {
        return Err("Every option must have exactly one value.".to_owned());
    }
    let allowed = required_names
        .iter()
        .chain(optional_names)
        .copied()
        .collect::<Vec<_>>();
    let mut parsed = std::collections::BTreeMap::new();
    for pair in arguments.chunks_exact(2) {
        let name = pair[0].as_str();
        if !allowed.contains(&name) {
            return Err(format!("Unknown option '{name}'."));
        }
        if pair[1].is_empty() {
            return Err(format!("Option '{name}' cannot be empty."));
        }
        if parsed.insert(name.to_owned(), pair[1].clone()).is_some() {
            return Err(format!("Option '{name}' was provided more than once."));
        }
    }
    for name in required_names {
        if !parsed.contains_key(*name) {
            return Err(format!("Missing required option '{name}'."));
        }
    }
    Ok(parsed)
}

fn required<'a>(
    values: &'a std::collections::BTreeMap<String, String>,
    name: &str,
) -> Result<&'a str, String> {
    values
        .get(name)
        .map(String::as_str)
        .ok_or_else(|| format!("Missing required option '{name}'."))
}

#[cfg(test)]
mod tests {
    use super::run;

    #[test]
    fn strict_cli_rejects_unknown_missing_and_duplicate_options() {
        assert!(run(vec!["unknown".to_owned()]).is_err());
        assert!(run(vec![
            "prepare".to_owned(),
            "--manifest".to_owned(),
            "study.json".to_owned()
        ])
        .is_err());
        assert!(run(vec![
            "prepare".to_owned(),
            "--manifest".to_owned(),
            "a".to_owned(),
            "--output".to_owned(),
            "b".to_owned(),
            "--output".to_owned(),
            "c".to_owned(),
        ])
        .is_err());
        assert!(run(vec![
            "prepare".to_owned(),
            "--manifest".to_owned(),
            "a".to_owned(),
            "--output".to_owned(),
            "b".to_owned(),
            "--network".to_owned(),
            "yes".to_owned(),
        ])
        .is_err());
    }
}
