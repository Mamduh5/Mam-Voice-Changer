use std::{
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{self, Receiver, SyncSender},
        Arc,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use cpal::traits::{DeviceTrait, StreamTrait};
use ringbuf::{traits::Consumer, HeapCons};

use crate::audio::{
    channel_mapper::mapped_sample,
    ring_buffer::{push_frame_or_drop, AudioRingBuffer},
    sample_format::InputSample,
    stream_config::StreamSpec,
};

use super::clip::{AudioClip, MAX_CLIP_SECONDS};

enum WorkerMessage {
    Wake,
    Stop,
}

pub struct CaptureHandle {
    stream: Option<cpal::Stream>,
    messages: SyncSender<WorkerMessage>,
    worker: Option<JoinHandle<Vec<f32>>>,
    finished: Arc<AtomicBool>,
    dropped_frames: Arc<AtomicU64>,
    error: Arc<std::sync::Mutex<Option<String>>>,
    sample_rate: u32,
    channels: usize,
}

impl CaptureHandle {
    pub fn start(device: &cpal::Device, spec: &StreamSpec) -> Result<Self, String> {
        let output_channels = usize::from(spec.config.channels).clamp(1, 2);
        let maximum_samples =
            spec.config.sample_rate.0 as usize * output_channels * MAX_CLIP_SECONDS;
        let capacity = (spec.buffer_frames as usize * output_channels * 8)
            .clamp(output_channels * 512, maximum_samples);
        let (producer, consumer) = AudioRingBuffer::new(capacity, 0).split();
        let (messages, receiver) = mpsc::sync_channel(1);
        let finished = Arc::new(AtomicBool::new(false));
        let worker_finished = Arc::clone(&finished);
        let worker = thread::Builder::new()
            .name("voice-lab-capture".to_owned())
            .spawn(move || drain_capture(consumer, receiver, maximum_samples, worker_finished))
            .map_err(|error| format!("Cannot start the Voice Lab capture worker: {error}"))?;
        let dropped_frames = Arc::new(AtomicU64::new(0));
        let error = Arc::new(std::sync::Mutex::new(None));
        let stream = match spec.sample_format {
            cpal::SampleFormat::F32 => build_typed::<f32>(
                device,
                spec,
                producer,
                output_channels,
                messages.clone(),
                Arc::clone(&dropped_frames),
                Arc::clone(&error),
            ),
            cpal::SampleFormat::I16 => build_typed::<i16>(
                device,
                spec,
                producer,
                output_channels,
                messages.clone(),
                Arc::clone(&dropped_frames),
                Arc::clone(&error),
            ),
            cpal::SampleFormat::U16 => build_typed::<u16>(
                device,
                spec,
                producer,
                output_channels,
                messages.clone(),
                Arc::clone(&dropped_frames),
                Arc::clone(&error),
            ),
            format => Err(format!(
                "Voice Lab cannot capture {format:?} input samples."
            )),
        }?;
        stream
            .play()
            .map_err(|error| format!("Cannot start microphone capture: {error}"))?;
        Ok(Self {
            stream: Some(stream),
            messages,
            worker: Some(worker),
            finished,
            dropped_frames,
            error,
            sample_rate: spec.config.sample_rate.0,
            channels: output_channels,
        })
    }

    pub fn is_finished(&self) -> bool {
        self.finished.load(Ordering::Acquire)
    }

    pub fn dropped_frames(&self) -> u64 {
        self.dropped_frames.load(Ordering::Relaxed)
    }

    pub fn error(&self) -> Option<String> {
        self.error.lock().ok().and_then(|error| error.clone())
    }

    pub fn finish(mut self) -> Result<AudioClip, String> {
        self.stream.take();
        let _ = self.messages.send(WorkerMessage::Stop);
        let samples = self
            .worker
            .take()
            .ok_or_else(|| "The Voice Lab capture worker is unavailable.".to_owned())?
            .join()
            .map_err(|_| "The Voice Lab capture worker stopped unexpectedly.".to_owned())?;
        if let Some(error) = self.error() {
            return Err(error);
        }
        AudioClip::new(
            "Microphone recording",
            self.sample_rate,
            self.channels,
            samples,
        )
    }
}

fn build_typed<T: InputSample>(
    device: &cpal::Device,
    spec: &StreamSpec,
    mut producer: ringbuf::HeapProd<f32>,
    output_channels: usize,
    messages: SyncSender<WorkerMessage>,
    dropped_frames: Arc<AtomicU64>,
    error: Arc<std::sync::Mutex<Option<String>>>,
) -> Result<cpal::Stream, String> {
    let input_channels = usize::from(spec.config.channels).max(1);
    let error_messages = messages.clone();
    device
        .build_input_stream(
            &spec.config,
            move |data: &[T], _| {
                let mut mapped = [0.0_f32; 2];
                for input_frame in data.chunks_exact(input_channels) {
                    for (channel, sample) in mapped[..output_channels].iter_mut().enumerate() {
                        *sample = mapped_sample(input_frame, channel, output_channels);
                    }
                    if !push_frame_or_drop(&mut producer, &mapped[..output_channels]) {
                        dropped_frames.fetch_add(1, Ordering::Relaxed);
                    }
                }
                let _ = messages.try_send(WorkerMessage::Wake);
            },
            move |stream_error| {
                if let Ok(mut slot) = error.lock() {
                    *slot = Some(format!("Microphone capture stopped: {stream_error}"));
                }
                let _ = error_messages.try_send(WorkerMessage::Stop);
            },
            None,
        )
        .map_err(|stream_error| format!("Cannot build microphone capture: {stream_error}"))
}

fn drain_capture(
    mut consumer: HeapCons<f32>,
    receiver: Receiver<WorkerMessage>,
    maximum_samples: usize,
    finished: Arc<AtomicBool>,
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
    }
    finished.store(true, Ordering::Release);
    samples
}

#[cfg(test)]
mod tests {
    use std::sync::{atomic::AtomicBool, mpsc, Arc};

    use crate::audio::{
        ring_buffer::{push_frame_or_drop, AudioRingBuffer},
        sample_format::InputSample,
    };

    use super::{drain_capture, WorkerMessage};

    #[test]
    fn capture_worker_stays_bounded_and_keeps_complete_frames() {
        let (mut producer, consumer) = AudioRingBuffer::new(8, 0).split();
        assert!(push_frame_or_drop(
            &mut producer,
            &[i16::MAX.normalized(), i16::MIN.normalized()]
        ));
        assert!(push_frame_or_drop(&mut producer, &[0.25, -0.25]));
        let (sender, receiver) = mpsc::sync_channel(1);
        sender.send(WorkerMessage::Stop).unwrap();
        let result = drain_capture(consumer, receiver, 2, Arc::new(AtomicBool::new(false)));
        assert_eq!(result, vec![1.0, -1.0]);
        assert_eq!(result.len() % 2, 0);
    }
}
