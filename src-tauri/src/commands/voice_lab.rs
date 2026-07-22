use std::path::PathBuf;

use crate::{
    dsp::chain::DspParameters,
    state::{app_state::AppState, engine_state::EngineState},
    voice_lab::session::{ClipVersion, VoiceLabStatus},
};

fn ensure_live_engine_stopped(state: &AppState) -> Result<(), String> {
    if live_engine_allows_lab(state.controller().status().state) {
        Ok(())
    } else {
        Err(
            "Stop the live Use/Test route before recording, rendering, or previewing in Voice Lab."
                .to_owned(),
        )
    }
}

fn live_engine_allows_lab(state: EngineState) -> bool {
    state == EngineState::Stopped
}

#[tauri::command]
pub fn get_voice_lab_status(state: tauri::State<'_, AppState>) -> Result<VoiceLabStatus, String> {
    state.voice_lab().status()
}

#[tauri::command]
pub fn start_voice_lab_capture(
    input_id: String,
    input_name: String,
    state: tauri::State<'_, AppState>,
) -> Result<VoiceLabStatus, String> {
    let _audio_operation = state
        .audio_operation()
        .lock()
        .map_err(|_| "Audio operations are temporarily unavailable.".to_owned())?;
    ensure_live_engine_stopped(&state)?;
    if state.voice_dataset().is_audio_active() {
        return Err(
            "Stop Voice Dataset recording or preview before starting Voice Lab audio.".to_owned(),
        );
    }
    state.voice_lab().start_capture(input_id, input_name)
}

#[tauri::command]
pub fn stop_voice_lab_capture(state: tauri::State<'_, AppState>) -> Result<VoiceLabStatus, String> {
    state.voice_lab().stop_capture()
}

#[tauri::command]
pub fn import_voice_lab_wav(
    path: String,
    state: tauri::State<'_, AppState>,
) -> Result<VoiceLabStatus, String> {
    state.voice_lab().import_wav(PathBuf::from(path))
}

#[tauri::command]
pub fn render_voice_lab(
    parameters: DspParameters,
    state: tauri::State<'_, AppState>,
) -> Result<VoiceLabStatus, String> {
    let _audio_operation = state
        .audio_operation()
        .lock()
        .map_err(|_| "Audio operations are temporarily unavailable.".to_owned())?;
    ensure_live_engine_stopped(&state)?;
    state.voice_lab().render(parameters)
}

#[tauri::command]
pub fn start_voice_lab_preview(
    version: ClipVersion,
    output_id: String,
    output_name: String,
    looping: bool,
    state: tauri::State<'_, AppState>,
) -> Result<VoiceLabStatus, String> {
    let _audio_operation = state
        .audio_operation()
        .lock()
        .map_err(|_| "Audio operations are temporarily unavailable.".to_owned())?;
    ensure_live_engine_stopped(&state)?;
    if state.voice_dataset().is_audio_active() {
        return Err(
            "Stop Voice Dataset recording or preview before starting Voice Lab audio.".to_owned(),
        );
    }
    state
        .voice_lab()
        .start_preview(version, output_id, output_name, looping)
}

#[tauri::command]
pub fn stop_voice_lab_preview(state: tauri::State<'_, AppState>) -> Result<VoiceLabStatus, String> {
    state.voice_lab().stop_preview()
}

#[tauri::command]
pub fn stop_voice_lab_audio(state: tauri::State<'_, AppState>) -> Result<VoiceLabStatus, String> {
    state.voice_lab().stop_audio()
}

#[tauri::command]
pub fn export_voice_lab_wav(
    version: ClipVersion,
    path: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let destination = PathBuf::from(path);
    let synthetic = version == ClipVersion::Processed
        && state
            .voice_lab()
            .status()
            .map(|status| status.processed_synthetic)
            .unwrap_or(false);
    state.voice_lab().export_wav(version, destination.clone())?;
    if synthetic {
        state
            .voice_model()
            .export_latest_conversion_provenance(&destination)
            .map_err(|error| error.message)?;
    }
    Ok(())
}

#[tauri::command]
pub fn clear_voice_lab(state: tauri::State<'_, AppState>) -> Result<VoiceLabStatus, String> {
    state.voice_lab().clear()
}

#[cfg(test)]
mod tests {
    use super::live_engine_allows_lab;
    use crate::state::engine_state::EngineState;

    #[test]
    fn only_a_fully_stopped_live_engine_allows_lab_audio_work() {
        assert!(live_engine_allows_lab(EngineState::Stopped));
        for state in [
            EngineState::Starting,
            EngineState::Running,
            EngineState::Degraded,
            EngineState::Recovering,
            EngineState::Stopping,
            EngineState::Error,
        ] {
            assert!(!live_engine_allows_lab(state));
        }
    }
}
