use std::{
    io::{BufRead, BufReader, Write},
    process::{Child, ChildStdin, Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    thread,
    time::{Duration, Instant},
};

use serde_json::json;

use super::{
    backend::hello_request,
    error::{VoiceModelError, VoiceModelErrorCode, VoiceModelResult},
    state::SeedVcBackendConfiguration,
    worker_protocol::{
        decode_event, WorkerCommand, WorkerEvent, WorkerEventKind, WorkerRequest,
        MAX_WORKER_MESSAGE_BYTES,
    },
};

const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(15);
const CANCEL_GRACE: Duration = Duration::from_secs(5);

pub struct WorkerRunResult {
    pub terminal_event: WorkerEvent,
    pub stderr_tail: Vec<String>,
}

pub fn run_worker_job(
    configuration: &SeedVcBackendConfiguration,
    request: WorkerRequest,
    cancellation: Arc<AtomicBool>,
    mut on_started: impl FnMut(u32),
    mut on_event: impl FnMut(&WorkerEvent),
) -> VoiceModelResult<WorkerRunResult> {
    let mut child = spawn_worker(configuration)?;
    on_started(child.id());
    let mut stdin = child.stdin.take().ok_or_else(|| {
        VoiceModelError::new(
            VoiceModelErrorCode::WorkerHandshakeFailed,
            "The worker stdin pipe is unavailable.",
        )
    })?;
    let stdout = child.stdout.take().ok_or_else(|| {
        VoiceModelError::new(
            VoiceModelErrorCode::WorkerHandshakeFailed,
            "The worker stdout pipe is unavailable.",
        )
    })?;
    let stderr = child.stderr.take().ok_or_else(|| {
        VoiceModelError::new(
            VoiceModelErrorCode::WorkerHandshakeFailed,
            "The worker stderr pipe is unavailable.",
        )
    })?;
    let (events_tx, events_rx) = mpsc::sync_channel::<Result<Vec<u8>, String>>(64);
    let stdout_thread = thread::Builder::new()
        .name("voice-model-worker-stdout".to_owned())
        .spawn(move || {
            let mut reader = BufReader::new(stdout);
            loop {
                match read_bounded_line(&mut reader) {
                    Ok(Some(line)) => {
                        if events_tx.send(Ok(line)).is_err() {
                            break;
                        }
                    }
                    Ok(None) => break,
                    Err(error) => {
                        let _ = events_tx.send(Err(error));
                        break;
                    }
                }
            }
        })
        .map_err(|error| VoiceModelError::storage("Cannot monitor worker stdout", error))?;
    let (stderr_tx, stderr_rx) = mpsc::sync_channel::<String>(64);
    let stderr_thread = thread::Builder::new()
        .name("voice-model-worker-stderr".to_owned())
        .spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines().map_while(Result::ok) {
                let bounded: String = line.chars().take(2_000).collect();
                if stderr_tx.send(bounded).is_err() {
                    break;
                }
            }
        })
        .map_err(|error| VoiceModelError::storage("Cannot monitor worker stderr", error))?;

    let hello_id = format!("{}-hello", request.request_id);
    write_request(&mut stdin, &hello_request(&hello_id))?;
    let ready = receive_event(&events_rx, &hello_id, HANDSHAKE_TIMEOUT)?;
    if ready.event != WorkerEventKind::Ready {
        terminate_worker(&mut child);
        return Err(VoiceModelError::new(
            VoiceModelErrorCode::WorkerHandshakeFailed,
            "The worker did not complete the protocol handshake.",
        ));
    }
    write_request(&mut stdin, &request)?;

    let mut cancel_sent = false;
    let mut cancel_started = None;
    let terminal = loop {
        if cancellation.load(Ordering::Acquire) && !cancel_sent {
            let cancel = WorkerRequest::new(
                request.request_id.clone(),
                WorkerCommand::CancelJob,
                json!({}),
            );
            write_request(&mut stdin, &cancel)?;
            cancel_sent = true;
            cancel_started = Some(Instant::now());
        }
        if cancel_started.is_some_and(|started| started.elapsed() >= CANCEL_GRACE) {
            terminate_worker(&mut child);
            break WorkerEvent {
                protocol_version: request.protocol_version,
                request_id: request.request_id.clone(),
                event: WorkerEventKind::Cancelled,
                payload: json!({"forced": true}),
            };
        }
        match events_rx.recv_timeout(Duration::from_millis(100)) {
            Ok(Ok(line)) => {
                let event = decode_event(&line, &request.request_id)?;
                on_event(&event);
                if matches!(
                    event.event,
                    WorkerEventKind::Completed
                        | WorkerEventKind::Cancelled
                        | WorkerEventKind::Failed
                ) {
                    break event;
                }
            }
            Ok(Err(message)) => {
                terminate_worker(&mut child);
                return Err(VoiceModelError::new(
                    VoiceModelErrorCode::WorkerMessageMalformed,
                    message,
                ));
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if let Some(status) = child.try_wait().map_err(|error| {
                    VoiceModelError::storage("Cannot inspect worker process", error)
                })? {
                    return Err(VoiceModelError::new(
                        VoiceModelErrorCode::WorkerExitedUnexpectedly,
                        format!("The model worker exited before a terminal event ({status})."),
                    ));
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err(VoiceModelError::new(
                    VoiceModelErrorCode::WorkerExitedUnexpectedly,
                    "The model worker closed its protocol stream unexpectedly.",
                ));
            }
        }
    };

    let shutdown = WorkerRequest::new(
        format!("{}-shutdown", request.request_id),
        WorkerCommand::Shutdown,
        json!({}),
    );
    let _ = write_request(&mut stdin, &shutdown);
    drop(stdin);
    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline {
        if child.try_wait().ok().flatten().is_some() {
            break;
        }
        thread::sleep(Duration::from_millis(20));
    }
    if child.try_wait().ok().flatten().is_none() {
        terminate_worker(&mut child);
    }
    let _ = stdout_thread.join();
    let _ = stderr_thread.join();
    let stderr_tail = stderr_rx.try_iter().collect();
    Ok(WorkerRunResult {
        terminal_event: terminal,
        stderr_tail,
    })
}

