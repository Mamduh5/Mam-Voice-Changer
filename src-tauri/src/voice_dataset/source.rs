#![allow(dead_code)]

use std::{
    path::{Path, PathBuf},
    vec::IntoIter,
};

use super::{
    error::DatasetResult,
    manifest::VoiceDatasetManifestV1,
    profile::VoiceProfileMetadata,
    take::{validate_relative_path, DatasetStatistics, SelectedTakeVersion, TakeReviewStatus},
};

#[derive(Clone, Debug)]
pub struct AcceptedDatasetTake {
    pub id: String,
    pub path: PathBuf,
    pub prompt_id: Option<String>,
    pub prompt_text: Option<String>,
    pub quality: super::quality::TakeQualityReport,
}

pub trait VoiceDatasetSource {
    fn profile_metadata(&self) -> DatasetResult<VoiceProfileMetadata>;
    fn accepted_takes(&self) -> DatasetResult<Box<dyn Iterator<Item = AcceptedDatasetTake>>>;
    fn dataset_statistics(&self) -> DatasetResult<DatasetStatistics>;
}

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
            let relative = if take.selected_version == SelectedTakeVersion::Trimmed {
                take.derived_file.as_ref().unwrap_or(&take.raw_file)
            } else {
                &take.raw_file
            };
            validate_relative_path(relative)?;
            accepted.push(AcceptedDatasetTake {
                id: take.id.clone(),
                path: self
                    .profile_root
                    .join(relative.replace('/', std::path::MAIN_SEPARATOR_STR)),
                prompt_id: take.prompt_id.clone(),
                prompt_text: take.prompt_text.clone(),
                quality: take.quality.clone(),
            });
        }
        let iterator: IntoIter<AcceptedDatasetTake> = accepted.into_iter();
        Ok(Box::new(iterator))
    }

    fn dataset_statistics(&self) -> DatasetResult<DatasetStatistics> {
        Ok(self.manifest.statistics.clone())
    }
}
