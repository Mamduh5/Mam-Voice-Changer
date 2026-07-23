use std::{path::Path, sync::Arc};

use serde::{Deserialize, Serialize};

use crate::{
    audio::{
        device::{find_device_with_fallback, DeviceDirection},
        stream_config,
    },
    dsp::chain::DspParameters,
};

use super::{
    capture::CaptureHandle,
    clip::{AudioClip, ClipSummary},
    offline::{ExistingDspOfflineProcessor, OfflineVoiceProcessor, RenderMetadata},
    output_config::{negotiate_preview_output, sample_format_label},
    preview::{
        convert_frame_position, PreparedPreview, PreparedPreviewCache, PreviewCacheKey,
        PreviewHandle,
    },
    wav,
};

const LAB_STREAM_BUFFER_FRAMES: u32 = 512;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ClipVersion {
    Original,
    Processed,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureStatus {
    pub active: bool,
    pub dropped_frames: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewStatus {
    pub active: bool,
    pub kind: Option<String>,
    pub looping: bool,
    pub position_ms: u64,
    pub duration_ms: u64,
    pub clip_sample_rate: Option<u32>,
    pub output_sample_rate: Option<u32>,
    pub resampling_active: bool,
    pub output_channels: Option<u16>,
    pub output_sample_format: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceLabStatus {
    pub original: Option<ClipSummary>,
    pub processed: Option<ClipSummary>,
    pub render_metadata: Option<RenderMetadata>,
    pub capture: CaptureStatus,
    pub preview: PreviewStatus,
    pub last_error: Option<String>,
    pub processed_synthetic: bool,
}

pub struct VoiceLabSession {
    original: Option<Arc<AudioClip>>,
    original_summary: Option<ClipSummary>,
    processed: Option<Arc<AudioClip>>,
    processed_summary: Option<ClipSummary>,
    render_metadata: Option<RenderMetadata>,
    capture: Option<CaptureHandle>,
    preview: Option<PreviewHandle>,
    last_preview_status: Option<PreviewStatus>,
    preview_cache: PreparedPreviewCache,
    backend: Box<dyn OfflineVoiceProcessor>,
    last_error: Option<String>,
    processed_synthetic: bool,
}

impl Default for VoiceLabSession {
    fn default() -> Self {
        Self {
            original: None,
            original_summary: None,
            processed: None,
            processed_summary: None,
            render_metadata: None,
            capture: None,
            preview: None,
            last_preview_status: None,
            preview_cache: PreparedPreviewCache::default(),
            backend: Box::<ExistingDspOfflineProcessor>::default(),
            last_error: None,
            processed_synthetic: false,
        }
    }
}

impl VoiceLabSession {
    pub fn status(&mut self) -> VoiceLabStatus {
        self.finalize_automatic_capture();
        self.finalize_preview();
        let capture = CaptureStatus {
            active: self.capture.is_some(),
            dropped_frames: self
                .capture
                .as_ref()
                .map_or(0, CaptureHandle::dropped_frames),
        };
        let preview = self.preview.as_ref().map_or_else(
            || {
                self.last_preview_status.clone().unwrap_or(PreviewStatus {
                    active: false,
                    kind: None,
                    looping: false,
                    position_ms: 0,
                    duration_ms: 0,
                    clip_sample_rate: None,
                    output_sample_rate: None,
                    resampling_active: false,
                    output_channels: None,
                    output_sample_format: None,
                })
            },
            |preview| preview_status(preview, true),
        );
        VoiceLabStatus {
            original: self.original_summary.clone(),
            processed: self.processed_summary.clone(),
            render_metadata: self.render_metadata,
            capture,
            preview,
            last_error: self.last_error.clone(),
            processed_synthetic: self.processed_synthetic,
        }
    }

    pub fn is_audio_active(&self) -> bool {
        self.capture.is_some() || self.preview.is_some()
    }

    pub fn start_capture(&mut self, input_id: &str, input_name: &str) -> Result<(), String> {
        self.stop_audio()?;
        let device = find_device_with_fallback(DeviceDirection::Input, input_id, input_name)
            .map_err(|error| error.to_string())?;
        let spec = stream_config::input_spec(&device, LAB_STREAM_BUFFER_FRAMES)
            .map_err(|error| error.to_string())?;
        self.capture = Some(CaptureHandle::start(&device, &spec)?);
        self.last_error = None;
        Ok(())
    }

    pub fn stop_capture(&mut self) -> Result<(), String> {
        let Some(capture) = self.capture.take() else {
            return Ok(());
        };
        self.store_capture(capture)
    }

    pub fn import_wav(&mut self, path: &Path) -> Result<(), String> {
        self.stop_audio()?;
        let clip = wav::import(path)?;
        self.set_original(clip);
        self.last_error = None;
        Ok(())
    }

    pub fn render(&mut self, parameters: DspParameters) -> Result<(), String> {
        self.stop_audio()?;
        let original = self
            .original
            .as_deref()
            .ok_or_else(|| "Record or import an original clip first.".to_owned())?;
        let rendered = self.backend.render(original, parameters)?;
        self.processed_summary = Some(rendered.clip.summary());
        self.processed = Some(Arc::new(rendered.clip));
        self.preview_cache.invalidate();
        self.last_preview_status = None;
        self.render_metadata = Some(rendered.metadata);
        self.processed_synthetic = false;
        self.last_error = None;
        Ok(())
    }

    pub fn start_preview(
        &mut self,
        version: ClipVersion,
        output_id: &str,
        output_name: &str,
        looping: bool,
    ) -> Result<(), String> {
        let previous_position = self
            .preview
            .as_ref()
            .map(|preview| (preview.position_frame(), preview.output_sample_rate));
        self.stop_preview();
        let (kind, clip) = match version {
            ClipVersion::Original => ("original", self.original.as_ref()),
            ClipVersion::Processed => ("processed", self.processed.as_ref()),
        };
        let clip =
            Arc::clone(clip.ok_or_else(|| format!("No {kind} clip is available to preview."))?);
        let device = find_device_with_fallback(DeviceDirection::Output, output_id, output_name)
            .map_err(|error| format!("Voice Lab output device unavailable: {error}"))?;
        let negotiated = negotiate_preview_output(&device, LAB_STREAM_BUFFER_FRAMES)?;
        let key = PreviewCacheKey {
            source_clip_id: clip.id.clone(),
            output_device_id: output_id.to_owned(),
            output_sample_rate: negotiated.spec.config.sample_rate.0,
            output_channels: negotiated.spec.config.channels,
            output_sample_format: sample_format_label(negotiated.spec.sample_format).to_owned(),
        };
        let prepared = if let Some(prepared) = self.preview_cache.get(&key) {
            prepared
        } else {
            let prepared = PreparedPreview::prepare(&clip, negotiated.spec.config.sample_rate.0)
                .map_err(|error| {
                    if error.contains("resampling failed") {
                        error
                    } else {
                        format!("Voice Lab preview preparation failed: {error}")
                    }
                })?;
            self.preview_cache.store(key, prepared)
        };
        let initial_frame = previous_position.map_or(0, |(source_frame, source_rate)| {
            convert_frame_position(
                source_frame,
                source_rate,
                prepared.output_sample_rate,
                prepared.frame_count,
            )
        });
        let preview = PreviewHandle::start(
            &device,
            &negotiated.spec,
            prepared,
            kind.to_owned(),
            looping,
            initial_frame,
            negotiated.sample_format_label,
        )?;
        self.last_preview_status = Some(preview_status(&preview, false));
        self.preview = Some(preview);
        self.last_error = None;
        Ok(())
    }

    pub fn stop_preview(&mut self) {
        if let Some(preview) = self.preview.take() {
            self.last_preview_status = Some(preview_status(&preview, false));
        }
    }

    pub fn export_wav(&self, version: ClipVersion, path: &Path) -> Result<(), String> {
        let (kind, clip) = match version {
            ClipVersion::Original => ("original", self.original.as_deref()),
            ClipVersion::Processed => ("processed", self.processed.as_deref()),
        };
        wav::export(
            path,
            clip.ok_or_else(|| format!("No {kind} clip is available to export."))?,
        )
    }

    pub fn load_synthetic_processed_wav(&mut self, path: &Path) -> Result<(), String> {
        self.stop_audio()?;
        if self.original.is_none() {
            return Err(
                "Record or import an original clip before loading a model result.".to_owned(),
            );
        }
        let mut clip = wav::import(path)?;
        clip.source_name = "Synthetic model conversion".to_owned();
        self.processed_summary = Some(clip.summary());
        self.processed = Some(Arc::new(clip));
        self.preview_cache.invalidate();
        self.last_preview_status = None;
        self.render_metadata = None;
        self.processed_synthetic = true;
        self.last_error = None;
        Ok(())
    }

    pub fn stop_audio(&mut self) -> Result<(), String> {
        self.stop_preview();
        self.stop_capture()
    }

    pub fn clear(&mut self) -> Result<(), String> {
        let _ = self.stop_audio();
        self.original = None;
        self.original_summary = None;
        self.processed = None;
        self.processed_summary = None;
        self.render_metadata = None;
        self.processed_synthetic = false;
        self.preview_cache.invalidate();
        self.last_preview_status = None;
        self.last_error = None;
        Ok(())
    }

    fn set_original(&mut self, clip: AudioClip) {
        self.original_summary = Some(clip.summary());
        self.original = Some(Arc::new(clip));
        self.preview_cache.invalidate();
        self.last_preview_status = None;
        self.processed = None;
        self.processed_summary = None;
        self.render_metadata = None;
        self.processed_synthetic = false;
    }

    fn store_capture(&mut self, capture: CaptureHandle) -> Result<(), String> {
        match capture.finish() {
            Ok(clip) => {
                self.set_original(clip);
                self.last_error = None;
                Ok(())
            }
            Err(error) => {
                self.last_error = Some(error.clone());
                Err(error)
            }
        }
    }

    fn finalize_automatic_capture(&mut self) {
        let should_finish = self.capture.as_ref().is_some_and(|capture| {
            if let Some(error) = capture.error() {
                self.last_error = Some(error);
                true
            } else {
                capture.is_finished()
            }
        });
        if should_finish {
            if let Some(capture) = self.capture.take() {
                let _ = self.store_capture(capture);
            }
        }
    }

    fn finalize_preview(&mut self) {
        let should_finish = self.preview.as_ref().is_some_and(|preview| {
            if let Some(error) = preview.error() {
                self.last_error = Some(error);
                true
            } else {
                preview.is_finished()
            }
        });
        if should_finish {
            self.stop_preview();
        }
    }
}

fn preview_status(preview: &PreviewHandle, active: bool) -> PreviewStatus {
    PreviewStatus {
        active,
        kind: Some(preview.kind.clone()),
        looping: preview.looping,
        position_ms: preview.position_ms(),
        duration_ms: preview.duration_ms(),
        clip_sample_rate: Some(preview.source_sample_rate),
        output_sample_rate: Some(preview.output_sample_rate),
        resampling_active: preview.source_sample_rate != preview.output_sample_rate,
        output_channels: Some(preview.output_channels),
        output_sample_format: Some(preview.output_sample_format.clone()),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::VoiceLabSession;
    use crate::voice_lab::clip::AudioClip;

    #[test]
    fn replacing_original_invalidates_processed_and_clear_drops_all_audio() {
        let mut session = VoiceLabSession::default();
        session.set_original(AudioClip::new("one", 48_000, 1, vec![0.0; 16]).unwrap());
        session.processed = Some(Arc::new(
            AudioClip::new("processed", 48_000, 1, vec![0.0; 16]).unwrap(),
        ));
        session.set_original(AudioClip::new("two", 48_000, 1, vec![0.0; 8]).unwrap());
        assert!(session.processed.is_none());
        session.clear().unwrap();
        let status = session.status();
        assert!(status.original.is_none());
        assert!(status.processed.is_none());
        assert!(!status.capture.active);
        assert!(!status.preview.active);
    }

    #[test]
    fn stopping_preview_is_idempotent_without_audio_hardware() {
        let mut session = VoiceLabSession::default();
        session.stop_preview();
        session.stop_preview();
        assert!(!session.status().preview.active);
    }
}
