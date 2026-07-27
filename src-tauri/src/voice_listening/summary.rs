use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

use serde::{Deserialize, Serialize};

use super::{
    package::{
        append_csv_row, format_number, read_key, require_empty_output, write_json, Renderer,
        TrialKey,
    },
    ratings::{load_ratings, Preference, RatingRow, DIMENSIONS},
    MINIMUM_CATEGORY_TRIALS, PACKAGE_SCHEMA_VERSION,
};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ListeningSummary {
    pub schema_version: u32,
    pub study_id: String,
    pub expected_trials: usize,
    pub rated_trials: usize,
    pub missing_trials: Vec<String>,
    pub renderers: BTreeMap<String, RendererStatistics>,
    pub paired_differences_world_minus_existing_dsp: BTreeMap<String, Option<f64>>,
    pub by_tag: BTreeMap<String, GroupStatistics>,
    pub by_transformation_type: BTreeMap<String, GroupStatistics>,
    pub objective_metric_linkage: String,
    pub warnings: Vec<String>,
    pub interpretation_limits: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RendererStatistics {
    pub rated_trials: usize,
    pub dimensions: BTreeMap<String, DimensionStatistics>,
    pub preference_wins: usize,
    pub preference_losses: usize,
    pub ties: usize,
    pub artifact_flag_counts: BTreeMap<String, usize>,
    pub mean_confidence: Option<f64>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DimensionStatistics {
    pub count: usize,
    pub mean: Option<f64>,
    pub median: Option<f64>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GroupStatistics {
    pub rated_trials: usize,
    pub source_clip_count: usize,
    pub renderers: BTreeMap<String, RendererStatistics>,
    pub warnings: Vec<String>,
}

#[derive(Default)]
struct RendererAccumulator {
    scores: [Vec<f64>; 7],
    wins: usize,
    losses: usize,
    ties: usize,
    artifacts: BTreeMap<String, usize>,
    confidence: Vec<f64>,
}

#[derive(Default)]
struct GroupAccumulator {
    renderers: BTreeMap<&'static str, RendererAccumulator>,
    trials: BTreeSet<String>,
    sources: BTreeSet<String>,
}

pub fn summarize_ratings(
    package: &Path,
    ratings_path: &Path,
    output: &Path,
) -> Result<ListeningSummary, String> {
    require_empty_output(output)?;
    fs::create_dir_all(output)
        .map_err(|error| format!("Cannot create listening results directory: {error}"))?;
    let key = read_key(package)?;
    let (rows, missing, expected) = load_ratings(package, ratings_path, false)?;
    let by_trial = key
        .trials
        .iter()
        .map(|trial| (trial.trial_id.as_str(), trial))
        .collect::<BTreeMap<_, _>>();

    let mut overall = GroupAccumulator::default();
    let mut tags = BTreeMap::<String, GroupAccumulator>::new();
    let mut transformations = BTreeMap::<String, GroupAccumulator>::new();
    let mut paired = [
        Vec::<f64>::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
    ];
    let mut warnings = Vec::new();
    for row in &rows {
        let trial = by_trial.get(row.trial_id.as_str()).ok_or_else(|| {
            format!(
                "Administrator key does not contain rated trial '{}'.",
                row.trial_id
            )
        })?;
        accumulate(&mut overall, trial, row);
        if trial.tags.is_empty() {
            warnings.push(format!(
                "Trial '{}' has no user-provided tags.",
                trial.trial_id
            ));
        }
        for tag in &trial.tags {
            accumulate(tags.entry(tag.clone()).or_default(), trial, row);
        }
        accumulate(
            transformations
                .entry(trial.transformation_type.clone())
                .or_default(),
            trial,
            row,
        );
        for (index, differences) in paired.iter_mut().enumerate() {
            let (world, existing) = scores_by_renderer(trial, row, index);
            differences.push(world - existing);
        }
    }
    if !missing.is_empty() {
        warnings.push(format!(
            "Ratings are incomplete: {} of {expected} expected trials are missing.",
            missing.len()
        ));
    }
    if overall.sources.len() <= 1 && !rows.is_empty() {
        warnings.push("The available results are driven by only one source clip.".to_owned());
    }
    if rows.len() < MINIMUM_CATEGORY_TRIALS {
        warnings.push(format!(
            "The complete study has fewer than the documented minimum of {MINIMUM_CATEGORY_TRIALS} rated trials."
        ));
    }

    let summary = ListeningSummary {
        schema_version: PACKAGE_SCHEMA_VERSION,
        study_id: key.study.id,
        expected_trials: expected,
        rated_trials: rows.len(),
        missing_trials: missing,
        renderers: finalize_group(&overall).renderers,
        paired_differences_world_minus_existing_dsp: DIMENSIONS
            .iter()
            .enumerate()
            .map(|(index, dimension)| {
                (
                    (*dimension).to_owned(),
                    mean(&paired[index]),
                )
            })
            .collect(),
        by_tag: tags
            .into_iter()
            .map(|(label, group)| (label, finalize_group(&group)))
            .collect(),
        by_transformation_type: transformations
            .into_iter()
            .map(|(label, group)| (label, finalize_group(&group)))
            .collect(),
        objective_metric_linkage:
            "The listening package administrator/render-metrics.csv joins by trial_id and renderer."
                .to_owned(),
        warnings,
        interpretation_limits: vec![
            "These are descriptive local 1-7 study scores, not standardized MOS.".to_owned(),
            "No statistical significance or causal relationship is claimed.".to_owned(),
            "The summary does not declare either renderer universally superior.".to_owned(),
            "Small samples, source selection, listener setup, and subjective judgment limit interpretation."
                .to_owned(),
        ],
    };
    write_outputs(output, &summary, &key.trials, &rows)?;
    Ok(summary)
}

fn accumulate(group: &mut GroupAccumulator, trial: &TrialKey, row: &RatingRow) {
    group.trials.insert(trial.trial_id.clone());
    group.sources.insert(trial.source_clip_id.clone());
    let (a, b) = (
        group
            .renderers
            .entry(trial.a_renderer.as_str())
            .or_default(),
        trial.b_renderer.as_str(),
    );
    for (target, score) in a.scores.iter_mut().zip(row.a_scores) {
        target.push(f64::from(score));
    }
    a.confidence.push(f64::from(row.confidence));
    for flag in &row.a_artifact_flags {
        *a.artifacts.entry(flag.clone()).or_default() += 1;
    }
    match row.preference {
        Preference::A => a.wins += 1,
        Preference::B => a.losses += 1,
        Preference::Tie => a.ties += 1,
    }

    let b = group.renderers.entry(b).or_default();
    for (target, score) in b.scores.iter_mut().zip(row.b_scores) {
        target.push(f64::from(score));
    }
    b.confidence.push(f64::from(row.confidence));
    for flag in &row.b_artifact_flags {
        *b.artifacts.entry(flag.clone()).or_default() += 1;
    }
    match row.preference {
        Preference::A => b.losses += 1,
        Preference::B => b.wins += 1,
        Preference::Tie => b.ties += 1,
    }
}

fn scores_by_renderer(trial: &TrialKey, row: &RatingRow, index: usize) -> (f64, f64) {
    let a = f64::from(row.a_scores[index]);
    let b = f64::from(row.b_scores[index]);
    if trial.a_renderer == Renderer::WorldReference {
        (a, b)
    } else {
        (b, a)
    }
}

fn finalize_group(group: &GroupAccumulator) -> GroupStatistics {
    let renderers = group
        .renderers
        .iter()
        .map(|(renderer, values)| {
            let dimensions = DIMENSIONS
                .iter()
                .enumerate()
                .map(|(index, name)| {
                    (
                        (*name).to_owned(),
                        DimensionStatistics {
                            count: values.scores[index].len(),
                            mean: mean(&values.scores[index]),
                            median: median(&values.scores[index]),
                        },
                    )
                })
                .collect();
            (
                (*renderer).to_owned(),
                RendererStatistics {
                    rated_trials: values.confidence.len(),
                    dimensions,
                    preference_wins: values.wins,
                    preference_losses: values.losses,
                    ties: values.ties,
                    artifact_flag_counts: values.artifacts.clone(),
                    mean_confidence: mean(&values.confidence),
                },
            )
        })
        .collect();
    let mut warnings = Vec::new();
    if group.trials.len() < MINIMUM_CATEGORY_TRIALS {
        warnings.push(format!(
            "Category has fewer than {MINIMUM_CATEGORY_TRIALS} rated trials."
        ));
    }
    if group.sources.len() <= 1 && !group.trials.is_empty() {
        warnings.push("Category is driven by only one source clip.".to_owned());
    }
    GroupStatistics {
        rated_trials: group.trials.len(),
        source_clip_count: group.sources.len(),
        renderers,
        warnings,
    }
}

fn mean(values: &[f64]) -> Option<f64> {
    (!values.is_empty()).then(|| values.iter().sum::<f64>() / values.len() as f64)
}

fn median(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let mut ordered = values.to_vec();
    ordered.sort_by(f64::total_cmp);
    let middle = ordered.len() / 2;
    Some(if ordered.len().is_multiple_of(2) {
        (ordered[middle - 1] + ordered[middle]) * 0.5
    } else {
        ordered[middle]
    })
}

fn write_outputs(
    output: &Path,
    summary: &ListeningSummary,
    trials: &[TrialKey],
    rows: &[RatingRow],
) -> Result<(), String> {
    write_json(&output.join("summary.json"), summary)?;

    let mut csv =
        "renderer,dimension,count,mean,median,preference_wins,preference_losses,ties,mean_confidence\n"
            .to_owned();
    for (renderer, statistics) in &summary.renderers {
        for (dimension, values) in &statistics.dimensions {
            append_csv_row(
                &mut csv,
                &[
                    renderer,
                    dimension,
                    &values.count.to_string(),
                    &values.mean.map(format_number).unwrap_or_default(),
                    &values.median.map(format_number).unwrap_or_default(),
                    &statistics.preference_wins.to_string(),
                    &statistics.preference_losses.to_string(),
                    &statistics.ties.to_string(),
                    &statistics
                        .mean_confidence
                        .map(format_number)
                        .unwrap_or_default(),
                ],
            );
        }
    }
    fs::write(output.join("summary.csv"), csv)
        .map_err(|error| format!("Cannot write listening summary CSV: {error}"))?;

    let mut markdown = format!(
        "# Local listening-study summary\n\n- Rated trials: {} / {}\n- Objective metrics: `{}`\n\n",
        summary.rated_trials, summary.expected_trials, summary.objective_metric_linkage
    );
    for (renderer, statistics) in &summary.renderers {
        markdown.push_str(&format!(
            "## {renderer}\n\n- Preference wins/losses/ties: {}/{}/{}\n- Mean confidence: {}\n\n| Dimension | N | Mean | Median |\n|---|---:|---:|---:|\n",
            statistics.preference_wins,
            statistics.preference_losses,
            statistics.ties,
            statistics
                .mean_confidence
                .map(format_number)
                .unwrap_or_else(|| "unavailable".to_owned())
        ));
        for (dimension, values) in &statistics.dimensions {
            markdown.push_str(&format!(
                "| {dimension} | {} | {} | {} |\n",
                values.count,
                values.mean.map(format_number).unwrap_or_default(),
                values.median.map(format_number).unwrap_or_default()
            ));
        }
        if !statistics.artifact_flag_counts.is_empty() {
            markdown.push_str("\nArtifact flags:\n\n");
            for (flag, count) in &statistics.artifact_flag_counts {
                markdown.push_str(&format!("- {flag}: {count}\n"));
            }
        }
        markdown.push('\n');
    }
    markdown.push_str("## Paired differences (WORLD - existing DSP)\n\n");
    for (dimension, difference) in &summary.paired_differences_world_minus_existing_dsp {
        markdown.push_str(&format!(
            "- {dimension}: {}\n",
            difference
                .map(format_number)
                .unwrap_or_else(|| "unavailable".to_owned())
        ));
    }
    markdown.push_str("\n## Results by user-provided tag\n\n");
    append_group_markdown(&mut markdown, &summary.by_tag);
    markdown.push_str("\n## Results by transformation type\n\n");
    append_group_markdown(&mut markdown, &summary.by_transformation_type);
    markdown.push_str("\n## Warnings\n\n");
    if summary.warnings.is_empty() {
        markdown.push_str("- None recorded.\n");
    } else {
        for warning in &summary.warnings {
            markdown.push_str(&format!("- {warning}\n"));
        }
    }
    markdown.push_str("\n## Interpretation limits\n\n");
    for limit in &summary.interpretation_limits {
        markdown.push_str(&format!("- {limit}\n"));
    }
    fs::write(output.join("summary.md"), markdown)
        .map_err(|error| format!("Cannot write listening summary Markdown: {error}"))?;

    let by_trial = trials
        .iter()
        .map(|trial| (trial.trial_id.as_str(), trial))
        .collect::<BTreeMap<_, _>>();
    let mut trial_csv = "trial_id,source_clip_id,a_renderer,b_renderer,preference,preferred_renderer,confidence,a_scores,b_scores,a_artifact_flags,b_artifact_flags,notes\n".to_owned();
    for row in rows {
        let trial = by_trial[row.trial_id.as_str()];
        let preferred = match row.preference {
            Preference::A => trial.a_renderer.as_str(),
            Preference::B => trial.b_renderer.as_str(),
            Preference::Tie => "tie",
        };
        append_csv_row(
            &mut trial_csv,
            &[
                &row.trial_id,
                &trial.source_clip_id,
                trial.a_renderer.as_str(),
                trial.b_renderer.as_str(),
                match row.preference {
                    Preference::A => "A",
                    Preference::B => "B",
                    Preference::Tie => "tie",
                },
                preferred,
                &row.confidence.to_string(),
                &row.a_scores.map(|value| value.to_string()).join(";"),
                &row.b_scores.map(|value| value.to_string()).join(";"),
                &row.a_artifact_flags.join(";"),
                &row.b_artifact_flags.join(";"),
                &row.notes,
            ],
        );
    }
    fs::write(output.join("trial-results.csv"), trial_csv)
        .map_err(|error| format!("Cannot write unblinded trial results: {error}"))
}

fn append_group_markdown(output: &mut String, groups: &BTreeMap<String, GroupStatistics>) {
    if groups.is_empty() {
        output.push_str("- No groups available.\n");
        return;
    }
    for (label, group) in groups {
        output.push_str(&format!(
            "- {label}: {} rated trial(s), {} source clip(s)",
            group.rated_trials, group.source_clip_count
        ));
        if !group.warnings.is_empty() {
            output.push_str(&format!("; {}", group.warnings.join(" ")));
        }
        output.push('\n');
    }
}
