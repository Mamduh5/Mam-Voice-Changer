use std::{path::PathBuf, sync::Mutex};

use crate::{
    audio::controller::EngineController,
    config::{
        application_settings::ApplicationSettingsStore,
        presets::{PresetError, PresetStore},
    },
    voice_dataset::controller::VoiceDatasetController,
    voice_lab::controller::VoiceLabController,
};

pub struct AppState {
    controller: EngineController,
    preset_store: Mutex<PresetStore>,
    application_settings: Mutex<ApplicationSettingsStore>,
    voice_lab: VoiceLabController,
    voice_dataset: VoiceDatasetController,
    audio_operation: Mutex<()>,
}

impl AppState {
    pub fn new(
        controller: EngineController,
        preset_path: PathBuf,
        application_settings_path: PathBuf,
        voice_dataset_root: PathBuf,
    ) -> Result<Self, PresetError> {
        let preset_store = PresetStore::load(preset_path)?;
        let application_settings = ApplicationSettingsStore::load(application_settings_path);
        controller
            .set_parameters(preset_store.selected_parameters()?)
            .map_err(PresetError::Validation)?;

        Ok(Self {
            controller,
            preset_store: Mutex::new(preset_store),
            application_settings: Mutex::new(application_settings),
            voice_lab: VoiceLabController::new().map_err(PresetError::Validation)?,
            voice_dataset: VoiceDatasetController::new(voice_dataset_root)
                .map_err(|error| PresetError::Validation(error.message))?,
            audio_operation: Mutex::new(()),
        })
    }

    pub fn controller(&self) -> &EngineController {
        &self.controller
    }

    pub fn preset_store(&self) -> &Mutex<PresetStore> {
        &self.preset_store
    }

    pub fn application_settings(&self) -> &Mutex<ApplicationSettingsStore> {
        &self.application_settings
    }

    pub fn voice_lab(&self) -> &VoiceLabController {
        &self.voice_lab
    }

    pub fn voice_dataset(&self) -> &VoiceDatasetController {
        &self.voice_dataset
    }

    pub fn audio_operation(&self) -> &Mutex<()> {
        &self.audio_operation
    }
}
