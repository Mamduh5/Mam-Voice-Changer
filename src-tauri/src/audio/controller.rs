use std::{
    sync::{
        mpsc::{self, Receiver, RecvTimeoutError, SyncSender},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};

use cpal::traits::StreamTrait;
use ringbuf::traits::Observer;
use serde::Deserialize;

use crate::{
    audio::{
        device::{find_device_with_fallback, DeviceDirection},
        input_stream,
        metrics::{AudioRoutePurpose, EngineStatus, SharedMetrics},
        output_stream::{self, OutputRole},
        reliability::ReliabilityProfile,
        ring_buffer::AudioRingBuffer,
        stream_config::{self, ActiveStreamFormat, StreamSpec},
        worker::{DspWorker, OutputTarget},
    },
    error::AudioError,
    state::{engine_state::EngineState, parameter_state::ParameterState},
};

const COMMAND_TIMEOUT: Duration = Duration::from_secs(10);
const WORKER_POLL_INTERVAL: Duration = Duration::from_millis(20);
const RECOVERY_BACKOFF_MS: [u64; 3] = [100, 300, 900];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StartupPrefillDecision {
    Waiting,
    Ready,
    TimedOut,
}

fn startup_prefill_decision(
    achieved_frames: usize,
    target_frames: usize,
    elapsed: Duration,
    timeout: Duration,
) -> StartupPrefillDecision {
    if achieved_frames >= target_frames {
        StartupPrefillDecision::Ready
    } else if elapsed >= timeout {
        StartupPrefillDecision::TimedOut
    } else {
        StartupPrefillDecision::Waiting
    }
}

fn estimated_device_latency_ms(
    input_buffer_frames: u32,
    maximum_output_buffer_frames: u32,
    prefill_frames: usize,
    sample_rate: u32,
) -> f32 {
    (input_buffer_frames + maximum_output_buffer_frames + prefill_frames as u32) as f32 * 1_000.0
        / sample_rate.max(1) as f32
}

#[derive(Clone, Debug, Deserialize)]
#[serde(
    tag = "mode",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum StartAudioRequest {
    Use {
        input_id: String,
        input_name: String,
        processed_destination_id: String,
        processed_destination_name: String,
        reliability_profile: ReliabilityProfile,
    },
    Test {
        input_id: String,
        input_name: String,
        monitor_id: String,
        monitor_name: String,
        reliability_profile: ReliabilityProfile,
    },
}

impl StartAudioRequest {
    fn validate(&self) -> Result<(), String> {
        match self {
            Self::Use {
                input_id,
                input_name,
                processed_destination_id,
                processed_destination_name,
                ..
            } => {
                validate_device("input microphone", input_id, input_name)?;
                validate_device(
                    "processed destination",
                    processed_destination_id,
                    processed_destination_name,
                )?;
            }
            Self::Test {
                input_id,
                input_name,
                monitor_id,
                monitor_name,
                ..
            } => {
                validate_device("input microphone", input_id, input_name)?;
                validate_device("Test monitor", monitor_id, monitor_name)?;
            }
        }
        Ok(())
    }

    fn input(&self) -> (&str, &str) {
        match self {
            Self::Use {
                input_id,
                input_name,
                ..
            }
            | Self::Test {
                input_id,
                input_name,
                ..
            } => (input_id, input_name),
        }
    }

    const fn reliability_profile(&self) -> ReliabilityProfile {
        match self {
            Self::Use {
                reliability_profile,
                ..
            }
            | Self::Test {
                reliability_profile,
                ..
            } => *reliability_profile,
        }
    }

    const fn route_purpose(&self) -> AudioRoutePurpose {
        match self {
            Self::Use { .. } => AudioRoutePurpose::Use,
            Self::Test { .. } => AudioRoutePurpose::Test,
        }
    }
}

fn validate_device(purpose: &str, id: &str, name: &str) -> Result<(), String> {
    if id.trim().is_empty() || name.trim().is_empty() {
        Err(format!("A valid {purpose} must be selected."))
    } else {
        Ok(())
    }
}

enum EngineCommand {
    Start {
        request: StartAudioRequest,
        reply: SyncSender<Result<(), String>>,
    },
    Stop {
        reply: SyncSender<Result<(), String>>,
    },
    StopTest {
        reply: SyncSender<Result<(), String>>,
    },
}

#[derive(Debug)]
pub enum RuntimeEvent {
    InputDeviceStopped(String),
    DestinationDeviceStopped(String),
    MonitorDeviceStopped(String),
    DspProducedInvalidAudio,
}

pub struct EngineController {
    commands: SyncSender<EngineCommand>,
    metrics: Arc<SharedMetrics>,
    parameters: Arc<ParameterState>,
}

impl EngineController {
    pub fn new() -> Result<Self, AudioError> {
        let (commands, receiver) = mpsc::sync_channel(8);
        let metrics = Arc::new(SharedMetrics::default());
        let parameters = Arc::new(ParameterState::default());
        let worker_metrics = Arc::clone(&metrics);
        let worker_parameters = Arc::clone(&parameters);
        thread::Builder::new()
            .name("mam-audio-engine".to_owned())
            .spawn(move || worker_loop(receiver, worker_metrics, worker_parameters))
            .map_err(|error| AudioError::WorkerStart(error.to_string()))?;
        Ok(Self {
            commands,
            metrics,
            parameters,
        })
    }

    pub fn start(&self, request: StartAudioRequest) -> Result<(), String> {
        request.validate()?;
        let (reply, response) = mpsc::sync_channel(1);
        self.commands
            .send(EngineCommand::Start { request, reply })
            .map_err(|_| AudioError::WorkerUnavailable.to_string())?;
        receive_response(response)
    }

    pub fn stop(&self) -> Result<(), String> {
        let (reply, response) = mpsc::sync_channel(1);
        self.commands
            .send(EngineCommand::Stop { reply })
            .map_err(|_| AudioError::WorkerUnavailable.to_string())?;
        receive_response(response)
    }

    pub fn stop_test(&self) -> Result<(), String> {
        let (reply, response) = mpsc::sync_channel(1);
        self.commands
            .send(EngineCommand::StopTest { reply })
            .map_err(|_| AudioError::WorkerUnavailable.to_string())?;
        receive_response(response)
    }

    pub fn status(&self) -> EngineStatus {
        self.metrics.snapshot()
    }

    pub fn parameters(&self) -> crate::dsp::chain::DspParameters {
        self.parameters.snapshot()
    }

    pub fn set_parameters(
        &self,
        parameters: crate::dsp::chain::DspParameters,
    ) -> Result<(), String> {
        self.parameters.update(parameters)
    }
}

fn receive_response(response: Receiver<Result<(), String>>) -> Result<(), String> {
    response
        .recv_timeout(COMMAND_TIMEOUT)
        .map_err(|_| AudioError::WorkerTimeout(COMMAND_TIMEOUT.as_secs()).to_string())?
}

struct StreamBundle {
    _input: cpal::Stream,
    _destination: Option<cpal::Stream>,
    monitor: Option<cpal::Stream>,
    _dsp_worker: DspWorker,
}

struct StartedStreams {
    bundle: StreamBundle,
    runtime_events: Receiver<RuntimeEvent>,
    format: ActiveStreamFormat,
    device_latency_ms: f32,
    dsp_latency_ms: f32,
}

struct RecoveryPlan {
    request: StartAudioRequest,
    attempts: usize,
    next_attempt: Instant,
}

impl RecoveryPlan {
    fn new(request: StartAudioRequest) -> Self {
        Self {
            request,
            attempts: 0,
            next_attempt: Instant::now() + Duration::from_millis(RECOVERY_BACKOFF_MS[0]),
        }
    }

    fn schedule_next(&mut self) -> bool {
        self.attempts += 1;
        if self.attempts >= RECOVERY_BACKOFF_MS.len() {
            return false;
        }
        self.next_attempt =
            Instant::now() + Duration::from_millis(RECOVERY_BACKOFF_MS[self.attempts]);
        true
    }
}

fn worker_loop(
    commands: Receiver<EngineCommand>,
    metrics: Arc<SharedMetrics>,
    parameters: Arc<ParameterState>,
) {
    let mut streams: Option<StreamBundle> = None;
    let mut runtime_events: Option<Receiver<RuntimeEvent>> = None;
    let mut active_request: Option<StartAudioRequest> = None;
    let mut recovery: Option<RecoveryPlan> = None;
    let mut state = EngineState::Stopped;

    loop {
        if let Some(receiver) = runtime_events.as_ref() {
            if let Ok(event) = receiver.try_recv() {
                match event {
                    RuntimeEvent::MonitorDeviceStopped(details) => {
                        let monitor_is_only_output =
                            active_request.as_ref().is_some_and(|request| {
                                matches!(request, StartAudioRequest::Test { .. })
                            });
                        if monitor_is_only_output {
                            streams = None;
                            runtime_events = None;
                            metrics.set_last_error(Some(format!(
                                "Test monitor stopped: {details}. Bounded recovery is in progress."
                            )));
                            if let Some(request) = active_request.clone() {
                                recovery = Some(RecoveryPlan::new(request));
                                transition(&mut state, EngineState::Recovering, &metrics);
                            } else {
                                transition(&mut state, EngineState::Error, &metrics);
                            }
                        } else {
                            if let Some(bundle) = streams.as_mut() {
                                bundle.monitor = None;
                            }
                            metrics.set_last_error(Some(format!(
                                "Local monitor stopped: {details}. The processed destination remains active."
                            )));
                            transition(&mut state, EngineState::Degraded, &metrics);
                        }
                    }
                    RuntimeEvent::InputDeviceStopped(details)
                    | RuntimeEvent::DestinationDeviceStopped(details) => {
                        streams = None;
                        runtime_events = None;
                        metrics.set_last_error(Some(format!(
                            "Audio stream stopped: {details}. Bounded recovery is in progress."
                        )));
                        if let Some(request) = active_request.clone() {
                            recovery = Some(RecoveryPlan::new(request));
                            transition(&mut state, EngineState::Recovering, &metrics);
                        } else {
                            transition(&mut state, EngineState::Error, &metrics);
                        }
                    }
                    RuntimeEvent::DspProducedInvalidAudio => {
                        streams = None;
                        runtime_events = None;
                        recovery = None;
                        metrics.set_last_error(Some(
                            "DSP processing produced invalid audio. Reset processing controls before restarting."
                                .to_owned(),
                        ));
                        transition(&mut state, EngineState::Error, &metrics);
                    }
                }
            }
        }

        if recovery
            .as_ref()
            .is_some_and(|plan| Instant::now() >= plan.next_attempt)
        {
            let Some(mut plan) = recovery.take() else {
                continue;
            };
            metrics.record_stream_restart();
            match start_streams(&plan.request, Arc::clone(&metrics), Arc::clone(&parameters)) {
                Ok(started) => {
                    metrics.set_stream_details(
                        started.format,
                        started.device_latency_ms,
                        started.dsp_latency_ms,
                    );
                    metrics.set_last_error(None);
                    streams = Some(started.bundle);
                    runtime_events = Some(started.runtime_events);
                    transition(&mut state, EngineState::Running, &metrics);
                }
                Err(error) => {
                    let message = error.to_string();
                    if plan.schedule_next() {
                        metrics.set_last_error(Some(format!(
                            "Recovery attempt failed: {message}. Another bounded retry is scheduled."
                        )));
                        recovery = Some(plan);
                    } else {
                        metrics.set_last_error(Some(format!(
                            "Audio recovery stopped after {} attempts: {message}",
                            RECOVERY_BACKOFF_MS.len()
                        )));
                        metrics.clear_stream_details();
                        transition(&mut state, EngineState::Error, &metrics);
                    }
                }
            }
        }

        match commands.recv_timeout(WORKER_POLL_INTERVAL) {
            Ok(EngineCommand::Start { request, reply }) => {
                if streams.take().is_some() || recovery.take().is_some() {
                    transition(&mut state, EngineState::Stopping, &metrics);
                    transition(&mut state, EngineState::Stopped, &metrics);
                }
                runtime_events = None;
                active_request = Some(request.clone());
                transition(&mut state, EngineState::Starting, &metrics);
                metrics.reset_session(request.reliability_profile(), request.route_purpose());
                metrics.set_last_error(None);
                match start_streams(&request, Arc::clone(&metrics), Arc::clone(&parameters)) {
                    Ok(started) => {
                        metrics.set_stream_details(
                            started.format,
                            started.device_latency_ms,
                            started.dsp_latency_ms,
                        );
                        streams = Some(started.bundle);
                        runtime_events = Some(started.runtime_events);
                        transition(&mut state, EngineState::Running, &metrics);
                        let _ = reply.try_send(Ok(()));
                    }
                    Err(error) => {
                        let message = error.to_string();
                        active_request = None;
                        metrics.set_route_purpose(None);
                        metrics.set_last_error(Some(message.clone()));
                        metrics.clear_stream_details();
                        transition(&mut state, EngineState::Error, &metrics);
                        let _ = reply.try_send(Err(message));
                    }
                }
            }
            Ok(EngineCommand::Stop { reply }) => {
                if streams.is_some() || recovery.is_some() {
                    transition(&mut state, EngineState::Stopping, &metrics);
                }
                streams = None;
                runtime_events = None;
                recovery = None;
                active_request = None;
                transition(&mut state, EngineState::Stopped, &metrics);
                metrics.set_route_purpose(None);
                metrics.clear_stream_details();
                let _ = reply.try_send(Ok(()));
            }
            Ok(EngineCommand::StopTest { reply }) => {
                if active_request
                    .as_ref()
                    .is_some_and(|request| matches!(request, StartAudioRequest::Test { .. }))
                {
                    if streams.is_some() || recovery.is_some() {
                        transition(&mut state, EngineState::Stopping, &metrics);
                    }
                    streams = None;
                    runtime_events = None;
                    recovery = None;
                    active_request = None;
                    transition(&mut state, EngineState::Stopped, &metrics);
                    metrics.set_route_purpose(None);
                    metrics.clear_stream_details();
                }
                let _ = reply.try_send(Ok(()));
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
}

fn transition(state: &mut EngineState, next: EngineState, metrics: &SharedMetrics) {
    if state.can_transition_to(next) {
        *state = next;
        metrics.set_state(next);
    }
}

#[allow(clippy::too_many_lines)]
fn start_streams(
    request: &StartAudioRequest,
    metrics: Arc<SharedMetrics>,
    parameters: Arc<ParameterState>,
) -> Result<StartedStreams, AudioError> {
    let (input_id, input_name) = request.input();
    let input_device = find_device_with_fallback(DeviceDirection::Input, input_id, input_name)?;
    let (destination_device, monitor_device) = match request {
        StartAudioRequest::Use {
            processed_destination_id,
            processed_destination_name,
            ..
        } => (
            Some(find_device_with_fallback(
                DeviceDirection::Output,
                processed_destination_id,
                processed_destination_name,
            )?),
            None,
        ),
        StartAudioRequest::Test {
            monitor_id,
            monitor_name,
            ..
        } => (
            None,
            Some(find_device_with_fallback(
                DeviceDirection::Output,
                monitor_id,
                monitor_name,
            )?),
        ),
    };
    let primary_output = destination_device
        .as_ref()
        .or(monitor_device.as_ref())
        .expect("a tagged audio route always has exactly one output");
    let reliability_profile = request.reliability_profile();
    let reliability = reliability_profile.config();
    let negotiated = stream_config::negotiate(
        &input_device,
        primary_output,
        reliability.requested_buffer_frames,
    )?;
    let destination_spec = destination_device
        .as_ref()
        .map(|_| negotiated.output.clone());
    let monitor_spec = monitor_device
        .as_ref()
        .map(|device| {
            if destination_device.is_none() {
                Ok(negotiated.output.clone())
            } else {
                stream_config::output_spec_at_rate(
                    device,
                    negotiated.sample_rate,
                    reliability.requested_buffer_frames,
                )
            }
        })
        .transpose()?;

    let dsp_channels = usize::from(negotiated.output.config.channels).max(1);
    let dsp_block_frames = [
        Some(negotiated.input.buffer_frames),
        destination_spec.as_ref().map(|spec| spec.buffer_frames),
        monitor_spec.as_ref().map(|spec| spec.buffer_frames),
    ]
    .into_iter()
    .flatten()
    .max()
    .unwrap_or(reliability.requested_buffer_frames) as usize;
    let input_capacity_frames = ((u64::from(negotiated.sample_rate)
        * u64::from(reliability.input_ring_milliseconds))
        / 1_000)
        .max((dsp_block_frames * 2) as u64) as usize;
    let output_capacity_frames = ((u64::from(negotiated.sample_rate)
        * u64::from(reliability.output_ring_milliseconds))
        / 1_000)
        .max((dsp_block_frames * 2) as u64) as usize;
    let prefill_target_frames = (reliability.startup_prefill_frames as usize)
        .min(output_capacity_frames / 2)
        .max(dsp_block_frames);

    let input_ring = AudioRingBuffer::new(input_capacity_frames * dsp_channels, 0);
    let (input_producer, input_consumer) = input_ring.split();

    let mut destination_consumer = None;
    let destination_target = destination_spec.as_ref().map(|spec| {
        let channels = usize::from(spec.config.channels).max(1);
        let (producer, consumer) =
            AudioRingBuffer::new(output_capacity_frames * channels, 0).split();
        destination_consumer = Some(consumer);
        OutputTarget::new(producer, channels, false)
    });
    let mut monitor_consumer = None;
    let monitor_target = monitor_spec.as_ref().map(|spec| {
        let channels = usize::from(spec.config.channels).max(1);
        let (producer, consumer) =
            AudioRingBuffer::new(output_capacity_frames * channels, 0).split();
        monitor_consumer = Some(consumer);
        OutputTarget::new(producer, channels, true)
    });

    let (runtime_tx, runtime_rx) = mpsc::sync_channel(8);
    let (dsp_worker, dsp_wake, dsp_latency_frames) = DspWorker::spawn(
        input_consumer,
        destination_target,
        monitor_target,
        Arc::clone(&parameters),
        Arc::clone(&metrics),
        runtime_tx.clone(),
        negotiated.sample_rate,
        dsp_channels,
        dsp_block_frames,
        reliability_profile,
    )?;
    let input = input_stream::build(
        &input_device,
        &negotiated.input,
        input_producer,
        dsp_channels,
        Arc::clone(&metrics),
        dsp_wake,
        runtime_tx.clone(),
    )?;
    input.play().map_err(|error| AudioError::PlayStream {
        direction: "input",
        details: error.to_string(),
    })?;

    let startup_started = Instant::now();
    let achieved = loop {
        let destination_fill = destination_consumer.as_ref().map(|consumer| {
            consumer.occupied_len()
                / usize::from(destination_spec.as_ref().unwrap().config.channels).max(1)
        });
        let monitor_fill = monitor_consumer.as_ref().map(|consumer| {
            consumer.occupied_len()
                / usize::from(monitor_spec.as_ref().unwrap().config.channels).max(1)
        });
        let achieved = [destination_fill, monitor_fill]
            .into_iter()
            .flatten()
            .min()
            .unwrap_or(0);
        match startup_prefill_decision(
            achieved,
            prefill_target_frames,
            startup_started.elapsed(),
            reliability_profile.startup_timeout(),
        ) {
            StartupPrefillDecision::Ready => break achieved,
            StartupPrefillDecision::TimedOut => {
                metrics.set_startup_prefill(prefill_target_frames, achieved, true);
                return Err(AudioError::StartupPrefillTimeout {
                    timeout_ms: reliability.startup_timeout_milliseconds,
                    achieved_frames: achieved,
                    target_frames: prefill_target_frames,
                });
            }
            StartupPrefillDecision::Waiting => {}
        }
        thread::sleep(Duration::from_millis(2));
    };
    metrics.set_startup_prefill(prefill_target_frames, achieved, false);

    let destination = match (
        destination_device.as_ref(),
        destination_spec.as_ref(),
        destination_consumer,
    ) {
        (Some(device), Some(spec), Some(consumer)) => Some(output_stream::build(
            device,
            spec,
            consumer,
            OutputRole::Destination,
            reliability.concealment_milliseconds,
            Arc::clone(&metrics),
            runtime_tx.clone(),
        )?),
        _ => None,
    };
    let monitor = match (
        monitor_device.as_ref(),
        monitor_spec.as_ref(),
        monitor_consumer,
    ) {
        (Some(device), Some(spec), Some(consumer)) => Some(output_stream::build(
            device,
            spec,
            consumer,
            OutputRole::Monitor,
            reliability.concealment_milliseconds,
            Arc::clone(&metrics),
            runtime_tx,
        )?),
        _ => None,
    };
    if let Some(stream) = destination.as_ref() {
        stream.play().map_err(|error| AudioError::PlayStream {
            direction: "processed destination",
            details: error.to_string(),
        })?;
    }
    if let Some(stream) = monitor.as_ref() {
        stream.play().map_err(|error| AudioError::PlayStream {
            direction: "local monitor",
            details: error.to_string(),
        })?;
    }

    let maximum_output_buffer = destination_spec
        .as_ref()
        .into_iter()
        .chain(monitor_spec.as_ref())
        .map(|spec| spec.buffer_frames)
        .max()
        .unwrap_or(0);
    let device_latency_ms = estimated_device_latency_ms(
        negotiated.input.buffer_frames,
        maximum_output_buffer,
        prefill_target_frames,
        negotiated.sample_rate,
    );
    let format = active_format(
        &negotiated.input,
        destination_spec.as_ref(),
        monitor_spec.as_ref(),
        negotiated.sample_rate,
        dsp_block_frames as u32,
    );
    Ok(StartedStreams {
        format,
        device_latency_ms,
        dsp_latency_ms: dsp_latency_frames as f32 * 1_000.0 / negotiated.sample_rate as f32,
        bundle: StreamBundle {
            _input: input,
            _destination: destination,
            monitor,
            _dsp_worker: dsp_worker,
        },
        runtime_events: runtime_rx,
    })
}

fn active_format(
    input: &StreamSpec,
    destination: Option<&StreamSpec>,
    monitor: Option<&StreamSpec>,
    sample_rate: u32,
    dsp_block_frames: u32,
) -> ActiveStreamFormat {
    ActiveStreamFormat {
        input_sample_rate: sample_rate,
        processed_destination_sample_rate: destination.map(|_| sample_rate),
        local_monitor_sample_rate: monitor.map(|_| sample_rate),
        input_channels: input.config.channels,
        processed_destination_channels: destination.map(|spec| spec.config.channels),
        local_monitor_channels: monitor.map(|spec| spec.config.channels),
        input_sample_format: format!("{:?}", input.sample_format),
        processed_destination_sample_format: destination
            .map(|spec| format!("{:?}", spec.sample_format)),
        local_monitor_sample_format: monitor.map(|spec| format!("{:?}", spec.sample_format)),
        input_buffer_frames: input.buffer_frames,
        processed_destination_buffer_frames: destination.map(|spec| spec.buffer_frames),
        local_monitor_buffer_frames: monitor.map(|spec| spec.buffer_frames),
        dsp_block_frames,
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{
        estimated_device_latency_ms, startup_prefill_decision, RecoveryPlan, StartAudioRequest,
        StartupPrefillDecision, RECOVERY_BACKOFF_MS,
    };
    use crate::audio::reliability::ReliabilityProfile;
    use serde_json::json;

    fn use_request() -> StartAudioRequest {
        StartAudioRequest::Use {
            input_id: "input".to_owned(),
            input_name: "Input".to_owned(),
            processed_destination_id: "destination".to_owned(),
            processed_destination_name: "Destination".to_owned(),
            reliability_profile: ReliabilityProfile::Balanced,
        }
    }

    #[test]
    fn tagged_start_requests_require_exactly_one_route_purpose() {
        assert!(use_request().validate().is_ok());
        assert!(StartAudioRequest::Test {
            input_id: "input".to_owned(),
            input_name: "Input".to_owned(),
            monitor_id: "headphones".to_owned(),
            monitor_name: "Headphones".to_owned(),
            reliability_profile: ReliabilityProfile::Balanced,
        }
        .validate()
        .is_ok());

        let ambiguous_use = json!({
            "mode": "use",
            "inputId": "input",
            "inputName": "Input",
            "processedDestinationId": "destination",
            "processedDestinationName": "Destination",
            "monitorId": "headphones",
            "monitorName": "Headphones",
            "reliabilityProfile": "balanced"
        });
        assert!(serde_json::from_value::<StartAudioRequest>(ambiguous_use).is_err());

        let test_with_destination = json!({
            "mode": "test",
            "inputId": "input",
            "inputName": "Input",
            "monitorId": "headphones",
            "monitorName": "Headphones",
            "processedDestinationId": "destination",
            "processedDestinationName": "Destination",
            "reliabilityProfile": "balanced"
        });
        assert!(serde_json::from_value::<StartAudioRequest>(test_with_destination).is_err());
    }

    #[test]
    fn recovery_attempts_and_backoff_are_strictly_bounded() {
        let mut plan = RecoveryPlan::new(use_request());
        assert_eq!(plan.attempts, 0);
        assert!(plan.schedule_next());
        assert!(plan.schedule_next());
        assert!(!plan.schedule_next());
        assert_eq!(plan.attempts, RECOVERY_BACKOFF_MS.len());
    }

    #[test]
    fn prefill_waits_for_target_and_timeout_is_bounded() {
        let timeout = Duration::from_millis(1_000);
        assert_eq!(
            startup_prefill_decision(1_023, 1_024, Duration::from_millis(999), timeout),
            StartupPrefillDecision::Waiting
        );
        assert_eq!(
            startup_prefill_decision(1_024, 1_024, Duration::from_millis(10), timeout),
            StartupPrefillDecision::Ready
        );
        assert_eq!(
            startup_prefill_decision(1_023, 1_024, timeout, timeout),
            StartupPrefillDecision::TimedOut
        );
    }

    #[test]
    fn reported_latency_includes_profile_prefill() {
        let low = estimated_device_latency_ms(128, 128, 256, 48_000);
        let balanced = estimated_device_latency_ms(256, 256, 1_024, 48_000);
        let reliable = estimated_device_latency_ms(512, 512, 2_048, 48_000);
        assert!(low < balanced && balanced < reliable);
    }
}
