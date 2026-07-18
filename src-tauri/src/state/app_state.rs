use crate::audio::controller::EngineController;

pub struct AppState {
    controller: EngineController,
}

impl AppState {
    pub fn new(controller: EngineController) -> Self {
        Self { controller }
    }

    pub fn controller(&self) -> &EngineController {
        &self.controller
    }
}
