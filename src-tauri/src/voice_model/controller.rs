use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, MutexGuard,
    },
    thread,
};

use crate::voice_dataset::{
    hash::sha256_samples,
    import::read_canonical_wav,
    source::{AcceptedDatasetTake, ManifestDatasetSource, VoiceDatasetSource},
    storage::{new_id, timestamp},
};

use super::{
    artifact::{
        require_approved, validate_display_name, verify_artifact, ModelApprovalStatus,
        VoiceModelArtifactV1,
    },
    artifact_storage::create_artifact,
    backend::{InferenceRequestContext, TrainingRequestContext, VoiceModelBackend},
    backend_registry,
    backend_validation::{static_readiness, validate_settings},
    consent::require_active_consent,
    error::{VoiceModelError, VoiceModelErrorCode, VoiceModelResult},
    evaluation::ModelEvaluationSummary,
    inference::{
        validate_configuration as validate_inference_configuration, validate_generated_wav,
    },
    recovery::recover_interrupted_jobs,
    snapshot::{
        create_snapshot, verify_snapshot, CreateTrainingSnapshotRequest, TrainingSnapshotV1,
    },
    state::{
        BackendReadiness, BackendValidationStatus, InferenceConfiguration, ModelBackendSettingsV1,
        OfflineConversionResult, TrainingConfiguration, TrainingJob, TrainingJobState,
        TrainingMetrics, VoiceModelStatus, MODEL_BACKEND_SETTINGS_SCHEMA_VERSION,
        WORKER_PROTOCOL_VERSION,
    },
    storage::{atomic_write_json, managed_join, read_json, remove_managed_directory},
    training::{require_transition, validate_configuration},
    worker_process::run_worker_job,
    worker_protocol::{push_bounded_log, WorkerEvent, WorkerEventKind},
};

const JOB_SCHEMA_VERSION: u32 = 1;
const SETTINGS_FILE: &str = "model-backends.json";

struct ControllerState {
    settings: ModelBackendSettingsV1,
    backend: BackendValidationStatus,
    active_training_job: Option<TrainingJob>,
    training_cancellation: Option<Arc<AtomicBool>>,
    active_inference: bool,
    inference_profile_id: Option<String>,
    inference_cancellation: Option<Arc<AtomicBool>>,
    latest_conversion: Option<OfflineConversionResult>,
    selected_artifact_id: Option<String>,
    last_error: Option<VoiceModelError>,
    logs: Vec<String>,
}

pub struct VoiceModelController {
    root: PathBuf,
    settings_path: PathBuf,
    state: Arc<Mutex<ControllerState>>,
}

impl VoiceModelController {
    pub fn new(root: PathBuf) -> VoiceModelResult<Self> {
        fs::create_dir_all(root.join("snapshots"))
            .and_then(|_| fs::create_dir_all(root.join("jobs")))
            .and_then(|_| fs::create_dir_all(root.join("profiles")))
            .and_then(|_| fs::create_dir_all(root.join("temporary-inference")))
            .map_err(|error| {
                VoiceModelError::storage("Cannot create voice-model storage", error)
            })?;
        let settings_path = root.join(SETTINGS_FILE);
        let settings = if settings_path.is_file() {
            read_json(&settings_path)?
        } else {
            ModelBackendSettingsV1::default()
        };
        let recovered = recover_interrupted_jobs(&root)?;
        let mut logs = Vec::new();
        for job_id in recovered {
            push_bounded_log(
                &mut logs,
                &format!("Marked interrupted job {job_id}; it was not resumed automatically."),
            );
        }
        let backend = static_readiness(&settings);
        Ok(Self {
            root,
            settings_path,
            state: Arc::new(Mutex::new(ControllerState {
                settings,
                backend,
                active_training_job: None,
                training_cancellation: None,
                active_inference: false,
                inference_profile_id: None,
                inference_cancellation: None,
                latest_conversion: None,
                selected_artifact_id: None,
                last_error: None,
                logs,
            })),
        })
    }

    pub fn read_backend_configuration(&self) -> VoiceModelResult<ModelBackendSettingsV1> {
        Ok(self.lock()?.settings.clone())
    }

    pub fn save_backend_configuration(
        &self,
        mut settings: ModelBackendSettingsV1,
    ) -> VoiceModelResult<ModelBackendSettingsV1> {
        settings.schema_version = MODEL_BACKEND_SETTINGS_SCHEMA_VERSION;
        if let Some(configuration) = settings.seed_vc.as_ref() {
            if configuration.pretrained_checkpoint_paths.len() > 16
                || [
                    &configuration.python_executable,
                    &configuration.worker_package_directory,
                    &configuration.seed_vc_directory,
                    &configuration.model_configuration_path,
                    &configuration.output_directory,
                ]
                .iter()
                .any(|value| value.trim().is_empty() || value.chars().count() > 2_000)
            {
                return Err(VoiceModelError::new(
                    VoiceModelErrorCode::BackendNotConfigured,
                    "Backend paths are required and must be reasonably bounded.",
                ));
            }
        }
        atomic_write_json(&self.settings_path, &settings)?;
        let mut state = self.lock()?;
        state.backend = static_readiness(&settings);
        state.settings = settings.clone();
        state.last_error = None;
        Ok(settings)
    }

    pub fn validate_backend(&self) -> VoiceModelResult<BackendValidationStatus> {
        let settings = self.lock()?.settings.clone();
        let result = validate_settings(&settings);
        let mut state = self.lock()?;
        match result {
            Ok(status) => {
                state.backend = status.clone();
                state.last_error = None;
                Ok(status)
            }
            Err(error) => {
                state.last_error = Some(error.clone());
                Err(error)
            }
        }
    }

