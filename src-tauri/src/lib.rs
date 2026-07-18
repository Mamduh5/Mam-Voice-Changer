mod audio;

use audio::{AudioEngine, DeviceList, EngineStatus, Parameters};
use std::sync::Mutex;

struct AppState(Mutex<AudioEngine>);

#[tauri::command]
fn list_audio_devices() -> Result<DeviceList, String> { audio::list_devices().map_err(|e| e.to_string()) }

#[tauri::command]
fn start_engine(input_id: String, output_id: String, parameters: Parameters, state: tauri::State<AppState>) -> Result<(), String> {
    state.0.lock().map_err(|_| "Audio engine lock poisoned".to_string())?.start(&input_id, &output_id, parameters).map_err(|e| e.to_string())
}

#[tauri::command]
fn stop_engine(state: tauri::State<AppState>) -> Result<(), String> {
    state.0.lock().map_err(|_| "Audio engine lock poisoned".to_string())?.stop(); Ok(())
}

#[tauri::command]
fn set_parameters(parameters: Parameters, state: tauri::State<AppState>) -> Result<(), String> {
    parameters.validate().map_err(|e| e.to_string())?;
    state.0.lock().map_err(|_| "Audio engine lock poisoned".to_string())?.set_parameters(parameters); Ok(())
}

#[tauri::command]
fn get_engine_status(state: tauri::State<AppState>) -> Result<EngineStatus, String> {
    Ok(state.0.lock().map_err(|_| "Audio engine lock poisoned".to_string())?.status())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default().manage(AppState(Mutex::new(AudioEngine::default())))
        .invoke_handler(tauri::generate_handler![list_audio_devices,start_engine,stop_engine,set_parameters,get_engine_status])
        .run(tauri::generate_context!()).expect("error while running Mam Voice Changer");
}
