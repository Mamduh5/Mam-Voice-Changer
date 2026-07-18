use std::{
    sync::{
        mpsc::{self, Receiver, RecvTimeoutError, SyncSender},
        Arc,
    },
    thread,
    time::Duration,
};

use cpal::traits::StreamTrait;
use serde::Deserialize;

use crate::{
    audio::{
        device::{find_device, DeviceDirection},
        input_stream,
        metrics::{EngineStatus, SharedMetrics},
        output_stream,
        ring_buffer::AudioRingBuffer,
        stream_config::{self, ActiveStreamFormat},
    },
    error::AudioError,
    state::{engine_state::EngineState, parameter_state::ParameterState},
};

const COMMAND_TIMEOUT: Duration = Duration::from_secs(10);
const WORKER_POLL_INTERVAL: Duration = Duration::from_millis(20);
const RING_BUFFER_MILLISECONDS: u32 = 250;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartRequest {
    pub input_id: String,
    pub output_id: String,
}

enum EngineCommand {
    Start {
        request: StartRequest,
        reply: SyncSender<Result<(), String>>,
    },
    Stop {
        reply: SyncSender<Result<(), String>>,
    },
}

#[derive(Clone, Copy)]
pub enum RuntimeEvent {
    InputStreamFailed,
    OutputStreamFailed,
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

    pub fn start(&self, request: StartRequest) -> Result<(), String> {
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
    _output: cpal::Stream,
}

struct StartedStreams {
    bundle: StreamBundle,
    runtime_events: Receiver<RuntimeEvent>,
    format: ActiveStreamFormat,
    estimated_latency_ms: f32,
}

fn worker_loop(
    commands: Receiver<EngineCommand>,
    metrics: Arc<SharedMetrics>,
    parameters: Arc<ParameterState>,
) {
    let mut streams: Option<StreamBundle> = None;
    let mut runtime_events: Option<Receiver<RuntimeEvent>> = None;
    let mut state = EngineState::Stopped;

    loop {
        if let Some(receiver) = runtime_events.as_ref() {
            if let Ok(event) = receiver.try_recv() {
                streams = None;
                runtime_events = None;
                let message = match event {
                    RuntimeEvent::InputStreamFailed => {
                        "The input device stopped responding. Refresh devices and restart the engine."
                    }
                    RuntimeEvent::OutputStreamFailed => {
                        "The output device stopped responding or was disconnected. Refresh devices and restart the engine."
                    }
                };
                metrics.set_last_error(Some(message.to_owned()));
                transition(&mut state, EngineState::Error, &metrics);
                metrics.clear_stream_details();
            }
        }

        match commands.recv_timeout(WORKER_POLL_INTERVAL) {
            Ok(EngineCommand::Start { request, reply }) => {
                if streams.take().is_some() {
                    transition(&mut state, EngineState::Stopping, &metrics);
                    transition(&mut state, EngineState::Stopped, &metrics);
                }
                runtime_events = None;
                transition(&mut state, EngineState::Starting, &metrics);
                metrics.reset_counters();
                metrics.set_last_error(None);
                match start_streams(&request, Arc::clone(&metrics), Arc::clone(&parameters)) {
                    Ok(started) => {
                        metrics.set_stream_details(started.format, started.estimated_latency_ms);
                        streams = Some(started.bundle);
                        runtime_events = Some(started.runtime_events);
                        transition(&mut state, EngineState::Running, &metrics);
                        let _ = reply.try_send(Ok(()));
                    }
                    Err(error) => {
                        let message = error.to_string();
                        metrics.set_last_error(Some(message.clone()));
                        metrics.clear_stream_details();
                        transition(&mut state, EngineState::Error, &metrics);
                        let _ = reply.try_send(Err(message));
                    }
                }
            }
            Ok(EngineCommand::Stop { reply }) => {
                if streams.is_some() {
                    transition(&mut state, EngineState::Stopping, &metrics);
                    streams = None;
                }
                runtime_events = None;
                transition(&mut state, EngineState::Stopped, &metrics);
                metrics.clear_stream_details();
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

fn start_streams(
    request: &StartRequest,
    metrics: Arc<SharedMetrics>,
    parameters: Arc<ParameterState>,
) -> Result<StartedStreams, AudioError> {
    let input_device = find_device(DeviceDirection::Input, &request.input_id)?;
    let output_device = find_device(DeviceDirection::Output, &request.output_id)?;
    let negotiated = stream_config::negotiate(&input_device, &output_device)?;
    let output_channels = usize::from(negotiated.output.config.channels);
    let prefill_frames = negotiated
        .input
        .buffer_frames
        .max(negotiated.output.buffer_frames)
        * 2;
    let prefill_samples = prefill_frames as usize * output_channels;
    let capacity_frames =
        (negotiated.sample_rate * RING_BUFFER_MILLISECONDS / 1_000).max(prefill_frames * 2);
    let ring = AudioRingBuffer::new(capacity_frames as usize * output_channels, prefill_samples);
    let (producer, consumer) = ring.split();
    let (runtime_tx, runtime_rx) = mpsc::sync_channel(4);

    let input = input_stream::build(
        &input_device,
        &negotiated.input,
        producer,
        output_channels,
        Arc::clone(&metrics),
        runtime_tx.clone(),
    )?;
    let output = output_stream::build(
        &output_device,
        &negotiated.output,
        consumer,
        metrics,
        parameters,
        runtime_tx,
    )?;

    input.play().map_err(|error| AudioError::PlayStream {
        direction: "input",
        details: error.to_string(),
    })?;
    output.play().map_err(|error| AudioError::PlayStream {
        direction: "output",
        details: error.to_string(),
    })?;

    Ok(StartedStreams {
        format: negotiated.active_format(),
        estimated_latency_ms: negotiated.estimated_latency_ms(prefill_frames),
        bundle: StreamBundle {
            _input: input,
            _output: output,
        },
        runtime_events: runtime_rx,
    })
}