    pub fn status(&self) -> VoiceModelResult<VoiceModelStatus> {
        let snapshots = self.list_snapshots()?;
        let artifacts = self.list_artifacts()?;
        let state = self.lock()?;
        Ok(VoiceModelStatus {
            backend: state.backend.clone(),
            active_training_job: state.active_training_job.clone(),
            active_inference: state.active_inference,
            latest_conversion: state.latest_conversion.clone(),
            selected_artifact_id: state.selected_artifact_id.clone(),
            last_error: state.last_error.clone(),
            logs: state.logs.clone(),
            snapshots,
            artifacts,
        })
    }

    pub fn list_snapshots(&self) -> VoiceModelResult<Vec<TrainingSnapshotV1>> {
        let mut snapshots = Vec::new();
        for entry in read_directories(&self.root.join("snapshots"))? {
            let manifest = entry.join("snapshot.json");
            if manifest.is_file() {
                let snapshot: TrainingSnapshotV1 = read_json(&manifest)?;
                verify_snapshot(&snapshot, &entry)?;
                snapshots.push(snapshot);
            }
        }
        snapshots.sort_by(|left, right| right.created_at.cmp(&left.created_at));
        Ok(snapshots)
    }

    pub fn create_snapshot(
        &self,
        source: &ManifestDatasetSource,
        request: CreateTrainingSnapshotRequest,
    ) -> VoiceModelResult<TrainingSnapshotV1> {
        require_active_consent(source, None)?;
        create_snapshot(&self.root.join("snapshots"), source, &request)
    }

    pub fn delete_snapshot(&self, snapshot_id: &str) -> VoiceModelResult<()> {
        validate_managed_id(snapshot_id, "snapshot")?;
        if self
            .list_artifacts()?
            .iter()
            .any(|artifact| artifact.snapshot_id == snapshot_id)
        {
            return Err(VoiceModelError::new(
                VoiceModelErrorCode::PartialDeletion,
                "Delete dependent model artifacts before deleting this snapshot.",
            ));
        }
        remove_managed_directory(
            &self.root.join("snapshots"),
            &self.root.join("snapshots").join(snapshot_id),
        )
    }

    pub fn list_jobs(&self) -> VoiceModelResult<Vec<TrainingJob>> {
        let mut jobs: Vec<TrainingJob> = Vec::new();
        for entry in read_directories(&self.root.join("jobs"))? {
            let manifest = entry.join("job.json");
            if manifest.is_file() {
                jobs.push(read_json(&manifest)?);
            }
        }
        jobs.sort_by(|left, right| right.started_at.cmp(&left.started_at));
        Ok(jobs)
    }

    pub fn start_training(
        &self,
        source: &ManifestDatasetSource,
        snapshot_id: &str,
        configuration: TrainingConfiguration,
    ) -> VoiceModelResult<TrainingJob> {
        require_active_consent(source, None)?;
        validate_managed_id(snapshot_id, "snapshot")?;
        let (settings, validation) = {
            let state = self.lock()?;
            if state
                .active_training_job
                .as_ref()
                .is_some_and(|job| !is_terminal(job.state))
            {
                return Err(VoiceModelError::new(
                    VoiceModelErrorCode::TrainingAlreadyActive,
                    "A local model training job is already active.",
                ));
            }
            (state.settings.clone(), state.backend.clone())
        };
        if validation.readiness != BackendReadiness::Ready {
            return Err(VoiceModelError::new(
                VoiceModelErrorCode::BackendNotConfigured,
                "Validate a ready local model backend before training.",
            ));
        }
        let capabilities = validation.capability_report.as_ref().ok_or_else(|| {
            VoiceModelError::new(
                VoiceModelErrorCode::BackendNotConfigured,
                "The backend capability report is unavailable.",
            )
        })?;
        let mut warnings = validate_configuration(&configuration, capabilities)?;
        let backend_configuration = settings.seed_vc.clone().ok_or_else(|| {
            VoiceModelError::new(
                VoiceModelErrorCode::BackendNotConfigured,
                "The Seed-VC backend is not configured.",
            )
        })?;
        let snapshot_directory = self.root.join("snapshots").join(snapshot_id);
        let snapshot: TrainingSnapshotV1 = read_json(&snapshot_directory.join("snapshot.json"))?;
        verify_snapshot(&snapshot, &snapshot_directory)?;
        let snapshot_size_bytes = super::storage::directory_size(&snapshot_directory);
        warnings.push(format!(
            "Managed snapshot storage estimate: {snapshot_size_bytes} bytes."
        ));
        if snapshot.profile_id != source.manifest().profile.id {
            return Err(VoiceModelError::new(
                VoiceModelErrorCode::ConsentInactive,
                "The snapshot belongs to a different consent profile.",
            ));
        }
        require_active_consent(source, Some(&snapshot.consent_version))?;

        let now = timestamp().map_err(dataset_clock_error)?;
        let job_id = new_id("job", &now);
        let job_directory = self.root.join("jobs").join(&job_id);
        fs::create_dir_all(&job_directory).map_err(|error| {
            VoiceModelError::storage("Cannot create training job storage", error)
        })?;
        let job = TrainingJob {
            schema_version: JOB_SCHEMA_VERSION,
            job_id: job_id.clone(),
            backend_id: "seed-vc-local".to_owned(),
            backend_version: capabilities.backend_version.clone(),
            worker_protocol_version: WORKER_PROTOCOL_VERSION,
            snapshot_id: snapshot.snapshot_id.clone(),
            snapshot_hash: snapshot.content_hash.clone(),
            profile_id: snapshot.profile_id.clone(),
            consent_version: snapshot.consent_version.clone(),
            configuration: configuration.clone(),
            state: TrainingJobState::Preparing,
            overall_progress: 0.0,
            current_step: 0,
            maximum_steps: configuration.maximum_steps,
            latest_metrics: TrainingMetrics::default(),
            started_at: now.clone(),
            updated_at: now,
            completed_at: None,
            worker_pid: None,
            last_checkpoint: None,
            log_file: "worker.log".to_owned(),
            error_summary: None,
            cancellation_requested: false,
            warnings,
        };
        atomic_write_json(&job_directory.join("job.json"), &job)?;
        let request_id = format!("training-{job_id}");
        let request =
            backend_registry::seed_vc().build_training_request(TrainingRequestContext {
                request_id: &request_id,
                snapshot: &snapshot,
                snapshot_directory: &snapshot_directory.to_string_lossy(),
                configuration: &configuration,
                backend: &backend_configuration,
                job_directory: &job_directory.to_string_lossy(),
                resume: false,
            })?;
        let cancellation = Arc::new(AtomicBool::new(false));
        {
            let mut state = self.lock()?;
            state.active_training_job = Some(job.clone());
            state.training_cancellation = Some(Arc::clone(&cancellation));
            state.last_error = None;
        }
        launch_training_thread(TrainingLaunch {
            shared: Arc::clone(&self.state),
            root: self.root.clone(),
            backend_configuration,
            request,
            cancellation,
            job_directory,
            job: job.clone(),
            snapshot,
        })?;
        Ok(job)
    }

