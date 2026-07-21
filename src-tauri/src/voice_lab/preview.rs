use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc, Mutex,
};

use cpal::traits::{DeviceTrait, StreamTrait};

use crate::audio::{sample_format::OutputSample, stream_config::StreamSpec};

use super::clip::AudioClip;

pub struct PreviewHandle {
    _stream: cpal::Stream,
    cursor: Arc<AtomicUsize>,
    finished: Arc<AtomicBool>,
    error: Arc<Mutex<Option<String>>>,
    frames: usize,
    sample_rate: u32,
    pub kind: String,
    pub looping: bool,
}

impl PreviewHandle {
    pub fn start(
        device: &cpal::Device,
        spec: &StreamSpec,
        clip: Arc<AudioClip>,
        kind: String,
        looping: bool,
    ) -> Result<Self, String> {
        let cursor = Arc::new(AtomicUsize::new(0));
        let finished = Arc::new(AtomicBool::new(false));
        let error = Arc::new(Mutex::new(None));
        let stream = match spec.sample_format {
            cpal::SampleFormat::F32 => build_typed::<f32>(
                device,
                spec,
                Arc::clone(&clip),
                looping,
                Arc::clone(&cursor),
                Arc::clone(&finished),
                Arc::clone(&error),
            ),
            cpal::SampleFormat::I16 => build_typed::<i16>(
                device,
                spec,
                Arc::clone(&clip),
                looping,
                Arc::clone(&cursor),
                Arc::clone(&finished),
                Arc::clone(&error),
            ),
            cpal::SampleFormat::U16 => build_typed::<u16>(
                device,
                spec,
                Arc::clone(&clip),
                looping,
                Arc::clone(&cursor),
                Arc::clone(&finished),
                Arc::clone(&error),
            ),
            format => Err(format!(
                "Voice Lab cannot preview {format:?} output samples."
            )),
        }?;
        stream
            .play()
            .map_err(|stream_error| format!("Cannot start Voice Lab preview: {stream_error}"))?;
        Ok(Self {
            _stream: stream,
            cursor,
            finished,
            error,
            frames: clip.frames(),
            sample_rate: clip.sample_rate,
            kind,
            looping,
        })
    }

    pub fn is_finished(&self) -> bool {
        self.finished.load(Ordering::Acquire)
    }

    pub fn position_ms(&self) -> u64 {
        (self.cursor.load(Ordering::Relaxed) as u64 * 1_000) / u64::from(self.sample_rate)
    }

    pub fn duration_ms(&self) -> u64 {
        (self.frames as u64 * 1_000) / u64::from(self.sample_rate)
    }

    pub fn error(&self) -> Option<String> {
        self.error.lock().ok().and_then(|error| error.clone())
    }
}

fn build_typed<T: OutputSample>(
    device: &cpal::Device,
    spec: &StreamSpec,
    clip: Arc<AudioClip>,
    looping: bool,
    cursor: Arc<AtomicUsize>,
    finished: Arc<AtomicBool>,
    error: Arc<Mutex<Option<String>>>,
) -> Result<cpal::Stream, String> {
    let output_channels = usize::from(spec.config.channels).max(1);
    let mut frame_cursor = 0_usize;
    let callback_cursor = Arc::clone(&cursor);
    let callback_finished = Arc::clone(&finished);
    device
        .build_output_stream(
            &spec.config,
            move |data: &mut [T], _| {
                for output_frame in data.chunks_mut(output_channels) {
                    let Some(source_frame) = next_frame(&mut frame_cursor, clip.frames(), looping)
                    else {
                        output_frame.fill(T::from_normalized(0.0));
                        callback_finished.store(true, Ordering::Release);
                        continue;
                    };
                    for (channel, output) in output_frame.iter_mut().enumerate() {
                        let source =
                            mapped_clip_sample(&clip, source_frame, channel, output_channels);
                        *output = T::from_normalized(source);
                    }
                    callback_cursor.store(frame_cursor, Ordering::Relaxed);
                }
            },
            move |stream_error| {
                if let Ok(mut slot) = error.lock() {
                    *slot = Some(format!("Voice Lab preview stopped: {stream_error}"));
                }
                finished.store(true, Ordering::Release);
            },
            None,
        )
        .map_err(|stream_error| format!("Cannot build Voice Lab preview: {stream_error}"))
}

fn next_frame(cursor: &mut usize, frames: usize, looping: bool) -> Option<usize> {
    if frames == 0 {
        return None;
    }
    if *cursor >= frames {
        if looping {
            *cursor = 0;
        } else {
            return None;
        }
    }
    let current = *cursor;
    *cursor += 1;
    Some(current)
}

fn mapped_clip_sample(
    clip: &AudioClip,
    frame: usize,
    output_channel: usize,
    output_channels: usize,
) -> f32 {
    let source = &clip.samples[frame * clip.channels..(frame + 1) * clip.channels];
    match (clip.channels, output_channels) {
        (_, 1) => source.iter().sum::<f32>() / source.len() as f32,
        (1, _) => source[0],
        (_, _) if output_channel < source.len() => source[output_channel],
        _ => source.iter().sum::<f32>() / source.len() as f32,
    }
}

#[cfg(test)]
mod tests {
    use super::{mapped_clip_sample, next_frame};
    use crate::voice_lab::clip::AudioClip;

    #[test]
    fn preview_cursor_stops_or_loops_at_the_clip_boundary() {
        let mut cursor = 0;
        assert_eq!(next_frame(&mut cursor, 2, false), Some(0));
        assert_eq!(next_frame(&mut cursor, 2, false), Some(1));
        assert_eq!(next_frame(&mut cursor, 2, false), None);
        assert_eq!(next_frame(&mut cursor, 2, true), Some(0));
    }

    #[test]
    fn preview_maps_mono_and_stereo_without_live_channel_state() {
        let mono = AudioClip::new("mono", 48_000, 1, vec![0.4]).unwrap();
        assert_eq!(mapped_clip_sample(&mono, 0, 1, 2), 0.4);
        let stereo = AudioClip::new("stereo", 48_000, 2, vec![0.6, -0.2]).unwrap();
        assert!((mapped_clip_sample(&stereo, 0, 0, 1) - 0.2).abs() < 0.0001);
    }
}
