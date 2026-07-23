use crate::{
    state::app_state::AppState,
    voice_lab::session::ClipVersion,
    voice_model::{
        artifact::VoiceModelArtifactV1,
        backend_registry,
        compatibility::BackendCompatibilityProfileV1,
        error::{VoiceModelError, VoiceModelErrorCode},
        evaluation::{built_in_evaluation_phrases, EvaluationPhrase, ModelEvaluationSummary},
        model_package::{ModelPackageExportResult, ModelPackageImportRequest},
        qualification::{
            ManualListeningQualification, QualificationRunV1, TrainingPreflightReport,
        },
        snapshot::{CreateTrainingSnapshotRequest, TrainingSnapshotV1},
        state::{
            BackendDescriptor, BackendValidationStatus, InferenceConfiguration,
            ModelBackendSettingsV1, OfflineConversionResult, TrainingConfiguration, TrainingJob,
            TrainingPreset, VoiceModelStatus,
        },
    },
};

fn source(
    state: &AppState,
    profile_id: &str,
) -> Result<crate::voice_dataset::source::ManifestDatasetSource, VoiceModelError> {
    state
        .voice_dataset()
        .snapshot_source(profile_id)
        .map_err(|error| VoiceModelError::new(VoiceModelErrorCode::ProfileMissing, error.message))
}

#[tauri::command]
pub fn list_model_backends() -> Vec<BackendDescriptor> {
    backend_registry::list_backends()
}

#[tauri::command]
pub fn list_backend_compatibility_profiles(
    state: tauri::State<'_, AppState>,
) -> Vec<BackendCompatibilityProfileV1> {
    state.voice_model().compatibility_profiles()
}

#[tauri::command]
pub fn repair_voice_model_indexes(
    state: tauri::State<'_, AppState>,
) -> Result<crate::voice_model::indexes::RecoveryIndexesV1, VoiceModelError> {
    state.voice_model().repair_indexes()
}

