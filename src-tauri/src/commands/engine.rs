use crate::{
    audio::{controller::StartAudioRequest, metrics::EngineStatus},
    state::app_state::AppState,
};

#[tauri::command]
pub fn start_engine(
    request: StartAudioRequest,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    state.controller().start(request)
}

#[tauri::command]
pub fn stop_engine(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.controller().stop()
}

#[tauri::command]
pub fn stop_test_route(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.controller().stop_test()
}

#[tauri::command]
pub fn get_engine_status(state: tauri::State<'_, AppState>) -> EngineStatus {
    state.controller().status()
}
