use std::{
    collections::{BTreeMap, HashSet},
    fs,
    path::Path,
};

use serde::{Deserialize, Serialize};

use crate::voice_dataset::{
    hash::{sha256_bytes, sha256_samples},
    import::read_canonical_wav,
    manifest::DATASET_MANIFEST_SCHEMA_VERSION,
    prompts::PromptCategory,
    quality::{QualityClassification, TakeQualityReport},
    source::{ManifestDatasetSource, VoiceDatasetSource},
    storage::{new_id, timestamp},
    take::SelectedTakeVersion,
};

use super::{
    error::{VoiceModelError, VoiceModelErrorCode, VoiceModelResult},
    storage::atomic_write_json,
};

pub const SNAPSHOT_SCHEMA_VERSION: u32 = 1;
pub const DEFAULT_MINIMUM_ACCEPTED_DURATION_MS: u64 = 30_000;

fn default_minimum_accepted_duration_ms() -> u64 {
    DEFAULT_MINIMUM_ACCEPTED_DURATION_MS
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateTrainingSnapshotRequest {
    pub profile_id: String,
    #[serde(default = "default_minimum_accepted_duration_ms")]
    pub minimum_accepted_duration_ms: u64,
    pub validation_percent: u8,
    pub split_seed: u64,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SplitMembership {
    Training,
    Validation,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SnapshotTake {
    pub take_id: String,
    pub file: String,
    pub raw_content_hash: String,
    pub selected_content_hash: String,
    pub selected_version: SelectedTakeVersion,
    pub prompt_id: Option<String>,
    pub prompt_text: Option<String>,
    pub prompt_category: Option<PromptCategory>,
    pub quality: TakeQualityReport,
    pub duration_ms: u64,
    pub manual_override: bool,
    pub split: SplitMembership,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TrainingValidationSplit {
    pub seed: u64,
    pub training_take_count: u32,
    pub validation_take_count: u32,
    pub training_duration_ms: u64,
    pub validation_duration_ms: u64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TrainingSnapshotV1 {
    pub schema_version: u32,
    pub snapshot_id: String,
    pub content_hash: String,
    pub profile_id: String,
    pub dataset_schema_version: u32,
    pub consent_version: String,
    pub consent_confirmed_at: String,
    pub prompt_pack_id: String,
    pub prompt_pack_version: u32,
    pub canonical_sample_rate: u32,
    pub canonical_channels: u16,
    pub total_duration_ms: u64,
    pub takes: Vec<SnapshotTake>,
    pub split: TrainingValidationSplit,
    pub warnings: Vec<String>,
    pub created_at: String,
}

pub fn create_snapshot(
    snapshots_root: &Path,
    source: &ManifestDatasetSource,
    request: &CreateTrainingSnapshotRequest,
) -> VoiceModelResult<TrainingSnapshotV1> {
    let manifest = source.manifest();
    if manifest.profile.id != request.profile_id {
        return Err(VoiceModelError::new(
            VoiceModelErrorCode::ProfileMissing,
            "The selected Dataset profile does not match the snapshot request.",
        ));
    }
    if !manifest.consent.consent_confirmed || manifest.consent.revoked_at.is_some() {
        return Err(VoiceModelError::new(
            VoiceModelErrorCode::ConsentInactive,
            "Active target-speaker consent is required to create a model snapshot.",
        ));
    }
    if manifest.schema_version != DATASET_MANIFEST_SCHEMA_VERSION {
        return Err(VoiceModelError::new(
            VoiceModelErrorCode::DatasetUnhealthy,
            "The Dataset manifest schema is not supported.",
        ));
    }
    if request.validation_percent > 50 {
        return Err(VoiceModelError::new(
            VoiceModelErrorCode::InvalidTrainingConfiguration,
            "Validation percentage must be between 0 and 50.",
        ));
    }

    let accepted: Vec<_> = source.accepted_takes().map_err(dataset_error)?.collect();
    if accepted.is_empty() {
        return Err(VoiceModelError::new(
            VoiceModelErrorCode::NoAcceptedTakes,
            "Accept at least one non-excluded Dataset take before snapshotting.",
        ));
    }
    let total_duration_ms = accepted.iter().map(|take| take.duration_ms).sum();
    if total_duration_ms < request.minimum_accepted_duration_ms {
        return Err(VoiceModelError::new(
            VoiceModelErrorCode::SnapshotTooSmall,
            format!(
                "Accepted duration is {} seconds; the configured minimum is {} seconds.",
                total_duration_ms / 1_000,
                request.minimum_accepted_duration_ms / 1_000
            ),
        ));
    }

    fs::create_dir_all(snapshots_root)
        .map_err(|error| VoiceModelError::storage("Cannot create snapshot storage", error))?;
    let now = timestamp().map_err(dataset_error)?;
    let snapshot_id = new_id("snapshot", &now);
    let final_directory = snapshots_root.join(&snapshot_id);
    let temporary_directory = snapshots_root.join(format!(".{snapshot_id}.tmp"));
    if temporary_directory.exists() {
        fs::remove_dir_all(&temporary_directory)
            .map_err(|error| VoiceModelError::storage("Cannot clear stale snapshot data", error))?;
    }
    fs::create_dir_all(temporary_directory.join("audio"))
        .map_err(|error| VoiceModelError::storage("Cannot create snapshot audio storage", error))?;

    let validation_ids = deterministic_validation_ids(
        &accepted,
        &snapshot_id,
        request.split_seed,
        request.validation_percent,
    );
    let mut takes = Vec::with_capacity(accepted.len());
    for (index, take) in accepted.iter().enumerate() {
        if !take.raw_path.is_file() || !take.path.is_file() {
            cleanup(&temporary_directory);
            return Err(VoiceModelError::new(
                VoiceModelErrorCode::DatasetUnhealthy,
                "An accepted Dataset take file is missing.",
            ));
        }
        let raw_samples = read_canonical_wav(&take.raw_path).map_err(dataset_error)?;
        let raw_hash = sha256_samples(&raw_samples);
        if raw_hash != take.manifest_content_hash {
            cleanup(&temporary_directory);
            return Err(VoiceModelError::new(
                VoiceModelErrorCode::SnapshotHashMismatch,
                "An accepted Dataset take no longer matches its manifest hash.",
            ));
        }
        let selected_samples = if take.path == take.raw_path {
            raw_samples
        } else {
            read_canonical_wav(&take.path).map_err(dataset_error)?
        };
        let selected_hash = sha256_samples(&selected_samples);
        let file = format!("audio/take-{index:06}.wav");
        fs::copy(
            &take.path,
            temporary_directory.join(file.replace('/', std::path::MAIN_SEPARATOR_STR)),
        )
        .map_err(|error| VoiceModelError::storage("Cannot copy snapshot audio", error))?;
        takes.push(SnapshotTake {
            take_id: take.id.clone(),
            file,
            raw_content_hash: raw_hash,
            selected_content_hash: selected_hash,
            selected_version: take.selected_version,
            prompt_id: take.prompt_id.clone(),
            prompt_text: take.prompt_text.clone(),
            prompt_category: take.prompt_category,
            quality: take.quality.clone(),
            duration_ms: take.duration_ms,
            manual_override: take.manual_override,
            split: if validation_ids.contains(&take.id) {
                SplitMembership::Validation
            } else {
                SplitMembership::Training
            },
        });
    }
    let split = summarize_split(request.split_seed, &takes);
    let warnings = snapshot_warnings(&takes, total_duration_ms, split.validation_take_count);
    let mut snapshot = TrainingSnapshotV1 {
        schema_version: SNAPSHOT_SCHEMA_VERSION,
        snapshot_id,
        content_hash: String::new(),
        profile_id: manifest.profile.id.clone(),
        dataset_schema_version: manifest.schema_version,
        consent_version: manifest.consent.consent_version.clone(),
        consent_confirmed_at: manifest.consent.confirmed_at.clone(),
        prompt_pack_id: manifest.prompt_pack.id.clone(),
        prompt_pack_version: manifest.prompt_pack.version,
        canonical_sample_rate: manifest.recording_format.sample_rate,
        canonical_channels: manifest.recording_format.channels,
        total_duration_ms,
        takes,
        split,
        warnings,
        created_at: now,
    };
    snapshot.content_hash = calculate_snapshot_hash(&snapshot)?;
    atomic_write_json(&temporary_directory.join("snapshot.json"), &snapshot)?;
    fs::rename(&temporary_directory, &final_directory).map_err(|error| {
        cleanup(&temporary_directory);
        VoiceModelError::new(
            VoiceModelErrorCode::AtomicWriteFailure,
            format!("Cannot commit the immutable training snapshot: {error}"),
        )
    })?;
    Ok(snapshot)
}

pub fn verify_snapshot(snapshot: &TrainingSnapshotV1, root: &Path) -> VoiceModelResult<()> {
    if snapshot.schema_version != SNAPSHOT_SCHEMA_VERSION {
        return Err(VoiceModelError::new(
            VoiceModelErrorCode::SnapshotHashMismatch,
            "The snapshot schema is unsupported.",
        ));
    }
    if calculate_snapshot_hash(snapshot)? != snapshot.content_hash {
        return Err(VoiceModelError::new(
            VoiceModelErrorCode::SnapshotHashMismatch,
            "The training snapshot manifest hash is invalid.",
        ));
    }
    for take in &snapshot.takes {
        let path = root.join(take.file.replace('/', std::path::MAIN_SEPARATOR_STR));
        let samples = read_canonical_wav(&path).map_err(dataset_error)?;
        if sha256_samples(&samples) != take.selected_content_hash {
            return Err(VoiceModelError::new(
                VoiceModelErrorCode::SnapshotHashMismatch,
                "A snapshot audio file hash is invalid.",
            ));
        }
    }
    Ok(())
}

fn calculate_snapshot_hash(snapshot: &TrainingSnapshotV1) -> VoiceModelResult<String> {
    let mut stable = snapshot.clone();
    stable.content_hash.clear();
    let bytes = serde_json::to_vec(&stable).map_err(|_| {
        VoiceModelError::new(
            VoiceModelErrorCode::SnapshotCreationFailed,
            "Cannot serialize the snapshot for hashing.",
        )
    })?;
    Ok(sha256_bytes(&bytes))
}

fn deterministic_validation_ids(
    takes: &[crate::voice_dataset::source::AcceptedDatasetTake],
    snapshot_id: &str,
    seed: u64,
    percent: u8,
) -> HashSet<String> {
    if takes.len() < 2 || percent == 0 {
        return HashSet::new();
    }
    let requested = ((takes.len() * usize::from(percent)).div_ceil(100))
        .max(1)
        .min(takes.len() - 1);
    let mut grouped: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
    for take in takes {
        let category = format!("{:?}", take.prompt_category);
        let key = sha256_bytes(format!("{snapshot_id}:{seed}:{}", take.id).as_bytes());
        grouped
            .entry(category)
            .or_default()
            .push((key, take.id.clone()));
    }
    for values in grouped.values_mut() {
        values.sort();
    }
    let mut selected = HashSet::new();
    while selected.len() < requested {
        let mut changed = false;
        for values in grouped.values_mut() {
            if let Some((_, id)) = values.pop() {
                selected.insert(id);
                changed = true;
                if selected.len() == requested {
                    break;
                }
            }
        }
        if !changed {
            break;
        }
    }
    selected
}

fn summarize_split(seed: u64, takes: &[SnapshotTake]) -> TrainingValidationSplit {
    let mut split = TrainingValidationSplit {
        seed,
        ..TrainingValidationSplit::default()
    };
    for take in takes {
        match take.split {
            SplitMembership::Training => {
                split.training_take_count += 1;
                split.training_duration_ms += take.duration_ms;
            }
            SplitMembership::Validation => {
                split.validation_take_count += 1;
                split.validation_duration_ms += take.duration_ms;
            }
        }
    }
    split
}

fn snapshot_warnings(
    takes: &[SnapshotTake],
    total_duration_ms: u64,
    validation_count: u32,
) -> Vec<String> {
    let mut warnings = Vec::new();
    if total_duration_ms < 5 * 60_000 {
        warnings.push("This is a small Dataset; training quality is not guaranteed.".to_owned());
    }
    if validation_count == 0 {
        warnings
            .push("This run has no validation split because the Dataset is very small.".to_owned());
    }
    let overridden = takes.iter().filter(|take| take.manual_override).count();
    if overridden * 4 > takes.len() {
        warnings.push("Many accepted takes manually override failed quality checks.".to_owned());
    }
    let low_snr = takes
        .iter()
        .filter(|take| take.quality.heuristic_signal_to_noise_db < 12.0)
        .count();
    if low_snr * 3 > takes.len() {
        warnings.push("Many takes have low estimated signal-to-noise ratio.".to_owned());
    }
    let pass = takes
        .iter()
        .filter(|take| take.quality.classification == QualityClassification::Pass)
        .count();
    if pass * 2 < takes.len() {
        warnings
            .push("Fewer than half of the takes passed all heuristic quality checks.".to_owned());
    }
    let unique_prompts: HashSet<_> = takes
        .iter()
        .filter_map(|take| take.prompt_id.as_ref())
        .collect();
    if unique_prompts.len() * 2 < takes.len() {
        warnings.push("The snapshot contains many repeated prompts.".to_owned());
    }
    let categories: HashSet<_> = takes
        .iter()
        .filter_map(|take| take.prompt_category)
        .collect();
    if categories.len() < 4 {
        warnings.push("Prompt-category coverage is low.".to_owned());
    }
    warnings
}

fn cleanup(path: &Path) {
    let _ = fs::remove_dir_all(path);
}

fn dataset_error(error: crate::voice_dataset::error::DatasetError) -> VoiceModelError {
    VoiceModelError::new(VoiceModelErrorCode::DatasetUnhealthy, error.message)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::deterministic_validation_ids;
    use crate::voice_dataset::{
        prompts::PromptCategory,
        quality::{analyze_take, CaptureMetrics},
        source::AcceptedDatasetTake,
        take::{SelectedTakeVersion, TakeSource},
    };

    fn accepted(id: &str, category: PromptCategory) -> AcceptedDatasetTake {
        AcceptedDatasetTake {
            id: id.to_owned(),
            path: PathBuf::from(format!("{id}.wav")),
            raw_path: PathBuf::from(format!("{id}.wav")),
            prompt_id: Some(format!("prompt-{id}")),
            prompt_text: Some("Test phrase".to_owned()),
            prompt_category: Some(category),
            quality: analyze_take(&vec![0.1; 48_000], 48_000, CaptureMetrics::default()),
            source: TakeSource::Recorded,
            selected_version: SelectedTakeVersion::Raw,
            manifest_content_hash: "hash".to_owned(),
            duration_ms: 1_000,
            manual_override: false,
        }
    }

    #[test]
    fn split_is_deterministic_non_empty_and_never_selects_every_take() {
        let takes = vec![
            accepted("a", PromptCategory::NeutralStatement),
            accepted("b", PromptCategory::Question),
            accepted("c", PromptCategory::Plosives),
            accepted("d", PromptCategory::Sibilants),
        ];
        let first = deterministic_validation_ids(&takes, "snapshot-1", 13, 25);
        let second = deterministic_validation_ids(&takes, "snapshot-1", 13, 25);
        assert_eq!(first, second);
        assert_eq!(first.len(), 1);
        assert!(first.len() < takes.len());
    }

    #[test]
    fn tiny_or_zero_percent_snapshot_uses_training_only() {
        let one = vec![accepted("a", PromptCategory::NeutralStatement)];
        assert!(deterministic_validation_ids(&one, "snapshot-1", 13, 20).is_empty());
        let two = vec![
            accepted("a", PromptCategory::NeutralStatement),
            accepted("b", PromptCategory::Question),
        ];
        assert!(deterministic_validation_ids(&two, "snapshot-1", 13, 0).is_empty());
    }
}
