use std::{
    sync::{
        mpsc::{self, Receiver, RecvTimeoutError, SyncSender},
        Arc,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use ringbuf::{
    traits::{Consumer, Observer},
    HeapCons, HeapProd,
};

use crate::{
    audio::{controller::RuntimeEvent, metrics::SharedMetrics, ring_buffer::push_or_drop_newest},
    dsp::{chain::DspChain, processor::AudioProcessor},
    error::AudioError,
    state::parameter_state::ParameterState,
};

const WAKE_CAPACITY: usize = 1;
const STOP_CAPACITY: usize = 1;
const WAKE_TIMEOUT: Duration = Duration::from_millis(20);

pub struct DspWorker {
    stop: SyncSender<()>,
    join: Option<JoinHandle<()>>,
}

impl DspWorker {
    #[allow(clippy::too_many_arguments)]
    pub fn spawn(
        input: HeapCons<f32>,
        output: HeapProd<f32>,
        parameters: Arc<ParameterState>,
        metrics: Arc<SharedMetrics>,
        runtime_events: SyncSender<RuntimeEvent>,
        sample_rate: u32,
        channels: usize,
        block_frames: usize,
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
                    output,
                    wake_rx,
                    stop_rx,
                    parameters,
                    metrics,
                    runtime_events,
                    channels,
                    block_frames,
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
    mut output: HeapProd<f32>,
    wake: Receiver<()>,
    stop: Receiver<()>,
    parameters: Arc<ParameterState>,
    metrics: Arc<SharedMetrics>,
    runtime_events: SyncSender<RuntimeEvent>,
    channels: usize,
    block_frames: usize,
    mut chain: DspChain,
) {
    let block_samples = block_frames.max(1) * channels.max(1);
    let mut block = vec![0.0_f32; block_samples];
    loop {
        if stop.try_recv().is_ok() {
            break;
        }

        match wake.recv_timeout(WAKE_TIMEOUT) {
            Ok(()) => {}
            Err(RecvTimeoutError::Timeout) => continue,
            Err(RecvTimeoutError::Disconnected) => break,
        }

        if input.occupied_len() < block_samples {
            metrics.record_dsp_input_underrun();
            continue;
        }

        while input.occupied_len() >= block_samples {
            for sample in &mut block {
                *sample = input.try_pop().unwrap_or_default();
            }

            chain.set_parameters(parameters.snapshot());
            chain.process(&mut block);
            if block.iter().any(|sample| !sample.is_finite()) {
                let _ = runtime_events.try_send(RuntimeEvent::DspProducedInvalidAudio);
                return;
            }

            let mut overrun = false;
            for sample in block.iter().copied() {
                overrun |= !push_or_drop_newest(&mut output, sample);
            }
            if overrun {
                metrics.record_dsp_output_overrun();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{mpsc, Arc};

    use ringbuf::traits::Producer;

    use super::DspWorker;
    use crate::{
        audio::{controller::RuntimeEvent, metrics::SharedMetrics, ring_buffer::AudioRingBuffer},
        state::parameter_state::ParameterState,
    };

    #[test]
    fn reports_input_underflow_and_output_overflow() {
        let input_ring = AudioRingBuffer::new(8, 0);
        let (mut input, input_consumer) = input_ring.split();
        let output_ring = AudioRingBuffer::new(1, 1);
        let (output_producer, _output) = output_ring.split();
        let metrics = Arc::new(SharedMetrics::default());
        let (events, _event_rx) = mpsc::sync_channel::<RuntimeEvent>(1);
        let (worker, wake, _) = DspWorker::spawn(
            input_consumer,
            output_producer,
            Arc::new(ParameterState::default()),
            Arc::clone(&metrics),
            events,
            48_000,
            1,
            2,
        )
        .unwrap();

        input.try_push(0.25).unwrap();
        wake.send(()).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(30));
        input.try_push(0.25).unwrap();
        input.try_push(0.25).unwrap();
        wake.send(()).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(30));
        drop(worker);

        let status = metrics.snapshot();
        assert!(status.dsp_input_underruns >= 1);
        assert!(status.dsp_output_overruns >= 1);
    }
}

