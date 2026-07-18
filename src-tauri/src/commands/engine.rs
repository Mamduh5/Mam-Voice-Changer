use crate::{
    audio::{controller::StartRequest, metrics::EngineStatus},
    state::app_state::AppState,
};

#[tauri::command]
pub fn start_engine(
    request: StartRequest,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    state.controller().start(request)
}

#[tauri::command]
pub fn stop_engine(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.controller().stop()
}

#[tauri::command]
pub fn get_engine_status(state: tauri::State<'_, AppState>) -> EngineStatus {
    state.controller().status()
}
