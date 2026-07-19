use std::{
    fs::{self, File},
    io::{self, Write},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::audio::device::DeviceInfo;

pub const APPLICATION_SETTINGS_SCHEMA_VERSION: u32 = 1;
pub const APPLICATION_SETTINGS_FILE_NAME: &str = "application-settings.json";
const MAX_DEVICE_ID_CHARS: usize = 512;
const MAX_FRIENDLY_NAME_CHARS: usize = 512;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApplicationSettingsDocument {
    pub schema_version: u32,
    pub selected_input_device_id: Option<String>,
    pub selected_output_device_id: Option<String>,
    pub last_known_input_friendly_name: Option<String>,
    pub last_known_output_friendly_name: Option<String>,
}

impl Default for ApplicationSettingsDocument {
    fn default() -> Self {
        Self {
            schema_version: APPLICATION_SETTINGS_SCHEMA_VERSION,
            selected_input_device_id: None,
            selected_output_device_id: None,
            last_known_input_friendly_name: None,
            last_known_output_friendly_name: None,
        }
    }
}

#[derive(Debug, Error)]
pub enum ApplicationSettingsError {
    #[error("Application settings storage failed: {0}")]
    Storage(#[from] io::Error),
    #[error("Application settings are not valid JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("{0}")]
    Validation(String),
}

pub struct ApplicationSettingsStore {
    path: PathBuf,
    document: ApplicationSettingsDocument,
    startup_warning: Option<String>,
}

impl ApplicationSettingsStore {
    pub fn load(path: PathBuf) -> Self {
        match load_document(&path) {
            Ok(document) => Self {
                path,
                document,
                startup_warning: None,
            },
            Err(error) => Self {
                path,
                document: ApplicationSettingsDocument::default(),
                startup_warning: Some(format!(
                    "Stored audio-device settings could not be restored: {error} Default devices will be used until the selection is saved again."
                )),
            },
        }
    }

    pub fn document(&self) -> &ApplicationSettingsDocument {
        &self.document
    }

    pub fn startup_warning(&self) -> Option<&str> {
        self.startup_warning.as_deref()
    }

    pub fn save_selection(
        &mut self,
        input_id: String,
        input_name: String,
        output_id: String,
        output_name: String,
    ) -> Result<(), ApplicationSettingsError> {
        let next = ApplicationSettingsDocument {
            schema_version: APPLICATION_SETTINGS_SCHEMA_VERSION,
            selected_input_device_id: Some(input_id),
            selected_output_device_id: Some(output_id),
            last_known_input_friendly_name: Some(input_name),
            last_known_output_friendly_name: Some(output_name),
        };
        validate_document(&next)?;
        persist_document(&self.path, &next)?;
        self.document = next;
        self.startup_warning = None;
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedDeviceSelections {
    pub selected_input_id: Option<String>,
    pub selected_output_id: Option<String>,
    pub restoration_warning: Option<String>,
}

pub fn resolve_device_selections(
    document: &ApplicationSettingsDocument,
    inputs: &[DeviceInfo],
    outputs: &[DeviceInfo],
) -> ResolvedDeviceSelections {
    let input = resolve_one(
        "input",
        document.selected_input_device_id.as_deref(),
        document.last_known_input_friendly_name.as_deref(),
        inputs,
        false,
    );
    let output = resolve_one(
        "output",
        document.selected_output_device_id.as_deref(),
        document.last_known_output_friendly_name.as_deref(),
        outputs,
        true,
    );
    let restoration_warning = [input.warning, output.warning]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" ");

    ResolvedDeviceSelections {
        selected_input_id: input.id,
        selected_output_id: output.id,
        restoration_warning: (!restoration_warning.is_empty()).then_some(restoration_warning),
    }
}

struct ResolvedDevice {
    id: Option<String>,
    warning: Option<String>,
}

fn resolve_one(
    direction: &str,
    stored_id: Option<&str>,
    stored_name: Option<&str>,
    devices: &[DeviceInfo],
    prefer_cable_input: bool,
) -> ResolvedDevice {
    if let Some(stored_id) = stored_id {
        let id_matches = devices
            .iter()
            .filter(|device| device.id == stored_id)
            .collect::<Vec<_>>();
        if let [device] = id_matches.as_slice() {
            return ResolvedDevice {
                id: Some(device.id.clone()),
                warning: None,
            };
        }
    }

    if let Some(stored_name) = stored_name {
        let normalized_name = normalize_friendly_name(stored_name);
        let name_matches = devices
            .iter()
            .filter(|device| normalize_friendly_name(&device.name) == normalized_name)
            .collect::<Vec<_>>();
        if let [device] = name_matches.as_slice() {
            return ResolvedDevice {
                id: Some(device.id.clone()),
                warning: Some(format!(
                    "The stored {direction} device identifier was unavailable or ambiguous; restored '{}' by its unique friendly name.",
                    device.name
                )),
            };
        }
    }

    let fallback = fallback_device(devices, prefer_cable_input);
    let had_stored_selection = stored_id.is_some() || stored_name.is_some();
    let warning = had_stored_selection.then(|| match fallback {
        Some(device) => format!(
            "The stored {direction} device was unavailable or ambiguous; selected fallback device '{}'.",
            device.name
        ),
        None => format!(
            "The stored {direction} device was unavailable and no {direction} devices are currently available."
        ),
    });

    ResolvedDevice {
        id: fallback.map(|device| device.id.clone()),
        warning,
    }
}

fn fallback_device(devices: &[DeviceInfo], prefer_cable_input: bool) -> Option<&DeviceInfo> {
    if prefer_cable_input {
        let cable_matches = devices
            .iter()
            .filter(|device| normalize_friendly_name(&device.name).contains("cable input"))
            .take(2)
            .collect::<Vec<_>>();
        if let [cable] = cable_matches.as_slice() {
            return Some(cable);
        }
    }
    let default_matches = devices
        .iter()
        .filter(|device| device.is_default)
        .take(2)
        .collect::<Vec<_>>();
    if let [default] = default_matches.as_slice() {
        return Some(default);
    }
    devices.first()
}

fn normalize_friendly_name(name: &str) -> String {
    name.trim().to_lowercase()
}

fn load_document(path: &Path) -> Result<ApplicationSettingsDocument, ApplicationSettingsError> {
    recover_interrupted_write(path)?;
    if !path.exists() {
        return Ok(ApplicationSettingsDocument::default());
    }

    let contents = fs::read_to_string(path)?;
    let document: ApplicationSettingsDocument = serde_json::from_str(&contents)?;
    validate_document(&document)?;
    remove_recovery_files(path);
    Ok(document)
}

fn validate_document(
    document: &ApplicationSettingsDocument,
) -> Result<(), ApplicationSettingsError> {
    if document.schema_version != APPLICATION_SETTINGS_SCHEMA_VERSION {
        return Err(ApplicationSettingsError::Validation(format!(
            "Unsupported application-settings schema version {}. Expected version {}.",
            document.schema_version, APPLICATION_SETTINGS_SCHEMA_VERSION
        )));
    }

    validate_device_pair(
        "input",
        document.selected_input_device_id.as_deref(),
        document.last_known_input_friendly_name.as_deref(),
    )?;
    validate_device_pair(
        "output",
        document.selected_output_device_id.as_deref(),
        document.last_known_output_friendly_name.as_deref(),
    )?;
    Ok(())
}

fn validate_device_pair(
    direction: &str,
    id: Option<&str>,
    friendly_name: Option<&str>,
) -> Result<(), ApplicationSettingsError> {
    if id.is_some() != friendly_name.is_some() {
        return Err(ApplicationSettingsError::Validation(format!(
            "The stored {direction} device identifier and friendly name must either both be present or both be absent."
        )));
    }
    if let Some(id) = id {
        validate_string(
            &format!("Stored {direction} device identifier"),
            id,
            MAX_DEVICE_ID_CHARS,
        )?;
    }
    if let Some(friendly_name) = friendly_name {
        validate_string(
            &format!("Stored {direction} friendly name"),
            friendly_name,
            MAX_FRIENDLY_NAME_CHARS,
        )?;
    }
    Ok(())
}

fn validate_string(
    label: &str,
    value: &str,
    maximum_chars: usize,
) -> Result<(), ApplicationSettingsError> {
    let length = value.chars().count();
    if value.trim().is_empty() || length > maximum_chars || value.chars().any(char::is_control) {
        return Err(ApplicationSettingsError::Validation(format!(
            "{label} must contain 1 to {maximum_chars} non-control characters."
        )));
    }
    Ok(())
}

fn persist_document(
    path: &Path,
    document: &ApplicationSettingsDocument,
) -> Result<(), ApplicationSettingsError> {
    validate_document(document)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let bytes = serde_json::to_vec_pretty(document)?;
    let temporary = recovery_path(path, ".tmp");
    let backup = recovery_path(path, ".bak");
    if temporary.exists() {
        fs::remove_file(&temporary)?;
    }

    let mut file = File::create(&temporary)?;
    file.write_all(&bytes)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    drop(file);

    if path.exists() {
        if backup.exists() {
            fs::remove_file(&backup)?;
        }
        fs::rename(path, &backup)?;
        if let Err(error) = fs::rename(&temporary, path) {
            let _ = fs::rename(&backup, path);
            let _ = fs::remove_file(&temporary);
            return Err(ApplicationSettingsError::Storage(error));
        }
        let _ = fs::remove_file(backup);
    } else {
        fs::rename(temporary, path)?;
    }
    Ok(())
}

fn recovery_path(path: &Path, suffix: &str) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(APPLICATION_SETTINGS_FILE_NAME);
    path.with_file_name(format!("{file_name}{suffix}"))
}

fn recover_interrupted_write(path: &Path) -> Result<(), ApplicationSettingsError> {
    if path.exists() {
        return Ok(());
    }

    let backup = recovery_path(path, ".bak");
    let temporary = recovery_path(path, ".tmp");
    if backup.exists() {
        fs::rename(&backup, path)?;
        let _ = fs::remove_file(temporary);
    } else if temporary.exists() {
        fs::rename(temporary, path)?;
    }
    Ok(())
}

fn remove_recovery_files(path: &Path) {
    for suffix in [".tmp", ".bak"] {
        let recovery = recovery_path(path, suffix);
        if recovery.exists() {
            let _ = fs::remove_file(recovery);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        sync::atomic::{AtomicU64, Ordering},
    };

    use super::{
        recovery_path, resolve_device_selections, ApplicationSettingsDocument,
        ApplicationSettingsStore, APPLICATION_SETTINGS_SCHEMA_VERSION,
    };
    use crate::audio::device::DeviceInfo;

    static TEST_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    fn test_path(label: &str) -> PathBuf {
        let sequence = TEST_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "mam-voice-changer-settings-{label}-{}-{sequence}.json",
            std::process::id()
        ))
    }

    fn cleanup(path: &Path) {
        for suffix in ["", ".tmp", ".bak"] {
            let target = if suffix.is_empty() {
                path.to_path_buf()
            } else {
                recovery_path(path, suffix)
            };
            let _ = fs::remove_file(target);
        }
    }

    fn device(id: &str, name: &str, is_default: bool) -> DeviceInfo {
        DeviceInfo {
            id: id.to_owned(),
            name: name.to_owned(),
            is_default,
        }
    }

    fn saved_document() -> ApplicationSettingsDocument {
        ApplicationSettingsDocument {
            schema_version: APPLICATION_SETTINGS_SCHEMA_VERSION,
            selected_input_device_id: Some("saved-input".to_owned()),
            selected_output_device_id: Some("saved-output".to_owned()),
            last_known_input_friendly_name: Some("Studio Microphone".to_owned()),
            last_known_output_friendly_name: Some("Studio Monitor".to_owned()),
        }
    }

    #[test]
    fn first_launch_without_a_settings_file_uses_defaults_without_a_warning() {
        let path = test_path("first-launch");
        cleanup(&path);

        let store = ApplicationSettingsStore::load(path.clone());

        assert_eq!(store.document(), &ApplicationSettingsDocument::default());
        assert_eq!(store.startup_warning(), None);
        assert!(!path.exists());
        cleanup(&path);
    }

    #[test]
    fn saves_and_restores_unique_device_identifiers() {
        let path = test_path("round-trip");
        cleanup(&path);
        let mut store = ApplicationSettingsStore::load(path.clone());
        store
            .save_selection(
                "saved-input".to_owned(),
                "Studio Microphone".to_owned(),
                "saved-output".to_owned(),
                "Studio Monitor".to_owned(),
            )
            .unwrap();

        let loaded = ApplicationSettingsStore::load(path.clone());
        let resolved = resolve_device_selections(
            loaded.document(),
            &[device("saved-input", "Studio Microphone", false)],
            &[device("saved-output", "Studio Monitor", false)],
        );

        assert_eq!(resolved.selected_input_id.as_deref(), Some("saved-input"));
        assert_eq!(resolved.selected_output_id.as_deref(), Some("saved-output"));
        assert_eq!(resolved.restoration_warning, None);
        cleanup(&path);
    }

    #[test]
    fn missing_saved_devices_use_existing_fallback_policy() {
        let resolved = resolve_device_selections(
            &saved_document(),
            &[
                device("first-input", "Other microphone", false),
                device("default-input", "Default microphone", true),
            ],
            &[
                device("default-output", "Default speakers", true),
                device("cable-output", "CABLE Input (VB-Audio)", false),
            ],
        );

        assert_eq!(resolved.selected_input_id.as_deref(), Some("default-input"));
        assert_eq!(resolved.selected_output_id.as_deref(), Some("cable-output"));
        assert!(resolved
            .restoration_warning
            .as_deref()
            .is_some_and(|warning| warning.contains("fallback")));
    }

    #[test]
    fn unique_normalized_name_restores_but_duplicate_names_do_not() {
        let document = saved_document();
        let unique = resolve_device_selections(
            &document,
            &[device("renamed-id", "  STUDIO MICROPHONE ", false)],
            &[device("renamed-output", "Studio Monitor", false)],
        );
        assert_eq!(unique.selected_input_id.as_deref(), Some("renamed-id"));
        assert!(unique
            .restoration_warning
            .as_deref()
            .is_some_and(|warning| warning.contains("unique friendly name")));

        let duplicated = resolve_device_selections(
            &document,
            &[
                device("duplicate-a", "Studio Microphone", false),
                device("duplicate-b", " studio microphone ", false),
                device("fallback", "Fallback microphone", true),
            ],
            &[device("saved-output", "Studio Monitor", false)],
        );
        assert_eq!(duplicated.selected_input_id.as_deref(), Some("fallback"));
        assert!(duplicated
            .restoration_warning
            .as_deref()
            .is_some_and(|warning| warning.contains("ambiguous")));
    }

    #[test]
    fn duplicate_identifiers_are_not_claimed_as_an_exact_restoration() {
        let document = saved_document();
        let resolved = resolve_device_selections(
            &document,
            &[
                device("saved-input", "Studio Microphone", false),
                device("saved-input", "Studio Microphone", false),
                device("fallback", "Fallback microphone", true),
            ],
            &[device("saved-output", "Studio Monitor", false)],
        );

        assert_eq!(resolved.selected_input_id.as_deref(), Some("fallback"));
        assert!(resolved.restoration_warning.is_some());
    }

    #[test]
    fn duplicate_cable_outputs_do_not_win_over_a_unique_windows_default() {
        let document = saved_document();
        let resolved = resolve_device_selections(
            &document,
            &[device("saved-input", "Studio Microphone", false)],
            &[
                device("cable-a", "CABLE Input", false),
                device("cable-b", "CABLE Input (duplicate)", false),
                device("default-output", "Default speakers", true),
            ],
        );

        assert_eq!(
            resolved.selected_output_id.as_deref(),
            Some("default-output")
        );
        assert!(resolved.restoration_warning.is_some());
    }

    #[test]
    fn corrupt_settings_are_preserved_and_reported_as_recoverable() {
        let path = test_path("corrupt");
        cleanup(&path);
        fs::write(&path, b"{not-json").unwrap();

        let store = ApplicationSettingsStore::load(path.clone());

        assert_eq!(store.document(), &ApplicationSettingsDocument::default());
        assert!(store
            .startup_warning()
            .is_some_and(|warning| warning.contains("not valid JSON")));
        assert_eq!(fs::read(&path).unwrap(), b"{not-json");
        cleanup(&path);
    }

    #[test]
    fn unsupported_schema_is_preserved_and_reported_as_recoverable() {
        let path = test_path("future-schema");
        cleanup(&path);
        let future = serde_json::json!({
            "schemaVersion": APPLICATION_SETTINGS_SCHEMA_VERSION + 1,
            "selectedInputDeviceId": null,
            "selectedOutputDeviceId": null,
            "lastKnownInputFriendlyName": null,
            "lastKnownOutputFriendlyName": null
        });
        fs::write(&path, serde_json::to_vec_pretty(&future).unwrap()).unwrap();

        let store = ApplicationSettingsStore::load(path.clone());

        assert_eq!(store.document(), &ApplicationSettingsDocument::default());
        assert!(store
            .startup_warning()
            .is_some_and(|warning| warning.contains("Unsupported")));
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&fs::read(&path).unwrap()).unwrap(),
            future
        );
        cleanup(&path);
    }

    #[test]
    fn replacement_is_atomic_and_interrupted_backup_is_recovered() {
        let path = test_path("atomic");
        cleanup(&path);
        let mut store = ApplicationSettingsStore::load(path.clone());
        store
            .save_selection(
                "input-one".to_owned(),
                "Input One".to_owned(),
                "output-one".to_owned(),
                "Output One".to_owned(),
            )
            .unwrap();
        store
            .save_selection(
                "input-two".to_owned(),
                "Input Two".to_owned(),
                "output-two".to_owned(),
                "Output Two".to_owned(),
            )
            .unwrap();

        let decoded: ApplicationSettingsDocument =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(
            decoded.selected_input_device_id.as_deref(),
            Some("input-two")
        );
        assert!(!recovery_path(&path, ".tmp").exists());
        assert!(!recovery_path(&path, ".bak").exists());

        fs::rename(&path, recovery_path(&path, ".bak")).unwrap();
        let recovered = ApplicationSettingsStore::load(path.clone());
        assert_eq!(
            recovered.document().selected_output_device_id.as_deref(),
            Some("output-two")
        );
        assert!(path.exists());
        assert!(!recovery_path(&path, ".bak").exists());
        cleanup(&path);
    }
}
