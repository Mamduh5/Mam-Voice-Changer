use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::qualification::{QualificationRunV1, TrainingPreflightReport};
use super::{artifact::VoiceModelArtifactV1, error::VoiceModelError, snapshot::TrainingSnapshotV1};

pub const MODEL_BACKEND_SETTINGS_SCHEMA_VERSION: u32 = 1;
pub const WORKER_PROTOCOL_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ModelDevice {
    Cpu,
    Cuda,
    DirectMl,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ModelPrecision {
    Float32,
    Float16,
    Bfloat16,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SeedVcBackendConfiguration {
    #[serde(default = "default_compatibility_profile_id")]
    pub compatibility_profile_id: String,
    pub python_executable: String,
    pub worker_package_directory: String,
    pub seed_vc_directory: String,
    pub model_configuration_path: String,
    #[serde(default)]
    pub model_configuration_expected_sha256: Option<String>,
    pub pretrained_checkpoint_paths: Vec<String>,
    #[serde(default)]
    pub pretrained_checkpoint_expected_sha256: Vec<String>,
    pub output_directory: String,
    pub device: ModelDevice,
    pub precision: ModelPrecision,
}

fn default_compatibility_profile_id() -> String {
    super::compatibility::SEED_VC_EXPERIMENTAL_PROFILE_ID.to_owned()
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModelBackendSettingsV1 {
    pub schema_version: u32,
    pub seed_vc: Option<SeedVcBackendConfiguration>,
}

impl Default for ModelBackendSettingsV1 {
    fn default() -> Self {
        Self {
            schema_version: MODEL_BACKEND_SETTINGS_SCHEMA_VERSION,
            seed_vc: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum BackendReadiness {
    NotConfigured,
    PythonMissing,
    WorkerMissing,
    BackendMissing,
    CheckpointMissing,
    ConfigurationInvalid,
    ProtocolMismatch,
    UnsupportedHardware,
    Ready,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackendDescriptor {
    pub backend_id: String,
    pub display_name: String,
    pub optional: bool,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BackendResourceReport {
    pub system_memory_bytes: Option<u64>,
    pub gpu_memory_bytes: Option<u64>,
    pub available_disk_bytes: Option<u64>,
    pub snapshot_size_bytes: Option<u64>,
    pub checkpoint_size_bytes: Option<u64>,
    pub risk_level: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BackendCapabilityReport {
    pub backend_id: String,
    pub backend_version: String,
    pub worker_version: String,
    pub protocol_version: u32,
    pub devices: Vec<ModelDevice>,
    pub precisions: Vec<ModelPrecision>,
    pub supports_resume: bool,
    pub supports_multiple_references: bool,
    pub resources: BackendResourceReport,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackendValidationStatus {
    pub readiness: BackendReadiness,
    pub message: String,
    pub capability_report: Option<BackendCapabilityReport>,
}

impl Default for BackendValidationStatus {
    fn default() -> Self {
        Self {
            readiness: BackendReadiness::NotConfigured,
            message: "Configure the optional local model backend.".to_owned(),
            capability_report: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TrainingPreset {
    QuickExperiment,
    BalancedFineTune,
    ExtendedFineTune,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ResumeBehavior {
    Never,
    FromLatestCheckpoint,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TrainingConfiguration {
    pub preset: TrainingPreset,
    pub maximum_steps: u32,
    pub save_interval: u32,
    pub batch_size: u16,
    pub worker_count: u16,
    pub device: ModelDevice,
    pub precision: ModelPrecision,
    pub resume_behavior: ResumeBehavior,
    pub random_seed: u64,
}

impl TrainingConfiguration {
    pub fn for_preset(preset: TrainingPreset) -> Self {
        match preset {
            TrainingPreset::QuickExperiment => Self {
                preset,
                maximum_steps: 100,
                save_interval: 50,
                batch_size: 1,
                worker_count: 0,
                device: ModelDevice::Cpu,
                precision: ModelPrecision::Float32,
                resume_behavior: ResumeBehavior::Never,
                random_seed: 13,
            },
            TrainingPreset::BalancedFineTune => Self {
                preset,
                maximum_steps: 1_000,
                save_interval: 250,
                batch_size: 2,
                worker_count: 0,
                device: ModelDevice::Cuda,
                precision: ModelPrecision::Float16,
                resume_behavior: ResumeBehavior::FromLatestCheckpoint,
                random_seed: 13,
            },
            TrainingPreset::ExtendedFineTune => Self {
                preset,
                maximum_steps: 3_000,
                save_interval: 500,
                batch_size: 2,
                worker_count: 0,
                device: ModelDevice::Cuda,
                precision: ModelPrecision::Float16,
                resume_behavior: ResumeBehavior::FromLatestCheckpoint,
                random_seed: 13,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TrainingJobState {
    Idle,
    Validating,
    Snapshotting,
    Preparing,
    Preprocessing,
    Training,
    SavingCheckpoint,
    EvaluatingCheckpoint,
    Cancelling,
    Cancelled,
    Completed,
    Failed,
    Interrupted,
    NeedsRecovery,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TrainingMetrics {
    pub training_loss: Option<f64>,
    pub validation_loss: Option<f64>,
    pub learning_rate: Option<f64>,
    pub backend_reported: bool,
    pub additional: BTreeMap<String, f64>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TrainingJob {
    pub schema_version: u32,
    pub job_id: String,
    pub backend_id: String,
    pub backend_version: String,
    pub worker_protocol_version: u32,
    #[serde(default)]
    pub compatibility_profile_id: String,
    #[serde(default)]
    pub environment_fingerprint: Option<super::qualification::ModelEnvironmentFingerprintV1>,
    #[serde(default)]
    pub checkpoint_identities: Vec<super::qualification::CheckpointFingerprint>,
    #[serde(default)]
    pub backend_revision: Option<String>,
    #[serde(default)]
    pub adapter_version: String,
    #[serde(default)]
    pub qualification_level: super::qualification::QualificationLevel,
    pub snapshot_id: String,
    pub snapshot_hash: String,
    pub profile_id: String,
    pub consent_version: String,
    pub configuration: TrainingConfiguration,
    pub state: TrainingJobState,
    pub overall_progress: f32,
    pub current_step: u32,
    pub maximum_steps: u32,
    pub latest_metrics: TrainingMetrics,
    pub started_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
    pub worker_pid: Option<u32>,
    pub last_checkpoint: Option<String>,
    #[serde(default)]
    pub last_checkpoint_hash: Option<String>,
    pub log_file: String,
    pub error_summary: Option<String>,
    pub cancellation_requested: bool,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InferenceConfiguration {
    pub diffusion_steps: u16,
    pub f0_conditioning: bool,
    pub pitch_adjustment_semitones: i16,
    pub length_adjustment: f32,
    pub device: ModelDevice,
    pub precision: ModelPrecision,
    pub reference_take_ids: Vec<String>,
}

impl Default for InferenceConfiguration {
    fn default() -> Self {
        Self {
            diffusion_steps: 25,
            f0_conditioning: false,
            pitch_adjustment_semitones: 0,
            length_adjustment: 1.0,
            device: ModelDevice::Cpu,
            precision: ModelPrecision::Float32,
            reference_take_ids: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OfflineConversionResult {
    pub result_id: String,
    pub artifact_id: String,
    pub artifact_display_name: String,
    pub profile_id: String,
    pub target_profile_display_name: String,
    pub source_clip_id: String,
    pub reference_take_ids: Vec<String>,
    pub reference_hashes: Vec<String>,
    pub backend_id: String,
    pub backend_version: String,
    pub synthetic: bool,
    pub output_file: String,
    pub duration_ms: u64,
    pub peak: f32,
    pub clipping: bool,
    pub waveform: Vec<f32>,
    pub created_at: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceModelStatus {
    pub backend: BackendValidationStatus,
    pub active_training_job: Option<TrainingJob>,
    pub active_inference: bool,
    pub latest_conversion: Option<OfflineConversionResult>,
    pub selected_artifact_id: Option<String>,
    pub last_error: Option<VoiceModelError>,
    pub logs: Vec<String>,
    pub snapshots: Vec<TrainingSnapshotV1>,
    pub artifacts: Vec<VoiceModelArtifactV1>,
    pub qualification: Option<QualificationRunV1>,
    pub qualification_active: bool,
    pub training_preflight: Option<TrainingPreflightReport>,
}
