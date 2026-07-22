use std::{
    fs,
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::voice_dataset::{
    hash::{sha256_bytes, sha256_file},
    storage::{new_id, timestamp},
};

use super::{
    compatibility::{
        BackendCompatibilityProfileV1, QualificationSupportStatus,
        COMPATIBILITY_PROFILE_SCHEMA_VERSION,
    },
    error::{VoiceModelError, VoiceModelErrorCode, VoiceModelResult},
    repository::{inspect_repository, BackendRepositoryInspection, CheckoutCleanliness},
    snapshot::TrainingSnapshotV1,
    state::{
        ModelBackendSettingsV1, ModelDevice, ModelPrecision, SeedVcBackendConfiguration,
        TrainingConfiguration,
    },
    storage::atomic_write_json,
    worker_process::run_worker_job,
    worker_protocol::{WorkerCommand, WorkerEventKind, WorkerRequest},
};

pub const QUALIFICATION_SCHEMA_VERSION: u32 = 1;
pub const ENVIRONMENT_FINGERPRINT_SCHEMA_VERSION: u32 = 1;
pub const QUALIFICATION_REPORT_SCHEMA_VERSION: u32 = 1;
pub const APPLICATION_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FileValidationState {
    Missing,
    Unreadable,
    Hashing,
    HashKnown,
    HashMismatch,
    IdentityUnspecified,
    Valid,
    Unsupported,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FileFingerprint {
    pub role: String,
    pub display_path: String,
    pub size_bytes: u64,
    pub content_hash: Option<String>,
    pub hash_algorithm: String,
    pub expected_hash: Option<String>,
    pub validation_state: FileValidationState,
    pub checked_at: String,
}

pub type CheckpointFingerprint = FileFingerprint;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PythonFingerprint {
    pub implementation: String,
    pub version: String,
    pub executable_label: String,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkerFingerprint {
    pub worker_version: String,
    pub adapter_version: String,
    pub protocol_version: u32,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BackendFingerprint {
    pub backend_id: String,
    pub compatibility_profile_id: String,
    pub repository_remote: Option<String>,
    pub commit_sha: Option<String>,
    pub checkout_cleanliness: Option<CheckoutCleanliness>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PythonPackageFingerprint {
    pub package: String,
    pub version: Option<String>,
    pub required: bool,
    pub compatible: Option<bool>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AcceleratorFingerprint {
    pub cuda_available: bool,
    pub cuda_runtime_version: Option<String>,
    pub gpu_name: Option<String>,
    pub gpu_count: u32,
    pub total_vram_bytes: Option<u64>,
    pub available_vram_bytes: Option<u64>,
    pub selected_device: Option<ModelDevice>,
    pub selected_precision: Option<ModelPrecision>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModelEnvironmentFingerprintV1 {
    pub schema_version: u32,
    pub fingerprint_id: String,
    pub generated_at: String,
    pub operating_system: String,
    pub architecture: String,
    pub python: PythonFingerprint,
    pub worker: WorkerFingerprint,
    pub backend: BackendFingerprint,
    pub packages: Vec<PythonPackageFingerprint>,
    pub accelerator: AcceleratorFingerprint,
    pub checkpoints: Vec<CheckpointFingerprint>,
    pub configuration_files: Vec<FileFingerprint>,
    pub aggregate_hash: String,
}

impl ModelEnvironmentFingerprintV1 {
    pub fn calculate_aggregate_hash(&self) -> VoiceModelResult<String> {
        let mut stable = self.clone();
        stable.fingerprint_id.clear();
        stable.generated_at.clear();
        stable.aggregate_hash.clear();
        let bytes = serde_json::to_vec(&stable).map_err(|_| {
            VoiceModelError::new(
                VoiceModelErrorCode::EnvironmentFingerprintInvalid,
                "Cannot serialize the environment fingerprint.",
            )
        })?;
        Ok(sha256_bytes(&bytes))
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum QualificationState {
    NotStarted,
    CollectingIdentity,
    ValidatingFiles,
    HashingCheckpoints,
    StartingWorker,
    CheckingProtocol,
    InspectingPackages,
    InspectingAccelerator,
    RunningImportSmokeTest,
    RunningAudioSmokeTest,
    RunningInferenceSmokeTest,
    EvaluatingResults,
    Qualified,
    QualifiedWithWarnings,
    Failed,
    Cancelled,
    Interrupted,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum QualificationLevel {
    #[default]
    None,
    ConfigurationValidated,
    BackendLoaded,
    InferenceGenerated,
    ManuallyListened,
    TrainingCompleted,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum QualificationCheckLayer {
    Static,
    Worker,
    Framework,
    BackendImport,
    Audio,
    Inference,
    ManualListening,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum QualificationCheckStatus {
    Passed,
    PassedWithWarning,
    Failed,
    Pending,
    Skipped,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QualificationCheckResult {
    pub code: String,
    pub label: String,
    pub layer: QualificationCheckLayer,
    pub status: QualificationCheckStatus,
    pub message: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ResourceRiskLevel {
    Low,
    Moderate,
    High,
    Unsupported,
    Unknown,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ResourceRiskReason {
    CpuOnlyTraining,
    InsufficientDisk,
    LowSystemMemory,
    LowVram,
    UnavailableVramMeasurement,
    UnsupportedPrecision,
    OversizedBatch,
    ExcessiveWorkers,
    LargeTrainingStepCount,
    TinyDataset,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResourceDiagnostics {
    pub logical_cpu_count: Option<u32>,
    pub total_memory_bytes: Option<u64>,
    pub available_memory_bytes: Option<u64>,
    pub process_memory_bytes: Option<u64>,
    pub free_disk_bytes: Option<u64>,
    pub snapshot_size_bytes: Option<u64>,
    pub checkpoint_size_bytes: Option<u64>,
    pub estimated_temporary_bytes: Option<u64>,
    pub total_vram_bytes: Option<u64>,
    pub available_vram_bytes: Option<u64>,
    pub risk_level: Option<ResourceRiskLevel>,
    pub reasons: Vec<ResourceRiskReason>,
}

pub fn assess_resource_risk(
    mut diagnostics: ResourceDiagnostics,
    device: ModelDevice,
    precision: ModelPrecision,
    batch_size: u16,
    worker_count: u16,
    maximum_steps: u32,
    dataset_duration_ms: u64,
) -> ResourceDiagnostics {
    let mut level = ResourceRiskLevel::Low;
    let mut raise = |candidate: ResourceRiskLevel| {
        let rank = |value| match value {
            ResourceRiskLevel::Low => 0,
            ResourceRiskLevel::Unknown => 1,
            ResourceRiskLevel::Moderate => 2,
            ResourceRiskLevel::High => 3,
            ResourceRiskLevel::Unsupported => 4,
        };
        if rank(candidate) > rank(level) {
            level = candidate;
        }
    };
    if device == ModelDevice::Cpu {
        diagnostics
            .reasons
            .push(ResourceRiskReason::CpuOnlyTraining);
        raise(ResourceRiskLevel::High);
        if precision != ModelPrecision::Float32 {
            diagnostics
                .reasons
                .push(ResourceRiskReason::UnsupportedPrecision);
            raise(ResourceRiskLevel::Unsupported);
        }
    }
    if diagnostics
        .estimated_temporary_bytes
        .zip(diagnostics.free_disk_bytes)
        .is_some_and(|(required, free)| free < required)
    {
        diagnostics
            .reasons
            .push(ResourceRiskReason::InsufficientDisk);
        raise(ResourceRiskLevel::Unsupported);
    }
    if diagnostics
        .available_memory_bytes
        .is_some_and(|value| value < 4 * 1024 * 1024 * 1024)
    {
        diagnostics
            .reasons
            .push(ResourceRiskReason::LowSystemMemory);
        raise(ResourceRiskLevel::High);
    }
    if device == ModelDevice::Cuda {
        match diagnostics.available_vram_bytes {
            Some(value) if value < 4 * 1024 * 1024 * 1024 => {
                diagnostics.reasons.push(ResourceRiskReason::LowVram);
                raise(ResourceRiskLevel::High);
            }
            None => {
                diagnostics
                    .reasons
                    .push(ResourceRiskReason::UnavailableVramMeasurement);
                raise(ResourceRiskLevel::Unknown);
            }
            _ => {}
        }
    }
    if batch_size > 8 {
        diagnostics.reasons.push(ResourceRiskReason::OversizedBatch);
        raise(ResourceRiskLevel::High);
    }
    if worker_count > 4 {
        diagnostics
            .reasons
            .push(ResourceRiskReason::ExcessiveWorkers);
        raise(ResourceRiskLevel::Moderate);
    }
    if maximum_steps > 10_000 {
        diagnostics
            .reasons
            .push(ResourceRiskReason::LargeTrainingStepCount);
        raise(ResourceRiskLevel::Moderate);
    }
    if dataset_duration_ms < 5 * 60_000 {
        diagnostics.reasons.push(ResourceRiskReason::TinyDataset);
        raise(ResourceRiskLevel::Moderate);
    }
    diagnostics
        .reasons
        .sort_by_key(|value| format!("{value:?}"));
    diagnostics.reasons.dedup();
    diagnostics.risk_level = Some(level);
    diagnostics
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TrainingPreflightReport {
    pub schema_version: u32,
    pub snapshot_id: String,
    pub snapshot_take_count: u32,
    pub training_duration_ms: u64,
    pub validation_duration_ms: u64,
    pub snapshot_bytes: u64,
    pub compatibility_profile_status: QualificationSupportStatus,
    pub environment_fingerprint_status: EnvironmentMatch,
    pub device: ModelDevice,
    pub precision: ModelPrecision,
    pub batch_size: u16,
    pub worker_count: u16,
    pub maximum_steps: u32,
    pub checkpoint_interval: u32,
    pub estimated_checkpoint_count: u32,
    pub estimated_disk_minimum_bytes: u64,
    pub estimated_disk_maximum_bytes: u64,
    pub resource_warnings: Vec<ResourceRiskReason>,
    pub consent_active: bool,
    pub qualification_level: QualificationLevel,
    pub fatal_failures: Vec<String>,
    pub acknowledgements_required: Vec<String>,
    pub can_start: bool,
}

pub fn build_training_preflight(
    snapshot: &TrainingSnapshotV1,
    snapshot_bytes: u64,
    configuration: &TrainingConfiguration,
    profile: &BackendCompatibilityProfileV1,
    qualification: Option<&QualificationRunV1>,
    consent_active: bool,
) -> TrainingPreflightReport {
    let estimated_checkpoint_count = configuration
        .maximum_steps
        .div_ceil(configuration.save_interval.max(1));
    let checkpoint_bytes = qualification
        .and_then(|run| run.resources.as_ref())
        .and_then(|resources| resources.checkpoint_size_bytes)
        .unwrap_or(512 * 1024 * 1024);
    let estimated_disk_minimum_bytes = snapshot_bytes
        .saturating_add(checkpoint_bytes.saturating_mul(u64::from(estimated_checkpoint_count)))
        .saturating_add(256 * 1024 * 1024);
    let estimated_disk_maximum_bytes = estimated_disk_minimum_bytes
        .saturating_mul(2)
        .saturating_add(checkpoint_bytes);
    let mut fatal_failures = Vec::new();
    let mut acknowledgements_required = Vec::new();
    let mut resource_warnings = Vec::new();
    let level = qualification.map_or(QualificationLevel::None, |run| run.final_level);
    if !consent_active {
        fatal_failures.push("Active target-speaker consent is required.".to_owned());
    }
    if qualification.is_none()
        || qualification.is_some_and(|run| {
            !matches!(
                run.state,
                QualificationState::Qualified | QualificationState::QualifiedWithWarnings
            )
        })
    {
        fatal_failures.push("A completed backend qualification is required.".to_owned());
    }
    if level < QualificationLevel::BackendLoaded {
        fatal_failures.push("Backend-load qualification is required before training.".to_owned());
    }
    if profile.support_status == QualificationSupportStatus::Experimental {
        acknowledgements_required
            .push("The selected compatibility profile is experimental.".to_owned());
    }
    if configuration.device == ModelDevice::Cpu {
        resource_warnings.push(ResourceRiskReason::CpuOnlyTraining);
        acknowledgements_required.push("CPU-only training may be extremely slow.".to_owned());
    }
    if snapshot.total_duration_ms < 5 * 60_000 {
        resource_warnings.push(ResourceRiskReason::TinyDataset);
        acknowledgements_required.push("The immutable Dataset snapshot is very small.".to_owned());
    }
    if configuration.batch_size > 8 {
        resource_warnings.push(ResourceRiskReason::OversizedBatch);
    }
    if configuration.worker_count > 4 {
        resource_warnings.push(ResourceRiskReason::ExcessiveWorkers);
    }
    if configuration.maximum_steps > 10_000 {
        resource_warnings.push(ResourceRiskReason::LargeTrainingStepCount);
    }
    if let Some(run) = qualification {
        if !run.warnings.is_empty() {
            acknowledgements_required.push("The qualification completed with warnings.".to_owned());
        }
        if run
            .repository
            .as_ref()
            .is_some_and(|repository| repository.cleanliness == CheckoutCleanliness::Dirty)
        {
            acknowledgements_required.push("The backend checkout is dirty.".to_owned());
        }
        if run
            .environment_fingerprint
            .as_ref()
            .is_some_and(|fingerprint| {
                fingerprint.checkpoints.iter().any(|checkpoint| {
                    checkpoint.validation_state == FileValidationState::IdentityUnspecified
                })
            })
        {
            acknowledgements_required
                .push("One or more checkpoint expected hashes are unspecified.".to_owned());
        }
        if let Some(resources) = &run.resources {
            let mut preflight_resources = resources.clone();
            preflight_resources.snapshot_size_bytes = Some(snapshot_bytes);
            preflight_resources.estimated_temporary_bytes = Some(estimated_disk_maximum_bytes);
            let assessed = assess_resource_risk(
                preflight_resources,
                configuration.device,
                configuration.precision,
                configuration.batch_size,
                configuration.worker_count,
                configuration.maximum_steps,
                snapshot.total_duration_ms,
            );
            resource_warnings.extend(assessed.reasons.iter().copied());
            if matches!(assessed.risk_level, Some(ResourceRiskLevel::Unsupported)) {
                fatal_failures.push(
                    "The selected training configuration is unsupported by measured resources."
                        .to_owned(),
                );
            } else if matches!(
                assessed.risk_level,
                Some(ResourceRiskLevel::Moderate | ResourceRiskLevel::High)
            ) {
                acknowledgements_required
                    .push("Measured resources indicate elevated training risk.".to_owned());
            }
            if resources
                .free_disk_bytes
                .is_some_and(|free| free < estimated_disk_maximum_bytes)
            {
                acknowledgements_required
                    .push("Available disk is close to the training estimate.".to_owned());
                if resources
                    .free_disk_bytes
                    .is_some_and(|free| free < estimated_disk_minimum_bytes)
                {
                    fatal_failures
                        .push("Available disk is below the minimum training estimate.".to_owned());
                    resource_warnings.push(ResourceRiskReason::InsufficientDisk);
                }
            }
        }
    }
    resource_warnings.sort_by_key(|value| format!("{value:?}"));
    resource_warnings.dedup();
    TrainingPreflightReport {
        schema_version: 1,
        snapshot_id: snapshot.snapshot_id.clone(),
        snapshot_take_count: snapshot.takes.len().min(u32::MAX as usize) as u32,
        training_duration_ms: snapshot.split.training_duration_ms,
        validation_duration_ms: snapshot.split.validation_duration_ms,
        snapshot_bytes,
        compatibility_profile_status: profile.support_status,
        environment_fingerprint_status: if qualification
            .and_then(|run| run.environment_fingerprint.as_ref())
            .is_some()
        {
            EnvironmentMatch::Identical
        } else {
            EnvironmentMatch::Unknown
        },
        device: configuration.device,
        precision: configuration.precision,
        batch_size: configuration.batch_size,
        worker_count: configuration.worker_count,
        maximum_steps: configuration.maximum_steps,
        checkpoint_interval: configuration.save_interval,
        estimated_checkpoint_count,
        estimated_disk_minimum_bytes,
        estimated_disk_maximum_bytes,
        resource_warnings,
        consent_active,
        qualification_level: level,
        can_start: fatal_failures.is_empty(),
        fatal_failures,
        acknowledgements_required,
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ManualListeningQualification {
    pub synthetic_output_played: bool,
    pub speech_intelligible: bool,
    pub no_severe_clipping: bool,
    pub no_severe_truncation: bool,
    pub no_source_target_mix_up: bool,
    pub synthetic_label_reviewed: bool,
    pub notes: Option<String>,
    pub confirmed_at: Option<String>,
}

impl ManualListeningQualification {
    pub fn complete(&self) -> bool {
        self.synthetic_output_played
            && self.speech_intelligible
            && self.no_severe_clipping
            && self.no_severe_truncation
            && self.no_source_target_mix_up
            && self.synthetic_label_reviewed
            && self.confirmed_at.is_some()
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QualificationInferenceSmokeResult {
    pub synthetic: bool,
    pub output_file: String,
    pub duration_ms: u64,
    pub peak: f32,
    pub clipping: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QualificationRunV1 {
    pub schema_version: u32,
    pub qualification_id: String,
    pub compatibility_profile_id: String,
    pub compatibility_profile_status: QualificationSupportStatus,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub state: QualificationState,
    pub completed_checks: Vec<QualificationCheckResult>,
    pub warnings: Vec<String>,
    pub failures: Vec<String>,
    pub environment_fingerprint: Option<ModelEnvironmentFingerprintV1>,
    pub repository: Option<BackendRepositoryInspection>,
    pub resources: Option<ResourceDiagnostics>,
    pub final_level: QualificationLevel,
    pub manual_listening: ManualListeningQualification,
    pub inference_smoke_result: Option<QualificationInferenceSmokeResult>,
    pub application_version: String,
    pub adapter_version: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QualificationReportV1 {
    pub schema_version: u32,
    pub run: QualificationRunV1,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WorkerQualificationResult {
    python: PythonFingerprint,
    worker: WorkerFingerprint,
    packages: Vec<PythonPackageFingerprint>,
    accelerator: AcceleratorFingerprint,
    resources: ResourceDiagnostics,
    checks: Vec<QualificationCheckResult>,
}

pub fn can_transition(from: QualificationState, to: QualificationState) -> bool {
    use QualificationState::*;
    matches!(
        (from, to),
        (NotStarted, CollectingIdentity | Cancelled)
            | (
                CollectingIdentity,
                ValidatingFiles | Failed | Cancelled | Interrupted
            )
            | (
                ValidatingFiles,
                HashingCheckpoints | Failed | Cancelled | Interrupted
            )
            | (
                HashingCheckpoints,
                StartingWorker | Failed | Cancelled | Interrupted
            )
            | (
                StartingWorker,
                CheckingProtocol | Failed | Cancelled | Interrupted
            )
            | (
                CheckingProtocol,
                InspectingPackages | Failed | Cancelled | Interrupted
            )
            | (
                InspectingPackages,
                InspectingAccelerator | Failed | Cancelled | Interrupted
            )
            | (
                InspectingAccelerator,
                RunningImportSmokeTest | Failed | Cancelled | Interrupted
            )
            | (
                RunningImportSmokeTest,
                RunningAudioSmokeTest | Failed | Cancelled | Interrupted
            )
            | (
                RunningAudioSmokeTest,
                RunningInferenceSmokeTest | EvaluatingResults | Failed | Cancelled | Interrupted
            )
            | (
                RunningInferenceSmokeTest,
                EvaluatingResults | Failed | Cancelled | Interrupted
            )
            | (
                EvaluatingResults,
                Qualified | QualifiedWithWarnings | Failed | Cancelled | Interrupted
            )
    )
}

pub fn run_qualification(
    models_root: &Path,
    settings: &ModelBackendSettingsV1,
    profile: &BackendCompatibilityProfileV1,
    inference_reference_path: Option<&Path>,
    cancellation: Arc<AtomicBool>,
    mut on_update: impl FnMut(&QualificationRunV1),
) -> VoiceModelResult<QualificationRunV1> {
    profile.validate().map_err(|message| {
        VoiceModelError::new(VoiceModelErrorCode::CompatibilityProfileInvalid, message)
    })?;
    if profile.schema_version != COMPATIBILITY_PROFILE_SCHEMA_VERSION {
        return Err(VoiceModelError::new(
            VoiceModelErrorCode::CompatibilityProfileInvalid,
            "The compatibility profile schema is unsupported.",
        ));
    }
    let configuration = settings.seed_vc.as_ref().ok_or_else(|| {
        VoiceModelError::new(
            VoiceModelErrorCode::BackendNotConfigured,
            "Configure the local model backend before qualification.",
        )
    })?;
    let now = timestamp().map_err(clock_error)?;
    let qualification_id = new_id("qualification", &now);
    let mut run = QualificationRunV1 {
        schema_version: QUALIFICATION_SCHEMA_VERSION,
        qualification_id: qualification_id.clone(),
        compatibility_profile_id: profile.profile_id.clone(),
        compatibility_profile_status: profile.support_status,
        started_at: now,
        ended_at: None,
        state: QualificationState::NotStarted,
        completed_checks: Vec::new(),
        warnings: profile.notices.clone(),
        failures: Vec::new(),
        environment_fingerprint: None,
        repository: None,
        resources: None,
        final_level: QualificationLevel::None,
        manual_listening: ManualListeningQualification::default(),
        inference_smoke_result: None,
        application_version: APPLICATION_VERSION.to_owned(),
        adapter_version: profile.worker_adapter_version.clone(),
    };
    let run_directory = models_root.join("qualifications").join(&qualification_id);
    fs::create_dir_all(&run_directory)
        .map_err(|error| VoiceModelError::storage("Cannot create qualification storage", error))?;

    advance(
        &mut run,
        QualificationState::CollectingIdentity,
        &mut on_update,
    )?;
    check_cancel(&mut run, &cancellation, &run_directory, &mut on_update)?;
    let repository = inspect_repository(Path::new(&configuration.seed_vc_directory));
    if repository.commit_sha.is_none() {
        run.warnings
            .push("The backend revision is unknown; reproducibility cannot be claimed.".to_owned());
    }
    if repository.cleanliness == CheckoutCleanliness::Dirty {
        run.warnings
            .push("The backend checkout is dirty.".to_owned());
    }
    if repository
        .commit_sha
        .as_deref()
        .is_none_or(|commit| !profile.supports_commit(commit))
    {
        run.warnings.push(
            "The configured revision is not pinned by this experimental compatibility profile."
                .to_owned(),
        );
    }
    run.repository = Some(repository.clone());

    advance(
        &mut run,
        QualificationState::ValidatingFiles,
        &mut on_update,
    )?;
    let static_failures = validate_static_files(configuration, profile);
    for failure in static_failures {
        run.failures.push(failure.clone());
        run.completed_checks.push(check(
            "staticFiles",
            "Required local files",
            QualificationCheckLayer::Static,
            QualificationCheckStatus::Failed,
            failure,
        ));
    }
    if !run.failures.is_empty() {
        return finish_failed(run, &run_directory, &mut on_update);
    }
    run.completed_checks.push(check(
        "staticFiles",
        "Required local files",
        QualificationCheckLayer::Static,
        QualificationCheckStatus::Passed,
        "Configured paths and profile-declared files exist.",
    ));

    advance(
        &mut run,
        QualificationState::HashingCheckpoints,
        &mut on_update,
    )?;
    let configuration_files = vec![fingerprint_file(
        "modelConfiguration",
        Path::new(&configuration.model_configuration_path),
        configuration.model_configuration_expected_sha256.as_deref(),
    )?];
    let mut checkpoints = Vec::new();
    for (index, path) in configuration.pretrained_checkpoint_paths.iter().enumerate() {
        check_cancel(&mut run, &cancellation, &run_directory, &mut on_update)?;
        let expected = configuration
            .pretrained_checkpoint_expected_sha256
            .get(index)
            .map(String::as_str)
            .filter(|value| !value.is_empty());
        let role = profile
            .checkpoint_roles
            .get(index)
            .map(|role| role.role.as_str())
            .unwrap_or("auxiliaryCheckpoint");
        let fingerprint = fingerprint_file(role, Path::new(path), expected)?;
        if fingerprint.validation_state == FileValidationState::IdentityUnspecified {
            run.warnings.push(format!(
                "Expected SHA-256 is unspecified for checkpoint role {role}."
            ));
        }
        if fingerprint.validation_state == FileValidationState::HashMismatch {
            run.failures.push(format!(
                "Checkpoint role {role} does not match its expected SHA-256."
            ));
        }
        checkpoints.push(fingerprint);
    }
    if !run.failures.is_empty() {
        return finish_failed(run, &run_directory, &mut on_update);
    }

    advance(&mut run, QualificationState::StartingWorker, &mut on_update)?;
    let inference_smoke = if let Some(reference_path) = inference_reference_path {
        let source_path = run_directory.join("project-smoke-source.wav");
        write_project_smoke_fixture(&source_path)?;
        Some((
            source_path,
            reference_path.to_path_buf(),
            run_directory.join("synthetic-smoke.wav"),
        ))
    } else {
        None
    };
    let request = WorkerRequest::new(
        format!("qualification-{qualification_id}"),
        WorkerCommand::QualifyBackend,
        json!({
            "backendId": profile.backend_id,
            "compatibilityProfileId": profile.profile_id,
            "adapterVersion": profile.worker_adapter_version,
            "seedVcDirectory": configuration.seed_vc_directory,
            "modelConfigurationPath": configuration.model_configuration_path,
            "pretrainedCheckpointPaths": configuration.pretrained_checkpoint_paths,
            "outputDirectory": configuration.output_directory,
            "requestedDevice": configuration.device,
            "requestedPrecision": configuration.precision,
            "packageRequirements": profile.package_requirements,
            "expectedFiles": profile.expected_files,
            "runInferenceSmokeTest": inference_smoke.is_some(),
            "inferenceSmokeSourcePath": inference_smoke.as_ref().map(|value| &value.0),
            "inferenceSmokeReferencePath": inference_smoke.as_ref().map(|value| &value.1),
            "inferenceSmokeOutputPath": inference_smoke.as_ref().map(|value| &value.2),
        }),
    );
    advance(
        &mut run,
        QualificationState::CheckingProtocol,
        &mut on_update,
    )?;
    let worker = run_worker_job(
        configuration,
        request,
        Arc::clone(&cancellation),
        |_| {},
        |event| {
            let next = match event.event {
                WorkerEventKind::PackageReport => Some(QualificationState::InspectingPackages),
                WorkerEventKind::AcceleratorReport => {
                    Some(QualificationState::InspectingAccelerator)
                }
                WorkerEventKind::BackendImportReport => {
                    Some(QualificationState::RunningImportSmokeTest)
                }
                WorkerEventKind::AudioSmokeReport => {
                    Some(QualificationState::RunningAudioSmokeTest)
                }
                WorkerEventKind::InferenceSmokeReport => {
                    Some(QualificationState::RunningInferenceSmokeTest)
                }
                _ => None,
            };
            if let Some(next) = next {
                if can_transition(run.state, next) {
                    run.state = next;
                    on_update(&run);
                }
            }
        },
    );
    let worker = match worker {
        Ok(result) if result.terminal_event.event == WorkerEventKind::Completed => {
            serde_json::from_value::<WorkerQualificationResult>(result.terminal_event.payload)
                .map_err(|_| {
                    VoiceModelError::new(
                        VoiceModelErrorCode::WorkerMessageMalformed,
                        "The worker qualification report is malformed.",
                    )
                })?
        }
        Ok(result) if result.terminal_event.event == WorkerEventKind::Cancelled => {
            run.state = QualificationState::Cancelled;
            run.ended_at = timestamp().ok();
            persist_report(&run_directory, &run)?;
            on_update(&run);
            return Ok(run);
        }
        Ok(result) => {
            let message = result
                .terminal_event
                .payload
                .get("message")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("Worker qualification failed.")
                .to_owned();
            run.failures.push(message);
            return finish_failed(run, &run_directory, &mut on_update);
        }
        Err(error) => {
            run.failures.push(error.message);
            return finish_failed(run, &run_directory, &mut on_update);
        }
    };
    run.completed_checks.extend(worker.checks);
    run.resources = Some(worker.resources);
    let fingerprint_id = new_id("fingerprint", &timestamp().map_err(clock_error)?);
    let mut fingerprint = ModelEnvironmentFingerprintV1 {
        schema_version: ENVIRONMENT_FINGERPRINT_SCHEMA_VERSION,
        fingerprint_id,
        generated_at: timestamp().map_err(clock_error)?,
        operating_system: std::env::consts::OS.to_owned(),
        architecture: std::env::consts::ARCH.to_owned(),
        python: worker.python,
        worker: worker.worker,
        backend: BackendFingerprint {
            backend_id: profile.backend_id.clone(),
            compatibility_profile_id: profile.profile_id.clone(),
            repository_remote: repository.remote_identity,
            commit_sha: repository.commit_sha,
            checkout_cleanliness: Some(repository.cleanliness),
        },
        packages: worker.packages,
        accelerator: worker.accelerator,
        checkpoints,
        configuration_files,
        aggregate_hash: String::new(),
    };
    fingerprint.aggregate_hash = fingerprint.calculate_aggregate_hash()?;
    run.environment_fingerprint = Some(fingerprint);
    if let Some((source_path, _, output_path)) = inference_smoke {
        let validation = super::inference::validate_generated_wav(&output_path);
        let _ = fs::remove_file(source_path);
        match validation {
            Ok(clip) => {
                let summary = clip.summary();
                run.inference_smoke_result = Some(QualificationInferenceSmokeResult {
                    synthetic: true,
                    output_file: "synthetic-smoke.wav".to_owned(),
                    duration_ms: summary.duration_ms,
                    peak: summary.peak,
                    clipping: summary.peak >= 0.995,
                });
                run.final_level = QualificationLevel::InferenceGenerated;
            }
            Err(error) => run.failures.push(error.message),
        }
    }
    if run.final_level != QualificationLevel::InferenceGenerated {
        run.final_level = if run.completed_checks.iter().any(|item| {
            item.layer == QualificationCheckLayer::BackendImport
                && item.status == QualificationCheckStatus::Passed
        }) {
            QualificationLevel::BackendLoaded
        } else {
            QualificationLevel::ConfigurationValidated
        };
    }
    if can_transition(run.state, QualificationState::EvaluatingResults) {
        advance(
            &mut run,
            QualificationState::EvaluatingResults,
            &mut on_update,
        )?;
    }
    if run
        .completed_checks
        .iter()
        .any(|item| item.status == QualificationCheckStatus::Failed)
    {
        run.failures
            .push("One or more layered qualification checks failed.".to_owned());
        return finish_failed(run, &run_directory, &mut on_update);
    }
    run.state = if run.warnings.is_empty()
        && profile.support_status == QualificationSupportStatus::Qualified
    {
        QualificationState::Qualified
    } else {
        QualificationState::QualifiedWithWarnings
    };
    run.ended_at = timestamp().ok();
    persist_report(&run_directory, &run)?;
    on_update(&run);
    Ok(run)
}

pub fn confirm_manual_listening(
    run_directory: &Path,
    mut run: QualificationRunV1,
    confirmation: ManualListeningQualification,
) -> VoiceModelResult<QualificationRunV1> {
    if !matches!(
        run.state,
        QualificationState::Qualified | QualificationState::QualifiedWithWarnings
    ) || run.final_level < QualificationLevel::InferenceGenerated
    {
        return Err(VoiceModelError::new(
            VoiceModelErrorCode::ManualQualificationIncomplete,
            "Generate a valid inference smoke-test result before manual listening qualification.",
        ));
    }
    if !confirmation.complete() {
        return Err(VoiceModelError::new(
            VoiceModelErrorCode::ManualQualificationIncomplete,
            "Complete every manual listening confirmation field.",
        ));
    }
    run.manual_listening = confirmation;
    run.final_level = QualificationLevel::ManuallyListened;
    persist_report(run_directory, &run)?;
    Ok(run)
}

pub fn report_text(report: &QualificationReportV1) -> String {
    let run = &report.run;
    let mut lines = vec![
        "Mam Voice Changer backend qualification report".to_owned(),
        format!("Report schema: {}", report.schema_version),
        format!("Application version: {}", run.application_version),
        format!("Qualification ID: {}", run.qualification_id),
        format!(
            "Profile: {} ({:?})",
            run.compatibility_profile_id, run.compatibility_profile_status
        ),
        format!("State: {:?}", run.state),
        format!("Qualification level: {:?}", run.final_level),
        format!("Adapter version: {}", run.adapter_version),
    ];
    if let Some(repository) = &run.repository {
        lines.push(format!(
            "Backend revision: {}",
            repository.commit_sha.as_deref().unwrap_or("unknown")
        ));
        lines.push(format!(
            "Checkout cleanliness: {:?}",
            repository.cleanliness
        ));
    }
    if let Some(fingerprint) = &run.environment_fingerprint {
        lines.push(format!(
            "Environment fingerprint: {}",
            fingerprint.aggregate_hash
        ));
        lines.push(format!("Python: {}", fingerprint.python.version));
        lines.push(format!(
            "Device: {:?}",
            fingerprint.accelerator.selected_device
        ));
    }
    lines.push("Checks:".to_owned());
    lines.extend(run.completed_checks.iter().map(|item| {
        format!(
            "- {:?} / {}: {:?} - {}",
            item.layer, item.code, item.status, item.message
        )
    }));
    lines.push("Warnings:".to_owned());
    lines.extend(run.warnings.iter().map(|warning| format!("- {warning}")));
    lines.push("Failures:".to_owned());
    lines.extend(run.failures.iter().map(|failure| format!("- {failure}")));
    lines.push(format!(
        "Manual listening: {}",
        if run.manual_listening.complete() {
            "complete"
        } else {
            "pending"
        }
    ));
    lines.join("\n")
}

fn validate_static_files(
    configuration: &SeedVcBackendConfiguration,
    profile: &BackendCompatibilityProfileV1,
) -> Vec<String> {
    let mut failures = Vec::new();
    for (path, label, directory) in [
        (&configuration.python_executable, "Python executable", false),
        (
            &configuration.worker_package_directory,
            "Worker package",
            true,
        ),
        (&configuration.seed_vc_directory, "Seed-VC checkout", true),
        (
            &configuration.model_configuration_path,
            "Model configuration",
            false,
        ),
    ] {
        let path = Path::new(path);
        if (directory && !path.is_dir()) || (!directory && !path.is_file()) {
            failures.push(format!("{label} is missing."));
        }
    }
    let root = Path::new(&configuration.seed_vc_directory);
    for expected in &profile.expected_files {
        if expected.required
            && !expected.relative_path.is_empty()
            && !root.join(&expected.relative_path).is_file()
        {
            failures.push(format!(
                "Required backend role {} is missing.",
                expected.role
            ));
        }
    }
    if configuration.pretrained_checkpoint_paths.is_empty() {
        failures.push("At least one checkpoint is required.".to_owned());
    }
    for checkpoint in &configuration.pretrained_checkpoint_paths {
        if !Path::new(checkpoint).is_file() {
            failures.push("A configured checkpoint is missing.".to_owned());
        }
    }
    failures
}

fn fingerprint_file(
    role: &str,
    path: &Path,
    expected: Option<&str>,
) -> VoiceModelResult<FileFingerprint> {
    let checked_at = timestamp().map_err(clock_error)?;
    let metadata = fs::metadata(path)
        .map_err(|error| VoiceModelError::storage("Cannot inspect qualification file", error))?;
    let content_hash = sha256_file(path)
        .map_err(|error| VoiceModelError::storage("Cannot hash qualification file", error))?;
    let expected_hash = expected.map(str::to_ascii_lowercase);
    let validation_state = match expected_hash.as_deref() {
        Some(expected) if expected == content_hash => FileValidationState::Valid,
        Some(_) => FileValidationState::HashMismatch,
        None => FileValidationState::IdentityUnspecified,
    };
    Ok(FileFingerprint {
        role: role.to_owned(),
        display_path: path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("configured-file")
            .to_owned(),
        size_bytes: metadata.len(),
        content_hash: Some(content_hash),
        hash_algorithm: "sha256".to_owned(),
        expected_hash,
        validation_state,
        checked_at,
    })
}

fn write_project_smoke_fixture(path: &Path) -> VoiceModelResult<()> {
    let specification = hound::WavSpec {
        channels: 1,
        sample_rate: 48_000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, specification)
        .map_err(|error| VoiceModelError::storage("Cannot create project smoke WAV", error))?;
    for frame in 0..48_000 {
        let time = frame as f32 / 48_000.0;
        let envelope = (time * 20.0).min(1.0) * ((1.0 - time) * 20.0).clamp(0.0, 1.0);
        let sample = envelope
            * (0.16 * (std::f32::consts::TAU * 120.0 * time).sin()
                + 0.08 * (std::f32::consts::TAU * 720.0 * time).sin()
                + 0.04 * (std::f32::consts::TAU * 1_100.0 * time).sin());
        writer
            .write_sample((sample.clamp(-1.0, 1.0) * f32::from(i16::MAX)) as i16)
            .map_err(|error| VoiceModelError::storage("Cannot write project smoke WAV", error))?;
    }
    writer
        .finalize()
        .map_err(|error| VoiceModelError::storage("Cannot finalize project smoke WAV", error))
}

fn advance(
    run: &mut QualificationRunV1,
    next: QualificationState,
    on_update: &mut impl FnMut(&QualificationRunV1),
) -> VoiceModelResult<()> {
    if !can_transition(run.state, next) {
        return Err(VoiceModelError::new(
            VoiceModelErrorCode::InvalidStateTransition,
            format!(
                "Qualification cannot move from {:?} to {next:?}.",
                run.state
            ),
        ));
    }
    run.state = next;
    on_update(run);
    Ok(())
}

fn check_cancel(
    run: &mut QualificationRunV1,
    cancellation: &AtomicBool,
    run_directory: &Path,
    on_update: &mut impl FnMut(&QualificationRunV1),
) -> VoiceModelResult<()> {
    if cancellation.load(Ordering::Acquire) {
        run.state = QualificationState::Cancelled;
        run.ended_at = timestamp().ok();
        persist_report(run_directory, run)?;
        on_update(run);
        return Err(VoiceModelError::new(
            VoiceModelErrorCode::QualificationCancelled,
            "Backend qualification was cancelled.",
        ));
    }
    Ok(())
}

fn finish_failed(
    mut run: QualificationRunV1,
    run_directory: &Path,
    on_update: &mut impl FnMut(&QualificationRunV1),
) -> VoiceModelResult<QualificationRunV1> {
    run.state = QualificationState::Failed;
    run.ended_at = timestamp().ok();
    persist_report(run_directory, &run)?;
    on_update(&run);
    Ok(run)
}

fn persist_report(directory: &Path, run: &QualificationRunV1) -> VoiceModelResult<()> {
    fs::create_dir_all(directory).map_err(|error| {
        VoiceModelError::storage("Cannot create qualification directory", error)
    })?;
    let report = QualificationReportV1 {
        schema_version: QUALIFICATION_REPORT_SCHEMA_VERSION,
        run: run.clone(),
    };
    atomic_write_json(&directory.join("qualification.json"), &report)?;
    fs::write(directory.join("qualification.txt"), report_text(&report))
        .map_err(|error| VoiceModelError::storage("Cannot write qualification text report", error))
}

fn check(
    code: &str,
    label: &str,
    layer: QualificationCheckLayer,
    status: QualificationCheckStatus,
    message: impl Into<String>,
) -> QualificationCheckResult {
    QualificationCheckResult {
        code: code.to_owned(),
        label: label.to_owned(),
        layer,
        status,
        message: message.into(),
    }
}

fn clock_error(error: crate::voice_dataset::error::DatasetError) -> VoiceModelError {
    VoiceModelError::new(VoiceModelErrorCode::StorageUnavailable, error.message)
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum EnvironmentMatch {
    Identical,
    Compatible,
    ChangedWithWarning,
    Incompatible,
    Unknown,
}

pub fn compare_environments(
    expected: &ModelEnvironmentFingerprintV1,
    current: &ModelEnvironmentFingerprintV1,
) -> EnvironmentMatch {
    if expected.aggregate_hash == current.aggregate_hash {
        return EnvironmentMatch::Identical;
    }
    if expected.backend.backend_id != current.backend.backend_id
        || expected.backend.compatibility_profile_id != current.backend.compatibility_profile_id
        || expected.backend.commit_sha != current.backend.commit_sha
        || expected.worker.adapter_version != current.worker.adapter_version
        || expected.worker.protocol_version != current.worker.protocol_version
        || expected
            .python
            .version
            .split('.')
            .take(2)
            .collect::<Vec<_>>()
            != current
                .python
                .version
                .split('.')
                .take(2)
                .collect::<Vec<_>>()
        || expected
            .checkpoints
            .iter()
            .map(|item| &item.content_hash)
            .collect::<Vec<_>>()
            != current
                .checkpoints
                .iter()
                .map(|item| &item.content_hash)
                .collect::<Vec<_>>()
        || expected
            .configuration_files
            .iter()
            .map(|item| &item.content_hash)
            .collect::<Vec<_>>()
            != current
                .configuration_files
                .iter()
                .map(|item| &item.content_hash)
                .collect::<Vec<_>>()
    {
        return EnvironmentMatch::Incompatible;
    }
    if expected.packages == current.packages && expected.accelerator == current.accelerator {
        EnvironmentMatch::Compatible
    } else {
        EnvironmentMatch::ChangedWithWarning
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qualification_transitions_are_explicit() {
        assert!(can_transition(
            QualificationState::NotStarted,
            QualificationState::CollectingIdentity
        ));
        assert!(can_transition(
            QualificationState::RunningAudioSmokeTest,
            QualificationState::EvaluatingResults
        ));
        assert!(!can_transition(
            QualificationState::NotStarted,
            QualificationState::Qualified
        ));
        assert!(!can_transition(
            QualificationState::Qualified,
            QualificationState::StartingWorker
        ));
    }

    #[test]
    fn fingerprint_hash_is_deterministic_and_excludes_volatile_ids_and_times() {
        let mut first = fixture_fingerprint();
        first.aggregate_hash = first.calculate_aggregate_hash().expect("hash");
        let mut second = first.clone();
        second.fingerprint_id = "different".to_owned();
        second.generated_at = "later".to_owned();
        second.aggregate_hash = second.calculate_aggregate_hash().expect("hash");
        assert_eq!(first.aggregate_hash, second.aggregate_hash);
        assert!(!serde_json::to_string(&first)
            .expect("json")
            .to_ascii_lowercase()
            .contains("token"));
    }

    #[test]
    fn environment_comparison_blocks_material_changes() {
        let mut first = fixture_fingerprint();
        first.aggregate_hash = first.calculate_aggregate_hash().expect("hash");
        let mut second = first.clone();
        assert_eq!(
            compare_environments(&first, &second),
            EnvironmentMatch::Identical
        );
        second.backend.commit_sha = Some("b".repeat(40));
        second.aggregate_hash = second.calculate_aggregate_hash().expect("hash");
        assert_eq!(
            compare_environments(&first, &second),
            EnvironmentMatch::Incompatible
        );
    }

    #[test]
    fn highest_manual_level_requires_every_listening_field() {
        let mut manual = ManualListeningQualification::default();
        assert!(!manual.complete());
        manual = ManualListeningQualification {
            synthetic_output_played: true,
            speech_intelligible: true,
            no_severe_clipping: true,
            no_severe_truncation: true,
            no_source_target_mix_up: true,
            synthetic_label_reviewed: true,
            notes: None,
            confirmed_at: Some("1".to_owned()),
        };
        assert!(manual.complete());
    }

    #[test]
    fn resource_diagnostics_classify_disk_ram_vram_cpu_precision_and_unknowns() {
        let base = ResourceDiagnostics {
            available_memory_bytes: Some(2 * 1024 * 1024 * 1024),
            free_disk_bytes: Some(100),
            estimated_temporary_bytes: Some(200),
            available_vram_bytes: None,
            ..ResourceDiagnostics::default()
        };
        let cpu = assess_resource_risk(
            base.clone(),
            ModelDevice::Cpu,
            ModelPrecision::Float16,
            16,
            8,
            20_000,
            30_000,
        );
        assert_eq!(cpu.risk_level, Some(ResourceRiskLevel::Unsupported));
        for reason in [
            ResourceRiskReason::CpuOnlyTraining,
            ResourceRiskReason::InsufficientDisk,
            ResourceRiskReason::LowSystemMemory,
            ResourceRiskReason::UnsupportedPrecision,
            ResourceRiskReason::OversizedBatch,
            ResourceRiskReason::ExcessiveWorkers,
            ResourceRiskReason::LargeTrainingStepCount,
            ResourceRiskReason::TinyDataset,
        ] {
            assert!(cpu.reasons.contains(&reason));
        }
        let cuda = assess_resource_risk(
            ResourceDiagnostics::default(),
            ModelDevice::Cuda,
            ModelPrecision::Float32,
            1,
            0,
            100,
            600_000,
        );
        assert!(cuda
            .reasons
            .contains(&ResourceRiskReason::UnavailableVramMeasurement));
    }

    fn fixture_fingerprint() -> ModelEnvironmentFingerprintV1 {
        ModelEnvironmentFingerprintV1 {
            schema_version: 1,
            fingerprint_id: "fingerprint-1".to_owned(),
            generated_at: "1".to_owned(),
            operating_system: "windows".to_owned(),
            architecture: "x86_64".to_owned(),
            python: PythonFingerprint {
                implementation: "CPython".to_owned(),
                version: "3.10.1".to_owned(),
                executable_label: "python.exe".to_owned(),
            },
            worker: WorkerFingerprint {
                worker_version: "0.2.0".to_owned(),
                adapter_version: "v2".to_owned(),
                protocol_version: 1,
            },
            backend: BackendFingerprint {
                backend_id: "seed-vc-local".to_owned(),
                compatibility_profile_id: "profile".to_owned(),
                repository_remote: Some("https://example.test/repo".to_owned()),
                commit_sha: Some("a".repeat(40)),
                checkout_cleanliness: Some(CheckoutCleanliness::Clean),
            },
            packages: vec![PythonPackageFingerprint {
                package: "torch".to_owned(),
                version: Some("2.1".to_owned()),
                required: true,
                compatible: Some(true),
            }],
            accelerator: AcceleratorFingerprint::default(),
            checkpoints: Vec::new(),
            configuration_files: Vec::new(),
            aggregate_hash: String::new(),
        }
    }
}
