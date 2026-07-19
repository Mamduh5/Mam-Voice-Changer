use serde::Serialize;

use crate::{
    audio::device::{self, DeviceInfo, DeviceList},
    config::application_settings::resolve_device_selections,
    state::app_state::AppState,
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioDeviceCatalog {
    pub inputs: Vec<DeviceInfo>,
    pub outputs: Vec<DeviceInfo>,
    pub selected_input_id: Option<String>,
    pub selected_output_id: Option<String>,
    pub restoration_warning: Option<String>,
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

    Ok(AudioDeviceCatalog {
        inputs,
        outputs,
        selected_input_id: resolved.selected_input_id,
        selected_output_id: resolved.selected_output_id,
        restoration_warning,
    })
}

#[tauri::command]
pub fn save_audio_device_selection(
    input_id: String,
    output_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let DeviceList { inputs, outputs } =
        device::list_devices().map_err(|error| error.to_string())?;
    let input = uniquely_selected_device("input", &input_id, &inputs)?;
    let output = uniquely_selected_device("output", &output_id, &outputs)?;

    state
        .application_settings()
        .lock()
        .map_err(|_| "Application settings storage is unavailable.".to_owned())?
        .save_selection(
            input.id.clone(),
            input.name.clone(),
            output.id.clone(),
            output.name.clone(),
        )
        .map_err(|error| error.to_string())
}

fn uniquely_selected_device<'a>(
    direction: &str,
    id: &str,
    devices: &'a [DeviceInfo],
) -> Result<&'a DeviceInfo, String> {
    let mut matches = devices.iter().filter(|device| device.id == id);
    let Some(device) = matches.next() else {
        return Err(format!(
            "The selected {direction} device is no longer available. Refresh devices and select another device."
        ));
    };
    if matches.next().is_some() {
        return Err(format!(
            "The selected {direction} device identifier is ambiguous. Refresh devices and choose an unambiguous endpoint."
        ));
    }
    Ok(device)
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
    use super::uniquely_selected_device;
    use crate::audio::device::DeviceInfo;

    fn device(id: &str, name: &str) -> DeviceInfo {
        DeviceInfo {
            id: id.to_owned(),
            name: name.to_owned(),
            is_default: false,
        }
    }

    #[test]
    fn saving_requires_one_current_device_for_each_identifier() {
        let devices = [device("one", "One"), device("two", "Two")];
        assert_eq!(
            uniquely_selected_device("input", "two", &devices)
                .unwrap()
                .name,
            "Two"
        );
        assert!(uniquely_selected_device("input", "missing", &devices).is_err());
    }

    #[test]
    fn saving_rejects_duplicate_identifiers() {
        let devices = [device("duplicate", "One"), device("duplicate", "Two")];
        assert!(uniquely_selected_device("output", "duplicate", &devices)
            .unwrap_err()
            .contains("ambiguous"));
    }
}