    pub fn resume_training(
        &self,
        source: &ManifestDatasetSource,
        job_id: &str,
    ) -> VoiceModelResult<TrainingJob> {
        validate_managed_id(job_id, "job")?;
        let job_directory = self.root.join("jobs").join(job_id);
        let mut job: TrainingJob = read_json(&job_directory.join("job.json"))?;
        if !matches!(
            job.state,
            TrainingJobState::Interrupted | TrainingJobState::NeedsRecovery
        ) {
            return Err(VoiceModelError::new(
                VoiceModelErrorCode::InvalidStateTransition,
                "Only an interrupted recovery job can resume.",
            ));
        }
        let checkpoint = find_checkpoint(&job_directory).ok_or_else(|| {
            VoiceModelError::new(
                VoiceModelErrorCode::CheckpointMissing,
                "No managed checkpoint is available for this interrupted job.",
            )
        })?;
        require_active_consent(source, Some(&job.consent_version))?;
        if source.manifest().profile.id != job.profile_id {
            return Err(VoiceModelError::new(
                VoiceModelErrorCode::ConsentInactive,
                "The recovery job belongs to a different consent profile.",
            ));
        }
        let (backend_configuration, capabilities) = {
            let state = self.lock()?;
            if state
                .active_training_job
                .as_ref()
                .is_some_and(|active| !is_terminal(active.state))
            {
                return Err(VoiceModelError::new(
                    VoiceModelErrorCode::TrainingAlreadyActive,
                    "A local model training job is already active.",
                ));
            }
            if state.backend.readiness != BackendReadiness::Ready {
                return Err(VoiceModelError::new(
                    VoiceModelErrorCode::BackendNotConfigured,
                    "Validate a ready local model backend before resuming.",
                ));
            }
            (
                state.settings.seed_vc.clone().ok_or_else(|| {
                    VoiceModelError::new(
                        VoiceModelErrorCode::BackendNotConfigured,
                        "The Seed-VC backend is not configured.",
                    )
                })?,
                state.backend.capability_report.clone().ok_or_else(|| {
                    VoiceModelError::new(
                        VoiceModelErrorCode::BackendNotConfigured,
                        "The backend capability report is unavailable.",
                    )
                })?,
            )
        };
        if !capabilities.supports_resume {
            return Err(VoiceModelError::new(
                VoiceModelErrorCode::UnsupportedHardware,
                "The configured backend does not report checkpoint resume support.",
            ));
        }
        validate_configuration(&job.configuration, &capabilities)?;
        let snapshot_directory = self.root.join("snapshots").join(&job.snapshot_id);
        let snapshot: TrainingSnapshotV1 = read_json(&snapshot_directory.join("snapshot.json"))?;
        verify_snapshot(&snapshot, &snapshot_directory)?;
        if snapshot.content_hash != job.snapshot_hash {
            return Err(VoiceModelError::new(
                VoiceModelErrorCode::SnapshotHashMismatch,
                "The recovery snapshot no longer matches the job manifest.",
            ));
        }
        if job.state == TrainingJobState::Interrupted {
            require_transition(job.state, TrainingJobState::NeedsRecovery)?;
            job.state = TrainingJobState::NeedsRecovery;
        }
        require_transition(job.state, TrainingJobState::Preparing)?;
        job.state = TrainingJobState::Preparing;
        job.worker_pid = None;
        job.completed_at = None;
        job.error_summary = None;
        job.cancellation_requested = false;
        job.last_checkpoint = checkpoint
            .strip_prefix(&job_directory)
            .ok()
            .map(|path| path.to_string_lossy().replace('\\', "/"));
        job.updated_at = timestamp().map_err(dataset_clock_error)?;
        atomic_write_json(&job_directory.join("job.json"), &job)?;
        let request_id = format!("resume-{job_id}");
        let request =
            backend_registry::seed_vc().build_training_request(TrainingRequestContext {
                request_id: &request_id,
                snapshot: &snapshot,
                snapshot_directory: &snapshot_directory.to_string_lossy(),
                configuration: &job.configuration,
                backend: &backend_configuration,
                job_directory: &job_directory.to_string_lossy(),
                resume: true,
            })?;
        let cancellation = Arc::new(AtomicBool::new(false));
        {
            let mut state = self.lock()?;
            state.active_training_job = Some(job.clone());
            state.training_cancellation = Some(Arc::clone(&cancellation));
            state.last_error = None;
        }
        launch_training_thread(TrainingLaunch {
            shared: Arc::clone(&self.state),
            root: self.root.clone(),
            backend_configuration,
            request,
            cancellation,
            job_directory,
            job: job.clone(),
            snapshot,
        })?;
        Ok(job)
    }

    pub fn cancel_training(&self) -> VoiceModelResult<TrainingJob> {
        let mut state = self.lock()?;
        let job = state.active_training_job.as_mut().ok_or_else(|| {
            VoiceModelError::new(
                VoiceModelErrorCode::InvalidStateTransition,
                "No training job is active.",
            )
        })?;
        if is_terminal(job.state) {
            return Ok(job.clone());
        }
        require_transition(job.state, TrainingJobState::Cancelling)?;
        job.state = TrainingJobState::Cancelling;
        job.cancellation_requested = true;
        job.updated_at = timestamp().map_err(dataset_clock_error)?;
        let result = job.clone();
        atomic_write_json(
            &self.root.join("jobs").join(&job.job_id).join("job.json"),
            job,
        )?;
        if let Some(cancellation) = state.training_cancellation.as_ref() {
            cancellation.store(true, Ordering::Release);
        }
        Ok(result)
    }

