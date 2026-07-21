use serde::{Deserialize, Serialize};

use crate::{
    audio::{
        device::{self, DeviceInfo, DeviceList},
        reliability::ReliabilityProfile,
    },
    config::application_settings::{
        resolve_device_selections, ApplicationPage, ApplicationSettingsDocument,
        APPLICATION_SETTINGS_SCHEMA_VERSION,
    },
    state::app_state::AppState,
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioDeviceCatalog {
    pub inputs: Vec<DeviceInfo>,
    pub outputs: Vec<DeviceInfo>,
    pub selected_input_id: Option<String>,
    pub processed_destination_id: Option<String>,
    pub local_monitor_id: Option<String>,
    pub reliability_profile: ReliabilityProfile,
    pub last_page: ApplicationPage,
    pub has_likely_virtual_destination: bool,
    pub restoration_warning: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SaveApplicationSettingsRequest {
    pub selected_input_id: Option<String>,
    pub processed_destination_id: Option<String>,
    pub local_monitor_id: Option<String>,
    pub reliability_profile: ReliabilityProfile,
    pub last_page: ApplicationPage,
}

#[tauri::command]
pub fn list_audio_devices(state: tauri::State<'_, AppState>) -> Result<AudioDeviceCatalog, String> {
    let DeviceList { inputs, outputs } =
        device::list_devices().map_err(|error| error.to_string())?;
    let (document, startup_warning) = {
        let store = state
            .application_settings()
            .lock()
            .map_err(|_| "Application settings storage is unavailable.".to_owned())?;
        (
            store.document().clone(),
            store.startup_warning().map(str::to_owned),
        )
    };
    let resolved = resolve_device_selections(&document, &inputs, &outputs);
    let restoration_warning = join_warnings(startup_warning, resolved.restoration_warning);
    let has_likely_virtual_destination = outputs.iter().any(|device| device.is_likely_virtual);

    Ok(AudioDeviceCatalog {
        inputs,
        outputs,
        selected_input_id: resolved.selected_input_id,
        processed_destination_id: resolved.processed_destination_id,
        local_monitor_id: resolved.local_monitor_id,
        reliability_profile: document.reliability_profile,
        last_page: document.last_page,
        has_likely_virtual_destination,
        restoration_warning,
    })
}

#[tauri::command]
pub fn save_application_settings(
    request: SaveApplicationSettingsRequest,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let DeviceList { inputs, outputs } =
        device::list_devices().map_err(|error| error.to_string())?;
    let input = selected_pair("input", request.selected_input_id.as_deref(), &inputs)?;
    let destination = selected_pair(
        "processed destination",
        request.processed_destination_id.as_deref(),
        &outputs,
    )?;
    let monitor = selected_pair(
        "local monitor",
        request.local_monitor_id.as_deref(),
        &outputs,
    )?;
    let document = ApplicationSettingsDocument {
        schema_version: APPLICATION_SETTINGS_SCHEMA_VERSION,
        selected_input_device_id: input.as_ref().map(|device| device.id.clone()),
        last_known_input_friendly_name: input.as_ref().map(|device| device.name.clone()),
        processed_destination_device_id: destination.as_ref().map(|device| device.id.clone()),
        last_known_processed_destination_friendly_name: destination
            .as_ref()
            .map(|device| device.name.clone()),
        local_monitor_device_id: monitor.as_ref().map(|device| device.id.clone()),
        last_known_local_monitor_friendly_name: monitor.as_ref().map(|device| device.name.clone()),
        reliability_profile: request.reliability_profile,
        last_page: request.last_page,
    };
    state
        .application_settings()
        .lock()
        .map_err(|_| "Application settings storage is unavailable.".to_owned())?
        .save(document)
        .map_err(|error| error.to_string())
}

fn selected_pair<'a>(
    purpose: &str,
    id: Option<&str>,
    devices: &'a [DeviceInfo],
) -> Result<Option<&'a DeviceInfo>, String> {
    let Some(id) = id else {
        return Ok(None);
    };
    let mut matches = devices.iter().filter(|device| device.id == id);
    let Some(device) = matches.next() else {
        return Err(format!(
            "The selected {purpose} is no longer available. Refresh devices and choose again."
        ));
    };
    if matches.next().is_some() {
        return Err(format!(
            "The selected {purpose} identifier is ambiguous. Refresh devices and choose an unambiguous endpoint."
        ));
    }
    Ok(Some(device))
}

fn join_warnings(first: Option<String>, second: Option<String>) -> Option<String> {
    let combined = [first, second]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" ");
    (!combined.is_empty()).then_some(combined)
}

#[cfg(test)]
mod tests {
    use super::selected_pair;
    use crate::audio::device::DeviceInfo;

    fn device(id: &str, name: &str) -> DeviceInfo {
        DeviceInfo {
            id: id.to_owned(),
            name: name.to_owned(),
            is_default: false,
            is_likely_virtual: false,
        }
    }

    #[test]
    fn optional_purposes_allow_none_but_reject_missing_or_duplicate_ids() {
        let devices = [device("one", "One"), device("two", "Two")];
        assert!(selected_pair("monitor", None, &devices).unwrap().is_none());
        assert_eq!(
            selected_pair("monitor", Some("two"), &devices)
                .unwrap()
                .unwrap()
                .name,
            "Two"
        );
        assert!(selected_pair("monitor", Some("missing"), &devices).is_err());

        let duplicate = [device("same", "One"), device("same", "Two")];
        assert!(selected_pair("monitor", Some("same"), &duplicate).is_err());
    }
}
