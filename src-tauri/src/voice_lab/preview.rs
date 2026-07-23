use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc, Mutex,
};

use cpal::traits::{DeviceTrait, StreamTrait};

use crate::audio::{
    offline_resampler::resample_linear_offline, sample_format::OutputSample,
    stream_config::StreamSpec,
};

use super::clip::{AudioClip, MAX_CLIP_SECONDS};

const PREVIEW_FADE_MS: usize = 5;

#[derive(Clone, Debug)]
pub struct PreparedPreview {
    pub source_clip_id: String,
    pub source_sample_rate: u32,
    pub output_sample_rate: u32,
    pub channels: usize,
    pub frame_count: usize,
    pub samples: Vec<f32>,
}

impl PreparedPreview {
    pub fn prepare(clip: &AudioClip, output_sample_rate: u32) -> Result<Self, String> {
        let maximum_frames = (output_sample_rate as usize)
            .checked_mul(MAX_CLIP_SECONDS)
            .ok_or_else(|| "the selected output rate exceeds preview limits".to_owned())?;
        let samples = resample_linear_offline(
            &clip.samples,
            clip.channels,
            clip.sample_rate,
            output_sample_rate,
            maximum_frames,
        )
        .map_err(|error| format!("Voice Lab preview resampling failed: {error}"))?;
        if samples.is_empty() || samples.iter().any(|sample| !sample.is_finite()) {
            return Err("Voice Lab preview preparation produced invalid audio.".to_owned());
        }
        Ok(Self {
            source_clip_id: clip.id.clone(),
            source_sample_rate: clip.sample_rate,
            output_sample_rate,
            channels: clip.channels,
            frame_count: samples.len() / clip.channels,
            samples,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreviewCacheKey {
    pub source_clip_id: String,
    pub output_device_id: String,
    pub output_sample_rate: u32,
    pub output_channels: u16,
    pub output_sample_format: String,
}

#[derive(Default)]
pub struct PreparedPreviewCache {
    entry: Option<(PreviewCacheKey, Arc<PreparedPreview>)>,
}

impl PreparedPreviewCache {
    pub fn get(&self, key: &PreviewCacheKey) -> Option<Arc<PreparedPreview>> {
        self.entry
            .as_ref()
            .and_then(|(cached_key, preview)| (cached_key == key).then(|| Arc::clone(preview)))
    }

    pub fn store(
        &mut self,
        key: PreviewCacheKey,
        preview: PreparedPreview,
    ) -> Arc<PreparedPreview> {
        let preview = Arc::new(preview);
        self.entry = Some((key, Arc::clone(&preview)));
        preview
    }

    pub fn invalidate(&mut self) {
        self.entry = None;
    }

    #[cfg(test)]
    fn contains(&self, key: &PreviewCacheKey) -> bool {
        self.entry
            .as_ref()
            .is_some_and(|(cached_key, _)| cached_key == key)
    }
}

pub struct PreviewHandle {
    _stream: cpal::Stream,
    cursor: Arc<AtomicUsize>,
    finished: Arc<AtomicBool>,
    error: Arc<Mutex<Option<String>>>,
    frames: usize,
    pub source_sample_rate: u32,
    pub output_sample_rate: u32,
    pub output_channels: u16,
    pub output_sample_format: String,
    pub kind: String,
    pub looping: bool,
}

impl PreviewHandle {
    #[allow(clippy::too_many_arguments)]
    pub fn start(
        device: &cpal::Device,
        spec: &StreamSpec,
        prepared: Arc<PreparedPreview>,
        kind: String,
        looping: bool,
        initial_frame: usize,
        output_sample_format: String,
    ) -> Result<Self, String> {
        if prepared.source_clip_id.is_empty() {
            return Err(
                "Voice Lab preview preparation failed: the source clip identity is missing."
                    .to_owned(),
            );
        }
        let bounded_initial_frame = initial_frame.min(prepared.frame_count.saturating_sub(1));
        let cursor = Arc::new(AtomicUsize::new(bounded_initial_frame));
        let finished = Arc::new(AtomicBool::new(false));
        let error = Arc::new(Mutex::new(None));
        let stream = match spec.sample_format {
            cpal::SampleFormat::F32 => build_typed::<f32>(
                device,
                spec,
                Arc::clone(&prepared),
                looping,
                bounded_initial_frame,
                Arc::clone(&cursor),
                Arc::clone(&finished),
                Arc::clone(&error),
            ),
            cpal::SampleFormat::I16 => build_typed::<i16>(
                device,
                spec,
                Arc::clone(&prepared),
                looping,
                bounded_initial_frame,
                Arc::clone(&cursor),
                Arc::clone(&finished),
                Arc::clone(&error),
            ),
            cpal::SampleFormat::U16 => build_typed::<u16>(
                device,
                spec,
                Arc::clone(&prepared),
                looping,
                bounded_initial_frame,
                Arc::clone(&cursor),
                Arc::clone(&finished),
                Arc::clone(&error),
            ),
            format => Err(format!(
                "Voice Lab found no supported output sample converter for {format:?}."
            )),
        }?;
        stream.play().map_err(|stream_error| {
            format!("Voice Lab output stream start failed: {stream_error}")
        })?;
        Ok(Self {
            _stream: stream,
            cursor,
            finished,
            error,
            frames: prepared.frame_count,
            source_sample_rate: prepared.source_sample_rate,
            output_sample_rate: prepared.output_sample_rate,
            output_channels: spec.config.channels,
            output_sample_format,
            kind,
            looping,
        })
    }

    pub fn is_finished(&self) -> bool {
        self.finished.load(Ordering::Acquire)
    }

    pub fn position_frame(&self) -> usize {
        self.cursor.load(Ordering::Relaxed).min(self.frames)
    }

    pub fn position_ms(&self) -> u64 {
        (self.position_frame() as u64 * 1_000) / u64::from(self.output_sample_rate)
    }

    pub fn duration_ms(&self) -> u64 {
        (self.frames as u64 * 1_000) / u64::from(self.output_sample_rate)
    }

    pub fn error(&self) -> Option<String> {
        self.error.lock().ok().and_then(|error| error.clone())
    }
}

#[allow(clippy::too_many_arguments)]
fn build_typed<T: OutputSample>(
    device: &cpal::Device,
    spec: &StreamSpec,
    prepared: Arc<PreparedPreview>,
    looping: bool,
    initial_frame: usize,
    cursor: Arc<AtomicUsize>,
    finished: Arc<AtomicBool>,
    error: Arc<Mutex<Option<String>>>,
) -> Result<cpal::Stream, String> {
    let output_channels = usize::from(spec.config.channels).max(1);
    let mut frame_cursor = initial_frame;
    let callback_cursor = Arc::clone(&cursor);
    let callback_finished = Arc::clone(&finished);
    device
        .build_output_stream(
            &spec.config,
            move |data: &mut [T], _| {
                for output_frame in data.chunks_mut(output_channels) {
                    let Some(source_frame) =
                        next_frame(&mut frame_cursor, prepared.frame_count, looping)
                    else {
                        output_frame.fill(T::from_normalized(0.0));
                        callback_finished.store(true, Ordering::Release);
                        continue;
                    };
                    let gain = preview_edge_gain(
                        source_frame,
                        prepared.frame_count,
                        prepared.output_sample_rate,
                    );
                    for (channel, output) in output_frame.iter_mut().enumerate() {
                        *output = T::from_normalized(
                            mapped_prepared_sample(
                                &prepared,
                                source_frame,
                                channel,
                                output_channels,
                            ) * gain,
                        );
                    }
                    callback_cursor.store(frame_cursor, Ordering::Relaxed);
                }
            },
            move |stream_error| {
                if let Ok(mut slot) = error.lock() {
                    *slot = Some(preview_device_error(&stream_error.to_string()));
                }
                finished.store(true, Ordering::Release);
            },
            None,
        )
        .map_err(|stream_error| format!("Voice Lab output stream creation failed: {stream_error}"))
}

fn preview_device_error(details: &str) -> String {
    format!("Voice Lab output device became unavailable during preview: {details}")
}

fn preview_edge_gain(frame: usize, frames: usize, sample_rate: u32) -> f32 {
    let fade_frames = (sample_rate as usize * PREVIEW_FADE_MS / 1_000).max(1);
    let trailing = frames.saturating_sub(frame + 1);
    frame.min(trailing).min(fade_frames) as f32 / fade_frames as f32
}

pub fn convert_frame_position(
    source_frame: usize,
    source_sample_rate: u32,
    target_sample_rate: u32,
    target_frame_count: usize,
) -> usize {
    if source_sample_rate == 0 || target_sample_rate == 0 || target_frame_count == 0 {
        return 0;
    }
    let target = (source_frame as u128)
        .saturating_mul(u128::from(target_sample_rate))
        .saturating_add(u128::from(source_sample_rate) / 2)
        / u128::from(source_sample_rate);
    usize::try_from(target)
        .unwrap_or(usize::MAX)
        .min(target_frame_count.saturating_sub(1))
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

fn mapped_prepared_sample(
    prepared: &PreparedPreview,
    frame: usize,
    output_channel: usize,
    output_channels: usize,
) -> f32 {
    let source = &prepared.samples[frame * prepared.channels..(frame + 1) * prepared.channels];
    match (prepared.channels, output_channels) {
        (_, 1) => source.iter().sum::<f32>() / source.len() as f32,
        (1, _) => source[0],
        (_, _) if output_channel < source.len() => source[output_channel],
        _ => source.iter().sum::<f32>() / source.len() as f32,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        convert_frame_position, mapped_prepared_sample, next_frame, preview_device_error,
        preview_edge_gain, PreparedPreview, PreparedPreviewCache, PreviewCacheKey,
    };
    use crate::voice_lab::clip::AudioClip;

    fn key(preview: &PreparedPreview, device: &str) -> PreviewCacheKey {
        PreviewCacheKey {
            source_clip_id: preview.source_clip_id.clone(),
            output_device_id: device.to_owned(),
            output_sample_rate: preview.output_sample_rate,
            output_channels: 2,
            output_sample_format: "f32".to_owned(),
        }
    }

    #[test]
    fn prepares_original_44_1_for_48_and_processed_48_for_44_1() {
        let original_samples = vec![0.25; 44_100];
        let original = AudioClip::new("original", 44_100, 1, original_samples.clone()).unwrap();
        let original_preview = PreparedPreview::prepare(&original, 48_000).unwrap();
        assert_eq!(original_preview.source_sample_rate, 44_100);
        assert_eq!(original_preview.output_sample_rate, 48_000);
        assert_eq!(original_preview.frame_count, 48_000);
        assert_eq!(original.samples, original_samples);

        let processed = AudioClip::new("processed", 48_000, 1, vec![0.1; 48_000]).unwrap();
        let processed_preview = PreparedPreview::prepare(&processed, 44_100).unwrap();
        assert_eq!(processed_preview.frame_count, 44_100);
        assert!(processed_preview
            .samples
            .iter()
            .all(|sample| sample.is_finite()));
        assert_ne!(
            original_preview.source_clip_id,
            processed_preview.source_clip_id
        );
    }

    #[test]
    fn same_rate_preview_does_not_resample_or_change_duration() {
        let samples = [0.2, -0.2].repeat(48_000);
        let clip = AudioClip::new("same", 48_000, 2, samples).unwrap();
        let prepared = PreparedPreview::prepare(&clip, 48_000).unwrap();
        assert_eq!(prepared.frame_count, clip.frames());
        assert_eq!(prepared.samples, clip.samples);
    }

    #[test]
    fn ab_position_conversion_is_time_based_and_bounded() {
        assert_eq!(
            convert_frame_position(22_050, 44_100, 48_000, 48_000),
            24_000
        );
        assert_eq!(
            convert_frame_position(48_000, 48_000, 44_100, 44_100),
            44_099
        );
        assert_eq!(convert_frame_position(10, 0, 48_000, 100), 0);
    }

    #[test]
    fn preview_cursor_stops_or_restarts_cleanly_at_the_loop_boundary() {
        let mut cursor = 0;
        assert_eq!(next_frame(&mut cursor, 2, false), Some(0));
        assert_eq!(next_frame(&mut cursor, 2, false), Some(1));
        assert_eq!(next_frame(&mut cursor, 2, false), None);
        assert_eq!(next_frame(&mut cursor, 2, true), Some(0));
        assert_eq!(cursor, 1);
        assert_eq!(preview_edge_gain(0, 48_000, 48_000), 0.0);
        assert_eq!(preview_edge_gain(47_999, 48_000, 48_000), 0.0);
        assert_eq!(preview_edge_gain(240, 48_000, 48_000), 1.0);
    }

    #[test]
    fn preview_maps_mono_to_stereo_without_live_channel_state() {
        let mono = AudioClip::new("mono", 48_000, 1, vec![0.4]).unwrap();
        let mono = PreparedPreview::prepare(&mono, 48_000).unwrap();
        assert_eq!(mapped_prepared_sample(&mono, 0, 0, 2), 0.4);
        assert_eq!(mapped_prepared_sample(&mono, 0, 1, 2), 0.4);
        let stereo = AudioClip::new("stereo", 48_000, 2, vec![0.6, -0.2]).unwrap();
        let stereo = PreparedPreview::prepare(&stereo, 48_000).unwrap();
        assert!((mapped_prepared_sample(&stereo, 0, 0, 1) - 0.2).abs() < 0.0001);
    }

    #[test]
    fn cache_keys_invalidate_for_clip_device_rate_and_source_changes() {
        let first = PreparedPreview::prepare(
            &AudioClip::new("one", 44_100, 1, vec![0.0]).unwrap(),
            48_000,
        )
        .unwrap();
        let second = PreparedPreview::prepare(
            &AudioClip::new("two", 44_100, 1, vec![0.0]).unwrap(),
            48_000,
        )
        .unwrap();
        let first_key = key(&first, "device-a");
        let second_key = key(&second, "device-a");
        let changed_device = key(&first, "device-b");
        let mut changed_rate = first_key.clone();
        changed_rate.output_sample_rate = 44_100;
        let mut cache = PreparedPreviewCache::default();
        cache.store(first_key.clone(), first);
        assert!(cache.contains(&first_key));
        assert!(!cache.contains(&second_key));
        assert!(!cache.contains(&changed_device));
        assert!(!cache.contains(&changed_rate));
        cache.invalidate();
        assert!(!cache.contains(&first_key));
    }

    #[test]
    fn device_removal_error_is_precise() {
        let error = preview_device_error("endpoint removed");
        assert!(error.contains("output device became unavailable"));
        assert!(error.contains("endpoint removed"));
    }
}