    pub fn delete_job(&self, job_id: &str) -> VoiceModelResult<()> {
        validate_managed_id(job_id, "job")?;
        if self
            .lock()?
            .active_training_job
            .as_ref()
            .is_some_and(|job| job.job_id == job_id && !is_terminal(job.state))
        {
            return Err(VoiceModelError::new(
                VoiceModelErrorCode::TrainingAlreadyActive,
                "Cancel the active training job before deleting it.",
            ));
        }
        remove_managed_directory(
            &self.root.join("jobs"),
            &self.root.join("jobs").join(job_id),
        )
    }

    pub fn read_job_log(&self, job_id: &str) -> VoiceModelResult<Vec<String>> {
        validate_managed_id(job_id, "job")?;
        let path = self.root.join("jobs").join(job_id).join("worker.log");
        if !path.is_file() {
            return Ok(Vec::new());
        }
        let contents = fs::read_to_string(path)
            .map_err(|error| VoiceModelError::storage("Cannot read the worker log", error))?;
        let mut lines: Vec<_> = contents
            .lines()
            .rev()
            .take(500)
            .map(str::to_owned)
            .collect();
        lines.reverse();
        Ok(lines)
    }

    pub fn list_artifacts(&self) -> VoiceModelResult<Vec<VoiceModelArtifactV1>> {
        let mut artifacts = Vec::new();
        for profile in read_directories(&self.root.join("profiles"))? {
            for directory in read_directories(&profile.join("artifacts"))? {
                let manifest = directory.join("artifact.json");
                if manifest.is_file() {
                    let mut artifact: VoiceModelArtifactV1 = read_json(&manifest)?;
                    if let Err(error) = verify_artifact(&artifact, &directory) {
                        artifact.approval_status =
                            if error.code == VoiceModelErrorCode::ArtifactMissing {
                                ModelApprovalStatus::MissingFiles
                            } else {
                                ModelApprovalStatus::Invalid
                            };
                    }
                    artifacts.push(artifact);
                }
            }
        }
        artifacts.sort_by(|left, right| right.created_at.cmp(&left.created_at));
        Ok(artifacts)
    }

    pub fn read_artifact(&self, artifact_id: &str) -> VoiceModelResult<VoiceModelArtifactV1> {
        self.find_artifact(artifact_id)
            .map(|(_, artifact)| artifact)
    }

    pub fn rename_artifact(
        &self,
        artifact_id: &str,
        display_name: &str,
    ) -> VoiceModelResult<VoiceModelArtifactV1> {
        let (directory, mut artifact) = self.find_artifact(artifact_id)?;
        artifact.display_name = validate_display_name(display_name)?;
        artifact.updated_at = timestamp().map_err(dataset_clock_error)?;
        atomic_write_json(&directory.join("artifact.json"), &artifact)?;
        Ok(artifact)
    }

    pub fn save_evaluation(
        &self,
        source: &ManifestDatasetSource,
        artifact_id: &str,
        evaluation: ModelEvaluationSummary,
    ) -> VoiceModelResult<VoiceModelArtifactV1> {
        let (directory, mut artifact) = self.find_artifact(artifact_id)?;
        require_active_consent(source, Some(&artifact.consent_version))?;
        if source.manifest().profile.id != artifact.profile_id {
            return Err(VoiceModelError::new(
                VoiceModelErrorCode::ConsentInactive,
                "The selected consent profile does not own this model.",
            ));
        }
        evaluation.validate_for_approval()?;
        artifact.evaluation = Some(evaluation);
        artifact.approval_status = ModelApprovalStatus::EvaluationInProgress;
        artifact.updated_at = timestamp().map_err(dataset_clock_error)?;
        atomic_write_json(&directory.join("artifact.json"), &artifact)?;
        Ok(artifact)
    }

    pub fn approve_artifact(
        &self,
        source: &ManifestDatasetSource,
        artifact_id: &str,
    ) -> VoiceModelResult<VoiceModelArtifactV1> {
        let (directory, mut artifact) = self.find_artifact(artifact_id)?;
        require_active_consent(source, Some(&artifact.consent_version))?;
        if source.manifest().profile.id != artifact.profile_id {
            return Err(VoiceModelError::new(
                VoiceModelErrorCode::ConsentInactive,
                "The selected consent profile does not own this model.",
            ));
        }
        verify_artifact(&artifact, &directory)?;
        artifact
            .evaluation
            .as_ref()
            .ok_or_else(|| {
                VoiceModelError::new(
                    VoiceModelErrorCode::EvaluationIncomplete,
                    "Complete manual model evaluation before approval.",
                )
            })?
            .validate_for_approval()?;
        artifact.approval_status = ModelApprovalStatus::ApprovedForOfflineUse;
        artifact.updated_at = timestamp().map_err(dataset_clock_error)?;
        atomic_write_json(&directory.join("artifact.json"), &artifact)?;
        Ok(artifact)
    }

    pub fn reject_artifact(
        &self,
        artifact_id: &str,
        notes: Option<String>,
    ) -> VoiceModelResult<VoiceModelArtifactV1> {
        let (directory, mut artifact) = self.find_artifact(artifact_id)?;
        artifact.approval_status = ModelApprovalStatus::Rejected;
        artifact.notes = notes.map(|value| validate_notes(&value)).transpose()?;
        artifact.updated_at = timestamp().map_err(dataset_clock_error)?;
        atomic_write_json(&directory.join("artifact.json"), &artifact)?;
        Ok(artifact)
    }

