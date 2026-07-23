use std::{
    sync::{
        mpsc::{self, Receiver, RecvTimeoutError, SyncSender},
        Arc,
    },
    thread::{self, JoinHandle},
    time::Instant,
};

use ringbuf::{
    traits::{Consumer, Observer},
    HeapCons, HeapProd,
};

use crate::{
    audio::{
        channel_mapper::mapped_sample, controller::RuntimeEvent, metrics::SharedMetrics,
        reliability::ReliabilityProfile, ring_buffer::push_frame_or_drop,
    },
    dsp::{chain::DspChain, processor::AudioProcessor},
    error::AudioError,
    state::parameter_state::ParameterState,
};

const WAKE_CAPACITY: usize = 1;
const STOP_CAPACITY: usize = 1;

pub struct OutputTarget {
    producer: HeapProd<f32>,
    channels: usize,
    monitor: bool,
    frame_scratch: Vec<f32>,
}

impl OutputTarget {
    pub fn new(producer: HeapProd<f32>, channels: usize, monitor: bool) -> Self {
        let channels = channels.max(1);
        Self {
            producer,
            channels,
            monitor,
            frame_scratch: vec![0.0; channels],
        }
    }
}

pub struct DspWorker {
    stop: SyncSender<()>,
    join: Option<JoinHandle<()>>,
}

impl DspWorker {
    #[allow(clippy::too_many_arguments)]
    pub fn spawn(
        input: HeapCons<f32>,
        destination: Option<OutputTarget>,
        monitor: Option<OutputTarget>,
        parameters: Arc<ParameterState>,
        metrics: Arc<SharedMetrics>,
        runtime_events: SyncSender<RuntimeEvent>,
        sample_rate: u32,
        channels: usize,
        block_frames: usize,
        profile: ReliabilityProfile,
    ) -> Result<(Self, SyncSender<()>, usize), AudioError> {
        let mut chain = DspChain::default();
        chain.prepare(sample_rate, channels, block_frames);
        chain.reset();
        let processing_latency_frames = chain.latency_frames() + block_frames;
        let (wake_tx, wake_rx) = mpsc::sync_channel(WAKE_CAPACITY);
        let (stop_tx, stop_rx) = mpsc::sync_channel(STOP_CAPACITY);
        let join = thread::Builder::new()
            .name("mam-dsp-worker".to_owned())
            .spawn(move || {
                run(
                    input,
                    destination,
                    monitor,
                    wake_rx,
                    stop_rx,
                    parameters,
                    metrics,
                    runtime_events,
                    sample_rate,
                    channels,
                    block_frames,
                    profile,
                    chain,
                )
            })
            .map_err(|error| AudioError::DspWorkerStart(error.to_string()))?;

        Ok((
            Self {
                stop: stop_tx,
                join: Some(join),
            },
            wake_tx,
            processing_latency_frames,
        ))
    }
}

