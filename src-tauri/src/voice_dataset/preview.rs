use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc, Mutex,
};

use cpal::traits::{DeviceTrait, StreamTrait};

use crate::audio::{sample_format::OutputSample, stream_config::StreamSpec};

use super::error::{DatasetError, DatasetErrorCode, DatasetResult};

const FADE_MS: usize = 5;

pub struct DatasetPreviewHandle {
    _stream: cpal::Stream,
    cursor: Arc<AtomicUsize>,
    paused: Arc<AtomicBool>,
    finished: Arc<AtomicBool>,
    error: Arc<Mutex<Option<String>>>,
    frames: usize,
    sample_rate: u32,
}

impl DatasetPreviewHandle {
    pub fn start(
        device: &cpal::Device,
        spec: &StreamSpec,
        samples: Arc<Vec<f32>>,
        seek_frame: usize,
    ) -> DatasetResult<Self> {
        let cursor = Arc::new(AtomicUsize::new(seek_frame.min(samples.len())));
        let paused = Arc::new(AtomicBool::new(false));
        let finished = Arc::new(AtomicBool::new(false));
        let error = Arc::new(Mutex::new(None));
        let stream = match spec.sample_format {
            cpal::SampleFormat::F32 => build_typed::<f32>(
                device,
                spec,
                Arc::clone(&samples),
                Arc::clone(&cursor),
                Arc::clone(&paused),
                Arc::clone(&finished),
                Arc::clone(&error),
            ),
            cpal::SampleFormat::I16 => build_typed::<i16>(
                device,
                spec,
                Arc::clone(&samples),
                Arc::clone(&cursor),
                Arc::clone(&paused),
                Arc::clone(&finished),
                Arc::clone(&error),
            ),
            cpal::SampleFormat::U16 => build_typed::<u16>(
                device,
                spec,
                Arc::clone(&samples),
                Arc::clone(&cursor),
                Arc::clone(&paused),
                Arc::clone(&finished),
                Arc::clone(&error),
            ),
            _ => Err(DatasetError::new(
                DatasetErrorCode::PreviewFailed,
                "The selected output format cannot preview dataset takes.",
            )),
        }?;
        stream.play().map_err(|_| {
            DatasetError::new(
                DatasetErrorCode::PreviewFailed,
                "The selected preview output could not start.",
            )
        })?;
        Ok(Self {
            _stream: stream,
            cursor,
            paused,
            finished,
            error,
            frames: samples.len(),
            sample_rate: spec.config.sample_rate.0,
        })
    }

    pub fn toggle_pause(&self) -> bool {
        let next = !self.paused.load(Ordering::Acquire);
        self.paused.store(next, Ordering::Release);
        next
    }
    pub fn is_paused(&self) -> bool {
        self.paused.load(Ordering::Acquire)
    }
    pub fn is_finished(&self) -> bool {
        self.finished.load(Ordering::Acquire)
    }
    pub fn position_ms(&self) -> u64 {
        self.cursor.load(Ordering::Relaxed) as u64 * 1_000 / u64::from(self.sample_rate)
    }
    pub fn duration_ms(&self) -> u64 {
        self.frames as u64 * 1_000 / u64::from(self.sample_rate)
    }
    pub fn error(&self) -> Option<String> {
        self.error.lock().ok().and_then(|value| value.clone())
    }
}

fn build_typed<T: OutputSample>(
    device: &cpal::Device,
    spec: &StreamSpec,
    samples: Arc<Vec<f32>>,
    cursor: Arc<AtomicUsize>,
    paused: Arc<AtomicBool>,
    finished: Arc<AtomicBool>,
    error: Arc<Mutex<Option<String>>>,
) -> DatasetResult<cpal::Stream> {
    let output_channels = usize::from(spec.config.channels).max(1);
    let fade_frames = (spec.config.sample_rate.0 as usize * FADE_MS / 1_000).max(1);
    let callback_finished = Arc::clone(&finished);
    device
        .build_output_stream(
            &spec.config,
            move |output: &mut [T], _| {
                for frame in output.chunks_mut(output_channels) {
                    if paused.load(Ordering::Acquire) {
                        frame.fill(T::from_normalized(0.0));
                        continue;
                    }
                    let index = cursor.fetch_add(1, Ordering::Relaxed);
                    if index >= samples.len() {
                        frame.fill(T::from_normalized(0.0));
                        callback_finished.store(true, Ordering::Release);
                        continue;
                    }
                    let edge = index.min(samples.len() - index - 1);
                    let gain = (edge as f32 / fade_frames as f32).clamp(0.0, 1.0);
                    frame.fill(T::from_normalized(samples[index] * gain));
                }
            },
            move |_| {
                if let Ok(mut slot) = error.lock() {
                    *slot = Some("preview output stopped".to_owned());
                }
                finished.store(true, Ordering::Release);
            },
            None,
        )
        .map_err(|_| {
            DatasetError::new(
                DatasetErrorCode::PreviewFailed,
                "The selected preview output configuration is unavailable.",
            )
        })
}

#[cfg(test)]
mod tests {
    #[test]
    fn fade_constant_is_short_and_nonzero() {
        assert_eq!(super::FADE_MS, 5);
    }
}
