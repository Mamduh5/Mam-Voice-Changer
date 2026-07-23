#![allow(dead_code)]

use std::{
    path::{Path, PathBuf},
    vec::IntoIter,
};

use super::{
    consent::ConsentMetadata,
    error::DatasetResult,
    manifest::{DatasetRecordingFormat, VoiceDatasetManifestV1},
    profile::VoiceProfileMetadata,
    prompts::PromptCategory,
    take::{
        validate_relative_path, DatasetStatistics, SelectedTakeVersion, TakeReviewStatus,
        TakeSource,
    },
};

#[derive(Clone, Debug)]
pub struct AcceptedDatasetTake {
    pub id: String,
    pub path: PathBuf,
    pub raw_path: PathBuf,
    pub prompt_id: Option<String>,
    pub prompt_text: Option<String>,
    pub prompt_category: Option<PromptCategory>,
    pub quality: super::quality::TakeQualityReport,
    pub source: TakeSource,
    pub selected_version: SelectedTakeVersion,
    pub manifest_content_hash: String,
    pub duration_ms: u64,
    pub manual_override: bool,
}

pub trait VoiceDatasetSource {
    fn profile_metadata(&self) -> DatasetResult<VoiceProfileMetadata>;
    fn accepted_takes(&self) -> DatasetResult<Box<dyn Iterator<Item = AcceptedDatasetTake>>>;
    fn dataset_statistics(&self) -> DatasetResult<DatasetStatistics>;
}

#[derive(Clone)]
pub struct ManifestDatasetSource {
    profile_root: PathBuf,
    manifest: VoiceDatasetManifestV1,
}

impl ManifestDatasetSource {
    pub fn new(profile_root: &Path, manifest: VoiceDatasetManifestV1) -> Self {
        Self {
            profile_root: profile_root.to_path_buf(),
            manifest,
        }
    }

    pub fn manifest(&self) -> &VoiceDatasetManifestV1 {
        &self.manifest
    }

    pub fn profile_root(&self) -> &Path {
        &self.profile_root
    }

    pub fn consent(&self) -> &ConsentMetadata {
        &self.manifest.consent
    }

    pub fn recording_format(&self) -> &DatasetRecordingFormat {
        &self.manifest.recording_format
    }
}

impl VoiceDatasetSource for ManifestDatasetSource {
    fn profile_metadata(&self) -> DatasetResult<VoiceProfileMetadata> {
        Ok(self.manifest.profile.clone())
    }

    fn accepted_takes(&self) -> DatasetResult<Box<dyn Iterator<Item = AcceptedDatasetTake>>> {
        let mut accepted = Vec::new();
        for take in &self.manifest.takes {
            if take.review_status != TakeReviewStatus::Accepted || take.exclude_from_training {
                continue;
            }
            if take.source == TakeSource::RecordedConsent {
                continue;
            }
            let relative = if take.selected_version == SelectedTakeVersion::Trimmed {
                take.derived_file.as_ref().unwrap_or(&take.raw_file)
            } else {
                &take.raw_file
            };
            validate_relative_path(relative)?;
            validate_relative_path(&take.raw_file)?;
            let quality = if take.selected_version == SelectedTakeVersion::Trimmed {
                take.trim
                    .as_ref()
                    .map(|trim| trim.derived_quality.clone())
                    .unwrap_or_else(|| take.quality.clone())
            } else {
                take.quality.clone()
            };
            accepted.push(AcceptedDatasetTake {
                id: take.id.clone(),
                path: self
                    .profile_root
                    .join(relative.replace('/', std::path::MAIN_SEPARATOR_STR)),
                raw_path: self
                    .profile_root
                    .join(take.raw_file.replace('/', std::path::MAIN_SEPARATOR_STR)),
                prompt_id: take.prompt_id.clone(),
                prompt_text: take.prompt_text.clone(),
                prompt_category: take.prompt_category,
                quality,
                source: take.source,
                selected_version: take.selected_version,
                manifest_content_hash: take.content_hash.clone(),
                duration_ms: if take.selected_version == SelectedTakeVersion::Trimmed {
                    take.trim
                        .as_ref()
                        .map(|trim| {
                            trim.end_frame.saturating_sub(trim.start_frame) * 1_000
                                / u64::from(take.sample_rate)
                        })
                        .unwrap_or(take.duration_ms)
                } else {
                    take.duration_ms
                },
                manual_override: take.manual_override,
            });
        }
        let iterator: IntoIter<AcceptedDatasetTake> = accepted.into_iter();
        Ok(Box::new(iterator))
    }

    fn dataset_statistics(&self) -> DatasetResult<DatasetStatistics> {
        Ok(self.manifest.statistics.clone())
    }
}
