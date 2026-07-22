use std::{
    collections::HashSet,
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::Serialize;

use super::{
    consent::{ConsentMetadata, CONSENT_VERSION},
    error::{DatasetError, DatasetErrorCode, DatasetResult},
    manifest::{
        decode_manifest, DatasetRecordingFormat, VoiceDatasetManifestV1,
        DATASET_MANIFEST_SCHEMA_VERSION,
    },
    profile::{
        CreateVoiceProfileRequest, ProfileHealth, UpdateVoiceProfileRequest, VoiceProfileIndexV1,
        VoiceProfileMetadata, VoiceProfileSummary,
    },
    prompts::{built_in_english_pack, PromptPackReference},
    take::{validate_relative_path, DatasetTake, SelectedTakeVersion, TakeReviewStatus},
};

pub const PROFILE_INDEX_SCHEMA_VERSION: u32 = 1;
const PROFILE_INDEX_FILE: &str = "profiles.json";
const MANIFEST_FILE: &str = "manifest.json";
const CONSENT_FILE: &str = "consent/consent.json";
static ID_SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub struct DatasetStorage {
    root: PathBuf,
}

impl DatasetStorage {
    pub fn load(root: PathBuf) -> DatasetResult<Self> {
        fs::create_dir_all(&root)
            .map_err(|error| DatasetError::storage("Cannot create dataset storage", error))?;
        let storage = Self { root };
        let index_path = storage.index_path();
        recover_missing_target(&index_path)?;
        if !index_path.exists() {
            storage.write_index(&VoiceProfileIndexV1 {
                schema_version: PROFILE_INDEX_SCHEMA_VERSION,
                profiles: Vec::new(),
                updated_at: timestamp()?,
            })?;
        }
        Ok(storage)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn list_profiles(&self) -> DatasetResult<Vec<VoiceProfileSummary>> {
        let index = self.read_index()?;
        Ok(index
            .profiles
            .into_iter()
            .map(|profile| {
                let profile_dir = self.profile_dir_unchecked(&profile.id);
                let health = self
                    .profile_health(&profile.id)
                    .unwrap_or(ProfileHealth::NeedsRepair);
                let managed_storage_bytes = directory_size(&profile_dir).unwrap_or(0);
                VoiceProfileSummary {
                    profile,
                    health,
                    managed_storage_bytes,
                }
            })
            .collect())
    }

    pub fn create_profile(
        &self,
        request: CreateVoiceProfileRequest,
    ) -> DatasetResult<VoiceDatasetManifestV1> {
        if !request.consent_confirmed || !request.confirmed_by_user {
            return Err(DatasetError::new(
                DatasetErrorCode::ConsentRequired,
                "Confirm that the target speaker consented before creating a local profile.",
            ));
        }
        if request.consent_version.trim() != CONSENT_VERSION {
            return Err(DatasetError::new(
                DatasetErrorCode::ConsentRequired,
                "Read and confirm the current dataset consent notice.",
            ));
        }
        let display_name = validate_text(&request.display_name, "Profile display name", 1, 80)?;
        let primary_language = validate_text(&request.primary_language, "Primary language", 1, 64)?;
        let description = validate_optional_text(request.description, "Description", 500)?;
        let locale_tag = validate_optional_text(request.locale_tag, "Locale tag", 32)?;
        let consent_notes = validate_optional_text(request.consent_notes, "Consent notes", 500)?;
        if request
            .collection_goal_minutes
            .is_some_and(|value| value == 0 || value > 600)
        {
            return Err(DatasetError::new(
                DatasetErrorCode::InvalidStateTransition,
                "Collection goal must be between 1 and 600 minutes.",
            ));
        }
        let now = timestamp()?;
        let mut index = self.read_index()?;
        let mut id = new_id("profile", &now);
        while index.profiles.iter().any(|profile| profile.id == id)
            || self.profile_dir_unchecked(&id).exists()
        {
            id = new_id("profile", &now);
        }
        let profile = VoiceProfileMetadata {
            id: id.clone(),
            display_name,
            description,
            primary_language,
            locale_tag,
            collection_goal_minutes: request.collection_goal_minutes,
            created_at: now.clone(),
            updated_at: now.clone(),
        };
        let consent = ConsentMetadata {
            consent_confirmed: true,
            consent_version: CONSENT_VERSION.to_owned(),
            confirmed_at: now.clone(),
            confirmed_by_user: true,
            recorded_consent_take_id: None,
            revoked_at: None,
            notes: consent_notes,
        };
        let pack = built_in_english_pack();
        let mut manifest = VoiceDatasetManifestV1 {
            schema_version: DATASET_MANIFEST_SCHEMA_VERSION,
            profile: profile.clone(),
            consent: consent.clone(),
            recording_format: DatasetRecordingFormat::default(),
            prompt_pack: PromptPackReference {
                id: pack.id,
                version: pack.version,
            },
            takes: Vec::new(),
            statistics: Default::default(),
            created_at: now.clone(),
            updated_at: now,
        };
        manifest.rebuild_statistics(pack.prompts.len());
        let profile_dir = self.profile_dir(&id)?;
        fs::create_dir_all(profile_dir.join("consent"))
            .and_then(|_| fs::create_dir_all(profile_dir.join("raw")))
            .and_then(|_| fs::create_dir_all(profile_dir.join("derived")))
            .map_err(|error| {
                DatasetError::storage("Cannot create profile storage", error).profile(&id)
            })?;
        atomic_write_json(&profile_dir.join(CONSENT_FILE), &consent)?;
        atomic_write_json(&profile_dir.join(MANIFEST_FILE), &manifest)?;
        index.profiles.push(profile);
        index.updated_at = timestamp()?;
        if let Err(error) = self.write_index(&index) {
            let _ = fs::remove_dir_all(&profile_dir);
            return Err(error);
        }
        Ok(manifest)
    }

    pub fn read_manifest(&self, profile_id: &str) -> DatasetResult<VoiceDatasetManifestV1> {
        let path = self.profile_dir(profile_id)?.join(MANIFEST_FILE);
        recover_missing_target(&path)?;
        let contents = fs::read_to_string(&path).map_err(|_| {
            DatasetError::new(
                DatasetErrorCode::ProfileNotFound,
                "The selected voice profile is unavailable.",
            )
            .profile(profile_id)
        })?;
        match decode_manifest(&contents) {
            Ok(manifest) => Ok(manifest),
            Err(primary) => {
                let backup = recovery_path(&path, ".bak");
                if backup.exists() {
                    if let Ok(backup_contents) = fs::read_to_string(&backup) {
                        if let Ok(manifest) = decode_manifest(&backup_contents) {
                            fs::copy(&backup, &path).map_err(|error| {
                                DatasetError::storage(
                                    "Cannot restore the last valid manifest",
                                    error,
                                )
                                .profile(profile_id)
                            })?;
                            return Ok(manifest);
                        }
                    }
                }
                Err(primary.profile(profile_id))
            }
        }
    }

    pub fn snapshot_source(
        &self,
        profile_id: &str,
    ) -> DatasetResult<super::source::ManifestDatasetSource> {
        let manifest = self.read_manifest(profile_id)?;
        Ok(super::source::ManifestDatasetSource::new(
            &self.profile_dir(profile_id)?,
            manifest,
        ))
    }

    pub fn commit_manifest(&self, manifest: &mut VoiceDatasetManifestV1) -> DatasetResult<()> {
        let pack = built_in_english_pack();
        manifest.rebuild_statistics(pack.prompts.len());
        manifest.updated_at = timestamp()?;
        manifest.validate()?;
        atomic_write_json(
            &self.profile_dir(&manifest.profile.id)?.join(MANIFEST_FILE),
            manifest,
        )?;
        let mut index = self.read_index()?;
        let indexed = index
            .profiles
            .iter_mut()
            .find(|profile| profile.id == manifest.profile.id)
            .ok_or_else(|| {
                DatasetError::new(
                    DatasetErrorCode::ProfileNotFound,
                    "The profile is missing from the dataset index.",
                )
                .profile(&manifest.profile.id)
            })?;
        *indexed = manifest.profile.clone();
        index.updated_at = manifest.updated_at.clone();
        self.write_index(&index)
    }

    pub fn update_profile(
        &self,
        profile_id: &str,
        request: UpdateVoiceProfileRequest,
    ) -> DatasetResult<VoiceDatasetManifestV1> {
        let mut manifest = self.read_manifest(profile_id)?;
        manifest.profile.display_name =
            validate_text(&request.display_name, "Profile display name", 1, 80)?;
        manifest.profile.description =
            validate_optional_text(request.description, "Description", 500)?;
        manifest.profile.primary_language =
            validate_text(&request.primary_language, "Primary language", 1, 64)?;
        manifest.profile.locale_tag = validate_optional_text(request.locale_tag, "Locale tag", 32)?;
        if request
            .collection_goal_minutes
            .is_some_and(|value| value == 0 || value > 600)
        {
            return Err(DatasetError::new(
                DatasetErrorCode::InvalidStateTransition,
                "Collection goal must be between 1 and 600 minutes.",
            ));
        }
        manifest.profile.collection_goal_minutes = request.collection_goal_minutes;
        manifest.profile.updated_at = timestamp()?;
        self.commit_manifest(&mut manifest)?;
        Ok(manifest)
    }

    pub fn add_take(
        &self,
        profile_id: &str,
        mut take: DatasetTake,
    ) -> DatasetResult<VoiceDatasetManifestV1> {
        let mut manifest = self.read_manifest(profile_id)?;
        if manifest.takes.iter().any(|existing| existing.id == take.id) {
            return Err(DatasetError::new(
                DatasetErrorCode::InvalidStateTransition,
                "A take with this identifier already exists.",
            )
            .profile(profile_id)
            .take(&take.id));
        }
        if manifest
            .takes
            .iter()
            .any(|existing| existing.content_hash == take.content_hash)
        {
            return Err(DatasetError::new(
                DatasetErrorCode::DuplicateImport,
                "This exact recording already exists in the selected profile.",
            )
            .profile(profile_id));
        }
        take.review_status = TakeReviewStatus::Pending;
        manifest.takes.push(take);
        self.commit_manifest(&mut manifest)?;
        Ok(manifest)
    }

    pub fn review_take(
        &self,
        profile_id: &str,
        take_id: &str,
        update: super::take::TakeReviewUpdate,
    ) -> DatasetResult<VoiceDatasetManifestV1> {
        let mut manifest = self.read_manifest(profile_id)?;
        let take = manifest
            .takes
            .iter_mut()
            .find(|take| take.id == take_id)
            .ok_or_else(|| {
                DatasetError::new(
                    DatasetErrorCode::TakeNotFound,
                    "The selected take was not found.",
                )
                .profile(profile_id)
                .take(take_id)
            })?;
        if update.status == TakeReviewStatus::Accepted
            && take.quality.classification == super::quality::QualityClassification::Fail
            && !update.warning_acknowledged
        {
            return Err(DatasetError::new(
                DatasetErrorCode::InvalidStateTransition,
                "A failed take requires explicit warning acknowledgement before manual acceptance.",
            )
            .profile(profile_id)
            .take(take_id));
        }
        if update.selected_version == SelectedTakeVersion::Trimmed && take.derived_file.is_none() {
            return Err(DatasetError::new(
                DatasetErrorCode::InvalidStateTransition,
                "Apply a trimmed version before selecting it.",
            )
            .take(take_id));
        }
        take.review_status = update.status;
        take.exclude_from_training =
            update.exclude_from_training || take.source == super::take::TakeSource::RecordedConsent;
        take.notes = validate_optional_text(update.notes, "Take notes", 500)?;
        take.warning_acknowledged = update.warning_acknowledged;
        take.manual_override = update.status == TakeReviewStatus::Accepted
            && take.quality.classification == super::quality::QualityClassification::Fail;
        take.selected_version = update.selected_version;
        self.commit_manifest(&mut manifest)?;
        Ok(manifest)
    }

    pub fn raw_take_path(&self, profile_id: &str, take_id: &str) -> DatasetResult<PathBuf> {
        validate_id(take_id, "take")?;
        Ok(self
            .profile_dir(profile_id)?
            .join("raw")
            .join(format!("{take_id}.wav")))
    }

    pub fn derived_take_path(&self, profile_id: &str, take_id: &str) -> DatasetResult<PathBuf> {
        validate_id(take_id, "take")?;
        Ok(self
            .profile_dir(profile_id)?
            .join("derived")
            .join(format!("{take_id}-trimmed.wav")))
    }

    pub fn resolve_take_file(
        &self,
        profile_id: &str,
        take: &DatasetTake,
        version: SelectedTakeVersion,
    ) -> DatasetResult<PathBuf> {
        let relative = match version {
            SelectedTakeVersion::Raw => &take.raw_file,
            SelectedTakeVersion::Trimmed => take.derived_file.as_ref().ok_or_else(|| {
                DatasetError::new(
                    DatasetErrorCode::TakeNotFound,
                    "This take has no trimmed version.",
                )
            })?,
        };
        validate_relative_path(relative)?;
        let profile = self.profile_dir(profile_id)?;
        let path = profile.join(relative.replace('/', std::path::MAIN_SEPARATOR_STR));
        if !path.starts_with(&profile) {
            return Err(DatasetError::new(
                DatasetErrorCode::PathValidationFailed,
                "The take path escapes managed storage.",
            ));
        }
        Ok(path)
    }

    pub fn apply_trim(
        &self,
        profile_id: &str,
        take_id: &str,
        start_frame: u64,
        end_frame: u64,
        quality: super::quality::TakeQualityReport,
        waveform: Vec<super::take::WaveformPoint>,
    ) -> DatasetResult<VoiceDatasetManifestV1> {
        let mut manifest = self.read_manifest(profile_id)?;
        let take = manifest
            .takes
            .iter_mut()
            .find(|take| take.id == take_id)
            .ok_or_else(|| {
                DatasetError::new(
                    DatasetErrorCode::TakeNotFound,
                    "The selected take was not found.",
                )
                .take(take_id)
            })?;
        if start_frame >= end_frame || end_frame > take.frame_count {
            return Err(DatasetError::new(
                DatasetErrorCode::InvalidTrimRange,
                "Trim boundaries must select a non-empty range inside the raw take.",
            )
            .take(take_id));
        }
        take.derived_file = Some(format!("derived/{take_id}-trimmed.wav"));
        take.trim = Some(super::take::TrimMetadata {
            start_frame,
            end_frame,
            derived_quality: quality,
            derived_waveform_envelope: waveform,
        });
        take.selected_version = SelectedTakeVersion::Trimmed;
        self.commit_manifest(&mut manifest)?;
        Ok(manifest)
    }

    pub fn reset_trim(
        &self,
        profile_id: &str,
        take_id: &str,
    ) -> DatasetResult<VoiceDatasetManifestV1> {
        let mut manifest = self.read_manifest(profile_id)?;
        let take = manifest
            .takes
            .iter_mut()
            .find(|take| take.id == take_id)
            .ok_or_else(|| {
                DatasetError::new(
                    DatasetErrorCode::TakeNotFound,
                    "The selected take was not found.",
                )
                .take(take_id)
            })?;
        let derived = take.derived_file.take();
        take.trim = None;
        take.selected_version = SelectedTakeVersion::Raw;
        self.commit_manifest(&mut manifest)?;
        if let Some(relative) = derived {
            let path = self
                .profile_dir(profile_id)?
                .join(relative.replace('/', std::path::MAIN_SEPARATOR_STR));
            if path.exists() {
                fs::remove_file(path).map_err(|error| DatasetError::new(DatasetErrorCode::PartialDeletion, format!("The manifest was reset, but the old derived file could not be removed: {error}")).take(take_id))?;
            }
        }
        Ok(manifest)
    }

    pub fn delete_take(
        &self,
        profile_id: &str,
        take_id: &str,
    ) -> DatasetResult<VoiceDatasetManifestV1> {
        let mut manifest = self.read_manifest(profile_id)?;
        let position = manifest
            .takes
            .iter()
            .position(|take| take.id == take_id)
            .ok_or_else(|| {
                DatasetError::new(
                    DatasetErrorCode::TakeNotFound,
                    "The selected take was not found.",
                )
                .take(take_id)
            })?;
        let take = manifest.takes.remove(position);
        let tombstone = self
            .profile_dir(profile_id)?
            .join(format!("deletion-{take_id}.json"));
        atomic_write_json(
            &tombstone,
            &vec![
                take.raw_file.clone(),
                take.derived_file.clone().unwrap_or_default(),
            ],
        )?;
        self.commit_manifest(&mut manifest)?;
        let mut failures = Vec::new();
        for relative in [Some(take.raw_file), take.derived_file]
            .into_iter()
            .flatten()
            .filter(|path| !path.is_empty())
        {
            let path = self
                .profile_dir(profile_id)?
                .join(relative.replace('/', std::path::MAIN_SEPARATOR_STR));
            if path.exists() {
                if let Err(error) = fs::remove_file(&path) {
                    failures.push(error.to_string());
                }
            }
        }
        if failures.is_empty() {
            let _ = fs::remove_file(tombstone);
        } else {
            return Err(DatasetError::new(DatasetErrorCode::PartialDeletion, format!("The take was removed from the manifest, but {} managed file(s) remain for retry.", failures.len())).profile(profile_id).take(take_id));
        }
        Ok(manifest)
    }

    pub fn delete_profile(&self, profile_id: &str) -> DatasetResult<()> {
        validate_id(profile_id, "profile")?;
        let mut index = self.read_index()?;
        let before = index.profiles.len();
        index.profiles.retain(|profile| profile.id != profile_id);
        if index.profiles.len() == before && !self.profile_dir_unchecked(profile_id).exists() {
            return Ok(());
        }
        index.updated_at = timestamp()?;
        self.write_index(&index)?;
        let dir = self.profile_dir_unchecked(profile_id);
        if dir.exists() {
            let marker = dir.join("deletion.json");
            atomic_write_json(
                &marker,
                &serde_json::json!({"profileId": profile_id, "startedAt": timestamp()?}),
            )?;
            fs::remove_dir_all(&dir).map_err(|_| DatasetError::new(DatasetErrorCode::PartialDeletion, "The profile is no longer selectable, but some managed files could not be removed. Retry deletion after closing programs that use them.").profile(profile_id))?;
        }
        Ok(())
    }

    pub fn profile_health(&self, profile_id: &str) -> DatasetResult<ProfileHealth> {
        let profile_dir = self.profile_dir(profile_id)?;
        if profile_dir.join("deletion.json").exists()
            || fs::read_dir(&profile_dir).ok().is_some_and(|entries| {
                entries
                    .filter_map(Result::ok)
                    .any(|entry| entry.file_name().to_string_lossy().starts_with("deletion-"))
            })
        {
            return Ok(ProfileHealth::NeedsRepair);
        }
        let manifest = match self.read_manifest(profile_id) {
            Ok(manifest) => manifest,
            Err(error) => {
                return Ok(match error.code {
                    DatasetErrorCode::FutureManifestSchema => ProfileHealth::UnsupportedSchema,
                    DatasetErrorCode::CorruptManifest => ProfileHealth::CorruptManifest,
                    _ => ProfileHealth::NeedsRepair,
                })
            }
        };
        let mut referenced = HashSet::new();
        for take in &manifest.takes {
            referenced.insert(take.raw_file.clone());
            if let Some(path) = &take.derived_file {
                referenced.insert(path.clone());
            }
            if !self
                .resolve_take_file(profile_id, take, SelectedTakeVersion::Raw)?
                .exists()
                || (take.derived_file.is_some()
                    && !self
                        .resolve_take_file(profile_id, take, SelectedTakeVersion::Trimmed)?
                        .exists())
            {
                return Ok(ProfileHealth::MissingFiles);
            }
        }
        for folder in ["raw", "derived"] {
            let dir = profile_dir.join(folder);
            if dir.exists() {
                for entry in fs::read_dir(dir)
                    .map_err(|error| DatasetError::storage("Cannot inspect profile files", error))?
                {
                    let entry = entry.map_err(|error| {
                        DatasetError::storage("Cannot inspect profile files", error)
                    })?;
                    let relative = format!("{folder}/{}", entry.file_name().to_string_lossy());
                    if !referenced.contains(&relative) {
                        return Ok(ProfileHealth::OrphanedFiles);
                    }
                }
            }
        }
        Ok(ProfileHealth::Healthy)
    }

    pub fn repair_profile(&self, profile_id: &str) -> DatasetResult<VoiceDatasetManifestV1> {
        let mut manifest = self.read_manifest(profile_id)?;
        for take in &mut manifest.takes {
            if take.derived_file.is_some()
                && !self
                    .resolve_take_file(profile_id, take, SelectedTakeVersion::Trimmed)?
                    .exists()
            {
                take.derived_file = None;
                take.trim = None;
                take.selected_version = SelectedTakeVersion::Raw;
            }
        }
        self.commit_manifest(&mut manifest)?;
        Ok(manifest)
    }

    fn read_index(&self) -> DatasetResult<VoiceProfileIndexV1> {
        let contents = fs::read_to_string(self.index_path())
            .map_err(|error| DatasetError::storage("Cannot read the profile index", error))?;
        let index: VoiceProfileIndexV1 = serde_json::from_str(&contents).map_err(|_| {
            DatasetError::new(
                DatasetErrorCode::CorruptManifest,
                "The voice profile index is corrupt.",
            )
        })?;
        if index.schema_version != PROFILE_INDEX_SCHEMA_VERSION {
            return Err(DatasetError::new(
                DatasetErrorCode::FutureManifestSchema,
                format!(
                    "Unsupported profile index schema version {}.",
                    index.schema_version
                ),
            ));
        }
        let mut ids = HashSet::new();
        if index
            .profiles
            .iter()
            .any(|profile| validate_id(&profile.id, "profile").is_err() || !ids.insert(&profile.id))
        {
            return Err(DatasetError::new(
                DatasetErrorCode::CorruptManifest,
                "The voice profile index contains invalid or duplicate identifiers.",
            ));
        }
        Ok(index)
    }
    fn write_index(&self, index: &VoiceProfileIndexV1) -> DatasetResult<()> {
        atomic_write_json(&self.index_path(), index)
    }
    fn index_path(&self) -> PathBuf {
        self.root.join(PROFILE_INDEX_FILE)
    }
    fn profile_dir(&self, profile_id: &str) -> DatasetResult<PathBuf> {
        validate_id(profile_id, "profile")?;
        Ok(self.profile_dir_unchecked(profile_id))
    }
    fn profile_dir_unchecked(&self, profile_id: &str) -> PathBuf {
        self.root.join(profile_id)
    }
}

pub fn timestamp() -> DatasetResult<String> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| {
            DatasetError::new(
                DatasetErrorCode::StorageUnavailable,
                "The system clock is before Unix epoch.",
            )
        })?
        .as_millis()
        .to_string())
}
pub fn new_id(kind: &str, timestamp: &str) -> String {
    let sequence = ID_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in timestamp
        .bytes()
        .chain(std::process::id().to_le_bytes())
        .chain(sequence.to_le_bytes())
    {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{kind}-{hash:016x}-{sequence:08x}")
}
pub fn validate_id(id: &str, kind: &str) -> DatasetResult<()> {
    let valid = id.starts_with(&format!("{kind}-"))
        && id.len() <= 80
        && id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-');
    if valid {
        Ok(())
    } else {
        Err(DatasetError::new(
            DatasetErrorCode::PathValidationFailed,
            format!("Invalid managed {kind} identifier."),
        ))
    }
}

fn validate_text(
    value: &str,
    label: &str,
    minimum: usize,
    maximum: usize,
) -> DatasetResult<String> {
    let trimmed = value.trim();
    let length = trimmed.chars().count();
    if !(minimum..=maximum).contains(&length) || trimmed.chars().any(char::is_control) {
        return Err(DatasetError::new(
            DatasetErrorCode::InvalidStateTransition,
            format!("{label} must contain {minimum} to {maximum} visible characters."),
        ));
    }
    Ok(trimmed.to_owned())
}
fn validate_optional_text(
    value: Option<String>,
    label: &str,
    maximum: usize,
) -> DatasetResult<Option<String>> {
    value
        .map(|value| validate_text(&value, label, 1, maximum))
        .transpose()
}
fn recovery_path(path: &Path, suffix: &str) -> PathBuf {
    path.with_file_name(format!(
        "{}{suffix}",
        path.file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("dataset.json")
    ))
}
fn recover_missing_target(path: &Path) -> DatasetResult<()> {
    if path.exists() {
        return Ok(());
    }
    let backup = recovery_path(path, ".bak");
    let temporary = recovery_path(path, ".tmp");
    let source = if backup.exists() {
        Some(backup)
    } else if temporary.exists() {
        Some(temporary)
    } else {
        None
    };
    if let Some(source) = source {
        fs::rename(source, path).map_err(|error| {
            DatasetError::new(
                DatasetErrorCode::AtomicWriteFailed,
                format!("Cannot recover an interrupted dataset write: {error}"),
            )
        })?;
    }
    Ok(())
}
pub fn atomic_write_json(path: &Path, value: &impl Serialize) -> DatasetResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| DatasetError::storage("Cannot create dataset directory", error))?;
    }
    let bytes = serde_json::to_vec_pretty(value).map_err(|_| {
        DatasetError::new(
            DatasetErrorCode::CorruptManifest,
            "Dataset metadata could not be serialized.",
        )
    })?;
    let temporary = recovery_path(path, ".tmp");
    let backup = recovery_path(path, ".bak");
    if temporary.exists() {
        fs::remove_file(&temporary)
            .map_err(|error| DatasetError::storage("Cannot clear a stale temporary file", error))?;
    }
    let mut file = File::create(&temporary)
        .map_err(|error| DatasetError::storage("Cannot create a temporary dataset file", error))?;
    file.write_all(&bytes)
        .and_then(|_| file.write_all(b"\n"))
        .and_then(|_| file.sync_all())
        .map_err(|error| {
            DatasetError::new(
                DatasetErrorCode::AtomicWriteFailed,
                format!("Cannot flush dataset metadata: {error}"),
            )
        })?;
    drop(file);
    if path.exists() {
        if backup.exists() {
            fs::remove_file(&backup).map_err(|error| {
                DatasetError::storage("Cannot clear an old metadata backup", error)
            })?;
        }
        fs::rename(path, &backup).map_err(|error| {
            DatasetError::new(
                DatasetErrorCode::AtomicWriteFailed,
                format!("Cannot preserve the last valid metadata: {error}"),
            )
        })?;
        if let Err(error) = fs::rename(&temporary, path) {
            let _ = fs::rename(&backup, path);
            return Err(DatasetError::new(
                DatasetErrorCode::AtomicWriteFailed,
                format!("Cannot replace dataset metadata: {error}"),
            ));
        }
        fs::remove_file(backup)
            .map_err(|error| DatasetError::storage("Cannot clear metadata backup", error))?;
    } else {
        fs::rename(temporary, path).map_err(|error| {
            DatasetError::new(
                DatasetErrorCode::AtomicWriteFailed,
                format!("Cannot install dataset metadata: {error}"),
            )
        })?;
    }
    Ok(())
}
fn directory_size(path: &Path) -> std::io::Result<u64> {
    if !path.exists() {
        return Ok(0);
    }
    let mut total = 0;
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let metadata = fs::symlink_metadata(entry.path())?;
        if metadata.file_type().is_symlink() {
            continue;
        }
        total += if metadata.is_dir() {
            directory_size(&entry.path())?
        } else {
            metadata.len()
        };
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::{atomic_write_json, new_id, DatasetStorage};
    use crate::voice_dataset::{
        consent::CONSENT_VERSION,
        controller::build_take,
        import::write_canonical_wav,
        profile::{CreateVoiceProfileRequest, ProfileHealth},
        quality::{analyze_take, CaptureMetrics},
        take::{SelectedTakeVersion, TakeReviewStatus, TakeSource},
    };
    use std::{
        fs,
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
    };
    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    fn root(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "mam-dataset-{label}-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ))
    }
    fn request(consent: bool) -> CreateVoiceProfileRequest {
        CreateVoiceProfileRequest {
            display_name: "Speaker / ../ safe".to_owned(),
            description: None,
            primary_language: "English".to_owned(),
            locale_tag: Some("en-US".to_owned()),
            collection_goal_minutes: Some(10),
            consent_confirmed: consent,
            confirmed_by_user: consent,
            consent_version: CONSENT_VERSION.to_owned(),
            consent_notes: None,
        }
    }

    #[test]
    fn profile_requires_consent_and_name_never_becomes_a_path() {
        let root = root("profile");
        let storage = DatasetStorage::load(root.clone()).unwrap();
        assert!(storage.create_profile(request(false)).is_err());
        let manifest = storage.create_profile(request(true)).unwrap();
        assert!(manifest.profile.id.starts_with("profile-"));
        assert!(!root.join("Speaker").exists());
        assert!(root
            .join(&manifest.profile.id)
            .join("consent/consent.json")
            .exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn manifest_round_trip_future_schema_and_atomic_replacement() {
        let root = root("roundtrip");
        let storage = DatasetStorage::load(root.clone()).unwrap();
        let manifest = storage.create_profile(request(true)).unwrap();
        assert_eq!(
            storage.read_manifest(&manifest.profile.id).unwrap(),
            manifest
        );
        let path = root.join(&manifest.profile.id).join("manifest.json");
        let mut value: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        value["schemaVersion"] = 99.into();
        atomic_write_json(&path, &value).unwrap();
        assert!(storage
            .read_manifest(&manifest.profile.id)
            .unwrap_err()
            .message
            .contains("99"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn opaque_ids_are_collision_safe_in_sequence() {
        assert_ne!(new_id("take", "1000"), new_id("take", "1000"));
    }

    #[test]
    fn profile_deletion_removes_managed_data_and_is_idempotent() {
        let root = root("delete");
        let storage = DatasetStorage::load(root.clone()).unwrap();
        let id = storage.create_profile(request(true)).unwrap().profile.id;
        storage.delete_profile(&id).unwrap();
        storage.delete_profile(&id).unwrap();
        assert!(!root.join(&id).exists());
        assert!(storage.list_profiles().unwrap().is_empty());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn review_updates_statistics_requires_failed_override_and_take_delete_is_reference_safe() {
        let root = root("review");
        let storage = DatasetStorage::load(root.clone()).unwrap();
        let profile_id = storage.create_profile(request(true)).unwrap().profile.id;
        let samples = vec![0.0; 48_000];
        let take_id = "take-0000000000000001-00000000";
        write_canonical_wav(
            &storage.raw_take_path(&profile_id, take_id).unwrap(),
            &samples,
        )
        .unwrap();
        let take = build_take(
            take_id,
            &samples,
            None,
            TakeSource::Recorded,
            None,
            analyze_take(&samples, 48_000, CaptureMetrics::default()),
            false,
        )
        .unwrap();
        let manifest = storage.add_take(&profile_id, take).unwrap();
        assert_eq!(manifest.statistics.pending_takes, 1);
        assert!(storage
            .review_take(
                &profile_id,
                take_id,
                crate::voice_dataset::take::TakeReviewUpdate {
                    status: TakeReviewStatus::Accepted,
                    exclude_from_training: false,
                    notes: None,
                    warning_acknowledged: false,
                    selected_version: SelectedTakeVersion::Raw,
                }
            )
            .is_err());
        let accepted = storage
            .review_take(
                &profile_id,
                take_id,
                crate::voice_dataset::take::TakeReviewUpdate {
                    status: TakeReviewStatus::Accepted,
                    exclude_from_training: false,
                    notes: Some("Manually retained".to_owned()),
                    warning_acknowledged: true,
                    selected_version: SelectedTakeVersion::Raw,
                },
            )
            .unwrap();
        assert_eq!(accepted.statistics.accepted_takes, 1);
        assert!(accepted.takes[0].manual_override);
        assert_eq!(
            accepted.takes[0].notes.as_deref(),
            Some("Manually retained")
        );
        let deleted = storage.delete_take(&profile_id, take_id).unwrap();
        assert!(deleted.takes.is_empty());
        assert!(!storage
            .raw_take_path(&profile_id, take_id)
            .unwrap()
            .exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn missing_and_orphaned_files_are_reported_without_silent_deletion() {
        let root = root("health");
        let storage = DatasetStorage::load(root.clone()).unwrap();
        let profile_id = storage.create_profile(request(true)).unwrap().profile.id;
        let orphan = root.join(&profile_id).join("raw/orphan.wav");
        fs::write(&orphan, b"orphan").unwrap();
        assert_eq!(
            storage.profile_health(&profile_id).unwrap(),
            ProfileHealth::OrphanedFiles
        );
        assert!(orphan.exists());
        fs::remove_file(orphan).unwrap();
        let samples = vec![0.1; 48_000];
        let take_id = "take-0000000000000002-00000000";
        let raw = storage.raw_take_path(&profile_id, take_id).unwrap();
        write_canonical_wav(&raw, &samples).unwrap();
        let take = build_take(
            take_id,
            &samples,
            None,
            TakeSource::Imported,
            None,
            analyze_take(&samples, 48_000, CaptureMetrics::default()),
            false,
        )
        .unwrap();
        storage.add_take(&profile_id, take).unwrap();
        fs::remove_file(raw).unwrap();
        assert_eq!(
            storage.profile_health(&profile_id).unwrap(),
            ProfileHealth::MissingFiles
        );
        fs::remove_dir_all(root).unwrap();
    }
}
