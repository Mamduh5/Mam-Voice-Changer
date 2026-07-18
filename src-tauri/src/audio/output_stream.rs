use std::sync::{mpsc::SyncSender, Arc};

use cpal::traits::DeviceTrait;
use ringbuf::HeapCons;

use crate::{
    audio::{
        controller::RuntimeEvent, metrics::SharedMetrics, ring_buffer::pop_or_silence,
        sample_format::OutputSample, stream_config::StreamSpec,
    },
    dsp::{chain::DspChain, processor::AudioProcessor},
    error::AudioError,
    state::parameter_state::ParameterState,
};

pub fn build(
    device: &cpal::Device,
    spec: &StreamSpec,
    consumer: HeapCons<f32>,
    metrics: Arc<SharedMetrics>,
    parameters: Arc<ParameterState>,
    runtime_events: SyncSender<RuntimeEvent>,
) -> Result<cpal::Stream, AudioError> {
    match spec.sample_format {
        cpal::SampleFormat::F32 => {
            build_typed::<f32>(device, spec, consumer, metrics, parameters, runtime_events)
        }
        cpal::SampleFormat::I16 => {
            build_typed::<i16>(device, spec, consumer, metrics, parameters, runtime_events)
        }
        cpal::SampleFormat::U16 => {
            build_typed::<u16>(device, spec, consumer, metrics, parameters, runtime_events)
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
    parameters: Arc<ParameterState>,
    runtime_events: SyncSender<RuntimeEvent>,
) -> Result<cpal::Stream, AudioError> {
    let channels = usize::from(spec.config.channels).max(1);
    let block_samples = (spec.buffer_frames as usize * channels).max(channels);
    let mut scratch = vec![0.0_f32; block_samples];
    let mut dsp = DspChain::default();
    dsp.prepare(
        spec.config.sample_rate.0,
        channels,
        spec.buffer_frames as usize,
    );
    dsp.reset();

    device
        .build_output_stream(
            &spec.config,
            move |data: &mut [T], _| {
                let mut peak = 0.0_f32;
                let mut underrun = false;
                dsp.set_parameters(parameters.snapshot());

                for output_chunk in data.chunks_mut(scratch.len()) {
                    let samples = &mut scratch[..output_chunk.len()];
                    for sample in samples.iter_mut() {
                        let (next, missing) = pop_or_silence(&mut consumer);
                        *sample = next;
                        underrun |= missing;
                    }
                    dsp.process(samples);
                    for (output, sample) in output_chunk.iter_mut().zip(samples.iter().copied()) {
                        peak = peak.max(sample.abs());
                        *output = T::from_normalized(sample);
                    }
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
