mod audio;
mod commands;
mod config;
mod dsp;
mod error;
mod state;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt().with_target(false).try_init()?;
    let controller = audio::controller::EngineController::new()?;

    tauri::Builder::default()
        .setup(move |app| {
            let app_data_dir = app.path().app_data_dir()?;
            let preset_path = app_data_dir.join(config::presets::PRESET_FILE_NAME);
            let application_settings_path =
                app_data_dir.join(config::application_settings::APPLICATION_SETTINGS_FILE_NAME);
            app.manage(state::app_state::AppState::new(
                controller,
                preset_path,
                application_settings_path,
            )?);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::devices::list_audio_devices,
            commands::devices::save_application_settings,
            commands::engine::start_engine,
            commands::engine::stop_engine,
            commands::engine::get_engine_status,
            commands::parameters::get_parameters,
            commands::parameters::set_parameters,
            commands::presets::list_presets,
            commands::presets::save_preset,
            commands::presets::rename_preset,
            commands::presets::duplicate_preset,
            commands::presets::delete_preset,
            commands::presets::apply_preset,
            commands::presets::reset_preset,
        ])
        .run(tauri::generate_context!())?;
    Ok(())
}
