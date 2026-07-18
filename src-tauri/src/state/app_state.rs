use std::{path::PathBuf, sync::Mutex};

use crate::{
    audio::controller::EngineController,
    config::presets::{PresetError, PresetStore},
};

pub struct AppState {
    controller: EngineController,
    preset_store: Mutex<PresetStore>,
}

impl AppState {
    pub fn new(controller: EngineController, preset_path: PathBuf) -> Result<Self, PresetError> {
        let preset_store = PresetStore::load(preset_path)?;
        controller
            .set_parameters(preset_store.selected_parameters()?)
            .map_err(PresetError::Validation)?;

        Ok(Self {
            controller,
            preset_store: Mutex::new(preset_store),
        })
    }

    pub fn controller(&self) -> &EngineController {
        &self.controller
    }

    pub fn preset_store(&self) -> &Mutex<PresetStore> {
        &self.preset_store
    }
}
