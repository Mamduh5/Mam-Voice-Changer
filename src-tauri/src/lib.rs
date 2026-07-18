mod audio;
mod commands;
mod error;
mod state;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt().with_target(false).try_init()?;
    let controller = audio::controller::EngineController::new()?;

    tauri::Builder::default()
        .manage(state::app_state::AppState::new(controller))
        .invoke_handler(tauri::generate_handler![
            commands::devices::list_audio_devices,
            commands::engine::start_engine,
            commands::engine::stop_engine,
            commands::engine::get_engine_status,
        ])
        .run(tauri::generate_context!())?;
    Ok(())
}