pub fn validate_worker_handshake(
    configuration: &SeedVcBackendConfiguration,
    validation_payload: serde_json::Value,
) -> VoiceModelResult<super::state::BackendCapabilityReport> {
    let request = WorkerRequest::new(
        "backend-validation",
        WorkerCommand::ValidateBackend,
        validation_payload,
    );
    let result = run_worker_job(
        configuration,
        request,
        Arc::new(AtomicBool::new(false)),
        |_| {},
        |_| {},
    )?;
    if result.terminal_event.event != WorkerEventKind::Completed {
        return Err(VoiceModelError::new(
            VoiceModelErrorCode::WorkerHandshakeFailed,
            result
                .terminal_event
                .payload
                .get("message")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("Backend validation did not complete."),
        ));
    }
    let report = result
        .terminal_event
        .payload
        .get("capabilityReport")
        .cloned()
        .ok_or_else(|| {
            VoiceModelError::new(
                VoiceModelErrorCode::WorkerHandshakeFailed,
                "The backend did not provide a capability report.",
            )
        })?;
    serde_json::from_value(report).map_err(|_| {
        VoiceModelError::new(
            VoiceModelErrorCode::WorkerMessageMalformed,
            "The backend capability report is malformed.",
        )
    })
}

fn spawn_worker(configuration: &SeedVcBackendConfiguration) -> VoiceModelResult<Child> {
    let mut command = Command::new(&configuration.python_executable);
    command
        .arg("-m")
        .arg("mam_voice_worker")
        .current_dir(&configuration.worker_package_directory)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env_clear();
    for name in ["SystemRoot", "WINDIR", "PATH", "TEMP", "TMP"] {
        if let Some(value) = std::env::var_os(name) {
            command.env(name, value);
        }
    }
    command
        .env("PYTHONNOUSERSITE", "1")
        .env("HF_HUB_OFFLINE", "1")
        .env("TRANSFORMERS_OFFLINE", "1")
        .env("MAM_VOICE_NO_DOWNLOADS", "1");
    command.spawn().map_err(|error| {
        VoiceModelError::new(
            VoiceModelErrorCode::PythonMissing,
            format!("Cannot start the configured Python executable: {error}"),
        )
    })
}

