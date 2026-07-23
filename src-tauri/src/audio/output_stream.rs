use std::{
    sync::{mpsc::SyncSender, Arc},
    time::{Duration, Instant},
};

use cpal::traits::DeviceTrait;
use ringbuf::{
    traits::{Consumer, Observer},
    HeapCons,
};

use crate::{
    audio::{
        controller::RuntimeEvent, dropout_concealment::DropoutConcealer, metrics::SharedMetrics,
        sample_format::OutputSample, stream_config::StreamSpec,
    },
    error::AudioError,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OutputRole {
    Destination,
    Monitor,
}

impl OutputRole {
    pub const fn is_monitor(self) -> bool {
        matches!(self, Self::Monitor)
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Destination => "processed destination",
            Self::Monitor => "local monitor",
        }
    }
}

pub fn build(
    device: &cpal::Device,
    spec: &StreamSpec,
    consumer: HeapCons<f32>,
    role: OutputRole,
    concealment_milliseconds: u32,
    metrics: Arc<SharedMetrics>,
    runtime_events: SyncSender<RuntimeEvent>,
) -> Result<cpal::Stream, AudioError> {
    match spec.sample_format {
        cpal::SampleFormat::F32 => build_typed::<f32>(
            device,
            spec,
            consumer,
            role,
            concealment_milliseconds,
            metrics,
            runtime_events,
        ),
        cpal::SampleFormat::I16 => build_typed::<i16>(
            device,
            spec,
            consumer,
            role,
            concealment_milliseconds,
            metrics,
            runtime_events,
        ),
        cpal::SampleFormat::U16 => build_typed::<u16>(
            device,
            spec,
            consumer,
            role,
            concealment_milliseconds,
            metrics,
            runtime_events,
        ),
        format => Err(AudioError::BuildStream {
            direction: role.label(),
            details: format!("sample format {format:?} is not supported"),
        }),
    }
}

#[allow(clippy::too_many_arguments)]
fn build_typed<T: OutputSample>(
    device: &cpal::Device,
    spec: &StreamSpec,
    mut consumer: HeapCons<f32>,
    role: OutputRole,
    concealment_milliseconds: u32,
    metrics: Arc<SharedMetrics>,
    runtime_events: SyncSender<RuntimeEvent>,
) -> Result<cpal::Stream, AudioError> {
    let channels = usize::from(spec.config.channels).max(1);
    let sample_rate = spec.config.sample_rate.0.max(1);
    let mut concealer = DropoutConcealer::new(channels, sample_rate, concealment_milliseconds);
    let mut real_frame = vec![0.0_f32; channels];
    let mut output_frame = vec![0.0_f32; channels];
    let mut last_callback: Option<Instant> = None;
    let callback_metrics = Arc::clone(&metrics);
    device
        .build_output_stream(
            &spec.config,
            move |data: &mut [T], _| {
                let now = Instant::now();
                let frames = data.len() / channels;
                let expected = Duration::from_secs_f64(frames as f64 / sample_rate as f64);
                if last_callback.is_some_and(|last| {
                    now.saturating_duration_since(last) > expected.saturating_mul(3)
                }) {
                    callback_metrics.record_output_callback_gap(role.is_monitor());
                }
                last_callback = Some(now);

                let mut peak = 0.0_f32;
                let mut underrun = false;
                let mut concealed = 0;
                for frame in data.chunks_mut(channels) {
                    let available = consumer.occupied_len() >= channels;
                    if available {
                        for sample in &mut real_frame {
                            *sample = consumer.try_pop().unwrap_or_default();
                        }
                    }
                    let was_concealed = concealer.process_frame(
                        available.then_some(real_frame.as_slice()),
                        &mut output_frame,
                    );
                    underrun |= was_concealed;
                    concealed += usize::from(was_concealed);
                    for (output, sample) in frame.iter_mut().zip(&output_frame) {
                        peak = peak.max(sample.abs());
                        *output = T::from_normalized(*sample);
                    }
                }
                callback_metrics.set_output_level(peak, role.is_monitor());
                callback_metrics
                    .update_output_ring_fill(consumer.occupied_len() / channels, role.is_monitor());
                if underrun {
                    callback_metrics.record_output_underrun(role.is_monitor());
                }
                if concealed > 0 {
                    callback_metrics.record_concealed_frames(concealed, role.is_monitor());
                }
            },
            move |error| {
                let event = match role {
                    OutputRole::Destination => {
                        RuntimeEvent::DestinationDeviceStopped(error.to_string())
                    }
                    OutputRole::Monitor => RuntimeEvent::MonitorDeviceStopped(error.to_string()),
                };
                let _ = runtime_events.try_send(event);
            },
            None,
        )
        .map_err(|error| AudioError::BuildStream {
            direction: role.label(),
            details: error.to_string(),
        })
}