impl Drop for DspWorker {
    fn drop(&mut self) {
        let _ = self.stop.try_send(());
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn run(
    mut input: HeapCons<f32>,
    mut destination: Option<OutputTarget>,
    mut monitor: Option<OutputTarget>,
    wake: Receiver<()>,
    stop: Receiver<()>,
    parameters: Arc<ParameterState>,
    metrics: Arc<SharedMetrics>,
    runtime_events: SyncSender<RuntimeEvent>,
    sample_rate: u32,
    channels: usize,
    block_frames: usize,
    profile: ReliabilityProfile,
    mut chain: DspChain,
) {
    let channels = channels.max(1);
    let block_frames = block_frames.max(1);
    let block_samples = block_frames * channels;
    let deadline_microseconds =
        (block_frames as u64 * 1_000_000 / u64::from(sample_rate.max(1))).max(1);
    let config = profile.config();
    let mut block = vec![0.0_f32; block_samples];
    let mut insufficient_wakes = 0_u32;
    loop {
        if stop.try_recv().is_ok() {
            break;
        }

        match wake.recv_timeout(profile.worker_wake_timeout()) {
            Ok(()) => {}
            Err(RecvTimeoutError::Timeout) => continue,
            Err(RecvTimeoutError::Disconnected) => break,
        }

        if input.occupied_len() < block_samples {
            insufficient_wakes = insufficient_wakes.saturating_add(1);
            if insufficient_wakes > config.underrun_tolerance_blocks {
                metrics.record_dsp_input_underrun();
                insufficient_wakes = 0;
            }
            metrics.update_input_ring_fill(input.occupied_len() / channels);
            continue;
        }
        insufficient_wakes = 0;

        while input.occupied_len() >= block_samples {
            for sample in &mut block {
                *sample = input.try_pop().unwrap_or_default();
            }
            metrics.update_input_ring_fill(input.occupied_len() / channels);

            let started = Instant::now();
            chain.set_parameters(parameters.snapshot());
            chain.process(&mut block);
            let elapsed = started.elapsed().as_micros().min(u128::from(u64::MAX)) as u64;
            metrics.record_dsp_deadline(elapsed, elapsed > deadline_microseconds);
            metrics.record_expander_attenuated_frames(chain.take_expander_attenuated_frames());
            if block.iter().any(|sample| !sample.is_finite()) {
                let _ = runtime_events.try_send(RuntimeEvent::DspProducedInvalidAudio);
                return;
            }

            if let Some(target) = destination.as_mut() {
                write_target(&block, channels, target, &metrics);
            }
            if let Some(target) = monitor.as_mut() {
                write_target(&block, channels, target, &metrics);
            }
        }
    }
}

fn write_target(
    block: &[f32],
    source_channels: usize,
    target: &mut OutputTarget,
    metrics: &SharedMetrics,
) {
    let mut overflow = false;
    for frame in block.chunks(source_channels) {
        for channel in 0..target.channels {
            target.frame_scratch[channel] = mapped_sample(frame, channel, target.channels);
        }
        overflow |= !push_frame_or_drop(&mut target.producer, &target.frame_scratch);
    }
    if overflow {
        metrics.record_output_ring_overflow(target.monitor);
    }
    metrics.update_output_ring_fill(
        target.producer.occupied_len() / target.channels,
        target.monitor,
    );
}

#[cfg(test)]
mod tests {
    use super::{write_target, OutputTarget};
    use crate::audio::{metrics::SharedMetrics, ring_buffer::AudioRingBuffer};
    use ringbuf::traits::Consumer;

    #[test]
    fn one_processed_block_fans_out_identically_without_unbounded_queues() {
        let (destination_producer, mut destination_consumer) = AudioRingBuffer::new(8, 0).split();
        let (monitor_producer, mut monitor_consumer) = AudioRingBuffer::new(8, 0).split();
        let metrics = SharedMetrics::default();
        let block = [0.1, -0.2, 0.3, -0.4];
        let mut destination = OutputTarget::new(destination_producer, 2, false);
        let mut monitor = OutputTarget::new(monitor_producer, 2, true);

        write_target(&block, 2, &mut destination, &metrics);
        write_target(&block, 2, &mut monitor, &metrics);
        let left: Vec<_> = (0..4)
            .map(|_| destination_consumer.try_pop().unwrap())
            .collect();
        let right: Vec<_> = (0..4)
            .map(|_| monitor_consumer.try_pop().unwrap())
            .collect();
        assert_eq!(left, right);

        write_target(&[0.5; 16], 2, &mut monitor, &metrics);
        assert!(metrics.snapshot().monitor_ring_overflows > 0);
    }

    #[test]
    fn dropped_monitor_consumer_does_not_block_destination_fanout() {
        let (destination_producer, mut destination_consumer) = AudioRingBuffer::new(4, 0).split();
        let (monitor_producer, monitor_consumer) = AudioRingBuffer::new(1, 0).split();
        drop(monitor_consumer);
        let metrics = SharedMetrics::default();
        let mut destination = OutputTarget::new(destination_producer, 1, false);
        let mut monitor = OutputTarget::new(monitor_producer, 1, true);

        write_target(&[0.25, 0.5], 1, &mut monitor, &metrics);
        write_target(&[0.25, 0.5], 1, &mut destination, &metrics);
        assert_eq!(destination_consumer.try_pop(), Some(0.25));
        assert_eq!(destination_consumer.try_pop(), Some(0.5));
    }
}
