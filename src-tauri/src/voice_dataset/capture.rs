use std::{
    sync::{
        atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering},
        mpsc::{self, Receiver, SyncSender},
        Arc,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use cpal::traits::{DeviceTrait, StreamTrait};
use ringbuf::{traits::Consumer, HeapCons};

use crate::audio::{
    channel_mapper::mapped_sample,
    ring_buffer::{push_frame_or_drop, AudioRingBuffer},
    sample_format::InputSample,
    stream_config::StreamSpec,
};

use super::{
    error::{DatasetError, DatasetErrorCode, DatasetResult},
    quality::CaptureMetrics,
};

pub const DATASET_MAX_TAKE_SECONDS: usize = 20;

enum WorkerMessage {
    Wake,
    Stop,
}

pub struct DatasetCaptureResult {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub metrics: CaptureMetrics,
    pub limit_reached: bool,
}

pub struct DatasetCaptureHandle {
    stream: Option<cpal::Stream>,
    messages: SyncSender<WorkerMessage>,
    worker: Option<JoinHandle<Vec<f32>>>,
    finished: Arc<AtomicBool>,
    frames_written: Arc<AtomicU64>,
    dropped_frames: Arc<AtomicU64>,
    callback_gaps: Arc<AtomicU64>,
    maximum_level_bits: Arc<AtomicU32>,
    non_finite_count: Arc<AtomicU64>,
    stream_error: Arc<std::sync::Mutex<Option<String>>>,
    sample_rate: u32,
}

impl DatasetCaptureHandle {
    pub fn start(device: &cpal::Device, spec: &StreamSpec) -> DatasetResult<Self> {
        let maximum_samples = spec.config.sample_rate.0 as usize * DATASET_MAX_TAKE_SECONDS;
        let capacity = (spec.buffer_frames as usize * 8).clamp(512, maximum_samples);
        let (producer, consumer) = AudioRingBuffer::new(capacity, 0).split();
        let (messages, receiver) = mpsc::sync_channel(1);
        let finished = Arc::new(AtomicBool::new(false));
        let frames_written = Arc::new(AtomicU64::new(0));
        let worker_finished = Arc::clone(&finished);
        let worker_frames = Arc::clone(&frames_written);
        let worker = thread::Builder::new()
            .name("voice-dataset-capture".to_owned())
            .spawn(move || {
                drain_capture(
                    consumer,
                    receiver,
                    maximum_samples,
                    worker_finished,
                    worker_frames,
                )
            })
            .map_err(|error| {
                DatasetError::new(
                    DatasetErrorCode::StorageUnavailable,
                    format!("Cannot start the dataset capture worker: {error}"),
                )
            })?;
        let dropped_frames = Arc::new(AtomicU64::new(0));
        let callback_gaps = Arc::new(AtomicU64::new(0));
        let maximum_level_bits = Arc::new(AtomicU32::new(0));
        let non_finite_count = Arc::new(AtomicU64::new(0));
        let stream_error = Arc::new(std::sync::Mutex::new(None));
        let stream = match spec.sample_format {
            cpal::SampleFormat::F32 => build_typed::<f32>(
                device,
                spec,
                producer,
                messages.clone(),
                Arc::clone(&dropped_frames),
                Arc::clone(&callback_gaps),
                Arc::clone(&maximum_level_bits),
                Arc::clone(&non_finite_count),
                Arc::clone(&stream_error),
            ),
            cpal::SampleFormat::I16 => build_typed::<i16>(
                device,
                spec,
                producer,
                messages.clone(),
                Arc::clone(&dropped_frames),
                Arc::clone(&callback_gaps),
                Arc::clone(&maximum_level_bits),
                Arc::clone(&non_finite_count),
                Arc::clone(&stream_error),
            ),
            cpal::SampleFormat::U16 => build_typed::<u16>(
                device,
                spec,
                producer,
                messages.clone(),
                Arc::clone(&dropped_frames),
                Arc::clone(&callback_gaps),
                Arc::clone(&maximum_level_bits),
                Arc::clone(&non_finite_count),
                Arc::clone(&stream_error),
            ),
            format => Err(DatasetError::new(
                DatasetErrorCode::MicrophoneUnavailable,
                format!("Dataset capture does not support {format:?} input samples."),
            )),
        }?;
        stream.play().map_err(|_| {
            DatasetError::new(
                DatasetErrorCode::MicrophoneUnavailable,
                "The selected microphone could not start.",
            )
        })?;
        Ok(Self {
            stream: Some(stream),
            messages,
            worker: Some(worker),
            finished,
            frames_written,
            dropped_frames,
            callback_gaps,
            maximum_level_bits,
            non_finite_count,
            stream_error,
            sample_rate: spec.config.sample_rate.0,
        })
    }

    pub fn is_finished(&self) -> bool {
        self.finished.load(Ordering::Acquire)
    }
    pub fn duration_ms(&self) -> u64 {
        self.frames_written.load(Ordering::Relaxed) * 1_000 / u64::from(self.sample_rate)
    }
    pub fn maximum_level(&self) -> f32 {
        f32::from_bits(self.maximum_level_bits.load(Ordering::Relaxed))
    }
    pub fn dropped_frames(&self) -> u64 {
        self.dropped_frames.load(Ordering::Relaxed)
    }
    pub fn stream_error(&self) -> Option<String> {
        self.stream_error
            .lock()
            .ok()
            .and_then(|value| value.clone())
    }

    pub fn finish(mut self) -> DatasetResult<DatasetCaptureResult> {
        self.stream.take();
        let _ = self.messages.send(WorkerMessage::Stop);
        let samples = self
            .worker
            .take()
            .ok_or_else(|| {
                DatasetError::new(
                    DatasetErrorCode::InvalidStateTransition,
                    "The dataset capture worker is unavailable.",
                )
            })?
            .join()
            .map_err(|_| {
                DatasetError::new(
                    DatasetErrorCode::InvalidStateTransition,
                    "The dataset capture worker stopped unexpectedly.",
                )
            })?;
        if self.stream_error().is_some() {
            return Err(DatasetError::new(
                DatasetErrorCode::MicrophoneUnavailable,
                "The selected microphone stopped during recording.",
            ));
        }
        let metrics = CaptureMetrics {
            callback_gaps: self.callback_gaps.load(Ordering::Relaxed),
            queue_overflow_count: self.dropped_frames.load(Ordering::Relaxed),
            dropped_frames: self.dropped_frames.load(Ordering::Relaxed),
            maximum_observed_level: self.maximum_level(),
            non_finite_input_count: self.non_finite_count.load(Ordering::Relaxed),
        };
        Ok(DatasetCaptureResult {
            limit_reached: samples.len() >= self.sample_rate as usize * DATASET_MAX_TAKE_SECONDS,
            samples,
            sample_rate: self.sample_rate,
            metrics,
        })
    }
}

#[allow(clippy::too_many_arguments)]
fn build_typed<T: InputSample>(
    device: &cpal::Device,
    spec: &StreamSpec,
    mut producer: ringbuf::HeapProd<f32>,
    messages: SyncSender<WorkerMessage>,
    dropped_frames: Arc<AtomicU64>,
    callback_gaps: Arc<AtomicU64>,
    maximum_level_bits: Arc<AtomicU32>,
    non_finite_count: Arc<AtomicU64>,
    stream_error: Arc<std::sync::Mutex<Option<String>>>,
) -> DatasetResult<cpal::Stream> {
    let channels = usize::from(spec.config.channels).max(1);
    let expected_callback =
        Duration::from_secs_f64(spec.buffer_frames as f64 / f64::from(spec.config.sample_rate.0));
    let mut previous_callback: Option<Instant> = None;
    let error_messages = messages.clone();
    device
        .build_input_stream(
            &spec.config,
            move |data: &[T], _| {
                let now = Instant::now();
                if previous_callback.is_some_and(|previous| {
                    now.duration_since(previous) > expected_callback.saturating_mul(3)
                }) {
                    callback_gaps.fetch_add(1, Ordering::Relaxed);
                }
                previous_callback = Some(now);
                for frame in data.chunks_exact(channels) {
                    let value = mapped_sample(frame, 0, 1);
                    let finite = if value.is_finite() {
                        value.clamp(-1.0, 1.0)
                    } else {
                        non_finite_count.fetch_add(1, Ordering::Relaxed);
                        0.0
                    };
                    update_maximum(&maximum_level_bits, finite.abs());
                    if !push_frame_or_drop(&mut producer, &[finite]) {
                        dropped_frames.fetch_add(1, Ordering::Relaxed);
                    }
                }
                let _ = messages.try_send(WorkerMessage::Wake);
            },
            move |_| {
                if let Ok(mut slot) = stream_error.lock() {
                    *slot = Some("microphone stream stopped".to_owned());
                }
                let _ = error_messages.try_send(WorkerMessage::Stop);
            },
            None,
        )
        .map_err(|_| {
            DatasetError::new(
                DatasetErrorCode::MicrophoneUnavailable,
                "The selected microphone stream configuration is unavailable.",
            )
        })
}

fn update_maximum(bits: &AtomicU32, value: f32) {
    let mut current = bits.load(Ordering::Relaxed);
    while f32::from_bits(current) < value {
        match bits.compare_exchange_weak(
            current,
            value.to_bits(),
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) {
            Ok(_) => break,
            Err(next) => current = next,
        }
    }
}

fn drain_capture(
    mut consumer: HeapCons<f32>,
    receiver: Receiver<WorkerMessage>,
    maximum_samples: usize,
    finished: Arc<AtomicBool>,
    frames_written: Arc<AtomicU64>,
) -> Vec<f32> {
    let mut samples = Vec::with_capacity(maximum_samples);
    let mut stopping = false;
    while !stopping && samples.len() < maximum_samples {
        match receiver.recv_timeout(Duration::from_millis(20)) {
            Ok(WorkerMessage::Stop) => stopping = true,
            Ok(WorkerMessage::Wake) | Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => stopping = true,
        }
        while samples.len() < maximum_samples {
            let Some(sample) = consumer.try_pop() else {
                break;
            };
            samples.push(sample);
        }
        frames_written.store(samples.len() as u64, Ordering::Relaxed);
    }
    finished.store(true, Ordering::Release);
    samples
}

#[cfg(test)]
mod tests {
    use super::{drain_capture, WorkerMessage};
    use crate::audio::ring_buffer::AudioRingBuffer;
    use std::sync::{
        atomic::{AtomicBool, AtomicU64},
        mpsc, Arc,
    };

    #[test]
    fn dataset_capture_worker_is_bounded() {
        let (mut producer, consumer) = AudioRingBuffer::new(8, 0).split();
        use ringbuf::traits::Producer;
        for value in [0.1, 0.2, 0.3] {
            producer.try_push(value).unwrap();
        }
        let (sender, receiver) = mpsc::sync_channel(1);
        sender.send(WorkerMessage::Stop).unwrap();
        let result = drain_capture(
            consumer,
            receiver,
            2,
            Arc::new(AtomicBool::new(false)),
            Arc::new(AtomicU64::new(0)),
        );
        assert_eq!(result, vec![0.1, 0.2]);
    }
}
