use serde::Serialize;
use thiserror::Error;

#[derive(Clone, Copy, Debug, Eq, Error, PartialEq, Serialize)]
#[allow(dead_code)]
#[serde(rename_all = "camelCase")]
pub enum DatasetErrorCode {
    #[error("consentRequired")]
    ConsentRequired,
    #[error("profileNotFound")]
    ProfileNotFound,
    #[error("futureManifestSchema")]
    FutureManifestSchema,
    #[error("corruptManifest")]
    CorruptManifest,
    #[error("storageUnavailable")]
    StorageUnavailable,
    #[error("atomicWriteFailed")]
    AtomicWriteFailed,
    #[error("audioOperationAlreadyActive")]
    AudioOperationAlreadyActive,
    #[error("noMicrophoneSelected")]
    NoMicrophoneSelected,
    #[error("microphoneUnavailable")]
    MicrophoneUnavailable,
    #[error("recordingAlreadyActive")]
    RecordingAlreadyActive,
    #[error("recordingTooShort")]
    RecordingTooShort,
    #[error("recordingLimitReached")]
    RecordingLimitReached,
    #[error("recordingOverflow")]
    RecordingOverflow,
    #[error("takeNotFound")]
    TakeNotFound,
    #[error("duplicateImport")]
    DuplicateImport,
    #[error("unsupportedWav")]
    UnsupportedWav,
    #[error("importTooLong")]
    ImportTooLong,
    #[error("invalidTrimRange")]
    InvalidTrimRange,
    #[error("previewOutputMissing")]
    PreviewOutputMissing,
    #[error("previewFailed")]
    PreviewFailed,
    #[error("exportFailed")]
    ExportFailed,
    #[error("partialDeletion")]
    PartialDeletion,
    #[error("invalidStateTransition")]
    InvalidStateTransition,
    #[error("pathValidationFailed")]
    PathValidationFailed,
}

#[derive(Clone, Debug, Error, Serialize)]
#[error("{message}")]
#[serde(rename_all = "camelCase")]
pub struct DatasetError {
    pub code: DatasetErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub take_id: Option<String>,
}

impl DatasetError {
    pub fn new(code: DatasetErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            profile_id: None,
            take_id: None,
        }
    }

    pub fn profile(mut self, profile_id: &str) -> Self {
        self.profile_id = Some(profile_id.to_owned());
        self
    }

    pub fn take(mut self, take_id: &str) -> Self {
        self.take_id = Some(take_id.to_owned());
        self
    }

    pub fn storage(context: &str, error: impl std::fmt::Display) -> Self {
        Self::new(
            DatasetErrorCode::StorageUnavailable,
            format!("{context}: {error}"),
        )
    }
}

pub type DatasetResult<T> = Result<T, DatasetError>;