fn write_request(stdin: &mut ChildStdin, request: &WorkerRequest) -> VoiceModelResult<()> {
    let bytes = serde_json::to_vec(request).map_err(|_| {
        VoiceModelError::new(
            VoiceModelErrorCode::WorkerMessageMalformed,
            "Cannot serialize a worker request.",
        )
    })?;
    if bytes.len() > MAX_WORKER_MESSAGE_BYTES {
        return Err(VoiceModelError::new(
            VoiceModelErrorCode::WorkerMessageMalformed,
            "The worker request exceeds the protocol size limit.",
        ));
    }
    stdin
        .write_all(&bytes)
        .and_then(|_| stdin.write_all(b"\n"))
        .and_then(|_| stdin.flush())
        .map_err(|error| {
            VoiceModelError::new(
                VoiceModelErrorCode::WorkerExitedUnexpectedly,
                format!("Cannot write to the model worker: {error}"),
            )
        })
}

fn receive_event(
    receiver: &mpsc::Receiver<Result<Vec<u8>, String>>,
    request_id: &str,
    timeout: Duration,
) -> VoiceModelResult<WorkerEvent> {
    match receiver.recv_timeout(timeout) {
        Ok(Ok(line)) => decode_event(&line, request_id),
        Ok(Err(message)) => Err(VoiceModelError::new(
            VoiceModelErrorCode::WorkerMessageMalformed,
            message,
        )),
        Err(_) => Err(VoiceModelError::new(
            VoiceModelErrorCode::WorkerHandshakeFailed,
            "Timed out waiting for the model worker handshake.",
        )),
    }
}

fn read_bounded_line(reader: &mut impl BufRead) -> Result<Option<Vec<u8>>, String> {
    let mut output = Vec::new();
    loop {
        let available = reader
            .fill_buf()
            .map_err(|error| format!("Cannot read worker protocol output: {error}"))?;
        if available.is_empty() {
            return if output.is_empty() {
                Ok(None)
            } else {
                Ok(Some(output))
            };
        }
        let newline = available.iter().position(|byte| *byte == b'\n');
        let used = newline.map_or(available.len(), |index| index + 1);
        if output.len() + used > MAX_WORKER_MESSAGE_BYTES + 1 {
            return Err("The worker emitted an oversized protocol message.".to_owned());
        }
        output.extend_from_slice(&available[..used]);
        reader.consume(used);
        if newline.is_some() {
            while matches!(output.last(), Some(b'\n' | b'\r')) {
                output.pop();
            }
            return Ok(Some(output));
        }
    }
}

fn terminate_worker(child: &mut Child) {
    #[cfg(target_os = "windows")]
    {
        let _ = Command::new("taskkill")
            .args(["/PID", &child.id().to_string(), "/T", "/F"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    let _ = child.kill();
    let _ = child.wait();
}

#[cfg(test)]
mod tests {
    use super::read_bounded_line;
    use crate::voice_model::worker_protocol::MAX_WORKER_MESSAGE_BYTES;
    use std::io::BufReader;

    #[test]
    fn bounded_line_reader_rejects_unterminated_oversized_output() {
        let bytes = vec![b'x'; MAX_WORKER_MESSAGE_BYTES + 2];
        assert!(read_bounded_line(&mut BufReader::new(bytes.as_slice())).is_err());
    }
}
