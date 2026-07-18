use std::{
    collections::HashSet,
    fs::{self, File},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::dsp::chain::DspParameters;

pub const PRESET_SCHEMA_VERSION: u32 = 1;
pub const PRESET_FILE_NAME: &str = "presets.json";
pub const NATURAL_PRESET_ID: &str = "builtin-natural";
const MAX_PRESET_NAME_CHARS: usize = 64;
const MAX_PRESET_ID_CHARS: usize = 128;
static ID_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PresetDocument {
    pub schema_version: u32,
    pub presets: Vec<UserPreset>,
    pub selected_preset_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UserPreset {
    pub id: String,
    pub name: String,
    pub parameters: DspParameters,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetView {
    pub id: String,
    pub name: String,
    pub parameters: DspParameters,
    pub built_in: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetCatalog {
    pub schema_version: u32,
    pub presets: Vec<PresetView>,
    pub selected_preset_id: Option<String>,
    pub active_parameters: DspParameters,
}

#[derive(Debug, Error)]
pub enum PresetError {
    #[error("Preset storage failed: {0}")]
    Storage(#[from] io::Error),
    #[error("Preset data is not valid JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("{0}")]
    Validation(String),
    #[error("Preset was not found.")]
    NotFound,
    #[error("Built-in presets cannot be changed or deleted.")]
    BuiltInReadOnly,
}

pub struct PresetStore {
    path: PathBuf,
    document: PresetDocument,
}

impl Default for PresetDocument {
    fn default() -> Self {
        Self {
            schema_version: PRESET_SCHEMA_VERSION,
            presets: Vec::new(),
            selected_preset_id: Some(NATURAL_PRESET_ID.to_owned()),
        }
    }
}

impl PresetStore {
    pub fn load(path: PathBuf) -> Result<Self, PresetError> {
        recover_interrupted_write(&path)?;
        if !path.exists() {
            let store = Self {
                path,
                document: PresetDocument::default(),
            };
            store.persist_document(&store.document)?;
            return Ok(store);
        }

        let contents = fs::read_to_string(&path)?;
        let document: PresetDocument = serde_json::from_str(&contents)?;
        validate_document(&document)?;
        remove_recovery_files(&path)?;

        Ok(Self { path, document })
    }

    pub fn catalog(&self) -> Result<PresetCatalog, PresetError> {
        let mut presets = built_in_presets();
        presets.extend(self.document.presets.iter().map(PresetView::from));

        Ok(PresetCatalog {
            schema_version: PRESET_SCHEMA_VERSION,
            presets,
            selected_preset_id: self.document.selected_preset_id.clone(),
            active_parameters: self.selected_parameters()?,
        })
    }

    pub fn selected_parameters(&self) -> Result<DspParameters, PresetError> {
        match self.document.selected_preset_id.as_deref() {
            Some(id) => self.parameters_for(id),
            None => Ok(DspParameters::default()),
        }
    }

    pub fn save_preset(
        &mut self,
        name: String,
        parameters: DspParameters,
    ) -> Result<(), PresetError> {
        let name = validate_name(&name)?;
        let parameters = parameters.validate().map_err(PresetError::Validation)?;
        let timestamp = unix_timestamp_millis()?;
        let mut id = new_user_preset_id(&timestamp);
        while self.document.presets.iter().any(|preset| preset.id == id) {
            id = new_user_preset_id(&timestamp);
        }

        let mut next = self.document.clone();
        next.presets.push(UserPreset {
            id: id.clone(),
            name,
            parameters,
            created_at: timestamp.clone(),
            updated_at: timestamp,
        });
        next.selected_preset_id = Some(id);
        self.commit(next)
    }

    pub fn rename_preset(&mut self, id: &str, name: String) -> Result<(), PresetError> {
        if is_built_in(id) {
            return Err(PresetError::BuiltInReadOnly);
        }
        let name = validate_name(&name)?;
        let mut next = self.document.clone();
        let preset = next
            .presets
            .iter_mut()
            .find(|preset| preset.id == id)
            .ok_or(PresetError::NotFound)?;
        preset.name = name;
        preset.updated_at = unix_timestamp_millis()?;
        self.commit(next)
    }

    pub fn duplicate_preset(&mut self, id: &str) -> Result<(), PresetError> {
        let source = self.view_for(id).ok_or(PresetError::NotFound)?;
        let name = unique_copy_name(&source.name, &self.document.presets);
        self.save_preset(name, source.parameters)
    }

    pub fn delete_preset(&mut self, id: &str) -> Result<(), PresetError> {
        if is_built_in(id) {
            return Err(PresetError::BuiltInReadOnly);
        }

        let mut next = self.document.clone();
        let before = next.presets.len();
        next.presets.retain(|preset| preset.id != id);
        if next.presets.len() == before {
            return Err(PresetError::NotFound);
        }
        if next.selected_preset_id.as_deref() == Some(id) {
            next.selected_preset_id = Some(NATURAL_PRESET_ID.to_owned());
        }
        self.commit(next)
    }

    pub fn select_preset(&mut self, id: &str) -> Result<DspParameters, PresetError> {
        let parameters = self.parameters_for(id)?;
        let mut next = self.document.clone();
        next.selected_preset_id = Some(id.to_owned());
        self.commit(next)?;
        Ok(parameters)
    }

    pub fn reset_to_default(&mut self) -> Result<DspParameters, PresetError> {
        self.select_preset(NATURAL_PRESET_ID)
    }

    fn parameters_for(&self, id: &str) -> Result<DspParameters, PresetError> {
        self.view_for(id)
            .map(|preset| preset.parameters)
            .ok_or(PresetError::NotFound)
    }

    fn view_for(&self, id: &str) -> Option<PresetView> {
        built_in_presets()
            .into_iter()
            .find(|preset| preset.id == id)
            .or_else(|| {
                self.document
                    .presets
                    .iter()
                    .find(|preset| preset.id == id)
                    .map(PresetView::from)
            })
    }

    fn commit(&mut self, next: PresetDocument) -> Result<(), PresetError> {
        validate_document(&next)?;
        self.persist_document(&next)?;
        self.document = next;
        Ok(())
    }

    fn persist_document(&self, document: &PresetDocument) -> Result<(), PresetError> {
        validate_document(document)?;
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }

        let bytes = serde_json::to_vec_pretty(document)?;
        let temporary = recovery_path(&self.path, ".tmp");
        let backup = recovery_path(&self.path, ".bak");
        if temporary.exists() {
            fs::remove_file(&temporary)?;
        }

        let mut file = File::create(&temporary)?;
        file.write_all(&bytes)?;
        file.write_all(b"\n")?;
        file.sync_all()?;
        drop(file);

        if self.path.exists() {
            if backup.exists() {
                fs::remove_file(&backup)?;
            }
            fs::rename(&self.path, &backup)?;
            if let Err(error) = fs::rename(&temporary, &self.path) {
                let _ = fs::rename(&backup, &self.path);
                let _ = fs::remove_file(&temporary);
                return Err(PresetError::Storage(error));
            }
            fs::remove_file(backup)?;
        } else {
            fs::rename(temporary, &self.path)?;
        }

        Ok(())
    }
}

impl From<&UserPreset> for PresetView {
    fn from(preset: &UserPreset) -> Self {
        Self {
            id: preset.id.clone(),
            name: preset.name.clone(),
            parameters: preset.parameters,
            built_in: false,
        }
    }
}

pub fn built_in_presets() -> Vec<PresetView> {
    vec![
        PresetView {
            id: NATURAL_PRESET_ID.to_owned(),
            name: "Natural".to_owned(),
            parameters: DspParameters::default(),
            built_in: true,
        },
        PresetView {
            id: "builtin-warm-tone".to_owned(),
            name: "Warm tone".to_owned(),
            parameters: DspParameters {
                warmth_db: 3.0,
                brightness_db: -0.5,
                ..DspParameters::default()
            },
            built_in: true,
        },
        PresetView {
            id: "builtin-bright-tone".to_owned(),
            name: "Bright tone".to_owned(),
            parameters: DspParameters {
                warmth_db: -0.5,
                brightness_db: 3.0,
                ..DspParameters::default()
            },
            built_in: true,
        },
    ]
}

fn validate_document(document: &PresetDocument) -> Result<(), PresetError> {
    if document.schema_version != PRESET_SCHEMA_VERSION {
        return Err(PresetError::Validation(format!(
            "Unsupported preset schema version {}. Expected version {}.",
            document.schema_version, PRESET_SCHEMA_VERSION
        )));
    }

    let built_in_ids: HashSet<String> = built_in_presets()
        .into_iter()
        .map(|preset| preset.id)
        .collect();
    let mut ids = built_in_ids.clone();

    for preset in &document.presets {
        validate_user_id(&preset.id)?;
        if !ids.insert(preset.id.clone()) {
            return Err(PresetError::Validation(format!(
                "Preset id '{}' is duplicated or reserved.",
                preset.id
            )));
        }
        validate_name(&preset.name)?;
        preset
            .parameters
            .validate()
            .map_err(PresetError::Validation)?;
        let created = validate_timestamp(&preset.created_at)?;
        let updated = validate_timestamp(&preset.updated_at)?;
        if updated < created {
            return Err(PresetError::Validation(format!(
                "Preset '{}' was updated before it was created.",
                preset.id
            )));
        }
    }

    if let Some(selected_id) = &document.selected_preset_id {
        if !ids.contains(selected_id) {
            return Err(PresetError::Validation(
                "The selected preset does not exist.".to_owned(),
            ));
        }
    }

    Ok(())
}

fn validate_name(name: &str) -> Result<String, PresetError> {
    let trimmed = name.trim();
    let length = trimmed.chars().count();
    if length == 0 || length > MAX_PRESET_NAME_CHARS {
        return Err(PresetError::Validation(format!(
            "Preset name must contain 1 to {MAX_PRESET_NAME_CHARS} characters."
        )));
    }
    if trimmed.chars().any(char::is_control) {
        return Err(PresetError::Validation(
            "Preset name cannot contain control characters.".to_owned(),
        ));
    }
    Ok(trimmed.to_owned())
}

fn validate_user_id(id: &str) -> Result<(), PresetError> {
    if id.is_empty()
        || id.len() > MAX_PRESET_ID_CHARS
        || !id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
        || !id.starts_with("user-")
    {
        return Err(PresetError::Validation(
            "A user preset has an invalid id.".to_owned(),
        ));
    }
    Ok(())
}

fn validate_timestamp(timestamp: &str) -> Result<u128, PresetError> {
    timestamp
        .parse::<u128>()
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| {
            PresetError::Validation("Preset timestamps must be Unix milliseconds.".to_owned())
        })
}

fn unix_timestamp_millis() -> Result<String, PresetError> {
    let duration = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|_| {
        PresetError::Validation("The system clock is before Unix epoch.".to_owned())
    })?;
    Ok(duration.as_millis().to_string())
}

fn new_user_preset_id(timestamp: &str) -> String {
    let sequence = ID_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    format!("user-{timestamp}-{}-{sequence}", std::process::id())
}

fn unique_copy_name(source_name: &str, existing: &[UserPreset]) -> String {
    for number in 1.. {
        let suffix = if number == 1 {
            " Copy".to_owned()
        } else {
            format!(" Copy {number}")
        };
        let base_length = MAX_PRESET_NAME_CHARS.saturating_sub(suffix.chars().count());
        let base: String = source_name.chars().take(base_length).collect();
        let candidate = format!("{base}{suffix}");
        if !existing.iter().any(|preset| preset.name == candidate) {
            return candidate;
        }
    }
    unreachable!()
}

fn is_built_in(id: &str) -> bool {
    built_in_presets().iter().any(|preset| preset.id == id)
}

fn recovery_path(path: &Path, suffix: &str) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(PRESET_FILE_NAME);
    path.with_file_name(format!("{file_name}{suffix}"))
}

fn recover_interrupted_write(path: &Path) -> Result<(), PresetError> {
    if path.exists() {
        return Ok(());
    }

    let backup = recovery_path(path, ".bak");
    let temporary = recovery_path(path, ".tmp");
    if backup.exists() {
        fs::rename(backup, path)?;
    } else if temporary.exists() {
        fs::rename(temporary, path)?;
    }
    Ok(())
}

fn remove_recovery_files(path: &Path) -> Result<(), PresetError> {
    for suffix in [".tmp", ".bak"] {
        let recovery = recovery_path(path, suffix);
        if recovery.exists() {
            fs::remove_file(recovery)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        sync::atomic::{AtomicU64, Ordering},
    };

    use super::{
        validate_document, PresetDocument, PresetStore, UserPreset, NATURAL_PRESET_ID,
        PRESET_SCHEMA_VERSION,
    };
    use crate::dsp::chain::DspParameters;

    static TEST_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    fn test_path(label: &str) -> PathBuf {
        let sequence = TEST_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "mam-voice-changer-{label}-{}-{sequence}.json",
            std::process::id()
        ))
    }

    fn cleanup(path: &Path) {
        for suffix in ["", ".tmp", ".bak"] {
            let target = if suffix.is_empty() {
                path.to_path_buf()
            } else {
                path.with_file_name(format!(
                    "{}{suffix}",
                    path.file_name().unwrap().to_string_lossy()
                ))
            };
            let _ = fs::remove_file(target);
        }
    }

    #[test]
    fn serializes_and_deserializes_versioned_document() {
        let document = PresetDocument::default();
        let json = serde_json::to_string(&document).unwrap();
        let decoded: PresetDocument = serde_json::from_str(&json).unwrap();

        assert_eq!(decoded, document);
        assert!(json.contains("\"schemaVersion\":1"));
        assert!(json.contains("\"selectedPresetId\":\"builtin-natural\""));
    }

    #[test]
    fn rejects_unsupported_schema_invalid_parameters_and_unknown_fields() {
        let mut document = PresetDocument {
            schema_version: PRESET_SCHEMA_VERSION + 1,
            ..PresetDocument::default()
        };
        assert!(validate_document(&document).is_err());

        document.schema_version = PRESET_SCHEMA_VERSION;
        document.presets.push(UserPreset {
            id: "user-invalid".to_owned(),
            name: "Invalid".to_owned(),
            parameters: DspParameters {
                dry_wet: 1.1,
                ..DspParameters::default()
            },
            created_at: "1000".to_owned(),
            updated_at: "1000".to_owned(),
        });
        assert!(validate_document(&document).is_err());

        let unknown = r#"{"schemaVersion":1,"presets":[],"selectedPresetId":null,"extra":true}"#;
        assert!(serde_json::from_str::<PresetDocument>(unknown).is_err());
    }

    #[test]
    fn persists_user_presets_and_last_selection() {
        let path = test_path("round-trip");
        cleanup(&path);

        let mut store = PresetStore::load(path.clone()).unwrap();
        let parameters = DspParameters {
            pitch_semitones: -2.0,
            warmth_db: 2.5,
            dry_wet: 0.4,
            ..DspParameters::default()
        };
        store
            .save_preset("Deeper warm".to_owned(), parameters)
            .unwrap();
        let selected = store.catalog().unwrap().selected_preset_id.unwrap();
        drop(store);

        let loaded = PresetStore::load(path.clone()).unwrap();
        let catalog = loaded.catalog().unwrap();
        assert_eq!(
            catalog.selected_preset_id.as_deref(),
            Some(selected.as_str())
        );
        assert_eq!(catalog.active_parameters, parameters);
        assert!(catalog
            .presets
            .iter()
            .any(|preset| !preset.built_in && preset.name == "Deeper warm"));

        cleanup(&path);
    }

    #[test]
    fn duplicate_delete_and_reset_keep_storage_consistent() {
        let path = test_path("operations");
        cleanup(&path);

        let mut store = PresetStore::load(path.clone()).unwrap();
        store
            .save_preset("Mine".to_owned(), DspParameters::default())
            .unwrap();
        let original_id = store.catalog().unwrap().selected_preset_id.unwrap();
        store.duplicate_preset(&original_id).unwrap();
        let duplicate_id = store.catalog().unwrap().selected_preset_id.unwrap();
        assert_ne!(duplicate_id, original_id);

        store.delete_preset(&duplicate_id).unwrap();
        assert_eq!(
            store.catalog().unwrap().selected_preset_id.as_deref(),
            Some(NATURAL_PRESET_ID)
        );
        store.select_preset(&original_id).unwrap();
        store.reset_to_default().unwrap();
        assert_eq!(
            store.catalog().unwrap().selected_preset_id.as_deref(),
            Some(NATURAL_PRESET_ID)
        );

        drop(store);
        let loaded = PresetStore::load(path.clone()).unwrap();
        assert_eq!(loaded.document.presets.len(), 1);
        assert_eq!(
            loaded.document.selected_preset_id.as_deref(),
            Some(NATURAL_PRESET_ID)
        );

        cleanup(&path);
    }

    #[test]
    fn rejects_corrupt_storage_without_replacing_it() {
        let path = test_path("corrupt");
        cleanup(&path);
        fs::write(&path, b"{not-json").unwrap();

        assert!(PresetStore::load(path.clone()).is_err());
        assert_eq!(fs::read(&path).unwrap(), b"{not-json");

        cleanup(&path);
    }
}