    pub fn delete_artifact(&self, artifact_id: &str) -> VoiceModelResult<()> {
        let (directory, _) = self.find_artifact(artifact_id)?;
        let profile_artifacts = directory.parent().ok_or_else(|| {
            VoiceModelError::new(
                VoiceModelErrorCode::PathValidationFailure,
                "The artifact directory is invalid.",
            )
        })?;
        remove_managed_directory(profile_artifacts, &directory)
    }

    pub fn prepare_voice_lab_source_path(&self) -> VoiceModelResult<(String, PathBuf)> {
        let now = timestamp().map_err(dataset_clock_error)?;
        let source_id = new_id("source", &now);
        let directory = self.root.join("temporary-inference").join(&source_id);
        fs::create_dir_all(&directory).map_err(|error| {
            VoiceModelError::storage("Cannot create inference source storage", error)
        })?;
        Ok((source_id, directory.join("source.wav")))
    }

    pub fn start_inference(
        &self,
        source: &ManifestDatasetSource,
        artifact_id: &str,
        source_id: String,
        source_path: PathBuf,
        configuration: InferenceConfiguration,
        evaluation_mode: bool,
    ) -> VoiceModelResult<()> {
        validate_inference_configuration(&configuration)?;
        let (artifact_directory, mut artifact) = self.find_artifact(artifact_id)?;
        if evaluation_mode {
            if !matches!(
                artifact.approval_status,
                ModelApprovalStatus::Unevaluated | ModelApprovalStatus::EvaluationInProgress
            ) {
                require_approved(&artifact)?;
            }
        } else {
            require_approved(&artifact)?;
        }
        verify_artifact(&artifact, &artifact_directory)?;
        require_active_consent(source, Some(&artifact.consent_version))?;
        if source.manifest().profile.id != artifact.profile_id {
            return Err(VoiceModelError::new(
                VoiceModelErrorCode::ConsentInactive,
                "The selected consent profile does not own this model.",
            ));
        }
        let canonical_source = fs::canonicalize(&source_path).map_err(|_| {
            VoiceModelError::new(
                VoiceModelErrorCode::SourceClipMissing,
                "The managed Voice Lab source clip is missing.",
            )
        })?;
        let canonical_temporary = fs::canonicalize(self.root.join("temporary-inference"))
            .map_err(|error| VoiceModelError::storage("Cannot resolve inference storage", error))?;
        if !canonical_source.starts_with(canonical_temporary) {
            return Err(VoiceModelError::new(
                VoiceModelErrorCode::PathValidationFailure,
                "The inference source is outside managed temporary storage.",
            ));
        }
        let (reference_paths, reference_ids, reference_hashes) =
            select_references(source, &configuration.reference_take_ids)?;
        let backend_configuration = {
            let state = self.lock()?;
            if state.active_inference {
                return Err(VoiceModelError::new(
                    VoiceModelErrorCode::InferenceAlreadyActive,
                    "An offline model conversion is already active.",
                ));
            }
            if state.backend.readiness != BackendReadiness::Ready {
                return Err(VoiceModelError::new(
                    VoiceModelErrorCode::BackendNotConfigured,
                    "Validate the local model backend before conversion.",
                ));
            }
            state.settings.seed_vc.clone().ok_or_else(|| {
                VoiceModelError::new(
                    VoiceModelErrorCode::BackendNotConfigured,
                    "The Seed-VC backend is not configured.",
                )
            })?
        };
        if evaluation_mode && artifact.approval_status == ModelApprovalStatus::Unevaluated {
            artifact.approval_status = ModelApprovalStatus::EvaluationInProgress;
            artifact.updated_at = timestamp().map_err(dataset_clock_error)?;
            atomic_write_json(&artifact_directory.join("artifact.json"), &artifact)?;
        }
        let now = timestamp().map_err(dataset_clock_error)?;
        let result_id = new_id("result", &now);
        let target_profile_display_name = source.manifest().profile.display_name.clone();
        let result_directory = self.root.join("temporary-inference").join(&result_id);
        fs::create_dir_all(&result_directory).map_err(|error| {
            VoiceModelError::storage("Cannot create inference result storage", error)
        })?;
        let output_path = result_directory.join("synthetic.wav");
        let request_id = format!("inference-{result_id}");
        let request =
            backend_registry::seed_vc().build_inference_request(InferenceRequestContext {
                request_id: &request_id,
                artifact: &artifact,
                artifact_directory: &artifact_directory.to_string_lossy(),
                source_path: &source_path.to_string_lossy(),
                reference_paths: &reference_paths,
                configuration: &configuration,
                output_path: &output_path.to_string_lossy(),
                backend: &backend_configuration,
            })?;
        let cancellation = Arc::new(AtomicBool::new(false));
        {
            let mut state = self.lock()?;
            state.active_inference = true;
            state.inference_profile_id = Some(artifact.profile_id.clone());
            state.inference_cancellation = Some(Arc::clone(&cancellation));
            state.last_error = None;
        }
        let shared = Arc::clone(&self.state);
        let root = self.root.clone();
        thread::Builder::new()
            .name("voice-model-inference".to_owned())
            .spawn(move || {
                let run = run_worker_job(
                    &backend_configuration,
                    request,
                    cancellation,
                    |_| {},
                    |event| {
                        if let Ok(mut state) = shared.lock() {
                            if let Some(message) = event
                                .payload
                                .get("message")
                                .and_then(serde_json::Value::as_str)
                            {
                                push_bounded_log(&mut state.logs, message);
                            }
                        }
                    },
                );
                let completion = match run {
                    Ok(result) if result.terminal_event.event == WorkerEventKind::Completed => {
                        validate_generated_wav(&output_path).and_then(|clip| {
                            let summary = clip.summary();
                            let relative = output_path
                                .strip_prefix(&root)
                                .map_err(|_| {
                                    VoiceModelError::new(
                                        VoiceModelErrorCode::PathValidationFailure,
                                        "The generated result escaped managed storage.",
                                    )
                                })?
                                .to_string_lossy()
                                .replace('\\', "/");
                            let converted = OfflineConversionResult {
                                result_id,
                                artifact_id: artifact.artifact_id,
                                artifact_display_name: artifact.display_name,
                                profile_id: artifact.profile_id,
                                target_profile_display_name,
                                source_clip_id: source_id,
                                reference_take_ids: reference_ids,
                                reference_hashes,
                                backend_id: artifact.backend_id,
                                backend_version: artifact.backend_version,
                                synthetic: true,
                                output_file: relative,
                                duration_ms: summary.duration_ms,
                                peak: summary.peak,
                                clipping: summary.peak >= 0.995,
                                waveform: summary.waveform,
                                created_at: timestamp().map_err(dataset_clock_error)?,
                            };
                            atomic_write_json(
                                &result_directory.join("provenance.json"),
                                &converted,
                            )?;
                            Ok(converted)
                        })
                    }
                    Ok(result) if result.terminal_event.event == WorkerEventKind::Cancelled => {
                        Err(VoiceModelError::new(
                            VoiceModelErrorCode::CancellationFailed,
                            "Offline conversion was cancelled.",
                        ))
                    }
                    Ok(result) => Err(VoiceModelError::new(
                        VoiceModelErrorCode::WorkerExitedUnexpectedly,
                        result
                            .terminal_event
                            .payload
                            .get("message")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or("The inference worker failed."),
                    )),
                    Err(error) => Err(error),
                };
                if let Ok(mut state) = shared.lock() {
                    state.active_inference = false;
                    state.inference_profile_id = None;
                    state.inference_cancellation = None;
                    match completion {
                        Ok(result) => state.latest_conversion = Some(result),
                        Err(error) => state.last_error = Some(error),
                    }
                }
            })
            .map_err(|error| {
                VoiceModelError::storage("Cannot start inference coordinator", error)
            })?;
        Ok(())
    }

