use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::voice_dataset::hash::sha256_file;

use super::{
    error::{VoiceModelError, VoiceModelErrorCode, VoiceModelResult},
    evaluation::ModelEvaluationSummary,
    qualification::{CheckpointFingerprint, ModelEnvironmentFingerprintV1, QualificationLevel},
    state::TrainingConfiguration,
    storage::{ensure_relative_path, managed_join},
};

pub const MODEL_ARTIFACT_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ModelApprovalStatus {
    Unevaluated,
    EvaluationInProgress,
    ApprovedForOfflineUse,
    Rejected,
    DisabledByConsent,
    Invalid,
    MissingFiles,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ArtifactFileRole {
    ModelWeights,
    ModelConfiguration,
    Auxiliary,
    #[default]
    Unknown,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LicensingStatus {
    VerifiedRedistributable,
    Restricted,
    #[default]
    Unknown,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ArtifactHealth {
    #[default]
    Healthy,
    Unqualified,
    IncompatibleEnvironment,
    MissingFiles,
    UnexpectedFiles,
    HashMismatch,
    DisabledByConsent,
    UnsupportedBackend,
    UnsupportedSchema,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PortabilityStatus {
    #[default]
    LocalOnly,
    PortableWithExternalDependencies,
    Portable,
    Incompatible,
    Unknown,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LicenseNoticeReference {
    pub role: String,
    pub label: String,
    pub status: LicensingStatus,
    pub notice: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModelArtifactFile {
    pub relative_path: String,
    pub content_hash: String,
    pub size_bytes: u64,
    #[serde(default)]
    pub role: ArtifactFileRole,
    #[serde(default)]
    pub licensing_status: LicensingStatus,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TrainingSummary {
    pub completed_steps: u32,
    pub final_training_loss: Option<f64>,
    pub final_validation_loss: Option<f64>,
    pub checkpoint_count: u32,
    pub duration_ms: u64,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VoiceModelArtifactV1 {
    pub schema_version: u32,
    pub artifact_id: String,
    pub profile_id: String,
    pub display_name: String,
    pub backend_id: String,
    pub backend_version: String,
    pub worker_protocol_version: u32,
    #[serde(default)]
    pub compatibility_profile_id: String,
    #[serde(default)]
    pub environment_fingerprint: Option<ModelEnvironmentFingerprintV1>,
    #[serde(default)]
    pub checkpoint_identities: Vec<CheckpointFingerprint>,
    #[serde(default)]
    pub backend_revision: Option<String>,
    #[serde(default)]
    pub adapter_version: String,
    pub snapshot_id: String,
    pub snapshot_hash: String,
    pub consent_version: String,
    pub consent_confirmed_at: String,
    pub training_configuration: TrainingConfiguration,
    pub training_summary: TrainingSummary,
    pub model_files: Vec<ModelArtifactFile>,
    pub model_content_hash: String,
    #[serde(default = "default_inference_sample_rate")]
    pub expected_inference_sample_rate: u32,
    #[serde(default)]
    pub supported_inference_controls: Vec<String>,
    #[serde(default)]
    pub portability_status: PortabilityStatus,
    #[serde(default)]
    pub qualification_level: QualificationLevel,
    #[serde(default)]
    pub license_notices: Vec<LicenseNoticeReference>,
    #[serde(default = "default_synthetic_notice")]
    pub synthetic_use_notice_version: String,
    #[serde(default)]
    pub health: ArtifactHealth,
    #[serde(default)]
    pub imported_package_id: Option<String>,
    pub evaluation: Option<ModelEvaluationSummary>,
    pub approval_status: ModelApprovalStatus,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

fn default_inference_sample_rate() -> u32 {
    48_000
}

fn default_synthetic_notice() -> String {
    "mam-synthetic-use-v1".to_owned()
}

pub fn verify_artifact(
    artifact: &VoiceModelArtifactV1,
    artifact_directory: &Path,
) -> VoiceModelResult<()> {
    if artifact.schema_version != MODEL_ARTIFACT_SCHEMA_VERSION {
        return Err(VoiceModelError::new(
            VoiceModelErrorCode::ArtifactSchemaUnsupported,
            "This model artifact schema is not supported.",
        )
        .artifact(&artifact.artifact_id));
    }
    if artifact.model_files.is_empty() {
        return Err(VoiceModelError::new(
            VoiceModelErrorCode::ArtifactMissing,
            "The model artifact contains no model files.",
        )
        .artifact(&artifact.artifact_id));
    }
    let mut combined = String::new();
    for file in &artifact.model_files {
        ensure_relative_path(&file.relative_path)?;
        let path = managed_join(artifact_directory, &file.relative_path)?;
        if !path.is_file() {
            return Err(VoiceModelError::new(
                VoiceModelErrorCode::ArtifactMissing,
                "A required model artifact file is missing.",
            )
            .artifact(&artifact.artifact_id));
        }
        let metadata = std::fs::metadata(&path)
            .map_err(|error| VoiceModelError::storage("Cannot inspect model artifact", error))?;
        let hash = sha256_file(&path)
            .map_err(|error| VoiceModelError::storage("Cannot hash model artifact", error))?;
        if metadata.len() != file.size_bytes || hash != file.content_hash {
            return Err(VoiceModelError::new(
                VoiceModelErrorCode::ArtifactHashMismatch,
                "A model artifact file failed integrity validation.",
            )
            .artifact(&artifact.artifact_id));
        }
        combined.push_str(&file.relative_path);
        combined.push_str(&hash);
    }
    let combined_hash = crate::voice_dataset::hash::sha256_bytes(combined.as_bytes());
    if combined_hash != artifact.model_content_hash {
        return Err(VoiceModelError::new(
            VoiceModelErrorCode::ArtifactHashMismatch,
            "The combined model artifact hash is invalid.",
        )
        .artifact(&artifact.artifact_id));
    }
    Ok(())
}

pub fn require_approved(artifact: &VoiceModelArtifactV1) -> VoiceModelResult<()> {
    match artifact.approval_status {
        ModelApprovalStatus::ApprovedForOfflineUse => Ok(()),
        ModelApprovalStatus::DisabledByConsent => Err(VoiceModelError::new(
            VoiceModelErrorCode::ArtifactDisabledByConsent,
            "This managed model is disabled because consent is inactive.",
        )),
        _ => Err(VoiceModelError::new(
            VoiceModelErrorCode::ModelNotApproved,
            "Complete manual evaluation and approve the model for local offline conversion.",
        )),
    }
}

pub fn validate_display_name(name: &str) -> VoiceModelResult<String> {
    let trimmed = name.trim();
    if !(1..=80).contains(&trimmed.chars().count()) || trimmed.chars().any(char::is_control) {
        return Err(VoiceModelError::new(
            VoiceModelErrorCode::InvalidStateTransition,
            "Model names must contain 1 to 80 visible characters.",
        ));
    }
    Ok(trimmed.to_owned())
}

#[cfg(test)]
mod tests {
    use super::{require_approved, ModelApprovalStatus};

    #[test]
    fn only_approved_artifacts_are_selectable() {
        let statuses = [
            ModelApprovalStatus::Unevaluated,
            ModelApprovalStatus::Rejected,
            ModelApprovalStatus::DisabledByConsent,
            ModelApprovalStatus::Invalid,
        ];
        for status in statuses {
            let artifact = super::VoiceModelArtifactV1 {
                schema_version: 1,
                artifact_id: "artifact-1".to_owned(),
                profile_id: "profile-1".to_owned(),
                display_name: "Test".to_owned(),
                backend_id: "mock".to_owned(),
                backend_version: "1".to_owned(),
                worker_protocol_version: 1,
                compatibility_profile_id: String::new(),
                environment_fingerprint: None,
                checkpoint_identities: Vec::new(),
                backend_revision: None,
                adapter_version: String::new(),
                snapshot_id: "snapshot-1".to_owned(),
                snapshot_hash: "hash".to_owned(),
                consent_version: "consent".to_owned(),
                consent_confirmed_at: "1".to_owned(),
                training_configuration:
                    crate::voice_model::state::TrainingConfiguration::for_preset(
                        crate::voice_model::state::TrainingPreset::QuickExperiment,
                    ),
                training_summary: Default::default(),
                model_files: Vec::new(),
                model_content_hash: String::new(),
                expected_inference_sample_rate: 48_000,
                supported_inference_controls: Vec::new(),
                portability_status: super::PortabilityStatus::Unknown,
                qualification_level: crate::voice_model::qualification::QualificationLevel::None,
                license_notices: Vec::new(),
                synthetic_use_notice_version: "mam-synthetic-use-v1".to_owned(),
                health: super::ArtifactHealth::Unqualified,
                imported_package_id: None,
                evaluation: None,
                approval_status: status,
                notes: None,
                created_at: "1".to_owned(),
                updated_at: "1".to_owned(),
            };
            assert!(require_approved(&artifact).is_err());
        }
    }
}
