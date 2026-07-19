use thiserror::Error;

#[derive(Debug, Error)]
pub enum AudioError {
    #[error("Invalid audio route: {0}")]
    InvalidConfiguration(String),
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
    #[error(
        "Output device '{output}' does not support the active DSP sample rate of {sample_rate} Hz."
    )]
    OutputSampleRateUnavailable { output: String, sample_rate: u32 },
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
    #[error("The DSP processing worker could not be started: {0}")]
    DspWorkerStart(String),
    #[error("The audio engine worker is unavailable. Restart the application.")]
    WorkerUnavailable,
    #[error("The audio engine did not respond within {0} seconds.")]
    WorkerTimeout(u64),
    #[error("Startup prefill timed out after {timeout_ms} ms: achieved {achieved_frames} of {target_frames} frames.")]
    StartupPrefillTimeout {
        timeout_ms: u64,
        achieved_frames: usize,
        target_frames: usize,
    },
}