    pub fn cancel_inference(&self) -> VoiceModelResult<()> {
        let state = self.lock()?;
        if !state.active_inference {
            return Ok(());
        }
        if let Some(cancellation) = state.inference_cancellation.as_ref() {
            cancellation.store(true, Ordering::Release);
        }
        Ok(())
    }

    pub fn has_active_work(&self) -> bool {
        self.lock().is_ok_and(|state| {
            state.active_inference
                || state
                    .active_training_job
                    .as_ref()
                    .is_some_and(|job| !is_terminal(job.state))
        })
    }

    pub fn request_shutdown_cancellation(&self) -> VoiceModelResult<()> {
        let should_cancel_training = self
            .lock()?
            .active_training_job
            .as_ref()
            .is_some_and(|job| {
                !is_terminal(job.state) && job.state != TrainingJobState::Cancelling
            });
        if should_cancel_training {
            self.cancel_training()?;
        }
        self.cancel_inference()
    }

    pub fn conversion_path(&self, result_id: &str) -> VoiceModelResult<PathBuf> {
        validate_managed_id(result_id, "result")?;
        let state = self.lock()?;
        let result = state
            .latest_conversion
            .as_ref()
            .filter(|result| result.result_id == result_id)
            .ok_or_else(|| {
                VoiceModelError::new(
                    VoiceModelErrorCode::GeneratedWavInvalid,
                    "The offline conversion result is unavailable.",
                )
            })?;
        managed_join(&self.root, &result.output_file)
    }

    pub fn export_latest_conversion_provenance(&self, wav_path: &Path) -> VoiceModelResult<()> {
        let result = self.lock()?.latest_conversion.clone().ok_or_else(|| {
            VoiceModelError::new(
                VoiceModelErrorCode::GeneratedWavInvalid,
                "No synthetic conversion provenance is available to export.",
            )
        })?;
        atomic_write_json(&wav_path.with_extension("json"), &result)
    }

    pub fn clear_conversion(&self) -> VoiceModelResult<()> {
        let result = self.lock()?.latest_conversion.take();
        if let Some(result) = result {
            let path = managed_join(&self.root, &result.output_file)?;
            if let Some(directory) = path.parent() {
                remove_managed_directory(&self.root.join("temporary-inference"), directory)?;
            }
        }
        Ok(())
    }

    pub fn disable_profile(&self, profile_id: &str) -> VoiceModelResult<()> {
        if let Ok(state) = self.lock() {
            if state
                .active_training_job
                .as_ref()
                .is_some_and(|job| job.profile_id == profile_id && !is_terminal(job.state))
            {
                if let Some(cancel) = state.training_cancellation.as_ref() {
                    cancel.store(true, Ordering::Release);
                }
            }
            if state.inference_profile_id.as_deref() == Some(profile_id) {
                if let Some(cancel) = state.inference_cancellation.as_ref() {
                    cancel.store(true, Ordering::Release);
                }
            }
        }
        let profile_root = self
            .root
            .join("profiles")
            .join(profile_id)
            .join("artifacts");
        for directory in read_directories(&profile_root)? {
            let manifest = directory.join("artifact.json");
            if manifest.is_file() {
                let mut artifact: VoiceModelArtifactV1 = read_json(&manifest)?;
                artifact.approval_status = ModelApprovalStatus::DisabledByConsent;
                artifact.updated_at = timestamp().map_err(dataset_clock_error)?;
                atomic_write_json(&manifest, &artifact)?;
            }
        }
        Ok(())
    }

    pub fn clear_error(&self) -> VoiceModelResult<()> {
        self.lock()?.last_error = None;
        Ok(())
    }

    fn find_artifact(
        &self,
        artifact_id: &str,
    ) -> VoiceModelResult<(PathBuf, VoiceModelArtifactV1)> {
        validate_managed_id(artifact_id, "artifact")?;
        for profile in read_directories(&self.root.join("profiles"))? {
            let directory = profile.join("artifacts").join(artifact_id);
            let manifest = directory.join("artifact.json");
            if manifest.is_file() {
                let artifact: VoiceModelArtifactV1 = read_json(&manifest)?;
                return Ok((directory, artifact));
            }
        }
        Err(VoiceModelError::new(
            VoiceModelErrorCode::ArtifactMissing,
            "The selected model artifact is unavailable.",
        ))
    }

