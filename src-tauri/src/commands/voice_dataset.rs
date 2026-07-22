use std::path::PathBuf;

use crate::{
    state::{app_state::AppState, engine_state::EngineState},
    voice_dataset::{
        controller::{
            DatasetExportOptions, PromptSelection, ReviewTakeRequest, VoiceDatasetStatus,
        },
        error::{DatasetError, DatasetErrorCode},
        profile::{CreateVoiceProfileRequest, UpdateVoiceProfileRequest, VoiceProfileSummary},
        prompts::PromptPack,
        take::SelectedTakeVersion,
    },
};

fn ensure_dataset_audio_available(state: &AppState) -> Result<(), DatasetError> {
    if state.controller().status().state != EngineState::Stopped
        || state.voice_lab().is_audio_active()
    {
        Err(DatasetError::new(
            DatasetErrorCode::AudioOperationAlreadyActive,
            "Stop Use, Test, or Voice Lab audio before starting Dataset recording or preview.",
        ))
    } else {
        Ok(())
    }
}

#[tauri::command]
pub fn list_voice_profiles(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<VoiceProfileSummary>, DatasetError> {
    state.voice_dataset().list_profiles()
}

#[tauri::command]
pub fn create_voice_profile(
    request: CreateVoiceProfileRequest,
    state: tauri::State<'_, AppState>,
) -> Result<VoiceDatasetStatus, DatasetError> {
    state.voice_dataset().create_profile(request)
}

#[tauri::command]
pub fn read_voice_profile(
    profile_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<VoiceDatasetStatus, DatasetError> {
    state.voice_dataset().select_profile(profile_id)
}

#[tauri::command]
pub fn update_voice_profile(
    profile_id: String,
    request: UpdateVoiceProfileRequest,
    state: tauri::State<'_, AppState>,
) -> Result<VoiceDatasetStatus, DatasetError> {
    state.voice_dataset().update_profile(&profile_id, request)
}

#[tauri::command]
pub fn delete_voice_profile(
    profile_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<VoiceDatasetStatus, DatasetError> {
    state.voice_dataset().delete_profile(&profile_id)
}

#[tauri::command]
pub fn get_voice_dataset_status(
    state: tauri::State<'_, AppState>,
) -> Result<VoiceDatasetStatus, DatasetError> {
    state.voice_dataset().status()
}

#[tauri::command]
pub fn list_dataset_prompts(state: tauri::State<'_, AppState>) -> Result<PromptPack, DatasetError> {
    state.voice_dataset().prompts()
}

#[tauri::command]
pub fn select_dataset_prompt(
    selection: PromptSelection,
    state: tauri::State<'_, AppState>,
) -> Result<VoiceDatasetStatus, DatasetError> {
    state.voice_dataset().select_prompt(selection)
}

#[tauri::command]
pub fn start_dataset_recording(
    input_id: String,
    input_name: String,
    recorded_consent: bool,
    state: tauri::State<'_, AppState>,
) -> Result<VoiceDatasetStatus, DatasetError> {
    let _operation = state.audio_operation().lock().map_err(|_| {
        DatasetError::new(
            DatasetErrorCode::AudioOperationAlreadyActive,
            "Audio operations are temporarily unavailable.",
        )
    })?;
    ensure_dataset_audio_available(&state)?;
    state
        .voice_dataset()
        .start_recording(&input_id, &input_name, recorded_consent)
}

#[tauri::command]
pub fn stop_dataset_recording(
    state: tauri::State<'_, AppState>,
) -> Result<VoiceDatasetStatus, DatasetError> {
    state.voice_dataset().stop_recording()
}

#[tauri::command]
pub fn discard_current_dataset_take(
    state: tauri::State<'_, AppState>,
) -> Result<VoiceDatasetStatus, DatasetError> {
    state.voice_dataset().discard_recording()
}

#[tauri::command]
pub fn import_dataset_wavs(
    paths: Vec<String>,
    selection: PromptSelection,
    state: tauri::State<'_, AppState>,
) -> Result<VoiceDatasetStatus, DatasetError> {
    state.voice_dataset().import_wavs(paths, selection)
}

#[tauri::command]
pub fn review_dataset_take(
    profile_id: String,
    take_id: String,
    request: ReviewTakeRequest,
    state: tauri::State<'_, AppState>,
) -> Result<VoiceDatasetStatus, DatasetError> {
    state
        .voice_dataset()
        .review_take(&profile_id, &take_id, request)
}

#[tauri::command]
pub fn set_dataset_trim(
    take_id: String,
    start_frame: u64,
    end_frame: u64,
    state: tauri::State<'_, AppState>,
) -> Result<VoiceDatasetStatus, DatasetError> {
    state
        .voice_dataset()
        .set_trim(take_id, start_frame, end_frame)
}

#[tauri::command]
pub fn auto_trim_dataset_take(
    take_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<VoiceDatasetStatus, DatasetError> {
    state.voice_dataset().auto_trim(take_id)
}

#[tauri::command]
pub fn apply_dataset_trim(
    state: tauri::State<'_, AppState>,
) -> Result<VoiceDatasetStatus, DatasetError> {
    state.voice_dataset().apply_trim()
}

#[tauri::command]
pub fn reset_dataset_trim(
    take_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<VoiceDatasetStatus, DatasetError> {
    state.voice_dataset().reset_trim(&take_id)
}

#[tauri::command]
pub fn preview_dataset_take(
    take_id: String,
    version: SelectedTakeVersion,
    output_id: String,
    output_name: String,
    seek_ms: u64,
    state: tauri::State<'_, AppState>,
) -> Result<VoiceDatasetStatus, DatasetError> {
    let _operation = state.audio_operation().lock().map_err(|_| {
        DatasetError::new(
            DatasetErrorCode::AudioOperationAlreadyActive,
            "Audio operations are temporarily unavailable.",
        )
    })?;
    ensure_dataset_audio_available(&state)?;
    state
        .voice_dataset()
        .start_preview(&take_id, version, &output_id, &output_name, seek_ms)
}

#[tauri::command]
pub fn pause_dataset_preview(
    state: tauri::State<'_, AppState>,
) -> Result<VoiceDatasetStatus, DatasetError> {
    state.voice_dataset().pause_preview()
}

#[tauri::command]
pub fn stop_dataset_preview(
    state: tauri::State<'_, AppState>,
) -> Result<VoiceDatasetStatus, DatasetError> {
    state.voice_dataset().stop_preview()?;
    state.voice_dataset().status()
}

#[tauri::command]
pub fn delete_dataset_take(
    take_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<VoiceDatasetStatus, DatasetError> {
    state.voice_dataset().delete_take(&take_id)
}

#[tauri::command]
pub fn export_voice_dataset(
    destination: String,
    options: DatasetExportOptions,
    state: tauri::State<'_, AppState>,
) -> Result<String, DatasetError> {
    state
        .voice_dataset()
        .export(&PathBuf::from(destination), options)
        .map(|path| path.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn repair_voice_profile(
    profile_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<VoiceDatasetStatus, DatasetError> {
    state.voice_dataset().repair_profile(&profile_id)
}

#[tauri::command]
pub fn leave_voice_dataset(state: tauri::State<'_, AppState>) -> Result<(), DatasetError> {
    state.voice_dataset().stop_audio()
}

#[tauri::command]
pub fn clear_voice_dataset_error(
    state: tauri::State<'_, AppState>,
) -> Result<VoiceDatasetStatus, DatasetError> {
    state.voice_dataset().clear_error()
}
