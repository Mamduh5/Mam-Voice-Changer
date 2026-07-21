use serde::Deserialize;

use crate::{
    audio::{
        device::{self, find_device_with_fallback, DeviceDirection, DeviceInfo, DeviceList},
        external_route::{
            discover_external_routes, manual_route, unpaired_capture_devices, ExternalAudioRoute,
            ExternalAudioRouteCatalog, PairingConfidence, RouteCompatibilityResult, RouteReadiness,
            RouteValidationStatus,
        },
        stream_config,
    },
    config::application_settings::{
        resolve_device_selections, ApplicationSettingsDocument, APPLICATION_SETTINGS_SCHEMA_VERSION,
    },
    state::app_state::AppState,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SaveExternalAudioRouteRequest {
    candidate_route_id: Option<String>,
    playback_device_id: String,
    capture_device_id: String,
    confirm_physical_endpoints: bool,
}

#[tauri::command]
pub fn list_external_audio_routes(
    state: tauri::State<'_, AppState>,
) -> Result<ExternalAudioRouteCatalog, String> {
    build_catalog(&state)
}

#[tauri::command]
pub fn save_external_audio_route(
    request: SaveExternalAudioRouteRequest,
    state: tauri::State<'_, AppState>,
) -> Result<ExternalAudioRouteCatalog, String> {
    require_stopped(&state)?;
    if request.playback_device_id == request.capture_device_id {
        return Err(
            "Playback and capture endpoints must be different directional devices.".to_owned(),
        );
    }
    let DeviceList { inputs, outputs } =
        device::list_devices().map_err(|error| error.to_string())?;
    let playback = unique_device("playback endpoint", &request.playback_device_id, &outputs)?;
    let capture = unique_device("capture endpoint", &request.capture_device_id, &inputs)?;
    let discovered = discover_external_routes(&inputs, &outputs);
    let automatic = request.candidate_route_id.as_deref().and_then(|route_id| {
        discovered.iter().find(|route| {
            route.route_id == route_id
                && route.playback_device.id == playback.id
                && route
                    .capture_device
                    .as_ref()
                    .map(|device| device.id.as_str())
                    == Some(capture.id.as_str())
                && route.validation_status == RouteValidationStatus::Ready
        })
    });
    if automatic.is_none()
        && (!playback.is_likely_virtual || !capture.is_likely_virtual)
        && !request.confirm_physical_endpoints
    {
        return Err(
            "This manual pair contains a likely physical endpoint. Confirm the advanced physical-endpoint warning before saving."
                .to_owned(),
        );
    }
    let route = automatic
        .cloned()
        .unwrap_or_else(|| manual_route(playback, capture));
    let mut store = state
        .application_settings()
        .lock()
        .map_err(|_| "Application settings storage is unavailable.".to_owned())?;
    let document = route_document(store.document(), &route);
    store.save(document).map_err(|error| error.to_string())?;
    drop(store);
    build_catalog(&state)
}

#[tauri::command]
pub fn delete_external_audio_route(
    state: tauri::State<'_, AppState>,
) -> Result<ExternalAudioRouteCatalog, String> {
    require_stopped(&state)?;
    let mut store = state
        .application_settings()
        .lock()
        .map_err(|_| "Application settings storage is unavailable.".to_owned())?;
    let mut document = store.document().clone();
    clear_route(&mut document);
    store.save(document).map_err(|error| error.to_string())?;
    drop(store);
    build_catalog(&state)
}

#[tauri::command]
pub fn validate_external_audio_route(
    input_id: String,
    route_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<RouteCompatibilityResult, String> {
    if !matches!(
        state.controller().status().state,
        crate::state::engine_state::EngineState::Stopped
            | crate::state::engine_state::EngineState::Error
    ) {
        return Ok(result(
            Some(route_id),
            RouteReadiness::EngineActive,
            "Stop the active Use or Test route before validating another route.",
            None,
            false,
        ));
    }
    let catalog = build_catalog(&state)?;
    let Some(route) = catalog
        .routes
        .iter()
        .find(|route| route.route_id == route_id)
    else {
        return Ok(result(
            Some(route_id),
            RouteReadiness::DeviceUnavailable,
            "The selected external route is no longer available. Refresh devices and choose again.",
            None,
            false,
        ));
    };
    let DeviceList { inputs, outputs } =
        device::list_devices().map_err(|error| error.to_string())?;
    let input_match_count = inputs
        .iter()
        .filter(|device| device.id == input_id && !device.is_likely_virtual)
        .count();
    let playback_match_count = outputs
        .iter()
        .filter(|device| device.id == route.playback_device.id)
        .count();
    let capture_match_count = route.capture_device.as_ref().map_or(0, |capture| {
        inputs
            .iter()
            .filter(|device| device.id == capture.id)
            .count()
    });
    if let Some(readiness) = inventory_readiness(
        route,
        input_match_count,
        playback_match_count,
        capture_match_count,
    ) {
        return Ok(readiness);
    }
    let input = inputs
        .iter()
        .find(|device| device.id == input_id && !device.is_likely_virtual)
        .expect("inventory readiness requires one physical input");
    let input_device = find_device_with_fallback(DeviceDirection::Input, &input.id, &input.name)
        .map_err(|error| error.to_string())?;
    let playback_device = find_device_with_fallback(
        DeviceDirection::Output,
        &route.playback_device.id,
        &route.playback_device.name,
    )
    .map_err(|error| error.to_string())?;
    match stream_config::negotiate(&input_device, &playback_device, 256) {
        Ok(negotiated) => Ok(result(
            Some(route.route_id.clone()),
            RouteReadiness::Ready,
            "Route configuration is ready. Playback availability does not prove that a receiving application is consuming the capture endpoint.",
            Some(negotiated.sample_rate),
            true,
        )),
        Err(error) => Ok(result(
            Some(route.route_id.clone()),
            RouteReadiness::IncompatibleFormat,
            &error.to_string(),
            None,
            true,
        )),
    }
}

pub(crate) fn selected_saved_route(
    state: &AppState,
    route_id: &str,
) -> Result<ExternalAudioRoute, String> {
    let catalog = build_catalog(state)?;
    let route = catalog
        .routes
        .into_iter()
        .find(|route| route.route_id == route_id)
        .ok_or_else(|| {
            "The selected external route is unavailable. Refresh devices and select it again."
                .to_owned()
        })?;
    if catalog.selected_route_id.as_deref() != Some(route_id) {
        return Err("Save the external route before starting Use.".to_owned());
    }
    if route.validation_status != RouteValidationStatus::Ready {
        return Err(route.compatibility.details.clone());
    }
    Ok(route)
}

fn build_catalog(state: &AppState) -> Result<ExternalAudioRouteCatalog, String> {
    let DeviceList { inputs, outputs } =
        device::list_devices().map_err(|error| error.to_string())?;
    let mut store = state
        .application_settings()
        .lock()
        .map_err(|_| "Application settings storage is unavailable.".to_owned())?;
    let document = store.document().clone();
    let resolved = resolve_device_selections(&document, &inputs, &outputs);
    let mut routes = discover_external_routes(&inputs, &outputs);
    let mut selected_route_id = None;
    let mut restoration_warning = resolved.restoration_warning;

    if let (Some(playback_id), Some(capture_id)) = (
        resolved.external_route_playback_id.as_deref(),
        resolved.external_route_capture_id.as_deref(),
    ) {
        let playback = unique_device("saved playback endpoint", playback_id, &outputs);
        let capture = unique_device("saved capture endpoint", capture_id, &inputs);
        if let (Ok(playback), Ok(capture)) = (playback, capture) {
            if document.external_route_manual {
                let saved = manual_route(playback, capture);
                selected_route_id = Some(saved.route_id.clone());
                if !routes.iter().any(|route| route.route_id == saved.route_id) {
                    routes.push(saved);
                }
            } else if let Some(saved) = routes.iter().find(|route| {
                route.playback_device.id == playback.id
                    && route
                        .capture_device
                        .as_ref()
                        .map(|device| device.id.as_str())
                        == Some(capture.id.as_str())
                    && route.validation_status == RouteValidationStatus::Ready
            }) {
                selected_route_id = Some(saved.route_id.clone());
            } else {
                append_warning(
                    &mut restoration_warning,
                    "The stored automatic external route is no longer a unique safe pair. Select and save the playback/capture pair manually.",
                );
            }
        }
    } else if let Some(playback_id) = resolved.external_route_playback_id.as_deref() {
        let matches = routes
            .iter()
            .filter(|route| {
                route.playback_device.id == playback_id
                    && route.validation_status == RouteValidationStatus::Ready
            })
            .cloned()
            .collect::<Vec<_>>();
        if let [route] = matches.as_slice() {
            let migrated = route_document(store.document(), route);
            if let Err(error) = store.save(migrated) {
                restoration_warning = Some(format!(
                    "{} Conservative route pairing was found but could not be persisted: {error}",
                    restoration_warning.unwrap_or_default()
                ));
            } else {
                selected_route_id = Some(route.route_id.clone());
            }
        }
    }

    let virtual_playback_devices = outputs
        .iter()
        .filter(|device| device.is_likely_virtual)
        .cloned()
        .collect();
    let virtual_capture_devices = inputs
        .iter()
        .filter(|device| device.is_likely_virtual)
        .cloned()
        .collect();
    let unpaired_capture_devices = unpaired_capture_devices(&inputs, &routes);
    Ok(ExternalAudioRouteCatalog {
        routes,
        virtual_playback_devices,
        virtual_capture_devices,
        unpaired_capture_devices,
        selected_route_id,
        restoration_warning,
    })
}

fn route_document(
    current: &ApplicationSettingsDocument,
    route: &ExternalAudioRoute,
) -> ApplicationSettingsDocument {
    let capture = route
        .capture_device
        .as_ref()
        .expect("saved external routes always contain a capture endpoint");
    ApplicationSettingsDocument {
        schema_version: APPLICATION_SETTINGS_SCHEMA_VERSION,
        selected_input_device_id: current.selected_input_device_id.clone(),
        last_known_input_friendly_name: current.last_known_input_friendly_name.clone(),
        selected_external_route_id: Some(route.route_id.clone()),
        external_route_playback_device_id: Some(route.playback_device.id.clone()),
        last_known_external_route_playback_name: Some(route.playback_device.name.clone()),
        external_route_capture_device_id: Some(capture.id.clone()),
        last_known_external_route_capture_name: Some(capture.name.clone()),
        external_route_pairing_source: Some(route.pairing_source),
        external_route_manual: route.manual,
        local_monitor_device_id: current.local_monitor_device_id.clone(),
        last_known_local_monitor_friendly_name: current
            .last_known_local_monitor_friendly_name
            .clone(),
        reliability_profile: current.reliability_profile,
        last_page: current.last_page,
    }
}

fn clear_route(document: &mut ApplicationSettingsDocument) {
    document.selected_external_route_id = None;
    document.external_route_playback_device_id = None;
    document.last_known_external_route_playback_name = None;
    document.external_route_capture_device_id = None;
    document.last_known_external_route_capture_name = None;
    document.external_route_pairing_source = None;
    document.external_route_manual = false;
}

fn append_warning(warning: &mut Option<String>, message: &str) {
    *warning = Some(match warning.take() {
        Some(existing) if !existing.is_empty() => format!("{existing} {message}"),
        _ => message.to_owned(),
    });
}

fn unique_device<'a>(
    purpose: &str,
    id: &str,
    devices: &'a [DeviceInfo],
) -> Result<&'a DeviceInfo, String> {
    let matches = devices
        .iter()
        .filter(|device| device.id == id)
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [device] => Ok(device),
        [] => Err(format!("The selected {purpose} is no longer available.")),
        _ => Err(format!(
            "The selected {purpose} identifier is ambiguous because multiple endpoints share its friendly-name fingerprint."
        )),
    }
}