    fn lock(&self) -> VoiceModelResult<MutexGuard<'_, ControllerState>> {
        self.state.lock().map_err(|_| {
            VoiceModelError::new(
                VoiceModelErrorCode::StorageUnavailable,
                "Voice-model state is unavailable.",
            )
        })
    }
}

fn handle_training_event(
    shared: &Arc<Mutex<ControllerState>>,
    job_directory: &Path,
    event: &WorkerEvent,
) {
    update_job(shared, job_directory, |job| match event.event {
        WorkerEventKind::PhaseStarted => {
            if let Some(phase) = event
                .payload
                .get("phase")
                .and_then(serde_json::Value::as_str)
            {
                let next = match phase {
                    "preprocessing" => Some(TrainingJobState::Preprocessing),
                    "training" => Some(TrainingJobState::Training),
                    "savingCheckpoint" => Some(TrainingJobState::SavingCheckpoint),
                    "evaluatingCheckpoint" => Some(TrainingJobState::EvaluatingCheckpoint),
                    _ => None,
                };
                if let Some(next) = next {
                    if require_transition(job.state, next).is_ok() {
                        job.state = next;
                    }
                }
            }
        }
        WorkerEventKind::Progress => {
            if let Some(progress) = event
                .payload
                .get("progress")
                .and_then(serde_json::Value::as_f64)
            {
                job.overall_progress = progress.clamp(0.0, 1.0) as f32;
            }
            if let Some(step) = event
                .payload
                .get("step")
                .and_then(serde_json::Value::as_u64)
            {
                job.current_step = step.min(u64::from(job.maximum_steps)) as u32;
            }
        }
        WorkerEventKind::Metric => {
            job.latest_metrics.backend_reported = true;
            job.latest_metrics.training_loss = event
                .payload
                .get("trainingLoss")
                .and_then(serde_json::Value::as_f64);
            job.latest_metrics.validation_loss = event
                .payload
                .get("validationLoss")
                .and_then(serde_json::Value::as_f64);
            job.latest_metrics.learning_rate = event
                .payload
                .get("learningRate")
                .and_then(serde_json::Value::as_f64);
        }
        WorkerEventKind::CheckpointSaved => {
            job.last_checkpoint = event
                .payload
                .get("relativePath")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned);
        }
        WorkerEventKind::Warning => {
            if let Some(message) = event
                .payload
                .get("message")
                .and_then(serde_json::Value::as_str)
            {
                job.warnings.push(message.chars().take(500).collect());
                job.warnings.truncate(50);
            }
        }
        _ => {}
    });
    if let Some(message) = event
        .payload
        .get("message")
        .and_then(serde_json::Value::as_str)
    {
        if let Ok(mut state) = shared.lock() {
            push_bounded_log(&mut state.logs, message);
        }
        append_worker_log(job_directory, message);
    }
}

struct TrainingLaunch {
    shared: Arc<Mutex<ControllerState>>,
    root: PathBuf,
    backend_configuration: super::state::SeedVcBackendConfiguration,
    request: super::worker_protocol::WorkerRequest,
    cancellation: Arc<AtomicBool>,
    job_directory: PathBuf,
    job: TrainingJob,
    snapshot: TrainingSnapshotV1,
}

fn launch_training_thread(launch: TrainingLaunch) -> VoiceModelResult<()> {
    thread::Builder::new()
        .name("voice-model-training".to_owned())
        .spawn(move || {
            let TrainingLaunch {
                shared,
                root,
                backend_configuration,
                request,
                cancellation,
                job_directory,
                job,
                snapshot,
            } = launch;
            let result = run_worker_job(
                &backend_configuration,
                request,
                cancellation,
                |pid| {
                    update_job(&shared, &job_directory, |active| {
                        active.worker_pid = Some(pid)
                    })
                },
                |event| handle_training_event(&shared, &job_directory, event),
            );
            if let Ok(run) = &result {
                for line in &run.stderr_tail {
                    append_worker_log(&job_directory, &format!("[stderr] {line}"));
                }
            }
            match result {
                Ok(run) if run.terminal_event.event == WorkerEventKind::Completed => {
                    match create_artifact(
                        &root,
                        &job_directory,
                        &job,
                        &snapshot,
                        run.terminal_event.payload,
                    ) {
                        Ok(artifact) => {
                            update_job(&shared, &job_directory, |active| {
                                active.state = TrainingJobState::Completed;
                                active.overall_progress = 1.0;
                                active.current_step = active.maximum_steps;
                                active.completed_at = timestamp().ok();
                                active.worker_pid = None;
                            });
                            if let Ok(mut state) = shared.lock() {
                                push_bounded_log(
                                    &mut state.logs,
                                    &format!(
                                        "Created unevaluated artifact {}.",
                                        artifact.artifact_id
                                    ),
                                );
                                state.training_cancellation = None;
                            }
                        }
                        Err(error) => fail_job(&shared, &job_directory, error),
                    }
                }
                Ok(run) if run.terminal_event.event == WorkerEventKind::Cancelled => {
                    update_job(&shared, &job_directory, |active| {
                        active.state = TrainingJobState::Cancelled;
                        active.completed_at = timestamp().ok();
                        active.worker_pid = None;
                    });
                    if let Ok(mut state) = shared.lock() {
                        state.training_cancellation = None;
                    }
                }
                Ok(run) => {
                    let message = run
                        .terminal_event
                        .payload
                        .get("message")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("The model worker reported a failure.");
                    fail_job(
                        &shared,
                        &job_directory,
                        VoiceModelError::new(
                            VoiceModelErrorCode::WorkerExitedUnexpectedly,
                            message,
                        ),
                    );
                }
                Err(error) => fail_job(&shared, &job_directory, error),
            }
        })
        .map_err(|error| {
            VoiceModelError::storage("Cannot start the training coordinator", error)
        })?;
    Ok(())
}

