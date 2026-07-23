use std::{
    fs::{self, File},
    io::{self, Write},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::audio::{
    device::DeviceInfo, external_route::PairingSource, reliability::ReliabilityProfile,
};

pub const APPLICATION_SETTINGS_SCHEMA_VERSION: u32 = 4;
pub const APPLICATION_SETTINGS_FILE_NAME: &str = "application-settings.json";
const MAX_DEVICE_ID_CHARS: usize = 512;
const MAX_FRIENDLY_NAME_CHARS: usize = 512;

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ApplicationPage {
    #[default]
    Use,
    Test,
    Diagnostics,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApplicationSettingsDocument {
    pub schema_version: u32,
    pub selected_input_device_id: Option<String>,
    pub last_known_input_friendly_name: Option<String>,
    pub selected_external_route_id: Option<String>,
    pub external_route_playback_device_id: Option<String>,
    pub last_known_external_route_playback_name: Option<String>,
    pub external_route_capture_device_id: Option<String>,
    pub last_known_external_route_capture_name: Option<String>,
    pub external_route_pairing_source: Option<PairingSource>,
    pub external_route_manual: bool,
    pub local_monitor_device_id: Option<String>,
    pub last_known_local_monitor_friendly_name: Option<String>,
    pub reliability_profile: ReliabilityProfile,
    pub last_page: ApplicationPage,
}

impl Default for ApplicationSettingsDocument {
    fn default() -> Self {
        Self {
            schema_version: APPLICATION_SETTINGS_SCHEMA_VERSION,
            selected_input_device_id: None,
            last_known_input_friendly_name: None,
            selected_external_route_id: None,
            external_route_playback_device_id: None,
            last_known_external_route_playback_name: None,
            external_route_capture_device_id: None,
            last_known_external_route_capture_name: None,
            external_route_pairing_source: None,
            external_route_manual: false,
            local_monitor_device_id: None,
            last_known_local_monitor_friendly_name: None,
            reliability_profile: ReliabilityProfile::Balanced,
            last_page: ApplicationPage::Use,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ApplicationSettingsDocumentV1 {
    schema_version: u32,
    selected_input_device_id: Option<String>,
    selected_output_device_id: Option<String>,
    last_known_input_friendly_name: Option<String>,
    last_known_output_friendly_name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ApplicationSettingsDocumentV2 {
    schema_version: u32,
    selected_input_device_id: Option<String>,
    last_known_input_friendly_name: Option<String>,
    processed_destination_device_id: Option<String>,
    last_known_processed_destination_friendly_name: Option<String>,
    local_monitor_device_id: Option<String>,
    last_known_local_monitor_friendly_name: Option<String>,
    local_monitor_enabled: bool,
    reliability_profile: ReliabilityProfile,
    last_page: ApplicationPage,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ApplicationSettingsDocumentV3 {
    schema_version: u32,
    selected_input_device_id: Option<String>,
    last_known_input_friendly_name: Option<String>,
    processed_destination_device_id: Option<String>,
    last_known_processed_destination_friendly_name: Option<String>,
    local_monitor_device_id: Option<String>,
    last_known_local_monitor_friendly_name: Option<String>,
    reliability_profile: ReliabilityProfile,
    last_page: ApplicationPage,
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
            Ok((document, migrated)) => {
                let mut store = Self {
                    path,
                    document,
                    startup_warning: None,
                };
                if migrated {
                    if let Err(error) = persist_document(&store.path, &store.document) {
                        store.startup_warning = Some(format!(
                            "Application settings were migrated in memory but could not be saved: {error}"
                        ));
                    }
                }
                store
            }
            Err(error) => Self {
                path,
                document: ApplicationSettingsDocument::default(),
                startup_warning: Some(format!(
                    "Stored application settings could not be restored: {error} Safe defaults are active; Test monitoring will not start automatically."
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

    pub fn save(
        &mut self,
        document: ApplicationSettingsDocument,
    ) -> Result<(), ApplicationSettingsError> {
        validate_document(&document)?;
        persist_document(&self.path, &document)?;
        self.document = document;
        self.startup_warning = None;
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedDeviceSelections {
    pub selected_input_id: Option<String>,
    pub external_route_playback_id: Option<String>,
    pub external_route_capture_id: Option<String>,
    pub local_monitor_id: Option<String>,
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
        FallbackPolicy::DefaultThenFirst,
    );
    let playback = resolve_one(
        "external-route playback endpoint",
        document.external_route_playback_device_id.as_deref(),
        document.last_known_external_route_playback_name.as_deref(),
        outputs,
        FallbackPolicy::LikelyVirtualOnly,
    );
    let capture = resolve_one(
        "external-route capture endpoint",
        document.external_route_capture_device_id.as_deref(),
        document.last_known_external_route_capture_name.as_deref(),
        inputs,
        FallbackPolicy::None,
    );
    let monitor = resolve_one(
        "local monitor",
        document.local_monitor_device_id.as_deref(),
        document.last_known_local_monitor_friendly_name.as_deref(),
        outputs,
        FallbackPolicy::DefaultThenFirst,
    );
    let restoration_warning = [
        input.warning,
        playback.warning,
        capture.warning,
        monitor.warning,
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" ");

    ResolvedDeviceSelections {
        selected_input_id: input.id,
        external_route_playback_id: playback.id,
        external_route_capture_id: capture.id,
        local_monitor_id: monitor.id,
        restoration_warning: (!restoration_warning.is_empty()).then_some(restoration_warning),
    }
}

#[derive(Clone, Copy)]
enum FallbackPolicy {
    DefaultThenFirst,
    LikelyVirtualOnly,
    None,
}

struct ResolvedDevice {
    id: Option<String>,
    warning: Option<String>,
}

fn resolve_one(
    purpose: &str,
    stored_id: Option<&str>,
    stored_name: Option<&str>,
    devices: &[DeviceInfo],
    fallback_policy: FallbackPolicy,
) -> ResolvedDevice {
    if let Some(stored_id) = stored_id {
        let matches = devices
            .iter()
            .filter(|device| device.id == stored_id)
            .collect::<Vec<_>>();
        if let [device] = matches.as_slice() {
            return ResolvedDevice {
                id: Some(device.id.clone()),
                warning: None,
            };
        }
    }

    if let Some(stored_name) = stored_name {
        let normalized_name = normalize_friendly_name(stored_name);
        let matches = devices
            .iter()
            .filter(|device| normalize_friendly_name(&device.name) == normalized_name)
            .collect::<Vec<_>>();
        if let [device] = matches.as_slice() {
            return ResolvedDevice {
                id: Some(device.id.clone()),
                warning: Some(format!(
                    "The stored {purpose} identifier was unavailable; restored '{}' by its unique friendly name.",
                    device.name
                )),
            };
        }
    }

    let fallback = fallback_device(devices, fallback_policy);
    let had_selection = stored_id.is_some() || stored_name.is_some();
    let warning = had_selection.then(|| match fallback {
        Some(device) => format!(
            "The stored {purpose} was unavailable; selected safe fallback '{}'.",
            device.name
        ),
        None => format!(
            "The stored {purpose} was unavailable and no safe automatic fallback was selected."
        ),
    });

    ResolvedDevice {
        id: fallback.map(|device| device.id.clone()),
        warning,
    }
}

fn fallback_device(devices: &[DeviceInfo], policy: FallbackPolicy) -> Option<&DeviceInfo> {
    match policy {
        FallbackPolicy::LikelyVirtualOnly => unique(devices, |device| device.is_likely_virtual),
        FallbackPolicy::DefaultThenFirst => unique(devices, |device| {
            device.is_default && !device.is_likely_virtual
        })
        .or_else(|| devices.iter().find(|device| !device.is_likely_virtual)),
        FallbackPolicy::None => None,
    }
}

fn unique(devices: &[DeviceInfo], predicate: impl Fn(&DeviceInfo) -> bool) -> Option<&DeviceInfo> {
    let mut matches = devices.iter().filter(|device| predicate(device));
    let first = matches.next()?;
    matches.next().is_none().then_some(first)
}

fn normalize_friendly_name(name: &str) -> String {
    name.trim().to_lowercase()
}

fn load_document(
    path: &Path,
) -> Result<(ApplicationSettingsDocument, bool), ApplicationSettingsError> {
    recover_interrupted_write(path)?;
    if !path.exists() {
        return Ok((ApplicationSettingsDocument::default(), false));
    }

    let contents = fs::read_to_string(path)?;
    let value: serde_json::Value = serde_json::from_str(&contents)?;
    let version = value
        .get("schemaVersion")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| {
            ApplicationSettingsError::Validation(
                "Application settings require an integer schemaVersion.".to_owned(),
            )
        })?;
    let result = match version {
        1 => {
            let legacy: ApplicationSettingsDocumentV1 = serde_json::from_value(value)?;
            (migrate_v1(legacy), true)
        }
        2 => {
            let legacy: ApplicationSettingsDocumentV2 = serde_json::from_value(value)?;
            (migrate_v2(legacy), true)
        }
        3 => {
            let legacy: ApplicationSettingsDocumentV3 = serde_json::from_value(value)?;
            (migrate_v3(legacy), true)
        }
        version if version == u64::from(APPLICATION_SETTINGS_SCHEMA_VERSION) => {
            (serde_json::from_value(value)?, false)
        }
        version => {
            return Err(ApplicationSettingsError::Validation(format!(
                "Unsupported application-settings schema version {version}. Expected version {APPLICATION_SETTINGS_SCHEMA_VERSION}."
            )))
        }
    };
    validate_document(&result.0)?;
    remove_recovery_files(path);
    Ok(result)
}

fn migrate_v1(document: ApplicationSettingsDocumentV1) -> ApplicationSettingsDocument {
    debug_assert_eq!(document.schema_version, 1);
    ApplicationSettingsDocument {
        schema_version: APPLICATION_SETTINGS_SCHEMA_VERSION,
        selected_input_device_id: document.selected_input_device_id,
        last_known_input_friendly_name: document.last_known_input_friendly_name,
        selected_external_route_id: None,
        external_route_playback_device_id: document.selected_output_device_id,
        last_known_external_route_playback_name: document.last_known_output_friendly_name,
        external_route_capture_device_id: None,
        last_known_external_route_capture_name: None,
        external_route_pairing_source: None,
        external_route_manual: false,
        local_monitor_device_id: None,
        last_known_local_monitor_friendly_name: None,
        reliability_profile: ReliabilityProfile::Balanced,
        last_page: ApplicationPage::Use,
    }
}

fn migrate_v2(document: ApplicationSettingsDocumentV2) -> ApplicationSettingsDocument {
    debug_assert_eq!(document.schema_version, 2);
    let _ = document.local_monitor_enabled;
    ApplicationSettingsDocument {
        schema_version: APPLICATION_SETTINGS_SCHEMA_VERSION,
        selected_input_device_id: document.selected_input_device_id,
        last_known_input_friendly_name: document.last_known_input_friendly_name,
        selected_external_route_id: None,
        external_route_playback_device_id: document.processed_destination_device_id,
        last_known_external_route_playback_name: document
            .last_known_processed_destination_friendly_name,
        external_route_capture_device_id: None,
        last_known_external_route_capture_name: None,
        external_route_pairing_source: None,
        external_route_manual: false,
        local_monitor_device_id: document.local_monitor_device_id,
        last_known_local_monitor_friendly_name: document.last_known_local_monitor_friendly_name,
        reliability_profile: document.reliability_profile,
        last_page: document.last_page,
    }
}

fn migrate_v3(document: ApplicationSettingsDocumentV3) -> ApplicationSettingsDocument {
    debug_assert_eq!(document.schema_version, 3);
    ApplicationSettingsDocument {
        schema_version: APPLICATION_SETTINGS_SCHEMA_VERSION,
        selected_input_device_id: document.selected_input_device_id,
        last_known_input_friendly_name: document.last_known_input_friendly_name,
        selected_external_route_id: None,
        external_route_playback_device_id: document.processed_destination_device_id,
        last_known_external_route_playback_name: document
            .last_known_processed_destination_friendly_name,
        external_route_capture_device_id: None,
        last_known_external_route_capture_name: None,
        external_route_pairing_source: None,
        external_route_manual: false,
        local_monitor_device_id: document.local_monitor_device_id,
        last_known_local_monitor_friendly_name: document.last_known_local_monitor_friendly_name,
        reliability_profile: document.reliability_profile,
        last_page: document.last_page,
    }
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
        "external-route playback endpoint",
        document.external_route_playback_device_id.as_deref(),
        document.last_known_external_route_playback_name.as_deref(),
    )?;
    validate_device_pair(
        "external-route capture endpoint",
        document.external_route_capture_device_id.as_deref(),
        document.last_known_external_route_capture_name.as_deref(),
    )?;
    validate_device_pair(
        "local monitor",
        document.local_monitor_device_id.as_deref(),
        document.last_known_local_monitor_friendly_name.as_deref(),
    )?;
    if document.selected_external_route_id.is_some()
        && (document.external_route_playback_device_id.is_none()
            || document.external_route_capture_device_id.is_none())
    {
        return Err(ApplicationSettingsError::Validation(
            "A selected external route requires both playback and capture endpoints.".to_owned(),
        ));
    }
    if document.external_route_manual
        && (document.external_route_pairing_source != Some(PairingSource::Manual)
            || document.selected_external_route_id.is_none())
    {
        return Err(ApplicationSettingsError::Validation(
            "A manual external route requires a selected route ID and manual pairing source."
                .to_owned(),
        ));
    }
    Ok(())
}

fn validate_device_pair(
    purpose: &str,
    id: Option<&str>,
    friendly_name: Option<&str>,
) -> Result<(), ApplicationSettingsError> {
    if id.is_some() != friendly_name.is_some() {
        return Err(ApplicationSettingsError::Validation(format!(
            "The stored {purpose} identifier and friendly name must both be present or both be absent."
        )));
    }
    if let Some(id) = id {
        validate_string(
            &format!("Stored {purpose} identifier"),
            id,
            MAX_DEVICE_ID_CHARS,
        )?;
    }
    if let Some(name) = friendly_name {
        validate_string(
            &format!("Stored {purpose} friendly name"),
            name,
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
    use std::{fs, path::PathBuf};

    use super::*;

    fn input(id: &str, name: &str, default: bool, virtual_endpoint: bool) -> DeviceInfo {
        DeviceInfo::test(
            id,
            name,
            crate::audio::device::DeviceDirection::Input,
            default,
            virtual_endpoint,
        )
    }

    fn output(id: &str, name: &str, default: bool, virtual_endpoint: bool) -> DeviceInfo {
        DeviceInfo::test(
            id,
            name,
            crate::audio::device::DeviceDirection::Output,
            default,
            virtual_endpoint,
        )
    }

    fn path(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "mam-voice-settings-{label}-{}.json",
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

    #[test]
    fn first_launch_is_balanced_on_use() {
        let defaults = ApplicationSettingsDocument::default();
        assert_eq!(defaults.last_page, ApplicationPage::Use);
        assert_eq!(defaults.reliability_profile, ReliabilityProfile::Balanced);
    }

    #[test]
    fn physical_speakers_are_not_an_automatic_processed_destination() {
        let resolved = resolve_device_selections(
            &ApplicationSettingsDocument::default(),
            &[input("mic", "Realtek microphone", true, false)],
            &[output("speakers", "Realtek speakers", true, false)],
        );
        assert_eq!(resolved.selected_input_id.as_deref(), Some("mic"));
        assert_eq!(resolved.external_route_playback_id, None);
        assert_eq!(resolved.local_monitor_id.as_deref(), Some("speakers"));
    }

    #[test]
    fn unique_virtual_output_is_the_only_automatic_destination() {
        let resolved = resolve_device_selections(
            &ApplicationSettingsDocument::default(),
            &[input("mic", "Microphone", true, false)],
            &[
                output("speakers", "Speakers", true, false),
                output("virtual", "Virtual Audio Router", false, true),
            ],
        );
        assert_eq!(
            resolved.external_route_playback_id.as_deref(),
            Some("virtual")
        );
    }

    #[test]
    fn route_endpoints_restore_only_by_unique_identity_or_friendly_name() {
        let document = ApplicationSettingsDocument {
            external_route_playback_device_id: Some("old-playback".to_owned()),
            last_known_external_route_playback_name: Some("Studio Playback".to_owned()),
            external_route_capture_device_id: Some("old-capture".to_owned()),
            last_known_external_route_capture_name: Some("Studio Capture".to_owned()),
            ..ApplicationSettingsDocument::default()
        };
        let resolved = resolve_device_selections(
            &document,
            &[
                input("new-capture", "Studio Capture", false, true),
                input("mic", "Physical microphone", true, false),
            ],
            &[output("new-playback", "Studio Playback", false, true)],
        );
        assert_eq!(
            resolved.external_route_playback_id.as_deref(),
            Some("new-playback")
        );
        assert_eq!(
            resolved.external_route_capture_id.as_deref(),
            Some("new-capture")
        );
        assert!(resolved.restoration_warning.is_some());
    }

    #[test]
    fn missing_or_ambiguous_capture_is_never_silently_restored() {
        let document = ApplicationSettingsDocument {
            external_route_capture_device_id: Some("old-capture".to_owned()),
            last_known_external_route_capture_name: Some("Duplicate Capture".to_owned()),
            ..ApplicationSettingsDocument::default()
        };
        let resolved = resolve_device_selections(
            &document,
            &[
                input("first", "Duplicate Capture", false, true),
                input("second", "Duplicate Capture", false, true),
                input("mic", "Physical microphone", true, false),
            ],
            &[],
        );
        assert_eq!(resolved.external_route_capture_id, None);
        assert!(resolved
            .restoration_warning
            .as_deref()
            .is_some_and(|warning| warning.contains("no safe automatic fallback")));
    }

    #[test]
    fn v1_migration_preserves_destination() {
        let path = path("v1");
        cleanup(&path);
        fs::write(
            &path,
            r#"{"schemaVersion":1,"selectedInputDeviceId":"mic","selectedOutputDeviceId":"out","lastKnownInputFriendlyName":"Mic","lastKnownOutputFriendlyName":"Output"}"#,
        )
        .unwrap();
        let store = ApplicationSettingsStore::load(path.clone());
        assert_eq!(
            store
                .document()
                .external_route_playback_device_id
                .as_deref(),
            Some("out")
        );
        assert_eq!(
            store.document().reliability_profile,
            ReliabilityProfile::Balanced
        );
        let persisted: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(persisted["schemaVersion"], 4);
        assert!(persisted.get("localMonitorEnabled").is_none());
        cleanup(&path);
    }

    #[test]
    fn v2_migration_discards_persisted_monitor_enablement_but_keeps_device() {
        let path = path("v2");
        cleanup(&path);
        fs::write(
            &path,
            r#"{"schemaVersion":2,"selectedInputDeviceId":"mic","lastKnownInputFriendlyName":"Mic","processedDestinationDeviceId":"destination","lastKnownProcessedDestinationFriendlyName":"Destination","localMonitorDeviceId":"headphones","lastKnownLocalMonitorFriendlyName":"Headphones","localMonitorEnabled":true,"reliabilityProfile":"reliable","lastPage":"test"}"#,
        )
        .unwrap();
        let store = ApplicationSettingsStore::load(path.clone());
        assert_eq!(
            store.document().local_monitor_device_id.as_deref(),
            Some("headphones")
        );
        assert_eq!(store.document().last_page, ApplicationPage::Test);
        let persisted: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(persisted["schemaVersion"], 4);
        assert!(persisted.get("localMonitorEnabled").is_none());
        cleanup(&path);
    }

    #[test]
    fn v3_migration_preserves_playback_candidate_without_inventing_capture() {
        let path = path("v3");
        cleanup(&path);
        fs::write(
            &path,
            r#"{"schemaVersion":3,"selectedInputDeviceId":"mic","lastKnownInputFriendlyName":"Mic","processedDestinationDeviceId":"playback","lastKnownProcessedDestinationFriendlyName":"CABLE Input","localMonitorDeviceId":"headphones","lastKnownLocalMonitorFriendlyName":"Headphones","reliabilityProfile":"balanced","lastPage":"use"}"#,
        )
        .unwrap();
        let store = ApplicationSettingsStore::load(path.clone());
        assert_eq!(
            store
                .document()
                .external_route_playback_device_id
                .as_deref(),
            Some("playback")
        );
        assert!(store.document().external_route_capture_device_id.is_none());
        assert!(store.document().selected_external_route_id.is_none());
        assert!(!store.document().external_route_manual);
        cleanup(&path);
    }

    #[test]
    fn malformed_or_future_settings_preserve_file_and_use_safe_defaults() {
        for (label, contents) in [
            ("malformed", "{not-json"),
            (
                "future",
                r#"{"schemaVersion":99,"selectedInputDeviceId":null}"#,
            ),
        ] {
            let path = path(label);
            cleanup(&path);
            fs::write(&path, contents).unwrap();
            let store = ApplicationSettingsStore::load(path.clone());
            assert!(store.startup_warning().is_some());
            assert_eq!(fs::read_to_string(&path).unwrap(), contents);
            cleanup(&path);
        }
    }

    #[test]
    fn current_settings_round_trip_all_separate_purposes() {
        let path = path("round-trip");
        cleanup(&path);
        let mut store = ApplicationSettingsStore::load(path.clone());
        let document = ApplicationSettingsDocument {
            schema_version: APPLICATION_SETTINGS_SCHEMA_VERSION,
            selected_input_device_id: Some("mic".to_owned()),
            last_known_input_friendly_name: Some("Mic".to_owned()),
            selected_external_route_id: Some("route".to_owned()),
            external_route_playback_device_id: Some("destination".to_owned()),
            last_known_external_route_playback_name: Some("Destination".to_owned()),
            external_route_capture_device_id: Some("capture".to_owned()),
            last_known_external_route_capture_name: Some("Capture".to_owned()),
            external_route_pairing_source: Some(PairingSource::Manual),
            external_route_manual: true,
            local_monitor_device_id: Some("headphones".to_owned()),
            last_known_local_monitor_friendly_name: Some("Headphones".to_owned()),
            reliability_profile: ReliabilityProfile::Reliable,
            last_page: ApplicationPage::Diagnostics,
        };
        store.save(document.clone()).unwrap();
        assert_eq!(
            ApplicationSettingsStore::load(path.clone()).document(),
            &document
        );
        cleanup(&path);
    }
}
