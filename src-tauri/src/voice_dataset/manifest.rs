use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use super::{
    consent::ConsentMetadata,
    error::{DatasetError, DatasetErrorCode, DatasetResult},
    profile::VoiceProfileMetadata,
    prompts::PromptPackReference,
    take::{rebuild_statistics, validate_take, DatasetStatistics, DatasetTake},
};

pub const DATASET_MANIFEST_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DatasetRecordingFormat {
    pub container: String,
    pub sample_format: String,
    pub sample_rate: u32,
    pub channels: u16,
    pub bits_per_sample: u16,
}

impl Default for DatasetRecordingFormat {
    fn default() -> Self {
        Self {
            container: "wav".to_owned(),
            sample_format: "pcm".to_owned(),
            sample_rate: 48_000,
            channels: 1,
            bits_per_sample: 24,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VoiceDatasetManifestV1 {
    pub schema_version: u32,
    pub profile: VoiceProfileMetadata,
    pub consent: ConsentMetadata,
    pub recording_format: DatasetRecordingFormat,
    pub prompt_pack: PromptPackReference,
    pub takes: Vec<DatasetTake>,
    pub statistics: DatasetStatistics,
    pub created_at: String,
    pub updated_at: String,
}

impl VoiceDatasetManifestV1 {
    pub fn rebuild_statistics(&mut self, total_prompts: usize) {
        self.statistics = rebuild_statistics(&self.takes, total_prompts);
    }

    pub fn validate(&self) -> DatasetResult<()> {
        if self.schema_version != DATASET_MANIFEST_SCHEMA_VERSION {
            return Err(DatasetError::new(
                DatasetErrorCode::FutureManifestSchema,
                format!(
                    "Unsupported dataset schema version {}. Expected version {}.",
                    self.schema_version, DATASET_MANIFEST_SCHEMA_VERSION
                ),
            ));
        }
        if self.recording_format != DatasetRecordingFormat::default() {
            return Err(DatasetError::new(
                DatasetErrorCode::CorruptManifest,
                "The dataset recording format is not canonical PCM24 mono 48 kHz WAV.",
            ));
        }
        let mut ids = HashSet::new();
        for take in &self.takes {
            if !ids.insert(&take.id) {
                return Err(DatasetError::new(
                    DatasetErrorCode::CorruptManifest,
                    "The dataset contains a duplicate take identifier.",
                ));
            }
            validate_take(take)?;
        }
        Ok(())
    }
}

pub fn decode_manifest(contents: &str) -> DatasetResult<VoiceDatasetManifestV1> {
    let value: serde_json::Value = serde_json::from_str(contents).map_err(|_| {
        DatasetError::new(
            DatasetErrorCode::CorruptManifest,
            "The dataset manifest is not valid JSON.",
        )
    })?;
    let version = value
        .get("schemaVersion")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| {
            DatasetError::new(
                DatasetErrorCode::CorruptManifest,
                "The dataset manifest is missing schemaVersion.",
            )
        })?;
    if version != u64::from(DATASET_MANIFEST_SCHEMA_VERSION) {
        return Err(DatasetError::new(
            DatasetErrorCode::FutureManifestSchema,
            format!("Unsupported dataset schema version {version}. Expected version {DATASET_MANIFEST_SCHEMA_VERSION}."),
        ));
    }
    let manifest: VoiceDatasetManifestV1 = serde_json::from_value(value).map_err(|_| {
        DatasetError::new(
            DatasetErrorCode::CorruptManifest,
            "The dataset manifest shape is invalid.",
        )
    })?;
    manifest.validate()?;
    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use super::decode_manifest;

    #[test]
    fn rejects_future_and_corrupt_manifests_without_migration() {
        assert!(decode_manifest(r#"{"schemaVersion":99}"#)
            .unwrap_err()
            .message
            .contains("99"));
        assert!(decode_manifest("{not-json").is_err());
    }
}