fn update_job(
    shared: &Arc<Mutex<ControllerState>>,
    job_directory: &Path,
    update: impl FnOnce(&mut TrainingJob),
) {
    if let Ok(mut state) = shared.lock() {
        if let Some(job) = state.active_training_job.as_mut() {
            update(job);
            if let Ok(now) = timestamp() {
                job.updated_at = now;
            }
            let _ = atomic_write_json(&job_directory.join("job.json"), job);
        }
    }
}

fn fail_job(shared: &Arc<Mutex<ControllerState>>, job_directory: &Path, error: VoiceModelError) {
    update_job(shared, job_directory, |job| {
        job.state = TrainingJobState::Failed;
        job.error_summary = Some(error.message.clone());
        job.completed_at = timestamp().ok();
        job.worker_pid = None;
    });
    if let Ok(mut state) = shared.lock() {
        state.last_error = Some(error);
        state.training_cancellation = None;
    }
}

fn append_worker_log(directory: &Path, message: &str) {
    use std::io::Write;
    let bounded: String = message.chars().take(2_000).collect();
    if let Ok(mut file) = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(directory.join("worker.log"))
    {
        let _ = writeln!(file, "{bounded}");
    }
}

fn select_references(
    source: &ManifestDatasetSource,
    requested: &[String],
) -> VoiceModelResult<(Vec<String>, Vec<String>, Vec<String>)> {
    let mut takes: Vec<AcceptedDatasetTake> = source
        .accepted_takes()
        .map_err(|error| {
            VoiceModelError::new(VoiceModelErrorCode::DatasetUnhealthy, error.message)
        })?
        .collect();
    if requested.is_empty() {
        takes.retain(|take| {
            !(take.manual_override
                && take.quality.classification
                    == crate::voice_dataset::quality::QualityClassification::Fail)
        });
        takes.sort_by(|left, right| {
            quality_rank(right)
                .cmp(&quality_rank(left))
                .then_with(|| {
                    right
                        .quality
                        .heuristic_signal_to_noise_db
                        .total_cmp(&left.quality.heuristic_signal_to_noise_db)
                })
                .then_with(|| right.duration_ms.cmp(&left.duration_ms))
        });
        takes.truncate(1);
    } else {
        let requested: std::collections::HashSet<_> = requested.iter().collect();
        takes.retain(|take| requested.contains(&take.id));
        if takes.len() != requested.len() {
            return Err(VoiceModelError::new(
                VoiceModelErrorCode::ReferenceAudioMissing,
                "Every selected reference must be an accepted, non-excluded Dataset take.",
            ));
        }
    }
    if takes.is_empty() {
        return Err(VoiceModelError::new(
            VoiceModelErrorCode::ReferenceAudioMissing,
            "No eligible target reference audio is available.",
        ));
    }
    let mut paths = Vec::new();
    let mut ids = Vec::new();
    let mut hashes = Vec::new();
    for take in takes {
        let samples = read_canonical_wav(&take.path).map_err(|error| {
            VoiceModelError::new(VoiceModelErrorCode::ReferenceAudioMissing, error.message)
        })?;
        paths.push(take.path.to_string_lossy().to_string());
        ids.push(take.id);
        hashes.push(sha256_samples(&samples));
    }
    Ok((paths, ids, hashes))
}

fn quality_rank(take: &AcceptedDatasetTake) -> u8 {
    use crate::voice_dataset::quality::QualityClassification::{Fail, Pass, Warning};
    match take.quality.classification {
        Pass => 3,
        Warning => 2,
        Fail => 1,
    }
}

fn read_directories(root: &Path) -> VoiceModelResult<Vec<PathBuf>> {
    if !root.exists() {
        return Ok(Vec::new());
    }
    let entries = fs::read_dir(root)
        .map_err(|error| VoiceModelError::storage("Cannot list managed model storage", error))?;
    Ok(entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_dir()
                && !path
                    .file_name()
                    .is_some_and(|name| name.to_string_lossy().starts_with('.'))
        })
        .collect())
}

fn find_checkpoint(root: &Path) -> Option<PathBuf> {
    let entries = fs::read_dir(root).ok()?;
    let mut candidates = Vec::new();
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path).ok()?;
        if metadata.file_type().is_symlink() {
            continue;
        }
        if metadata.is_dir() {
            if let Some(checkpoint) = find_checkpoint(&path) {
                candidates.push(checkpoint);
            }
        } else if path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|extension| ["pth", "pt", "safetensors"].contains(&extension))
        {
            candidates.push(path);
        }
    }
    candidates.into_iter().max_by_key(|path| {
        fs::metadata(path)
            .and_then(|metadata| metadata.modified())
            .ok()
    })
}

fn validate_managed_id(id: &str, kind: &str) -> VoiceModelResult<()> {
    let valid = id.starts_with(&format!("{kind}-"))
        && id.len() <= 80
        && id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-');
    if valid {
        Ok(())
    } else {
        Err(VoiceModelError::new(
            VoiceModelErrorCode::PathValidationFailure,
            format!("Invalid managed {kind} identifier."),
        ))
    }
}

fn validate_notes(notes: &str) -> VoiceModelResult<String> {
    let trimmed = notes.trim();
    if trimmed.chars().count() > 2_000 || trimmed.chars().any(char::is_control) {
        return Err(VoiceModelError::new(
            VoiceModelErrorCode::InvalidStateTransition,
            "Model notes must be at most 2,000 visible characters.",
        ));
    }
    Ok(trimmed.to_owned())
}

fn is_terminal(state: TrainingJobState) -> bool {
    matches!(
        state,
        TrainingJobState::Cancelled
            | TrainingJobState::Completed
            | TrainingJobState::Failed
            | TrainingJobState::Interrupted
    )
}

fn dataset_clock_error(error: crate::voice_dataset::error::DatasetError) -> VoiceModelError {
    VoiceModelError::new(VoiceModelErrorCode::StorageUnavailable, error.message)
}