#[tauri::command]
pub fn run_backend_qualification(
    profile_id: Option<String>,
    reference_take_id: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<QualificationRunV1, VoiceModelError> {
    let dataset_source = profile_id
        .as_deref()
        .map(|profile_id| source(&state, profile_id))
        .transpose()?;
    state
        .voice_model()
        .qualify_backend(dataset_source.as_ref(), reference_take_id.as_deref())
}

#[tauri::command]
pub fn load_qualification_smoke_into_voice_lab(
    state: tauri::State<'_, AppState>,
) -> Result<crate::voice_lab::session::VoiceLabStatus, VoiceModelError> {
    let path = state.voice_model().qualification_smoke_path()?;
    state
        .voice_lab()
        .load_synthetic_processed_wav(path)
        .map_err(|message| VoiceModelError::new(VoiceModelErrorCode::GeneratedWavInvalid, message))
}

#[tauri::command]
pub fn cancel_backend_qualification(
    state: tauri::State<'_, AppState>,
) -> Result<(), VoiceModelError> {
    state.voice_model().cancel_qualification()
}

#[tauri::command]
pub fn confirm_backend_manual_listening(
    confirmation: ManualListeningQualification,
    state: tauri::State<'_, AppState>,
) -> Result<QualificationRunV1, VoiceModelError> {
    state
        .voice_model()
        .confirm_qualification_listening(confirmation)
}

#[tauri::command]
pub fn save_backend_qualification_report(
    destination: String,
    human_readable: bool,
    state: tauri::State<'_, AppState>,
) -> Result<(), VoiceModelError> {
    state
        .voice_model()
        .save_qualification_report(std::path::Path::new(&destination), human_readable)
}

#[tauri::command]
pub fn list_voice_model_training_presets() -> Vec<TrainingConfiguration> {
    [
        TrainingPreset::QuickExperiment,
        TrainingPreset::BalancedFineTune,
        TrainingPreset::ExtendedFineTune,
    ]
    .into_iter()
    .map(TrainingConfiguration::for_preset)
    .collect()
}

#[tauri::command]
pub fn list_voice_model_evaluation_phrases() -> Vec<EvaluationPhrase> {
    built_in_evaluation_phrases()
}

#[tauri::command]
pub fn read_model_backend_configuration(
    state: tauri::State<'_, AppState>,
) -> Result<ModelBackendSettingsV1, VoiceModelError> {
    state.voice_model().read_backend_configuration()
}

#[tauri::command]
pub fn save_model_backend_configuration(
    settings: ModelBackendSettingsV1,
    state: tauri::State<'_, AppState>,
) -> Result<ModelBackendSettingsV1, VoiceModelError> {
    state.voice_model().save_backend_configuration(settings)
}

#[tauri::command]
pub fn validate_model_backend(
    state: tauri::State<'_, AppState>,
) -> Result<BackendValidationStatus, VoiceModelError> {
    state.voice_model().validate_backend()
}

#[tauri::command]
pub fn get_voice_model_status(
    state: tauri::State<'_, AppState>,
) -> Result<VoiceModelStatus, VoiceModelError> {
    state.voice_model().status()
}

#[tauri::command]
pub fn list_training_snapshots(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<TrainingSnapshotV1>, VoiceModelError> {
    state.voice_model().list_snapshots()
}

#[tauri::command]
pub fn create_training_snapshot(
    request: CreateTrainingSnapshotRequest,
    state: tauri::State<'_, AppState>,
) -> Result<TrainingSnapshotV1, VoiceModelError> {
    let source = source(&state, &request.profile_id)?;
    state.voice_model().create_snapshot(&source, request)
}

#[tauri::command]
pub fn delete_training_snapshot(
    snapshot_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), VoiceModelError> {
    state.voice_model().delete_snapshot(&snapshot_id)
}

#[tauri::command]
pub fn list_training_jobs(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<TrainingJob>, VoiceModelError> {
    state.voice_model().list_jobs()
}

#[tauri::command]
pub fn start_voice_model_training(
    profile_id: String,
    snapshot_id: String,
    configuration: TrainingConfiguration,
    warnings_acknowledged: bool,
    state: tauri::State<'_, AppState>,
) -> Result<TrainingJob, VoiceModelError> {
    let source = source(&state, &profile_id)?;
    state
        .voice_model()
        .start_training(&source, &snapshot_id, configuration, warnings_acknowledged)
}

#[tauri::command]
pub fn create_training_preflight(
    profile_id: String,
    snapshot_id: String,
    configuration: TrainingConfiguration,
    state: tauri::State<'_, AppState>,
) -> Result<TrainingPreflightReport, VoiceModelError> {
    let source = source(&state, &profile_id)?;
    state
        .voice_model()
        .training_preflight(&source, &snapshot_id, &configuration)
}

#[tauri::command]
pub fn cancel_voice_model_training(
    state: tauri::State<'_, AppState>,
) -> Result<TrainingJob, VoiceModelError> {
    state.voice_model().cancel_training()
}

#[tauri::command]
pub fn resume_voice_model_training(
    profile_id: String,
    job_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<TrainingJob, VoiceModelError> {
    let source = source(&state, &profile_id)?;
    state.voice_model().resume_training(&source, &job_id)
}

#[tauri::command]
pub fn delete_training_job(
    job_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), VoiceModelError> {
    state.voice_model().delete_job(&job_id)
}

#[tauri::command]
pub fn read_training_job_log(
    job_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<String>, VoiceModelError> {
    state.voice_model().read_job_log(&job_id)
}

#[tauri::command]
pub fn list_voice_model_artifacts(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<VoiceModelArtifactV1>, VoiceModelError> {
    state.voice_model().list_artifacts()
}

#[tauri::command]
pub fn read_voice_model_artifact(
    artifact_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<VoiceModelArtifactV1, VoiceModelError> {
    state.voice_model().read_artifact(&artifact_id)
}

#[tauri::command]
pub fn rename_voice_model_artifact(
    artifact_id: String,
    display_name: String,
    state: tauri::State<'_, AppState>,
) -> Result<VoiceModelArtifactV1, VoiceModelError> {
    state
        .voice_model()
        .rename_artifact(&artifact_id, &display_name)
}

#[tauri::command]
pub fn approve_voice_model_artifact(
    profile_id: String,
    artifact_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<VoiceModelArtifactV1, VoiceModelError> {
    let source = source(&state, &profile_id)?;
    state.voice_model().approve_artifact(&source, &artifact_id)
}

#[tauri::command]
pub fn reject_voice_model_artifact(
    artifact_id: String,
    notes: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<VoiceModelArtifactV1, VoiceModelError> {
    state.voice_model().reject_artifact(&artifact_id, notes)
}

#[tauri::command]
pub fn delete_voice_model_artifact(
    artifact_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), VoiceModelError> {
    state.voice_model().delete_artifact(&artifact_id)
}

#[tauri::command]
pub fn export_voice_model_package(
    artifact_id: String,
    destination: String,
    licensing_acknowledged: bool,
    state: tauri::State<'_, AppState>,
) -> Result<ModelPackageExportResult, VoiceModelError> {
    state.voice_model().export_artifact_package(
        &artifact_id,
        std::path::Path::new(&destination),
        licensing_acknowledged,
    )
}

#[tauri::command]
pub fn import_voice_model_package(
    request: ModelPackageImportRequest,
    state: tauri::State<'_, AppState>,
) -> Result<VoiceModelArtifactV1, VoiceModelError> {
    let source = source(&state, &request.profile_id)?;
    state
        .voice_model()
        .import_artifact_package(&source, request)
}

#[tauri::command]
pub fn start_offline_voice_conversion(
    profile_id: String,
    artifact_id: String,
    configuration: InferenceConfiguration,
    state: tauri::State<'_, AppState>,
) -> Result<(), VoiceModelError> {
    let source = source(&state, &profile_id)?;
    let (source_id, source_path) = state.voice_model().prepare_voice_lab_source_path()?;
    state
        .voice_lab()
        .stop_audio()
        .and_then(|_| {
            state
                .voice_lab()
                .export_wav(ClipVersion::Original, source_path.clone())
        })
        .map_err(|message| VoiceModelError::new(VoiceModelErrorCode::SourceClipMissing, message))?;
    state.voice_model().start_inference(
        &source,
        &artifact_id,
        source_id,
        source_path,
        configuration,
        false,
    )
}

#[tauri::command]
pub fn start_model_evaluation_conversion(
    profile_id: String,
    artifact_id: String,
    configuration: InferenceConfiguration,
    state: tauri::State<'_, AppState>,
) -> Result<(), VoiceModelError> {
    let source = source(&state, &profile_id)?;
    let (source_id, source_path) = state.voice_model().prepare_voice_lab_source_path()?;
    state
        .voice_lab()
        .stop_audio()
        .and_then(|_| {
            state
                .voice_lab()
                .export_wav(ClipVersion::Original, source_path.clone())
        })
        .map_err(|message| VoiceModelError::new(VoiceModelErrorCode::SourceClipMissing, message))?;
    state.voice_model().start_inference(
        &source,
        &artifact_id,
        source_id,
        source_path,
        configuration,
        true,
    )
}

#[tauri::command]
pub fn cancel_offline_voice_conversion(
    state: tauri::State<'_, AppState>,
) -> Result<(), VoiceModelError> {
    state.voice_model().cancel_inference()
}

#[tauri::command]
pub fn read_offline_conversion_result(
    state: tauri::State<'_, AppState>,
) -> Result<Option<OfflineConversionResult>, VoiceModelError> {
    Ok(state.voice_model().status()?.latest_conversion)
}

#[tauri::command]
pub fn load_offline_conversion_into_voice_lab(
    result_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<crate::voice_lab::session::VoiceLabStatus, VoiceModelError> {
    let path = state.voice_model().conversion_path(&result_id)?;
    state
        .voice_lab()
        .load_synthetic_processed_wav(path)
        .map_err(|message| VoiceModelError::new(VoiceModelErrorCode::GeneratedWavInvalid, message))
}

#[tauri::command]
pub fn clear_offline_conversion_result(
    state: tauri::State<'_, AppState>,
) -> Result<(), VoiceModelError> {
    state.voice_model().clear_conversion()
}

#[tauri::command]
pub fn save_model_evaluation_ratings(
    profile_id: String,
    artifact_id: String,
    evaluation: ModelEvaluationSummary,
    state: tauri::State<'_, AppState>,
) -> Result<VoiceModelArtifactV1, VoiceModelError> {
    let source = source(&state, &profile_id)?;
    state
        .voice_model()
        .save_evaluation(&source, &artifact_id, evaluation)
}

#[tauri::command]
pub fn clear_voice_model_error(state: tauri::State<'_, AppState>) -> Result<(), VoiceModelError> {
    state.voice_model().clear_error()
}

#[tauri::command]
pub fn cancel_model_work_for_shutdown(
    state: tauri::State<'_, AppState>,
) -> Result<(), VoiceModelError> {
    state.voice_model().request_shutdown_cancellation()
}
