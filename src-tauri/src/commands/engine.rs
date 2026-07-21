use serde::Deserialize;

use crate::{
    audio::{
        controller::StartAudioRequest,
        device::{self, DeviceInfo},
        metrics::EngineStatus,
        reliability::ReliabilityProfile,
    },
    commands::external_routes::selected_saved_route,
    state::app_state::AppState,
};

#[derive(Clone, Debug, Deserialize)]
#[serde(
    tag = "mode",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum StartEngineRequest {
    Use {
        input_id: String,
        input_name: String,
        external_route_id: String,
        reliability_profile: ReliabilityProfile,
    },
    Test {
        input_id: String,
        input_name: String,
        monitor_id: String,
        monitor_name: String,
        reliability_profile: ReliabilityProfile,
    },
}

#[tauri::command]
pub fn start_engine(
    request: StartEngineRequest,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let _audio_operation = state
        .audio_operation()
        .lock()
        .map_err(|_| "Audio operations are temporarily unavailable.".to_owned())?;
    if state.voice_lab().is_audio_active() {
        return Err("Stop Voice Lab recording or preview before starting Use or Test.".to_owned());
    }
    let request = match request {
        StartEngineRequest::Use {
            input_id,
            input_name,
            external_route_id,
            reliability_profile,
        } => {
            let input = resolve_physical_input(&input_id, &input_name)?;
            let route = selected_saved_route(&state, &external_route_id)?;
            StartAudioRequest::Use {
                input_id: input.id.clone(),
                input_name: input.name.clone(),
                processed_destination_id: route.playback_device.id,
                processed_destination_name: route.playback_device.name,
                reliability_profile,
            }
        }
        StartEngineRequest::Test {
            input_id,
            input_name,
            monitor_id,
            monitor_name,
            reliability_profile,
        } => {
            let input = resolve_physical_input(&input_id, &input_name)?;
            StartAudioRequest::Test {
                input_id: input.id.clone(),
                input_name: input.name.clone(),
                monitor_id,
                monitor_name,
                reliability_profile,
            }
        }
    };
    state.controller().start(request)
}

fn resolve_physical_input(id: &str, friendly_name: &str) -> Result<DeviceInfo, String> {
    let devices = device::list_devices().map_err(|error| error.to_string())?;
    resolve_physical_input_from(id, friendly_name, &devices.inputs)
}

fn resolve_physical_input_from(
    id: &str,
    friendly_name: &str,
    inputs: &[DeviceInfo],
) -> Result<DeviceInfo, String> {
    let exact = inputs
        .iter()
        .filter(|device| device.id == id && !device.is_likely_virtual)
        .collect::<Vec<_>>();
    if let [device] = exact.as_slice() {
        return Ok((*device).clone());
    }
    let normalized_name = friendly_name.trim().to_lowercase();
    let friendly = inputs
        .iter()
        .filter(|device| {
            !device.is_likely_virtual && device.name.trim().to_lowercase() == normalized_name
        })
        .collect::<Vec<_>>();
    if let [device] = friendly.as_slice() {
        return Ok((*device).clone());
    }
    Err(
        "The selected physical input is unavailable or ambiguous. Refresh devices and select it again."
            .to_owned(),
    )
}

#[tauri::command]
pub fn stop_engine(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.controller().stop()
}

#[tauri::command]
pub fn stop_test_route(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.controller().stop_test()
}

#[tauri::command]
pub fn get_engine_status(state: tauri::State<'_, AppState>) -> EngineStatus {
    state.controller().status()
}

#[cfg(test)]
mod tests {
    use super::{resolve_physical_input_from, StartEngineRequest};
    use crate::audio::device::{DeviceDirection, DeviceInfo};
    use serde_json::json;

    #[test]
    fn public_start_requests_are_strictly_route_specific() {
        let use_request = json!({
            "mode": "use",
            "inputId": "input",
            "inputName": "Input",
            "externalRouteId": "route",
            "reliabilityProfile": "balanced"
        });
        assert!(serde_json::from_value::<StartEngineRequest>(use_request).is_ok());

        let ambiguous = json!({
            "mode": "use",
            "inputId": "input",
            "inputName": "Input",
            "externalRouteId": "route",
            "monitorId": "headphones",
            "monitorName": "Headphones",
            "reliabilityProfile": "balanced"
        });
        assert!(serde_json::from_value::<StartEngineRequest>(ambiguous).is_err());
    }

    #[test]
    fn route_start_accepts_only_one_resolvable_physical_input() {
        let physical = DeviceInfo::test(
            "physical",
            "Physical microphone",
            DeviceDirection::Input,
            true,
            false,
        );
        let virtual_capture = DeviceInfo::test(
            "virtual",
            "Studio Virtual Capture",
            DeviceDirection::Input,
            false,
            true,
        );
        assert_eq!(
            resolve_physical_input_from(
                "physical",
                "Physical microphone",
                std::slice::from_ref(&physical),
            )
            .unwrap()
            .id,
            "physical"
        );
        assert!(resolve_physical_input_from(
            "virtual",
            "Studio Virtual Capture",
            &[virtual_capture]
        )
        .is_err());
        assert!(resolve_physical_input_from(
            "missing",
            "Physical microphone",
            &[physical.clone(), physical]
        )
        .is_err());
    }
}
