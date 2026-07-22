use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{
    error::{VoiceModelError, VoiceModelErrorCode, VoiceModelResult},
    state::WORKER_PROTOCOL_VERSION,
};

pub const MAX_WORKER_MESSAGE_BYTES: usize = 256 * 1024;
pub const MAX_UI_LOG_ENTRIES: usize = 200;
pub const MAX_LOG_ENTRY_CHARS: usize = 2_000;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum WorkerCommand {
    Hello,
    ValidateBackend,
    InspectCapabilities,
    PreprocessSnapshot,
    StartTraining,
    ResumeTraining,
    CancelJob,
    InspectArtifact,
    RunInference,
    Shutdown,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkerRequest {
    pub protocol_version: u32,
    pub request_id: String,
    pub command: WorkerCommand,
    pub payload: Value,
}

impl WorkerRequest {
    pub fn new(request_id: impl Into<String>, command: WorkerCommand, payload: Value) -> Self {
        Self {
            protocol_version: WORKER_PROTOCOL_VERSION,
            request_id: request_id.into(),
            command,
            payload,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum WorkerEventKind {
    Ready,
    CapabilityReport,
    PhaseStarted,
    Progress,
    Metric,
    CheckpointSaved,
    Warning,
    Completed,
    Cancelled,
    Failed,
    Log,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkerEvent {
    pub protocol_version: u32,
    pub request_id: String,
    pub event: WorkerEventKind,
    pub payload: Value,
}

pub fn decode_event(line: &[u8], request_id: &str) -> VoiceModelResult<WorkerEvent> {
    if line.len() > MAX_WORKER_MESSAGE_BYTES {
        return Err(VoiceModelError::new(
            VoiceModelErrorCode::WorkerMessageMalformed,
            "The worker emitted an oversized protocol message.",
        ));
    }
    let event: WorkerEvent = serde_json::from_slice(line).map_err(|_| {
        VoiceModelError::new(
            VoiceModelErrorCode::WorkerMessageMalformed,
            "The worker emitted malformed protocol JSON.",
        )
    })?;
    if event.protocol_version != WORKER_PROTOCOL_VERSION {
        return Err(VoiceModelError::new(
            VoiceModelErrorCode::ProtocolMismatch,
            "The worker protocol version is incompatible.",
        ));
    }
    if event.request_id != request_id {
        return Err(VoiceModelError::new(
            VoiceModelErrorCode::WorkerMessageMalformed,
            "The worker response did not match the active request.",
        ));
    }
    Ok(event)
}

pub fn push_bounded_log(logs: &mut Vec<String>, value: &str) {
    let entry: String = value.chars().take(MAX_LOG_ENTRY_CHARS).collect();
    logs.push(entry);
    if logs.len() > MAX_UI_LOG_ENTRIES {
        logs.drain(..logs.len() - MAX_UI_LOG_ENTRIES);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        decode_event, push_bounded_log, WorkerEventKind, MAX_UI_LOG_ENTRIES,
        MAX_WORKER_MESSAGE_BYTES,
    };

    #[test]
    fn validates_version_shape_size_and_request_correlation() {
        let valid = br#"{"protocolVersion":1,"requestId":"r-1","event":"progress","payload":{"progress":0.5}}"#;
        assert!(decode_event(valid, "r-1").is_ok());
        assert!(decode_event(valid, "r-2").is_err());
        assert!(decode_event(br#"{"protocolVersion":9}"#, "r-1").is_err());
        assert!(decode_event(b"not json", "r-1").is_err());
        assert!(decode_event(&vec![b'x'; MAX_WORKER_MESSAGE_BYTES + 1], "r-1").is_err());
    }

    #[test]
    fn bounds_ui_logs() {
        let mut logs = Vec::new();
        for index in 0..(MAX_UI_LOG_ENTRIES + 4) {
            push_bounded_log(&mut logs, &format!("line-{index}"));
        }
        assert_eq!(logs.len(), MAX_UI_LOG_ENTRIES);
        assert_eq!(logs[0], "line-4");
    }

    #[test]
    fn decodes_all_structured_terminal_and_progress_event_kinds() {
        for name in [
            "ready",
            "capabilityReport",
            "phaseStarted",
            "progress",
            "metric",
            "checkpointSaved",
            "warning",
            "completed",
            "cancelled",
            "failed",
            "log",
        ] {
            let line = format!(
                r#"{{"protocolVersion":1,"requestId":"r-1","event":"{name}","payload":{{}}}}"#
            );
            assert!(decode_event(line.as_bytes(), "r-1").is_ok(), "{name}");
        }
        let unknown =
            br#"{"protocolVersion":1,"requestId":"r-1","event":"consoleText","payload":{}}"#;
        assert!(decode_event(unknown, "r-1").is_err());
        let completed = decode_event(
            br#"{"protocolVersion":1,"requestId":"r-1","event":"completed","payload":{}}"#,
            "r-1",
        )
        .expect("completed event");
        assert_eq!(completed.event, WorkerEventKind::Completed);
    }
}
