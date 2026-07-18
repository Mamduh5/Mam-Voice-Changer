use thiserror::Error;

#[derive(Debug, Error)]
pub enum AudioError {
    #[error("Unable to enumerate {direction} audio devices: {details}")]
    DeviceEnumeration {
        direction: &'static str,
        details: String,
    },
    #[error("Unable to read an audio device name: {0}")]
    DeviceName(String),
    #[error("The selected {direction} device is no longer available (ID: {id}). Refresh devices and select another device.")]
    DeviceNotFound { direction: &'static str, id: String },
    #[error("Unable to query supported formats for {direction} device '{name}': {details}")]
    SupportedFormats {
        direction: &'static str,
        name: String,
        details: String,
    },
    #[error("No compatible sample rate exists between input '{input}' and output '{output}'. Configure both devices to a common rate such as 48 kHz in Windows Sound settings.")]
    NoCommonSampleRate { input: String, output: String },
    #[error("Unable to create the {direction} audio stream: {details}")]
    BuildStream {
        direction: &'static str,
        details: String,
    },
    #[error("Unable to start the {direction} audio stream: {details}")]
    PlayStream {
        direction: &'static str,
        details: String,
    },
    #[error("The audio engine worker could not be started: {0}")]
    WorkerStart(String),
    #[error("The audio engine worker is unavailable. Restart the application.")]
    WorkerUnavailable,
    #[error("The audio engine did not respond within {0} seconds.")]
    WorkerTimeout(u64),
}
