use crate::{dsp::chain::DspParameters, state::app_state::AppState};

#[tauri::command]
pub fn get_parameters(state: tauri::State<'_, AppState>) -> DspParameters {
    state.controller().parameters()
}

#[tauri::command]
pub fn set_parameters(
    parameters: DspParameters,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    state.controller().set_parameters(parameters)
}

#[tauri::command]
pub fn persist_audio_parameters(
    parameters: DspParameters,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let parameters = parameters.validate()?;
    let mut store = state
        .application_settings()
        .lock()
        .map_err(|_| "Application settings storage is unavailable.".to_owned())?;
    let mut document = store.document().clone();
    document.dsp_parameters = parameters;
    document.dsp_parameters_initialized = true;
    store.save(document).map_err(|error| error.to_string())
}
