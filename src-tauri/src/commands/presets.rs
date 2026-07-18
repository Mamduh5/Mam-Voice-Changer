use crate::{
    config::presets::PresetCatalog, dsp::chain::DspParameters, state::app_state::AppState,
};

#[tauri::command]
pub fn list_presets(state: tauri::State<'_, AppState>) -> Result<PresetCatalog, String> {
    state
        .preset_store()
        .lock()
        .map_err(|_| "Preset storage is unavailable.".to_owned())?
        .catalog()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn save_preset(
    name: String,
    parameters: DspParameters,
    state: tauri::State<'_, AppState>,
) -> Result<PresetCatalog, String> {
    let catalog = {
        let mut store = state
            .preset_store()
            .lock()
            .map_err(|_| "Preset storage is unavailable.".to_owned())?;
        store
            .save_preset(name, parameters)
            .map_err(|error| error.to_string())?;
        store.catalog().map_err(|error| error.to_string())?
    };
    state
        .controller()
        .set_parameters(catalog.active_parameters)?;
    Ok(catalog)
}

#[tauri::command]
pub fn rename_preset(
    id: String,
    name: String,
    state: tauri::State<'_, AppState>,
) -> Result<PresetCatalog, String> {
    let mut store = state
        .preset_store()
        .lock()
        .map_err(|_| "Preset storage is unavailable.".to_owned())?;
    store
        .rename_preset(&id, name)
        .map_err(|error| error.to_string())?;
    store.catalog().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn duplicate_preset(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<PresetCatalog, String> {
    let catalog = {
        let mut store = state
            .preset_store()
            .lock()
            .map_err(|_| "Preset storage is unavailable.".to_owned())?;
        store
            .duplicate_preset(&id)
            .map_err(|error| error.to_string())?;
        store.catalog().map_err(|error| error.to_string())?
    };
    state
        .controller()
        .set_parameters(catalog.active_parameters)?;
    Ok(catalog)
}

#[tauri::command]
pub fn delete_preset(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<PresetCatalog, String> {
    let catalog = {
        let mut store = state
            .preset_store()
            .lock()
            .map_err(|_| "Preset storage is unavailable.".to_owned())?;
        store
            .delete_preset(&id)
            .map_err(|error| error.to_string())?;
        store.catalog().map_err(|error| error.to_string())?
    };
    state
        .controller()
        .set_parameters(catalog.active_parameters)?;
    Ok(catalog)
}

#[tauri::command]
pub fn apply_preset(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<PresetCatalog, String> {
    let catalog = {
        let mut store = state
            .preset_store()
            .lock()
            .map_err(|_| "Preset storage is unavailable.".to_owned())?;
        store
            .select_preset(&id)
            .map_err(|error| error.to_string())?;
        store.catalog().map_err(|error| error.to_string())?
    };
    state
        .controller()
        .set_parameters(catalog.active_parameters)?;
    Ok(catalog)
}

#[tauri::command]
pub fn reset_preset(state: tauri::State<'_, AppState>) -> Result<PresetCatalog, String> {
    let catalog = {
        let mut store = state
            .preset_store()
            .lock()
            .map_err(|_| "Preset storage is unavailable.".to_owned())?;
        store
            .reset_to_default()
            .map_err(|error| error.to_string())?;
        store.catalog().map_err(|error| error.to_string())?
    };
    state
        .controller()
        .set_parameters(catalog.active_parameters)?;
    Ok(catalog)
}
