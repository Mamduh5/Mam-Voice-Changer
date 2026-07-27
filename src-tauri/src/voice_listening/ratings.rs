use std::{
    collections::{BTreeSet, HashSet},
    fs,
    path::Path,
};

use serde::{Deserialize, Serialize};

pub(crate) const DIMENSIONS: [&str; 7] = [
    "naturalness",
    "intelligibility",
    "consonant_clarity",
    "pitch_plausibility",
    "vocal_character_plausibility",
    "artifact_absence",
    "overall_quality",
];
pub(crate) const RATINGS_HEADER: [&str; 20] = [
    "trial_id",
    "a_naturalness",
    "a_intelligibility",
    "a_consonant_clarity",
    "a_pitch_plausibility",
    "a_vocal_character_plausibility",
    "a_artifact_absence",
    "a_overall_quality",
    "b_naturalness",
    "b_intelligibility",
    "b_consonant_clarity",
    "b_pitch_plausibility",
    "b_vocal_character_plausibility",
    "b_artifact_absence",
    "b_overall_quality",
    "preference",
    "confidence",
    "a_artifact_flags",
    "b_artifact_flags",
    "notes",
];
const ARTIFACT_FLAGS: [&str; 10] = [
    "metallic",
    "buzzy",
    "phasey",
    "muffled",
    "harsh",
    "unstable pitch",
    "distorted consonants",
    "excessive breath/noise",
    "timing artifact",
    "other",
];
const MAX_NOTES_CHARS: usize = 512;
const RENDERER_LABELS: [&str; 3] = ["existingdsp", "worldreference", "signalsmith"];

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RatingsValidation {
    pub valid: bool,
    pub expected_trials: usize,
    pub completed_trials: usize,
    pub missing_trials: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct RatingRow {
    pub trial_id: String,
    pub a_scores: [u8; 7],
    pub b_scores: [u8; 7],
    pub preference: Preference,
    pub confidence: u8,
    pub a_artifact_flags: Vec<String>,
    pub b_artifact_flags: Vec<String>,
    pub notes: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Preference {
    A,
    B,
    Tie,
}

pub fn validate_ratings_file(
    package: &Path,
    ratings_path: &Path,
) -> Result<RatingsValidation, String> {
    validate_participant_blinding(package, ratings_path)?;
    let (rows, missing, expected) = load_ratings(package, ratings_path, true)?;
    Ok(RatingsValidation {
        valid: true,
        expected_trials: expected,
        completed_trials: rows.len(),
        missing_trials: missing,
    })
}

pub(crate) fn load_ratings(
    package: &Path,
    ratings_path: &Path,
    require_complete: bool,
) -> Result<(Vec<RatingRow>, Vec<String>, usize), String> {
    let expected_owned = expected_participant_trials(package)?;
    let expected = expected_owned
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let contents = fs::read_to_string(ratings_path)
        .map_err(|error| format!("Cannot read ratings CSV: {error}"))?;
    let parsed = parse_csv(&contents)?;
    if parsed.is_empty() {
        return Err("Ratings CSV is empty.".to_owned());
    }
    let expected_header = RATINGS_HEADER
        .iter()
        .map(|value| (*value).to_owned())
        .collect::<Vec<_>>();
    if parsed[0] != expected_header {
        return Err(format!(
            "Ratings CSV header must exactly match: {}",
            RATINGS_HEADER.join(",")
        ));
    }
    let mut rows = Vec::new();
    let mut seen = HashSet::new();
    for (index, fields) in parsed.iter().enumerate().skip(1) {
        let row_number = index + 1;
        if fields.iter().all(|field| field.is_empty()) {
            continue;
        }
        if fields.len() != RATINGS_HEADER.len() {
            return Err(format!(
                "Ratings row {row_number} has {} fields; expected {}.",
                fields.len(),
                RATINGS_HEADER.len()
            ));
        }
        let trial_id = required(fields, 0, row_number)?;
        if !expected.contains(trial_id) {
            return Err(format!(
                "Ratings row {row_number} field 'trial_id' contains unknown trial '{trial_id}'."
            ));
        }
        if !seen.insert(trial_id.to_owned()) {
            return Err(format!(
                "Ratings row {row_number} duplicates trial '{trial_id}'."
            ));
        }
        let a_scores = parse_scores(fields, 1, row_number)?;
        let b_scores = parse_scores(fields, 8, row_number)?;
        let preference = match required(fields, 15, row_number)? {
            "A" => Preference::A,
            "B" => Preference::B,
            "tie" => Preference::Tie,
            value => {
                return Err(format!(
                    "Ratings row {row_number} field 'preference' must be A, B, or tie; found '{value}'."
                ))
            }
        };
        let confidence = parse_integer(fields, 16, row_number, 1, 5)?;
        let a_artifact_flags = parse_artifacts(&fields[17], row_number, "a_artifact_flags")?;
        let b_artifact_flags = parse_artifacts(&fields[18], row_number, "b_artifact_flags")?;
        if fields[19].chars().count() > MAX_NOTES_CHARS {
            return Err(format!(
                "Ratings row {row_number} field 'notes' exceeds {MAX_NOTES_CHARS} characters."
            ));
        }
        rows.push(RatingRow {
            trial_id: trial_id.to_owned(),
            a_scores,
            b_scores,
            preference,
            confidence,
            a_artifact_flags,
            b_artifact_flags,
            notes: fields[19].clone(),
        });
    }
    let completed = rows
        .iter()
        .map(|row| row.trial_id.as_str())
        .collect::<BTreeSet<_>>();
    let missing = expected
        .difference(&completed)
        .map(|value| (*value).to_owned())
        .collect::<Vec<_>>();
    if require_complete && !missing.is_empty() {
        return Err(format!(
            "Ratings CSV is missing expected trial(s): {}.",
            missing.join(", ")
        ));
    }
    rows.sort_by(|left, right| left.trial_id.cmp(&right.trial_id));
    Ok((rows, missing, expected.len()))
}

fn expected_participant_trials(package: &Path) -> Result<Vec<String>, String> {
    let path = package.join("participant/trials.csv");
    let contents = fs::read_to_string(&path)
        .map_err(|error| format!("Cannot read participant trials.csv: {error}"))?;
    let rows = parse_csv(&contents)?;
    let expected_header = ["trial_id", "reference", "a", "b", "first_output"];
    let expected_header_owned = expected_header.map(str::to_owned);
    if rows.first().map(Vec::as_slice) != Some(expected_header_owned.as_slice()) {
        return Err(
            "Participant trials.csv has an unexpected header and cannot define blinded trials."
                .to_owned(),
        );
    }
    let mut ids = Vec::new();
    let mut seen = HashSet::new();
    for (index, row) in rows.iter().enumerate().skip(1) {
        if row.len() != expected_header.len() || row[0].is_empty() {
            return Err(format!(
                "Participant trials.csv row {} is malformed.",
                index + 1
            ));
        }
        if !seen.insert(row[0].clone()) {
            return Err(format!(
                "Participant trials.csv duplicates trial '{}'.",
                row[0]
            ));
        }
        ids.push(row[0].clone());
    }
    if ids.is_empty() {
        return Err("Participant trials.csv contains no trials.".to_owned());
    }
    ids.sort();
    Ok(ids)
}

fn required(fields: &[String], index: usize, row: usize) -> Result<&str, String> {
    if fields[index].is_empty() {
        Err(format!(
            "Ratings row {row} field '{}' is required.",
            RATINGS_HEADER[index]
        ))
    } else {
        Ok(&fields[index])
    }
}

fn parse_scores(fields: &[String], start: usize, row: usize) -> Result<[u8; 7], String> {
    let mut scores = [0_u8; 7];
    for (offset, target) in scores.iter_mut().enumerate() {
        *target = parse_integer(fields, start + offset, row, 1, 7)?;
    }
    Ok(scores)
}

fn parse_integer(
    fields: &[String],
    index: usize,
    row: usize,
    minimum: u8,
    maximum: u8,
) -> Result<u8, String> {
    let value = required(fields, index, row)?;
    value
        .parse::<u8>()
        .ok()
        .filter(|parsed| (minimum..=maximum).contains(parsed))
        .ok_or_else(|| {
            format!(
                "Ratings row {row} field '{}' must be an integer from {minimum} to {maximum}; found '{value}'.",
                RATINGS_HEADER[index]
            )
        })
}

fn parse_artifacts(value: &str, row: usize, field: &str) -> Result<Vec<String>, String> {
    if value.is_empty() {
        return Ok(Vec::new());
    }
    let mut seen = HashSet::new();
    let mut parsed = Vec::new();
    for flag in value.split(';').map(str::trim) {
        if !ARTIFACT_FLAGS.contains(&flag) {
            return Err(format!(
                "Ratings row {row} field '{field}' contains unknown artifact flag '{flag}'."
            ));
        }
        if seen.insert(flag) {
            parsed.push(flag.to_owned());
        }
    }
    parsed.sort();
    Ok(parsed)
}

fn validate_participant_blinding(package: &Path, ratings_path: &Path) -> Result<(), String> {
    let participant = package.join("participant");
    inspect_participant_tree(&participant, &participant)?;
    let ratings = fs::read_to_string(ratings_path)
        .map_err(|error| format!("Cannot inspect ratings CSV for blinding: {error}"))?;
    reject_renderer_labels(&ratings, "ratings CSV")
}

fn inspect_participant_tree(root: &Path, current: &Path) -> Result<(), String> {
    for entry in fs::read_dir(current)
        .map_err(|error| format!("Cannot inspect participant package: {error}"))?
    {
        let entry =
            entry.map_err(|error| format!("Cannot inspect participant package: {error}"))?;
        let path = entry.path();
        let relative = path.strip_prefix(root).unwrap_or(&path);
        reject_renderer_labels(&relative.to_string_lossy(), "participant-facing filename")?;
        if path.is_dir() {
            inspect_participant_tree(root, &path)?;
        } else if path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|value| {
                value.eq_ignore_ascii_case("md") || value.eq_ignore_ascii_case("csv")
            })
        {
            let contents = fs::read_to_string(&path).map_err(|error| {
                format!(
                    "Cannot inspect participant file '{}': {error}",
                    relative.display()
                )
            })?;
            reject_renderer_labels(
                &contents,
                &format!("participant file '{}'", relative.display()),
            )?;
        }
    }
    Ok(())
}

fn reject_renderer_labels(value: &str, location: &str) -> Result<(), String> {
    let normalized = value.to_ascii_lowercase().replace(['-', '_', ' '], "");
    if let Some(label) = RENDERER_LABELS
        .iter()
        .find(|label| normalized.contains(**label))
    {
        Err(format!(
            "Blinding validation failed: {location} contains renderer label '{label}'."
        ))
    } else {
        Ok(())
    }
}

pub(crate) fn parse_csv(contents: &str) -> Result<Vec<Vec<String>>, String> {
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let mut chars = contents.chars().peekable();
    while let Some(character) = chars.next() {
        if quoted {
            if character == '"' {
                if chars.peek() == Some(&'"') {
                    chars.next();
                    field.push('"');
                } else {
                    quoted = false;
                }
            } else {
                field.push(character);
            }
            continue;
        }
        match character {
            '"' if field.is_empty() => quoted = true,
            ',' => {
                row.push(std::mem::take(&mut field));
            }
            '\n' => {
                row.push(std::mem::take(&mut field));
                if row.last().is_some_and(|value| value.ends_with('\r')) {
                    row.last_mut().unwrap().pop();
                }
                rows.push(std::mem::take(&mut row));
            }
            '\r' if chars.peek() == Some(&'\n') => {}
            '"' => return Err("Ratings CSV contains a quote inside an unquoted field.".to_owned()),
            other => field.push(other),
        }
    }
    if quoted {
        return Err("Ratings CSV ends inside a quoted field.".to_owned());
    }
    if !field.is_empty() || !row.is_empty() {
        row.push(field);
        rows.push(row);
    }
    Ok(rows)
}
