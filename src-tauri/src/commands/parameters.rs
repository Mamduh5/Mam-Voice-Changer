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
