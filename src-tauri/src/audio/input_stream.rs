use std::{
    sync::{mpsc::SyncSender, Arc},
    time::{Duration, Instant},
};

use cpal::traits::DeviceTrait;
use ringbuf::{traits::Observer, HeapProd};

use crate::{
    audio::{
        channel_mapper::mapped_sample, controller::RuntimeEvent, metrics::SharedMetrics,
        ring_buffer::push_or_drop_newest, sample_format::InputSample, stream_config::StreamSpec,
    },
    error::AudioError,
};

pub fn build(
    device: &cpal::Device,
    spec: &StreamSpec,
    producer: HeapProd<f32>,
    output_channels: usize,
    metrics: Arc<SharedMetrics>,
    dsp_wake: SyncSender<()>,
    runtime_events: SyncSender<RuntimeEvent>,
) -> Result<cpal::Stream, AudioError> {
    match spec.sample_format {
        cpal::SampleFormat::F32 => build_typed::<f32>(
            device,
            spec,
            producer,
            output_channels,
            metrics,
            dsp_wake,
            runtime_events,
        ),
        cpal::SampleFormat::I16 => build_typed::<i16>(
            device,
            spec,
            producer,
            output_channels,
            metrics,
            dsp_wake,
            runtime_events,
        ),
        cpal::SampleFormat::U16 => build_typed::<u16>(
            device,
            spec,
            producer,
            output_channels,
            metrics,
            dsp_wake,
            runtime_events,
        ),
        format => Err(AudioError::BuildStream {
            direction: "input",
            details: format!("sample format {format:?} is not supported"),
        }),
    }
}

fn build_typed<T: InputSample>(
    device: &cpal::Device,
    spec: &StreamSpec,
    mut producer: HeapProd<f32>,
    output_channels: usize,
    metrics: Arc<SharedMetrics>,
    dsp_wake: SyncSender<()>,
    runtime_events: SyncSender<RuntimeEvent>,
) -> Result<cpal::Stream, AudioError> {
    let input_channels = usize::from(spec.config.channels);
    let sample_rate = spec.config.sample_rate.0.max(1);
    let mut last_callback: Option<Instant> = None;
    let callback_metrics = Arc::clone(&metrics);
    device
        .build_input_stream(
            &spec.config,
            move |data: &[T], _| {
                let now = Instant::now();
                let frames = data.len() / input_channels.max(1);
                let expected = Duration::from_secs_f64(frames as f64 / sample_rate as f64);
                if last_callback.is_some_and(|last| {
                    now.saturating_duration_since(last) > expected.saturating_mul(3)
                }) {
                    callback_metrics.record_input_callback_gap();
                }
                last_callback = Some(now);
                let mut peak = 0.0_f32;
                let mut overrun = false;
                for frame in data.chunks_exact(input_channels) {
                    for sample in frame {
                        peak = peak.max(sample.normalized().abs());
                    }
                    for output_channel in 0..output_channels {
                        let sample = mapped_sample(frame, output_channel, output_channels);
                        if !push_or_drop_newest(&mut producer, sample) {
                            overrun = true;
                        }
                    }
                }
                callback_metrics.set_input_level(peak);
                callback_metrics
                    .update_input_ring_fill(producer.occupied_len() / output_channels.max(1));
                if overrun {
                    callback_metrics.record_input_overrun();
                }
                let _ = dsp_wake.try_send(());
            },
            move |error| {
                let _ =
                    runtime_events.try_send(RuntimeEvent::InputDeviceStopped(error.to_string()));
            },
            None,
        )
        .map_err(|error| AudioError::BuildStream {
            direction: "input",
            details: error.to_string(),
        })
}
