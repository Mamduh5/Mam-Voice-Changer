use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, SyncSender},
        Arc, Mutex,
    },
    thread,
};

use serde::{Deserialize, Serialize};

use crate::audio::{
    device::{self, find_device_with_fallback, DeviceDirection},
    stream_config,
};

use super::{
    capture::{DatasetCaptureHandle, DatasetCaptureResult, DATASET_MAX_TAKE_SECONDS},
    error::{DatasetError, DatasetErrorCode, DatasetResult},
    hash::sha256_samples,
    import::{
        self, read_canonical_wav, resample_to_canonical, write_canonical_wav, CANONICAL_SAMPLE_RATE,
    },
    manifest::VoiceDatasetManifestV1,
    preview::DatasetPreviewHandle,
    profile::{CreateVoiceProfileRequest, UpdateVoiceProfileRequest, VoiceProfileSummary},
    prompts::{built_in_english_pack, PromptCategory, PromptPack, VoicePrompt},
    quality::{analyze_take, CaptureMetrics, CLIPPING_THRESHOLD, SILENCE_THRESHOLD},
    storage::{new_id, timestamp, DatasetStorage},
    take::{waveform_envelope, DatasetTake, SelectedTakeVersion, TakeReviewStatus, TakeSource},
};

const DATASET_STREAM_BUFFER_FRAMES: u32 = 512;
const WAVEFORM_BUCKETS: usize = 96;
const MAX_IMPORT_BATCH: usize = 32;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PromptSelection {
    pub prompt_id: Option<String>,
    pub custom_prompt_text: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReviewTakeRequest {
    pub status: TakeReviewStatus,
    pub exclude_from_training: bool,
    pub notes: Option<String>,
    pub warning_acknowledged: bool,
    pub selected_version: SelectedTakeVersion,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DatasetExportOptions {
    pub include_rejected: bool,
    pub include_raw_masters: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DatasetRecordingStatus {
    pub active: bool,
    pub finalizing: bool,
    pub duration_ms: u64,
    pub maximum_duration_ms: u64,
    pub remaining_ms: u64,
    pub input_level: f32,
    pub clipping: bool,
    pub dropped_frames: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DatasetPreviewStatus {
    pub active: bool,
    pub paused: bool,
    pub take_id: Option<String>,
    pub version: Option<SelectedTakeVersion>,
    pub position_ms: u64,
    pub duration_ms: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceDatasetStatus {
    pub current_profile_id: Option<String>,
    pub current_prompt_id: Option<String>,
    pub current_prompt_text: Option<String>,
    pub current_prompt_category: Option<PromptCategory>,
    pub manifest: Option<VoiceDatasetManifestV1>,
    pub recording: DatasetRecordingStatus,
    pub preview: DatasetPreviewStatus,
    pub last_error: Option<DatasetError>,
}

struct RecordingContext {
    handle: DatasetCaptureHandle,
    profile_id: String,
    prompt: Option<VoicePrompt>,
    source: TakeSource,
}

struct PreviewContext {
    handle: DatasetPreviewHandle,
    take_id: String,
    version: SelectedTakeVersion,
}

#[derive(Default)]
struct DatasetRuntime {
    current_profile_id: Option<String>,
    current_prompt: Option<VoicePrompt>,
    recording: Option<RecordingContext>,
    preview: Option<PreviewContext>,
    finalizing: bool,
    trim_draft: Option<(String, u64, u64)>,
    last_error: Option<DatasetError>,
}

struct VoiceDatasetSession {
    storage: DatasetStorage,
    runtime: Mutex<DatasetRuntime>,
    audio_active: AtomicBool,
}

impl VoiceDatasetSession {
    fn new(root: PathBuf) -> DatasetResult<Self> {
        Ok(Self {
            storage: DatasetStorage::load(root)?,
            runtime: Mutex::new(DatasetRuntime::default()),
            audio_active: AtomicBool::new(false),
        })
    }

    pub fn is_audio_active(&self) -> bool {
        self.audio_active.load(Ordering::Acquire)
    }
    pub fn list_profiles(&self) -> DatasetResult<Vec<VoiceProfileSummary>> {
        self.storage.list_profiles()
    }

    pub fn snapshot_source(
        &self,
        profile_id: &str,
    ) -> DatasetResult<super::source::ManifestDatasetSource> {
        self.storage.snapshot_source(profile_id)
    }
    pub fn prompts(&self) -> PromptPack {
        built_in_english_pack()
    }

    pub fn create_profile(
        &self,
        request: CreateVoiceProfileRequest,
    ) -> DatasetResult<VoiceDatasetStatus> {
        let manifest = self.storage.create_profile(request)?;
        let mut runtime = self.lock_runtime()?;
        runtime.current_profile_id = Some(manifest.profile.id);
        runtime.current_prompt = built_in_english_pack().prompts.first().cloned();
        runtime.last_error = None;
        drop(runtime);
        self.status()
    }

    pub fn select_profile(&self, profile_id: String) -> DatasetResult<VoiceDatasetStatus> {
        self.stop_audio()?;
        self.storage.read_manifest(&profile_id)?;
        let mut runtime = self.lock_runtime()?;
        runtime.current_profile_id = Some(profile_id);
        runtime.current_prompt = built_in_english_pack().prompts.first().cloned();
        runtime.last_error = None;
        drop(runtime);
        self.status()
    }

    pub fn update_profile(
        &self,
        profile_id: &str,
        request: UpdateVoiceProfileRequest,
    ) -> DatasetResult<VoiceDatasetStatus> {
        self.storage.update_profile(profile_id, request)?;
        self.status()
    }

    pub fn select_prompt(&self, selection: PromptSelection) -> DatasetResult<VoiceDatasetStatus> {
        let prompt = resolve_prompt(selection)?;
        self.lock_runtime()?.current_prompt = prompt;
        self.status()
    }

    pub fn start_recording(
        &self,
        input_id: &str,
        input_name: &str,
        recorded_consent: bool,
    ) -> DatasetResult<VoiceDatasetStatus> {
        if input_id.trim().is_empty() {
            return Err(DatasetError::new(
                DatasetErrorCode::NoMicrophoneSelected,
                "Select a physical recording microphone.",
            ));
        }
        let (profile_id, prompt) = {
            let runtime = self.lock_runtime()?;
            if runtime.recording.is_some() {
                return Err(DatasetError::new(
                    DatasetErrorCode::RecordingAlreadyActive,
                    "A dataset recording is already active.",
                ));
            }
            if runtime.preview.is_some() {
                return Err(DatasetError::new(
                    DatasetErrorCode::AudioOperationAlreadyActive,
                    "Stop dataset preview before recording.",
                ));
            }
            (
                runtime.current_profile_id.clone().ok_or_else(|| {
                    DatasetError::new(
                        DatasetErrorCode::ProfileNotFound,
                        "Create or select a consenting voice profile first.",
                    )
                })?,
                runtime.current_prompt.clone(),
            )
        };
        let manifest = self.storage.read_manifest(&profile_id)?;
        if !manifest.consent.consent_confirmed || manifest.consent.revoked_at.is_some() {
            return Err(DatasetError::new(
                DatasetErrorCode::ConsentRequired,
                "The selected profile does not have active consent.",
            ));
        }
        let selected = physical_device(
            DeviceDirection::Input,
            input_id,
            input_name,
            DatasetErrorCode::MicrophoneUnavailable,
        )?;
        let device =
            find_device_with_fallback(DeviceDirection::Input, &selected.id, &selected.name)
                .map_err(|_| {
                    DatasetError::new(
                        DatasetErrorCode::MicrophoneUnavailable,
                        "The selected physical microphone is unavailable.",
                    )
                })?;
        let spec =
            stream_config::input_spec(&device, DATASET_STREAM_BUFFER_FRAMES).map_err(|_| {
                DatasetError::new(
                    DatasetErrorCode::MicrophoneUnavailable,
                    "The selected microphone has no supported stream configuration.",
                )
            })?;
        let handle = DatasetCaptureHandle::start(&device, &spec)?;
        let mut runtime = self.lock_runtime()?;
        runtime.recording = Some(RecordingContext {
            handle,
            profile_id,
            prompt,
            source: if recorded_consent {
                TakeSource::RecordedConsent
            } else {
                TakeSource::Recorded
            },
        });
        runtime.last_error = None;
        self.audio_active.store(true, Ordering::Release);
        drop(runtime);
        self.status()
    }

    pub fn stop_recording(&self) -> DatasetResult<VoiceDatasetStatus> {
        let recording = {
            let mut runtime = self.lock_runtime()?;
            runtime.finalizing = true;
            runtime.recording.take().ok_or_else(|| {
                DatasetError::new(
                    DatasetErrorCode::InvalidStateTransition,
                    "No dataset recording is active.",
                )
            })?
        };
        self.audio_active.store(false, Ordering::Release);
        let result = self.finalize_recording(recording);
        let mut runtime = self.lock_runtime()?;
        runtime.finalizing = false;
        match &result {
            Ok(_) => runtime.last_error = None,
            Err(error) => runtime.last_error = Some(error.clone()),
        }
        drop(runtime);
        result?;
        self.status()
    }

    pub fn discard_recording(&self) -> DatasetResult<VoiceDatasetStatus> {
        let recording = self.lock_runtime()?.recording.take();
        drop(recording);
        self.audio_active.store(false, Ordering::Release);
        self.status()
    }

    pub fn import_wavs(
        &self,
        paths: Vec<String>,
        selection: PromptSelection,
    ) -> DatasetResult<VoiceDatasetStatus> {
        if paths.is_empty() || paths.len() > MAX_IMPORT_BATCH {
            return Err(DatasetError::new(
                DatasetErrorCode::UnsupportedWav,
                format!("Select between 1 and {MAX_IMPORT_BATCH} WAV files."),
            ));
        }
        let profile_id = self.current_profile_id()?;
        let prompt = resolve_prompt(selection)?;
        for path in paths {
            let selected = PathBuf::from(path);
            let imported = import::import_wav(&selected)?;
            let take_id = new_id("take", &timestamp()?);
            let raw_path = self.storage.raw_take_path(&profile_id, &take_id)?;
            let capture = CaptureMetrics {
                non_finite_input_count: imported.non_finite_count,
                ..CaptureMetrics::default()
            };
            let quality = analyze_take(&imported.samples, CANONICAL_SAMPLE_RATE, capture);
            let take = build_take(
                &take_id,
                &imported.samples,
                prompt.as_ref(),
                TakeSource::Imported,
                Some(imported.original_format),
                quality,
                false,
            )?;
            write_canonical_wav(&raw_path, &imported.samples)?;
            if let Err(error) = self.storage.add_take(&profile_id, take) {
                let _ = fs::remove_file(raw_path);
                return Err(error);
            }
        }
        self.status()
    }

    pub fn review_take(
        &self,
        profile_id: &str,
        take_id: &str,
        request: ReviewTakeRequest,
    ) -> DatasetResult<VoiceDatasetStatus> {
        self.storage.review_take(
            profile_id,
            take_id,
            super::take::TakeReviewUpdate {
                status: request.status,
                exclude_from_training: request.exclude_from_training,
                notes: request.notes,
                warning_acknowledged: request.warning_acknowledged,
                selected_version: request.selected_version,
            },
        )?;
        self.status()
    }

    pub fn set_trim(
        &self,
        take_id: String,
        start_frame: u64,
        end_frame: u64,
    ) -> DatasetResult<VoiceDatasetStatus> {
        let profile_id = self.current_profile_id()?;
        let manifest = self.storage.read_manifest(&profile_id)?;
        let take = manifest
            .takes
            .iter()
            .find(|take| take.id == take_id)
            .ok_or_else(|| {
                DatasetError::new(
                    DatasetErrorCode::TakeNotFound,
                    "The selected take was not found.",
                )
            })?;
        if start_frame >= end_frame || end_frame > take.frame_count {
            return Err(DatasetError::new(
                DatasetErrorCode::InvalidTrimRange,
                "Trim boundaries must select a non-empty range inside the take.",
            ));
        }
        self.lock_runtime()?.trim_draft = Some((take_id, start_frame, end_frame));
        self.status()
    }

    pub fn auto_trim(&self, take_id: String) -> DatasetResult<VoiceDatasetStatus> {
        let profile_id = self.current_profile_id()?;
        let manifest = self.storage.read_manifest(&profile_id)?;
        let take = manifest
            .takes
            .iter()
            .find(|take| take.id == take_id)
            .ok_or_else(|| {
                DatasetError::new(
                    DatasetErrorCode::TakeNotFound,
                    "The selected take was not found.",
                )
            })?;
        let samples = read_canonical_wav(&self.storage.resolve_take_file(
            &profile_id,
            take,
            SelectedTakeVersion::Raw,
        )?)?;
        let padding = (CANONICAL_SAMPLE_RATE / 20) as usize;
        let first = samples
            .iter()
            .position(|sample| sample.abs() >= SILENCE_THRESHOLD)
            .unwrap_or(0)
            .saturating_sub(padding);
        let last = samples
            .iter()
            .rposition(|sample| sample.abs() >= SILENCE_THRESHOLD)
            .map_or(samples.len(), |index| {
                (index + 1 + padding).min(samples.len())
            });
        self.set_trim(take_id, first as u64, last as u64)
    }

    pub fn apply_trim(&self) -> DatasetResult<VoiceDatasetStatus> {
        let profile_id = self.current_profile_id()?;
        let (take_id, start, end) = self.lock_runtime()?.trim_draft.clone().ok_or_else(|| {
            DatasetError::new(
                DatasetErrorCode::InvalidTrimRange,
                "Set or auto-detect trim boundaries first.",
            )
        })?;
        let manifest = self.storage.read_manifest(&profile_id)?;
        let take = manifest
            .takes
            .iter()
            .find(|take| take.id == take_id)
            .ok_or_else(|| {
                DatasetError::new(
                    DatasetErrorCode::TakeNotFound,
                    "The selected take was not found.",
                )
            })?;
        let samples = read_canonical_wav(&self.storage.resolve_take_file(
            &profile_id,
            take,
            SelectedTakeVersion::Raw,
        )?)?;
        if start >= end || end as usize > samples.len() {
            return Err(DatasetError::new(
                DatasetErrorCode::InvalidTrimRange,
                "Trim boundaries are outside the raw take.",
            ));
        }
        let trimmed = &samples[start as usize..end as usize];
        let derived_path = self.storage.derived_take_path(&profile_id, &take_id)?;
        write_canonical_wav(&derived_path, trimmed)?;
        let quality = analyze_take(trimmed, CANONICAL_SAMPLE_RATE, CaptureMetrics::default());
        if let Err(error) = self.storage.apply_trim(
            &profile_id,
            &take_id,
            start,
            end,
            quality,
            waveform_envelope(trimmed, WAVEFORM_BUCKETS),
        ) {
            let _ = fs::remove_file(derived_path);
            return Err(error);
        }
        self.lock_runtime()?.trim_draft = None;
        self.status()
    }

    pub fn reset_trim(&self, take_id: &str) -> DatasetResult<VoiceDatasetStatus> {
        let profile_id = self.current_profile_id()?;
        self.storage.reset_trim(&profile_id, take_id)?;
        self.status()
    }

    pub fn start_preview(
        &self,
        take_id: &str,
        version: SelectedTakeVersion,
        output_id: &str,
        output_name: &str,
        seek_ms: u64,
    ) -> DatasetResult<VoiceDatasetStatus> {
        self.stop_preview()?;
        let profile_id = self.current_profile_id()?;
        let manifest = self.storage.read_manifest(&profile_id)?;
        let take = manifest
            .takes
            .iter()
            .find(|take| take.id == take_id)
            .ok_or_else(|| {
                DatasetError::new(
                    DatasetErrorCode::TakeNotFound,
                    "The selected take was not found.",
                )
                .take(take_id)
            })?;
        let samples = Arc::new(read_canonical_wav(&self.storage.resolve_take_file(
            &profile_id,
            take,
            version,
        )?)?);
        let selected = physical_device(
            DeviceDirection::Output,
            output_id,
            output_name,
            DatasetErrorCode::PreviewOutputMissing,
        )?;
        let device =
            find_device_with_fallback(DeviceDirection::Output, &selected.id, &selected.name)
                .map_err(|_| {
                    DatasetError::new(
                        DatasetErrorCode::PreviewOutputMissing,
                        "The selected physical preview output is unavailable.",
                    )
                })?;
        let spec = stream_config::output_spec_at_rate(
            &device,
            CANONICAL_SAMPLE_RATE,
            DATASET_STREAM_BUFFER_FRAMES,
        )
        .map_err(|_| {
            DatasetError::new(
                DatasetErrorCode::PreviewFailed,
                "The preview output does not support 48 kHz playback.",
            )
        })?;
        let seek_frame =
            (seek_ms.saturating_mul(u64::from(CANONICAL_SAMPLE_RATE)) / 1_000) as usize;
        let handle = DatasetPreviewHandle::start(&device, &spec, samples, seek_frame)?;
        self.lock_runtime()?.preview = Some(PreviewContext {
            handle,
            take_id: take_id.to_owned(),
            version,
        });
        self.audio_active.store(true, Ordering::Release);
        self.status()
    }

    pub fn pause_preview(&self) -> DatasetResult<VoiceDatasetStatus> {
        let runtime = self.lock_runtime()?;
        runtime
            .preview
            .as_ref()
            .ok_or_else(|| {
                DatasetError::new(
                    DatasetErrorCode::InvalidStateTransition,
                    "No dataset preview is active.",
                )
            })?
            .handle
            .toggle_pause();
        drop(runtime);
        self.status()
    }

    pub fn stop_preview(&self) -> DatasetResult<()> {
        self.lock_runtime()?.preview = None;
        self.audio_active
            .store(self.lock_runtime()?.recording.is_some(), Ordering::Release);
        Ok(())
    }

    pub fn delete_take(&self, take_id: &str) -> DatasetResult<VoiceDatasetStatus> {
        let profile_id = self.current_profile_id()?;
        let should_stop = self
            .lock_runtime()?
            .preview
            .as_ref()
            .is_some_and(|preview| preview.take_id == take_id);
        if should_stop {
            self.stop_preview()?;
        }
        self.storage.delete_take(&profile_id, take_id)?;
        self.status()
    }

    pub fn delete_profile(&self, profile_id: &str) -> DatasetResult<VoiceDatasetStatus> {
        {
            let runtime = self.lock_runtime()?;
            let owns_audio = runtime
                .recording
                .as_ref()
                .is_some_and(|recording| recording.profile_id == profile_id)
                || (runtime.current_profile_id.as_deref() == Some(profile_id)
                    && runtime.preview.is_some());
            drop(runtime);
            if owns_audio {
                self.stop_audio()?;
            }
        }
        self.storage.delete_profile(profile_id)?;
        let mut runtime = self.lock_runtime()?;
        if runtime.current_profile_id.as_deref() == Some(profile_id) {
            *runtime = DatasetRuntime::default();
        }
        drop(runtime);
        self.status()
    }

    pub fn repair_profile(&self, profile_id: &str) -> DatasetResult<VoiceDatasetStatus> {
        self.storage.repair_profile(profile_id)?;
        self.status()
    }

    pub fn export(
        &self,
        destination: &Path,
        options: DatasetExportOptions,
    ) -> DatasetResult<PathBuf> {
        let profile_id = self.current_profile_id()?;
        let manifest = self.storage.read_manifest(&profile_id)?;
        if !destination.is_dir() {
            return Err(DatasetError::new(
                DatasetErrorCode::ExportFailed,
                "Choose an existing export directory.",
            ));
        }
        let package = destination.join(format!("mam-voice-dataset-{profile_id}"));
        if package.exists() {
            return Err(DatasetError::new(DatasetErrorCode::ExportFailed, "An export package with this profile identifier already exists at the selected destination."));
        }
        fs::create_dir_all(package.join("audio")).map_err(|error| {
            DatasetError::new(
                DatasetErrorCode::ExportFailed,
                format!("Cannot create the export package: {error}"),
            )
        })?;
        let result = (|| {
            let mut exported = manifest.clone();
            exported.takes.retain(|take| {
                (take.review_status == TakeReviewStatus::Accepted
                    || (options.include_rejected
                        && take.review_status == TakeReviewStatus::Rejected))
                    && !take.exclude_from_training
                    && take.source != TakeSource::RecordedConsent
            });
            for take in &mut exported.takes {
                let selected = if options.include_raw_masters {
                    SelectedTakeVersion::Raw
                } else {
                    take.selected_version
                };
                let source = self
                    .storage
                    .resolve_take_file(&profile_id, take, selected)?;
                let file_name = format!("{}.wav", take.id);
                fs::copy(source, package.join("audio").join(&file_name)).map_err(|error| {
                    DatasetError::new(
                        DatasetErrorCode::ExportFailed,
                        format!("Cannot copy an accepted take: {error}"),
                    )
                })?;
                take.raw_file = format!("audio/{file_name}");
                take.derived_file = None;
                take.trim = None;
                take.selected_version = SelectedTakeVersion::Raw;
            }
            exported.rebuild_statistics(built_in_english_pack().prompts.len());
            super::storage::atomic_write_json(&package.join("manifest.json"), &exported)?;
            super::storage::atomic_write_json(&package.join("consent.json"), &manifest.consent)?;
            super::storage::atomic_write_json(
                &package.join("prompt-pack.json"),
                &built_in_english_pack(),
            )?;
            fs::write(package.join("README.txt"), "Mam Voice Changer local dataset export schema v1\n\nThis package contains reviewed recordings for possible future offline training. It does not contain a cloned voice or a model. Consent metadata is a product safeguard, not legal verification. Exported copies are outside application management. Quality values are heuristic and require human listening.\n").map_err(|error| DatasetError::new(DatasetErrorCode::ExportFailed, format!("Cannot write the export README: {error}")))?;
            Ok::<_, DatasetError>(())
        })();
        if let Err(error) = result {
            let _ = fs::remove_dir_all(&package);
            return Err(error);
        }
        Ok(package)
    }

    pub fn stop_audio(&self) -> DatasetResult<()> {
        let mut runtime = self.lock_runtime()?;
        let recording = runtime.recording.take();
        runtime.preview = None;
        runtime.finalizing = false;
        drop(runtime);
        self.audio_active.store(false, Ordering::Release);
        if let Some(recording) = recording {
            if recording.handle.is_finished() && recording.handle.stream_error().is_none() {
                self.finalize_recording(recording)?;
            }
        }
        Ok(())
    }

    pub fn clear_error(&self) -> DatasetResult<VoiceDatasetStatus> {
        self.lock_runtime()?.last_error = None;
        self.status()
    }

    pub fn status(&self) -> DatasetResult<VoiceDatasetStatus> {
        let finished_recording = {
            let mut runtime = self.lock_runtime()?;
            if runtime.recording.as_ref().is_some_and(|recording| {
                recording.handle.is_finished() || recording.handle.stream_error().is_some()
            }) {
                runtime.finalizing = true;
                runtime.recording.take()
            } else {
                None
            }
        };
        if let Some(recording) = finished_recording {
            self.audio_active.store(false, Ordering::Release);
            let result = self.finalize_recording(recording);
            let mut runtime = self.lock_runtime()?;
            runtime.finalizing = false;
            if let Err(error) = result {
                runtime.last_error = Some(error);
            }
        }
        let mut runtime = self.lock_runtime()?;
        if runtime
            .preview
            .as_ref()
            .is_some_and(|preview| preview.handle.is_finished() || preview.handle.error().is_some())
        {
            runtime.preview = None;
            self.audio_active
                .store(runtime.recording.is_some(), Ordering::Release);
        }
        let manifest = runtime
            .current_profile_id
            .as_ref()
            .map(|profile_id| self.storage.read_manifest(profile_id))
            .transpose()?;
        let recording = runtime.recording.as_ref().map_or(
            DatasetRecordingStatus {
                active: false,
                finalizing: runtime.finalizing,
                duration_ms: 0,
                maximum_duration_ms: DATASET_MAX_TAKE_SECONDS as u64 * 1_000,
                remaining_ms: DATASET_MAX_TAKE_SECONDS as u64 * 1_000,
                input_level: 0.0,
                clipping: false,
                dropped_frames: 0,
            },
            |recording| {
                let duration_ms = recording.handle.duration_ms();
                let input_level = recording.handle.maximum_level();
                DatasetRecordingStatus {
                    active: true,
                    finalizing: false,
                    duration_ms,
                    maximum_duration_ms: DATASET_MAX_TAKE_SECONDS as u64 * 1_000,
                    remaining_ms: (DATASET_MAX_TAKE_SECONDS as u64 * 1_000)
                        .saturating_sub(duration_ms),
                    input_level,
                    clipping: input_level >= CLIPPING_THRESHOLD,
                    dropped_frames: recording.handle.dropped_frames(),
                }
            },
        );
        let preview = runtime.preview.as_ref().map_or(
            DatasetPreviewStatus {
                active: false,
                paused: false,
                take_id: None,
                version: None,
                position_ms: 0,
                duration_ms: 0,
            },
            |preview| DatasetPreviewStatus {
                active: true,
                paused: preview.handle.is_paused(),
                take_id: Some(preview.take_id.clone()),
                version: Some(preview.version),
                position_ms: preview.handle.position_ms(),
                duration_ms: preview.handle.duration_ms(),
            },
        );
        Ok(VoiceDatasetStatus {
            current_profile_id: runtime.current_profile_id.clone(),
            current_prompt_id: runtime
                .current_prompt
                .as_ref()
                .map(|prompt| prompt.id.clone()),
            current_prompt_text: runtime
                .current_prompt
                .as_ref()
                .map(|prompt| prompt.text.clone()),
            current_prompt_category: runtime
                .current_prompt
                .as_ref()
                .map(|prompt| prompt.category),
            manifest,
            recording,
            preview,
            last_error: runtime.last_error.clone(),
        })
    }

    fn finalize_recording(&self, recording: RecordingContext) -> DatasetResult<()> {
        let DatasetCaptureResult {
            samples,
            sample_rate,
            metrics,
            limit_reached,
        } = recording.handle.finish()?;
        if samples.is_empty() {
            return Err(DatasetError::new(
                DatasetErrorCode::RecordingTooShort,
                "The recording contained no complete audio frames.",
            ));
        }
        let canonical = if sample_rate == CANONICAL_SAMPLE_RATE {
            samples
        } else {
            resample_to_canonical(&samples, sample_rate, DATASET_MAX_TAKE_SECONDS)?
        };
        let mut quality = analyze_take(&canonical, CANONICAL_SAMPLE_RATE, metrics);
        if limit_reached {
            quality.reasons.push(super::quality::QualityReason {
                code: super::quality::QualityReasonCode::TooLong,
                guidance: "Recording stopped automatically at the twenty-second prompted-take limit. Confirm the phrase was complete.".to_owned(),
            });
            if quality.classification == super::quality::QualityClassification::Pass {
                quality.classification = super::quality::QualityClassification::Warning;
            }
        }
        let take_id = new_id("take", &timestamp()?);
        let path = self
            .storage
            .raw_take_path(&recording.profile_id, &take_id)?;
        let take = build_take(
            &take_id,
            &canonical,
            recording.prompt.as_ref(),
            recording.source,
            None,
            quality,
            recording.source == TakeSource::RecordedConsent,
        )?;
        write_canonical_wav(&path, &canonical)?;
        let mut manifest = match self.storage.add_take(&recording.profile_id, take) {
            Ok(manifest) => manifest,
            Err(error) => {
                let _ = fs::remove_file(path);
                return Err(error);
            }
        };
        if recording.source == TakeSource::RecordedConsent {
            manifest.consent.recorded_consent_take_id = Some(take_id);
            super::storage::atomic_write_json(
                &self
                    .storage
                    .root()
                    .join(&recording.profile_id)
                    .join("consent/consent.json"),
                &manifest.consent,
            )?;
            self.storage.commit_manifest(&mut manifest)?;
        }
        Ok(())
    }

    fn current_profile_id(&self) -> DatasetResult<String> {
        self.lock_runtime()?
            .current_profile_id
            .clone()
            .ok_or_else(|| {
                DatasetError::new(
                    DatasetErrorCode::ProfileNotFound,
                    "Create or select a voice profile first.",
                )
            })
    }
    fn lock_runtime(&self) -> DatasetResult<std::sync::MutexGuard<'_, DatasetRuntime>> {
        self.runtime.lock().map_err(|_| {
            DatasetError::new(
                DatasetErrorCode::StorageUnavailable,
                "Voice Dataset state is temporarily unavailable.",
            )
        })
    }
}

type DatasetOperation = Box<dyn FnOnce(&VoiceDatasetSession) + Send>;

enum DatasetCommand {
    Run(DatasetOperation),
}

pub struct VoiceDatasetController {
    commands: SyncSender<DatasetCommand>,
    audio_active: Arc<AtomicBool>,
}

impl VoiceDatasetController {
    pub fn new(root: PathBuf) -> DatasetResult<Self> {
        let (commands, receiver) = mpsc::sync_channel(32);
        let (initialized, initialization) = mpsc::sync_channel(1);
        let audio_active = Arc::new(AtomicBool::new(false));
        let thread_audio_active = Arc::clone(&audio_active);
        thread::Builder::new()
            .name("voice-dataset-session".to_owned())
            .spawn(move || match VoiceDatasetSession::new(root) {
                Ok(session) => {
                    let _ = initialized.send(Ok(()));
                    run_session(session, receiver, thread_audio_active);
                }
                Err(error) => {
                    let _ = initialized.send(Err(error));
                }
            })
            .map_err(|error| {
                DatasetError::new(
                    DatasetErrorCode::StorageUnavailable,
                    format!("Cannot start the Voice Dataset controller: {error}"),
                )
            })?;
        initialization.recv().map_err(|_| {
            DatasetError::new(
                DatasetErrorCode::StorageUnavailable,
                "Voice Dataset initialization stopped unexpectedly.",
            )
        })??;
        Ok(Self {
            commands,
            audio_active,
        })
    }

    pub fn is_audio_active(&self) -> bool {
        self.audio_active.load(Ordering::Acquire)
    }

    pub fn list_profiles(&self) -> DatasetResult<Vec<VoiceProfileSummary>> {
        self.request(|session| session.list_profiles())
    }

    pub fn snapshot_source(
        &self,
        profile_id: &str,
    ) -> DatasetResult<super::source::ManifestDatasetSource> {
        let profile_id = profile_id.to_owned();
        self.request(move |session| session.snapshot_source(&profile_id))
    }

    pub fn prompts(&self) -> DatasetResult<PromptPack> {
        self.request(|session| Ok(session.prompts()))
    }

    pub fn create_profile(
        &self,
        request: CreateVoiceProfileRequest,
    ) -> DatasetResult<VoiceDatasetStatus> {
        self.request(move |session| session.create_profile(request))
    }

    pub fn select_profile(&self, profile_id: String) -> DatasetResult<VoiceDatasetStatus> {
        self.request(move |session| session.select_profile(profile_id))
    }

    pub fn update_profile(
        &self,
        profile_id: &str,
        request: UpdateVoiceProfileRequest,
    ) -> DatasetResult<VoiceDatasetStatus> {
        let profile_id = profile_id.to_owned();
        self.request(move |session| session.update_profile(&profile_id, request))
    }

    pub fn select_prompt(&self, selection: PromptSelection) -> DatasetResult<VoiceDatasetStatus> {
        self.request(move |session| session.select_prompt(selection))
    }

    pub fn start_recording(
        &self,
        input_id: &str,
        input_name: &str,
        recorded_consent: bool,
    ) -> DatasetResult<VoiceDatasetStatus> {
        let input_id = input_id.to_owned();
        let input_name = input_name.to_owned();
        self.request(move |session| {
            session.start_recording(&input_id, &input_name, recorded_consent)
        })
    }

    pub fn stop_recording(&self) -> DatasetResult<VoiceDatasetStatus> {
        self.request(VoiceDatasetSession::stop_recording)
    }

    pub fn discard_recording(&self) -> DatasetResult<VoiceDatasetStatus> {
        self.request(VoiceDatasetSession::discard_recording)
    }

    pub fn import_wavs(
        &self,
        paths: Vec<String>,
        selection: PromptSelection,
    ) -> DatasetResult<VoiceDatasetStatus> {
        self.request(move |session| session.import_wavs(paths, selection))
    }

    pub fn review_take(
        &self,
        profile_id: &str,
        take_id: &str,
        request: ReviewTakeRequest,
    ) -> DatasetResult<VoiceDatasetStatus> {
        let profile_id = profile_id.to_owned();
        let take_id = take_id.to_owned();
        self.request(move |session| session.review_take(&profile_id, &take_id, request))
    }

    pub fn set_trim(
        &self,
        take_id: String,
        start_frame: u64,
        end_frame: u64,
    ) -> DatasetResult<VoiceDatasetStatus> {
        self.request(move |session| session.set_trim(take_id, start_frame, end_frame))
    }

    pub fn auto_trim(&self, take_id: String) -> DatasetResult<VoiceDatasetStatus> {
        self.request(move |session| session.auto_trim(take_id))
    }

    pub fn apply_trim(&self) -> DatasetResult<VoiceDatasetStatus> {
        self.request(VoiceDatasetSession::apply_trim)
    }

    pub fn reset_trim(&self, take_id: &str) -> DatasetResult<VoiceDatasetStatus> {
        let take_id = take_id.to_owned();
        self.request(move |session| session.reset_trim(&take_id))
    }

    pub fn start_preview(
        &self,
        take_id: &str,
        version: SelectedTakeVersion,
        output_id: &str,
        output_name: &str,
        seek_ms: u64,
    ) -> DatasetResult<VoiceDatasetStatus> {
        let take_id = take_id.to_owned();
        let output_id = output_id.to_owned();
        let output_name = output_name.to_owned();
        self.request(move |session| {
            session.start_preview(&take_id, version, &output_id, &output_name, seek_ms)
        })
    }

    pub fn pause_preview(&self) -> DatasetResult<VoiceDatasetStatus> {
        self.request(VoiceDatasetSession::pause_preview)
    }

    pub fn stop_preview(&self) -> DatasetResult<()> {
        self.request(VoiceDatasetSession::stop_preview)
    }

    pub fn delete_take(&self, take_id: &str) -> DatasetResult<VoiceDatasetStatus> {
        let take_id = take_id.to_owned();
        self.request(move |session| session.delete_take(&take_id))
    }

    pub fn delete_profile(&self, profile_id: &str) -> DatasetResult<VoiceDatasetStatus> {
        let profile_id = profile_id.to_owned();
        self.request(move |session| session.delete_profile(&profile_id))
    }

    pub fn repair_profile(&self, profile_id: &str) -> DatasetResult<VoiceDatasetStatus> {
        let profile_id = profile_id.to_owned();
        self.request(move |session| session.repair_profile(&profile_id))
    }

    pub fn export(
        &self,
        destination: &Path,
        options: DatasetExportOptions,
    ) -> DatasetResult<PathBuf> {
        let destination = destination.to_owned();
        self.request(move |session| session.export(&destination, options))
    }

    pub fn stop_audio(&self) -> DatasetResult<()> {
        self.request(VoiceDatasetSession::stop_audio)
    }

    pub fn clear_error(&self) -> DatasetResult<VoiceDatasetStatus> {
        self.request(VoiceDatasetSession::clear_error)
    }

    pub fn status(&self) -> DatasetResult<VoiceDatasetStatus> {
        self.request(VoiceDatasetSession::status)
    }

    fn request<T: Send + 'static>(
        &self,
        operation: impl FnOnce(&VoiceDatasetSession) -> DatasetResult<T> + Send + 'static,
    ) -> DatasetResult<T> {
        let (reply, response) = mpsc::sync_channel(1);
        let audio_active = Arc::clone(&self.audio_active);
        self.commands
            .send(DatasetCommand::Run(Box::new(move |session| {
                let result = operation(session);
                audio_active.store(session.is_audio_active(), Ordering::Release);
                let _ = reply.send(result);
            })))
            .map_err(|_| {
                DatasetError::new(
                    DatasetErrorCode::StorageUnavailable,
                    "Voice Dataset is unavailable.",
                )
            })?;
        response.recv().map_err(|_| {
            DatasetError::new(
                DatasetErrorCode::StorageUnavailable,
                "Voice Dataset stopped before the operation completed.",
            )
        })?
    }
}

fn run_session(
    session: VoiceDatasetSession,
    receiver: Receiver<DatasetCommand>,
    audio_active: Arc<AtomicBool>,
) {
    while let Ok(DatasetCommand::Run(operation)) = receiver.recv() {
        operation(&session);
    }
    let _ = session.stop_audio();
    audio_active.store(false, Ordering::Release);
}

pub(crate) fn build_take(
    take_id: &str,
    samples: &[f32],
    prompt: Option<&VoicePrompt>,
    source: TakeSource,
    original_format: Option<super::take::OriginalFormatMetadata>,
    quality: super::quality::TakeQualityReport,
    exclude: bool,
) -> DatasetResult<DatasetTake> {
    Ok(DatasetTake {
        id: take_id.to_owned(),
        prompt_id: prompt.map(|prompt| prompt.id.clone()),
        prompt_text: prompt.map(|prompt| prompt.text.clone()),
        prompt_category: prompt.map(|prompt| prompt.category),
        source,
        raw_file: format!("raw/{take_id}.wav"),
        derived_file: None,
        selected_version: SelectedTakeVersion::Raw,
        original_format,
        sample_rate: CANONICAL_SAMPLE_RATE,
        channels: 1,
        frame_count: samples.len() as u64,
        duration_ms: samples.len() as u64 * 1_000 / u64::from(CANONICAL_SAMPLE_RATE),
        waveform_envelope: waveform_envelope(samples, WAVEFORM_BUCKETS),
        quality,
        trim: None,
        review_status: TakeReviewStatus::Pending,
        exclude_from_training: exclude,
        notes: None,
        manual_override: false,
        warning_acknowledged: false,
        created_at: timestamp()?,
        content_hash: sha256_samples(samples),
    })
}

fn resolve_prompt(selection: PromptSelection) -> DatasetResult<Option<VoicePrompt>> {
    if let Some(custom) = selection.custom_prompt_text {
        let text = custom.trim();
        if text.is_empty() || text.chars().count() > 500 || text.chars().any(char::is_control) {
            return Err(DatasetError::new(
                DatasetErrorCode::InvalidStateTransition,
                "Custom prompt text must contain 1 to 500 visible characters.",
            ));
        }
        return Ok(Some(VoicePrompt {
            id: new_id("prompt", &timestamp()?),
            text: text.to_owned(),
            category: PromptCategory::Custom,
            recommended_take_duration_ms: None,
        }));
    }
    selection
        .prompt_id
        .map(|id| {
            built_in_english_pack()
                .prompts
                .into_iter()
                .find(|prompt| prompt.id == id)
                .ok_or_else(|| {
                    DatasetError::new(
                        DatasetErrorCode::InvalidStateTransition,
                        "The selected prompt does not exist.",
                    )
                })
        })
        .transpose()
}

fn physical_device(
    direction: DeviceDirection,
    id: &str,
    name: &str,
    code: DatasetErrorCode,
) -> DatasetResult<device::DeviceInfo> {
    let list = device::list_devices()
        .map_err(|_| DatasetError::new(code, "Audio devices could not be enumerated."))?;
    let devices = match direction {
        DeviceDirection::Input => list.inputs,
        DeviceDirection::Output => list.outputs,
    };
    let exact = devices
        .iter()
        .filter(|device| device.id == id && !device.is_likely_virtual)
        .collect::<Vec<_>>();
    if let [device] = exact.as_slice() {
        return Ok((*device).clone());
    }
    let normalized = name.trim().to_lowercase();
    let friendly = devices
        .iter()
        .filter(|device| {
            !device.is_likely_virtual && device.name.trim().to_lowercase() == normalized
        })
        .collect::<Vec<_>>();
    if let [device] = friendly.as_slice() {
        return Ok((*device).clone());
    }
    Err(DatasetError::new(code, "The selected physical audio device is unavailable or ambiguous. Refresh and select it again."))
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
    };

    use super::{build_take, resolve_prompt, PromptSelection};
    use crate::voice_dataset::{
        consent::CONSENT_VERSION,
        controller::{DatasetExportOptions, VoiceDatasetSession},
        import::write_canonical_wav,
        profile::CreateVoiceProfileRequest,
        quality::{analyze_take, CaptureMetrics},
        take::{SelectedTakeVersion, TakeReviewStatus, TakeSource},
    };

    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    fn root(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "mam-dataset-controller-{label}-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ))
    }
    fn profile_request() -> CreateVoiceProfileRequest {
        CreateVoiceProfileRequest {
            display_name: "Consenting speaker".to_owned(),
            description: None,
            primary_language: "English".to_owned(),
            locale_tag: Some("en-US".to_owned()),
            collection_goal_minutes: Some(10),
            consent_confirmed: true,
            confirmed_by_user: true,
            consent_version: CONSENT_VERSION.to_owned(),
            consent_notes: None,
        }
    }

    #[test]
    fn custom_prompt_is_utf8_and_take_remains_pending() {
        let prompt = resolve_prompt(PromptSelection {
            prompt_id: None,
            custom_prompt_text: Some("สวัสดี โลก".to_owned()),
        })
        .unwrap()
        .unwrap();
        let samples = vec![0.1; 48_000];
        let take = build_take(
            "take-0000000000000000-00000000",
            &samples,
            Some(&prompt),
            TakeSource::Imported,
            None,
            analyze_take(&samples, 48_000, CaptureMetrics::default()),
            false,
        )
        .unwrap();
        assert_eq!(take.prompt_text.as_deref(), Some("สวัสดี โลก"));
        assert_eq!(take.review_status, TakeReviewStatus::Pending);
        assert!(!take.content_hash.is_empty());
    }

    #[test]
    fn export_includes_only_accepted_non_excluded_takes_by_default() {
        let managed = root("export-managed");
        let destination = root("export-destination");
        fs::create_dir_all(&destination).unwrap();
        let session = VoiceDatasetSession::new(managed.clone()).unwrap();
        let status = session.create_profile(profile_request()).unwrap();
        let profile_id = status.current_profile_id.unwrap();
        let samples: Vec<f32> = (0..48_000)
            .map(|index| (index as f32 * 0.04).sin() * 0.2)
            .collect();
        for (index, (source, review, excluded)) in [
            (TakeSource::Recorded, TakeReviewStatus::Accepted, false),
            (TakeSource::Imported, TakeReviewStatus::Rejected, false),
            (
                TakeSource::RecordedConsent,
                TakeReviewStatus::Accepted,
                true,
            ),
        ]
        .into_iter()
        .enumerate()
        {
            let id = format!("take-000000000000000{}-00000000", index + 1);
            let take_samples = samples
                .iter()
                .map(|sample| sample + index as f32 * 0.001)
                .collect::<Vec<_>>();
            let path = session.storage.raw_take_path(&profile_id, &id).unwrap();
            write_canonical_wav(&path, &take_samples).unwrap();
            let take = build_take(
                &id,
                &take_samples,
                None,
                source,
                None,
                analyze_take(&take_samples, 48_000, CaptureMetrics::default()),
                excluded,
            )
            .unwrap();
            session.storage.add_take(&profile_id, take).unwrap();
            session
                .storage
                .review_take(
                    &profile_id,
                    &id,
                    crate::voice_dataset::take::TakeReviewUpdate {
                        status: review,
                        exclude_from_training: excluded,
                        notes: None,
                        warning_acknowledged: true,
                        selected_version: SelectedTakeVersion::Raw,
                    },
                )
                .unwrap();
        }
        let package = session
            .export(
                &destination,
                DatasetExportOptions {
                    include_rejected: false,
                    include_raw_masters: false,
                },
            )
            .unwrap();
        let exported = fs::read_to_string(package.join("manifest.json")).unwrap();
        let exported_manifest = crate::voice_dataset::manifest::decode_manifest(&exported).unwrap();
        assert_eq!(exported_manifest.takes.len(), 1);
        assert_eq!(exported_manifest.takes[0].source, TakeSource::Recorded);
        assert!(!exported.contains(&managed.to_string_lossy().to_string()));
        assert_eq!(fs::read_dir(package.join("audio")).unwrap().count(), 1);
        session.stop_audio().unwrap();
        assert_eq!(
            session
                .storage
                .read_manifest(&profile_id)
                .unwrap()
                .takes
                .len(),
            3
        );
        fs::remove_dir_all(managed).unwrap();
        fs::remove_dir_all(destination).unwrap();
    }
}
