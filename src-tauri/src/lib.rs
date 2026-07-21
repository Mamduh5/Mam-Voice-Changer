mod audio;
mod commands;
mod config;
mod dsp;
mod error;
mod state;
mod voice_lab;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt().with_target(false).try_init()?;
    let controller = audio::controller::EngineController::new()?;

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
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
            commands::external_routes::list_external_audio_routes,
            commands::external_routes::save_external_audio_route,
            commands::external_routes::delete_external_audio_route,
            commands::external_routes::validate_external_audio_route,
            commands::engine::start_engine,
            commands::engine::stop_engine,
            commands::engine::stop_test_route,
            commands::engine::get_engine_status,
            commands::parameters::get_parameters,
            commands::parameters::set_parameters,
            commands::presets::list_presets,
            commands::presets::save_preset,
            commands::presets::save_voice_lab_preset,
            commands::presets::rename_preset,
            commands::presets::duplicate_preset,
            commands::presets::delete_preset,
            commands::presets::apply_preset,
            commands::presets::reset_preset,
            commands::voice_lab::get_voice_lab_status,
            commands::voice_lab::start_voice_lab_capture,
            commands::voice_lab::stop_voice_lab_capture,
            commands::voice_lab::import_voice_lab_wav,
            commands::voice_lab::render_voice_lab,
            commands::voice_lab::start_voice_lab_preview,
            commands::voice_lab::stop_voice_lab_preview,
            commands::voice_lab::stop_voice_lab_audio,
            commands::voice_lab::export_voice_lab_wav,
            commands::voice_lab::clear_voice_lab,
        ])
        .run(tauri::generate_context!())?;
    Ok(())
}