fn require_stopped(state: &AppState) -> Result<(), String> {
    if matches!(
        state.controller().status().state,
        crate::state::engine_state::EngineState::Stopped
            | crate::state::engine_state::EngineState::Error
    ) {
        Ok(())
    } else {
        Err("Stop the active Use or Test route before changing external routing.".to_owned())
    }
}

fn result(
    route_id: Option<String>,
    readiness: RouteReadiness,
    message: &str,
    negotiated_sample_rate: Option<u32>,
    capture_endpoint_available: bool,
) -> RouteCompatibilityResult {
    RouteCompatibilityResult {
        route_id,
        readiness,
        message: message.to_owned(),
        negotiated_sample_rate,
        capture_endpoint_available,
    }
}

fn inventory_readiness(
    route: &ExternalAudioRoute,
    input_match_count: usize,
    playback_match_count: usize,
    capture_match_count: usize,
) -> Option<RouteCompatibilityResult> {
    if route.pairing_confidence == PairingConfidence::Ambiguous {
        return Some(result(
            Some(route.route_id.clone()),
            RouteReadiness::AmbiguousPair,
            "Multiple capture endpoints are equally plausible. Save a manual pair.",
            None,
            false,
        ));
    }
    if route.capture_device.is_none() {
        return Some(result(
            Some(route.route_id.clone()),
            RouteReadiness::MissingCapture,
            "The virtual playback endpoint has no paired capture endpoint. Save a manual pair.",
            None,
            false,
        ));
    }
    if route.validation_status == RouteValidationStatus::IncompatibleFormat {
        return Some(result(
            Some(route.route_id.clone()),
            RouteReadiness::IncompatibleFormat,
            &route.compatibility.details,
            None,
            capture_match_count == 1,
        ));
    }
    if input_match_count != 1 {
        return Some(result(
            Some(route.route_id.clone()),
            RouteReadiness::MissingInput,
            "Select one available physical input microphone.",
            None,
            capture_match_count == 1,
        ));
    }
    if playback_match_count != 1 {
        return Some(result(
            Some(route.route_id.clone()),
            RouteReadiness::MissingPlayback,
            "The virtual playback endpoint is missing or ambiguous. Refresh devices.",
            None,
            capture_match_count == 1,
        ));
    }
    if capture_match_count != 1 {
        return Some(result(
            Some(route.route_id.clone()),
            RouteReadiness::MissingCapture,
            "The paired capture endpoint is missing or ambiguous. Refresh devices.",
            None,
            false,
        ));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::device::{DeviceDirection, DeviceInfo};

    fn ready_route() -> ExternalAudioRoute {
        manual_route(
            &DeviceInfo::test(
                "playback",
                "Studio Virtual Playback",
                DeviceDirection::Output,
                false,
                true,
            ),
            &DeviceInfo::test(
                "capture",
                "Studio Virtual Capture",
                DeviceDirection::Input,
                false,
                true,
            ),
        )
    }

    #[test]
    fn ready_inventory_continues_to_dynamic_format_negotiation() {
        assert!(inventory_readiness(&ready_route(), 1, 1, 1).is_none());
    }

    #[test]
    fn missing_input_playback_capture_and_removed_devices_are_precise() {
        let route = ready_route();
        assert_eq!(
            inventory_readiness(&route, 0, 1, 1).unwrap().readiness,
            RouteReadiness::MissingInput
        );
        assert_eq!(
            inventory_readiness(&route, 1, 0, 1).unwrap().readiness,
            RouteReadiness::MissingPlayback
        );
        assert_eq!(
            inventory_readiness(&route, 1, 1, 0).unwrap().readiness,
            RouteReadiness::MissingCapture
        );
    }

    #[test]
    fn ambiguous_and_incompatible_routes_do_not_become_ready() {
        let ambiguous = discover_external_routes(
            &[
                DeviceInfo::test(
                    "capture-a",
                    "Studio Virtual Capture",
                    DeviceDirection::Input,
                    false,
                    true,
                ),
                DeviceInfo::test(
                    "capture-b",
                    "Studio Virtual Capture",
                    DeviceDirection::Input,
                    false,
                    true,
                ),
            ],
            &[DeviceInfo::test(
                "playback",
                "Studio Virtual Playback",
                DeviceDirection::Output,
                false,
                true,
            )],
        );
        assert_eq!(
            inventory_readiness(&ambiguous[0], 1, 1, 2)
                .unwrap()
                .readiness,
            RouteReadiness::AmbiguousPair
        );

        let mut incompatible = ready_route();
        incompatible.validation_status = RouteValidationStatus::IncompatibleFormat;
        incompatible
            .compatibility
            .common_virtual_sample_rates
            .clear();
        assert_eq!(
            inventory_readiness(&incompatible, 1, 1, 1)
                .unwrap()
                .readiness,
            RouteReadiness::IncompatibleFormat
        );
    }
}
