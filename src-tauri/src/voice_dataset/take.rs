use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use super::{
    error::{DatasetError, DatasetErrorCode, DatasetResult},
    prompts::PromptCategory,
    quality::{QualityClassification, TakeQualityReport},
};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TakeSource {
    Recorded,
    Imported,
    RecordedConsent,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TakeReviewStatus {
    Pending,
    Accepted,
    Rejected,
    NeedsRedo,
    Deleting,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SelectedTakeVersion {
    Raw,
    Trimmed,
}

#[derive(Clone, Debug)]
pub struct TakeReviewUpdate {
    pub status: TakeReviewStatus,
    pub exclude_from_training: bool,
    pub notes: Option<String>,
    pub warning_acknowledged: bool,
    pub selected_version: SelectedTakeVersion,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WaveformPoint {
    pub minimum: f32,
    pub maximum: f32,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OriginalFormatMetadata {
    pub sample_rate: u32,
    pub channels: u16,
    pub bits_per_sample: u16,
    pub sample_format: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TrimMetadata {
    pub start_frame: u64,
    pub end_frame: u64,
    pub derived_quality: TakeQualityReport,
    pub derived_waveform_envelope: Vec<WaveformPoint>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DatasetTake {
    pub id: String,
    pub prompt_id: Option<String>,
    pub prompt_text: Option<String>,
    pub prompt_category: Option<PromptCategory>,
    pub source: TakeSource,
    pub raw_file: String,
    pub derived_file: Option<String>,
    pub selected_version: SelectedTakeVersion,
    pub original_format: Option<OriginalFormatMetadata>,
    pub sample_rate: u32,
    pub channels: u16,
    pub frame_count: u64,
    pub duration_ms: u64,
    pub waveform_envelope: Vec<WaveformPoint>,
    pub quality: TakeQualityReport,
    pub trim: Option<TrimMetadata>,
    pub review_status: TakeReviewStatus,
    pub exclude_from_training: bool,
    pub notes: Option<String>,
    pub manual_override: bool,
    pub warning_acknowledged: bool,
    pub created_at: String,
    pub content_hash: String,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DatasetStatistics {
    pub total_takes: u32,
    pub accepted_takes: u32,
    pub rejected_takes: u32,
    pub pending_takes: u32,
    pub warning_takes: u32,
    pub failed_takes: u32,
    pub accepted_duration_ms: u64,
    pub completed_prompts: u32,
    pub total_prompts: u32,
    pub category_coverage: HashMap<PromptCategory, u32>,
    pub custom_takes: u32,
    pub excluded_takes: u32,
}

pub fn waveform_envelope(samples: &[f32], buckets: usize) -> Vec<WaveformPoint> {
    if samples.is_empty() {
        return Vec::new();
    }
    let bucket_size = samples.len().div_ceil(buckets.max(1));
    samples
        .chunks(bucket_size)
        .map(|bucket| WaveformPoint {
            minimum: bucket.iter().copied().fold(f32::INFINITY, f32::min),
            maximum: bucket.iter().copied().fold(f32::NEG_INFINITY, f32::max),
        })
        .collect()
}

pub fn rebuild_statistics(takes: &[DatasetTake], total_prompts: usize) -> DatasetStatistics {
    let mut statistics = DatasetStatistics {
        total_takes: takes.len() as u32,
        total_prompts: total_prompts as u32,
        ..DatasetStatistics::default()
    };
    let mut prompts = HashSet::new();
    for take in takes {
        match take.review_status {
            TakeReviewStatus::Accepted => {
                statistics.accepted_takes += 1;
                if !take.exclude_from_training {
                    statistics.accepted_duration_ms += selected_duration_ms(take);
                }
            }
            TakeReviewStatus::Rejected | TakeReviewStatus::NeedsRedo => {
                statistics.rejected_takes += 1
            }
            TakeReviewStatus::Pending | TakeReviewStatus::Deleting => statistics.pending_takes += 1,
        }
        match take.quality.classification {
            QualityClassification::Warning => statistics.warning_takes += 1,
            QualityClassification::Fail => statistics.failed_takes += 1,
            QualityClassification::Pass => {}
        }
        if let Some(prompt_id) = &take.prompt_id {
            if take.review_status == TakeReviewStatus::Accepted {
                prompts.insert(prompt_id.clone());
            }
        }
        if let Some(category) = take.prompt_category {
            if take.review_status == TakeReviewStatus::Accepted {
                *statistics.category_coverage.entry(category).or_default() += 1;
            }
            if category == PromptCategory::Custom {
                statistics.custom_takes += 1;
            }
        }
        statistics.excluded_takes += u32::from(take.exclude_from_training);
    }
    statistics.completed_prompts = prompts.len() as u32;
    statistics
}

pub fn validate_take(take: &DatasetTake) -> DatasetResult<()> {
    validate_relative_path(&take.raw_file)?;
    if let Some(path) = &take.derived_file {
        validate_relative_path(path)?;
    }
    if take.selected_version == SelectedTakeVersion::Trimmed && take.derived_file.is_none() {
        return Err(DatasetError::new(
            DatasetErrorCode::CorruptManifest,
            "A take selects a missing trimmed file.",
        ));
    }
    if take.channels != 1 || take.sample_rate != 48_000 || take.frame_count == 0 {
        return Err(DatasetError::new(
            DatasetErrorCode::CorruptManifest,
            "A take does not use the canonical dataset format.",
        ));
    }
    Ok(())
}

pub fn validate_relative_path(path: &str) -> DatasetResult<()> {
    let candidate = std::path::Path::new(path);
    let valid = !path.is_empty()
        && !candidate.is_absolute()
        && !path.contains('\\')
        && candidate
            .components()
            .all(|component| matches!(component, std::path::Component::Normal(_)));
    if !valid {
        return Err(DatasetError::new(
            DatasetErrorCode::PathValidationFailed,
            "Dataset manifests may contain only normalized relative managed paths.",
        ));
    }
    Ok(())
}

fn selected_duration_ms(take: &DatasetTake) -> u64 {
    if take.selected_version == SelectedTakeVersion::Trimmed {
        take.trim
            .as_ref()
            .map(|trim| {
                (trim.end_frame.saturating_sub(trim.start_frame) * 1_000)
                    / u64::from(take.sample_rate)
            })
            .unwrap_or(take.duration_ms)
    } else {
        take.duration_ms
    }
}

#[cfg(test)]
mod tests {
    use super::{validate_relative_path, waveform_envelope};

    #[test]
    fn manifest_paths_are_relative_and_traversal_safe() {
        assert!(validate_relative_path("raw/take-1.wav").is_ok());
        assert!(validate_relative_path("../escape.wav").is_err());
        assert!(validate_relative_path("C:/escape.wav").is_err());
        assert!(validate_relative_path("raw\\escape.wav").is_err());
    }

    #[test]
    fn waveform_contains_no_raw_sample_array() {
        let waveform = waveform_envelope(&[-0.5, 0.25, -0.1, 0.8], 2);
        assert_eq!(waveform.len(), 2);
        assert_eq!(waveform[0].minimum, -0.5);
        assert_eq!(waveform[1].maximum, 0.8);
    }
}
