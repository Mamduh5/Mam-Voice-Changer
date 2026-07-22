use serde::Serialize;
use thiserror::Error;

pub type VoiceModelResult<T> = Result<T, VoiceModelError>;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum VoiceModelErrorCode {
    ConsentInactive,
    ProfileMissing,
    DatasetUnhealthy,
    NoAcceptedTakes,
    SnapshotTooSmall,
    SnapshotHashMismatch,
    SnapshotCreationFailed,
    BackendNotConfigured,
    PythonMissing,
    #[allow(dead_code)]
    WorkerMissing,
    #[allow(dead_code)]
    BackendMissing,
    CheckpointMissing,
    ProtocolMismatch,
    WorkerHandshakeFailed,
    WorkerExitedUnexpectedly,
    WorkerMessageMalformed,
    UnsupportedHardware,
    InvalidTrainingConfiguration,
    TrainingAlreadyActive,
    InferenceAlreadyActive,
    CancellationFailed,
    #[allow(dead_code)]
    JobInterrupted,
    ArtifactMissing,
    ArtifactHashMismatch,
    ArtifactSchemaUnsupported,
    ArtifactDisabledByConsent,
    ModelNotApproved,
    SourceClipMissing,
    ReferenceAudioMissing,
    GeneratedWavInvalid,
    GeneratedWavEmpty,
    GeneratedAudioNonFinite,
    EvaluationIncomplete,
    StorageUnavailable,
    AtomicWriteFailure,
    PartialDeletion,
    PathValidationFailure,
    InvalidStateTransition,
    UnexpectedOutput,
    CompatibilityProfileInvalid,
    QualificationAlreadyActive,
    QualificationCancelled,
    QualificationMissing,
    EnvironmentFingerprintInvalid,
    EnvironmentMismatch,
    CheckpointHashMismatch,
    ManualQualificationIncomplete,
    PackageInvalid,
    PackageLimitExceeded,
    PackageHashMismatch,
    PackageSchemaUnsupported,
    LicensingAcknowledgementRequired,
}

#[derive(Clone, Debug, Error, Serialize)]
#[error("{message}")]
#[serde(rename_all = "camelCase")]
pub struct VoiceModelError {
    pub code: VoiceModelErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact_id: Option<String>,
}

impl VoiceModelError {
    pub fn new(code: VoiceModelErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: sanitize_message(message.into()),
            job_id: None,
            artifact_id: None,
        }
    }

    pub fn storage(context: &str, error: impl std::fmt::Display) -> Self {
        Self::new(
            VoiceModelErrorCode::StorageUnavailable,
            format!("{context}: {error}"),
        )
    }

    pub fn artifact(mut self, id: &str) -> Self {
        self.artifact_id = Some(id.to_owned());
        self
    }
}

fn sanitize_message(message: String) -> String {
    let mut sanitized = message;
    if let Some(index) = sanitized.find(":\\") {
        let start = sanitized[..index]
            .rfind(char::is_whitespace)
            .map_or(0, |value| value + 1);
        let end = sanitized[index..]
            .find(char::is_whitespace)
            .map_or(sanitized.len(), |value| index + value);
        sanitized.replace_range(start..end, "[local path]");
    }
    sanitized
}
