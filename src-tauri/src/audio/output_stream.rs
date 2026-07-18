use std::sync::{mpsc::SyncSender, Arc};

use cpal::traits::DeviceTrait;
use ringbuf::HeapCons;

use crate::{
    audio::{
        controller::RuntimeEvent, metrics::SharedMetrics, ring_buffer::pop_or_silence,
        sample_format::OutputSample, stream_config::StreamSpec,
    },
    error::AudioError,
};

pub fn build(
    device: &cpal::Device,
    spec: &StreamSpec,
    consumer: HeapCons<f32>,
    metrics: Arc<SharedMetrics>,
    runtime_events: SyncSender<RuntimeEvent>,
) -> Result<cpal::Stream, AudioError> {
    match spec.sample_format {
        cpal::SampleFormat::F32 => {
            build_typed::<f32>(device, spec, consumer, metrics, runtime_events)
        }
        cpal::SampleFormat::I16 => {
            build_typed::<i16>(device, spec, consumer, metrics, runtime_events)
        }
        cpal::SampleFormat::U16 => {
            build_typed::<u16>(device, spec, consumer, metrics, runtime_events)
        }
        format => Err(AudioError::BuildStream {
            direction: "output",
            details: format!("sample format {format:?} is not supported"),
        }),
    }
}

fn build_typed<T: OutputSample>(
    device: &cpal::Device,
    spec: &StreamSpec,
    mut consumer: HeapCons<f32>,
    metrics: Arc<SharedMetrics>,
    runtime_events: SyncSender<RuntimeEvent>,
) -> Result<cpal::Stream, AudioError> {
    device
        .build_output_stream(
            &spec.config,
            move |data: &mut [T], _| {
                let mut peak = 0.0_f32;
                let mut underrun = false;
                for output in data {
                    let (sample, missing) = pop_or_silence(&mut consumer);
                    underrun |= missing;
                    peak = peak.max(sample.abs());
                    *output = T::from_normalized(sample);
                }
                metrics.set_output_level(peak);
                if underrun {
                    metrics.record_output_underrun();
                }
            },
            move |_| {
                let _ = runtime_events.try_send(RuntimeEvent::OutputStreamFailed);
            },
            None,
        )
        .map_err(|error| AudioError::BuildStream {
            direction: "output",
            details: error.to_string(),
        })
}
