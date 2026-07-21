use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::audio::device::{normalized_virtual_family, DeviceInfo};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PairingConfidence {
    Exact,
    High,
    Manual,
    Ambiguous,
    Unpaired,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PairingSource {
    KnownPattern,
    NormalizedName,
    VendorFamily,
    Manual,
    None,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RouteValidationStatus {
    Ready,
    MissingCapture,
    AmbiguousPair,
    IncompatibleFormat,
    PhysicalConfirmationRequired,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RouteReadiness {
    Ready,
    MissingInput,
    MissingPlayback,
    MissingCapture,
    AmbiguousPair,
    IncompatibleFormat,
    DeviceUnavailable,
    EngineActive,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteCompatibilityDetails {
    pub common_virtual_sample_rates: Vec<u32>,
    pub details: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalAudioRoute {
    pub route_id: String,
    pub display_name: String,
    pub playback_device: DeviceInfo,
    pub capture_device: Option<DeviceInfo>,
    pub candidate_capture_devices: Vec<DeviceInfo>,
    pub pairing_confidence: PairingConfidence,
    pub pairing_source: PairingSource,
    pub validation_status: RouteValidationStatus,
    pub compatibility: RouteCompatibilityDetails,
    pub manual: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalAudioRouteCatalog {
    pub routes: Vec<ExternalAudioRoute>,
    pub virtual_playback_devices: Vec<DeviceInfo>,
    pub virtual_capture_devices: Vec<DeviceInfo>,
    pub unpaired_capture_devices: Vec<DeviceInfo>,
    pub selected_route_id: Option<String>,
    pub restoration_warning: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteCompatibilityResult {
    pub route_id: Option<String>,
    pub readiness: RouteReadiness,
    pub message: String,
    pub negotiated_sample_rate: Option<u32>,
    pub capture_endpoint_available: bool,
}

#[derive(Clone, Copy)]
struct PairScore {
    score: u16,
    confidence: PairingConfidence,
    source: PairingSource,
}

pub fn discover_external_routes(
    inputs: &[DeviceInfo],
    outputs: &[DeviceInfo],
) -> Vec<ExternalAudioRoute> {
    let playback = outputs
        .iter()
        .filter(|device| device.is_likely_virtual)
        .collect::<Vec<_>>();
    let capture = inputs
        .iter()
        .filter(|device| device.is_likely_virtual)
        .collect::<Vec<_>>();
    let playback_id_counts = id_counts(&playback);
    let capture_id_counts = id_counts(&capture);

    playback
        .iter()
        .map(|playback_device| {
            let scored = capture
                .iter()
                .filter_map(|capture_device| {
                    pair_score(playback_device, capture_device)
                        .map(|score| (*capture_device, score))
                })
                .collect::<Vec<_>>();
            let maximum = scored.iter().map(|(_, score)| score.score).max();
            let best = maximum
                .map(|maximum| {
                    scored
                        .iter()
                        .filter(|(_, score)| score.score == maximum)
                        .copied()
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let playback_is_duplicate = playback_id_counts
                .get(&playback_device.id)
                .is_some_and(|count| *count > 1);
            let mutual = best.first().and_then(|(capture_device, score)| {
                let capture_best = playback
                    .iter()
                    .filter_map(|candidate_playback| {
                        pair_score(candidate_playback, capture_device)
                            .map(|candidate_score| (*candidate_playback, candidate_score))
                    })
                    .collect::<Vec<_>>();
                let capture_maximum = capture_best.iter().map(|(_, score)| score.score).max()?;
                let capture_winners = capture_best
                    .iter()
                    .filter(|(_, candidate_score)| candidate_score.score == capture_maximum)
                    .collect::<Vec<_>>();
                (best.len() == 1
                    && capture_winners.len() == 1
                    && capture_winners[0].0.id == playback_device.id
                    && !playback_is_duplicate
                    && capture_id_counts
                        .get(&capture_device.id)
                        .is_some_and(|count| *count == 1))
                .then_some((*capture_device, *score))
            });

            if let Some((capture_device, score)) = mutual {
                build_route(
                    playback_device,
                    Some(capture_device),
                    Vec::new(),
                    score.confidence,
                    score.source,
                    false,
                )
            } else if !best.is_empty() {
                build_route(
                    playback_device,
                    None,
                    best.iter().map(|(device, _)| (*device).clone()).collect(),
                    PairingConfidence::Ambiguous,
                    PairingSource::None,
                    false,
                )
            } else {
                build_route(
                    playback_device,
                    None,
                    Vec::new(),
                    PairingConfidence::Unpaired,
                    PairingSource::None,
                    false,
                )
            }
        })
        .collect()
}

pub fn manual_route(playback: &DeviceInfo, capture: &DeviceInfo) -> ExternalAudioRoute {
    build_route(
        playback,
        Some(capture),
        Vec::new(),
        PairingConfidence::Manual,
        PairingSource::Manual,
        true,
    )
}

pub fn stable_route_id(playback_id: &str, capture_id: Option<&str>, manual: bool) -> String {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    let source = format!(
        "external-route:{playback_id}:{}:{}",
        capture_id.unwrap_or("unpaired"),
        if manual { "manual" } else { "automatic" }
    );
    for byte in source.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("external-route-{hash:016x}")
}

pub fn unpaired_capture_devices(
    inputs: &[DeviceInfo],
    routes: &[ExternalAudioRoute],
) -> Vec<DeviceInfo> {
    let paired = routes
        .iter()
        .filter_map(|route| {
            route
                .capture_device
                .as_ref()
                .map(|device| device.id.as_str())
        })
        .collect::<HashSet<_>>();
    inputs
        .iter()
        .filter(|device| device.is_likely_virtual && !paired.contains(device.id.as_str()))
        .cloned()
        .collect()
}

fn build_route(
    playback: &DeviceInfo,
    capture: Option<&DeviceInfo>,
    candidates: Vec<DeviceInfo>,
    confidence: PairingConfidence,
    source: PairingSource,
    manual: bool,
) -> ExternalAudioRoute {
    let common_rates = capture
        .map(|capture| common_sample_rates(playback, capture))
        .unwrap_or_default();
    let validation_status = match (capture, confidence, common_rates.is_empty()) {
        (None, PairingConfidence::Ambiguous, _) => RouteValidationStatus::AmbiguousPair,
        (None, _, _) => RouteValidationStatus::MissingCapture,
        (Some(_), _, true) => RouteValidationStatus::IncompatibleFormat,
        (Some(_), _, false)
            if !manual && (!playback.is_likely_virtual || !capture.unwrap().is_likely_virtual) =>
        {
            RouteValidationStatus::PhysicalConfirmationRequired
        }
        (Some(_), _, false) => RouteValidationStatus::Ready,
    };
    let display_name = capture.map_or_else(
        || playback.name.clone(),
        |capture| format!("{} -> {}", playback.name, capture.name),
    );
    ExternalAudioRoute {
        route_id: stable_route_id(
            &playback.id,
            capture.map(|device| device.id.as_str()),
            manual,
        ),
        display_name,
        playback_device: playback.clone(),
        capture_device: capture.cloned(),
        candidate_capture_devices: candidates,
        pairing_confidence: confidence,
        pairing_source: source,
        validation_status,
        compatibility: RouteCompatibilityDetails {
            details: if capture.is_none() {
                "A capture endpoint must be selected before this route can be used.".to_owned()
            } else if common_rates.is_empty() {
                "The playback and capture endpoints do not advertise 44.1 or 48 kHz in common. Align them in Windows Sound settings.".to_owned()
            } else {
                format!(
                    "Both virtual endpoints advertise {}.",
                    common_rates
                        .iter()
                        .map(|rate| format!("{} Hz", rate))
                        .collect::<Vec<_>>()
                        .join(" and ")
                )
            },
            common_virtual_sample_rates: common_rates,
        },
        manual,
    }
}

fn common_sample_rates(first: &DeviceInfo, second: &DeviceInfo) -> Vec<u32> {
    first
        .common_sample_rates
        .iter()
        .filter(|rate| second.common_sample_rates.contains(rate))
        .copied()
        .collect()
}

fn id_counts(devices: &[&DeviceInfo]) -> HashMap<String, usize> {
    let mut counts = HashMap::new();
    for device in devices {
        *counts.entry(device.id.clone()).or_default() += 1;
    }
    counts
}

fn pair_score(playback: &DeviceInfo, capture: &DeviceInfo) -> Option<PairScore> {
    if !playback.is_likely_virtual || !capture.is_likely_virtual {
        return None;
    }
    let playback_family = playback
        .virtual_family
        .clone()
        .unwrap_or_else(|| normalized_virtual_family(&playback.name));
    let capture_family = capture
        .virtual_family
        .clone()
        .unwrap_or_else(|| normalized_virtual_family(&capture.name));
    if playback_family == capture_family && known_complement(&playback.name, &capture.name) {
        Some(PairScore {
            score: 300,
            confidence: PairingConfidence::Exact,
            source: PairingSource::KnownPattern,
        })
    } else if playback_family == capture_family
        && pairing_base(&playback.name) == pairing_base(&capture.name)
    {
        Some(PairScore {
            score: 200,
            confidence: PairingConfidence::High,
            source: PairingSource::NormalizedName,
        })
    } else {
        None
    }
}

fn known_complement(playback: &str, capture: &str) -> bool {
    let playback = playback.to_lowercase();
    let capture = capture.to_lowercase();
    [
        ("cable input", "cable output"),
        ("virtual in", "virtual out"),
        ("playback", "recording"),
        ("input", "output"),
    ]
    .iter()
    .any(|(playback_hint, capture_hint)| {
        playback.contains(playback_hint) && capture.contains(capture_hint)
    })
}

fn pairing_base(name: &str) -> String {
    normalized_virtual_family(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::device::{DeviceDirection, DeviceInfo};

    fn playback(id: &str, name: &str) -> DeviceInfo {
        DeviceInfo::test(id, name, DeviceDirection::Output, false, true)
    }

    fn capture(id: &str, name: &str) -> DeviceInfo {
        DeviceInfo::test(id, name, DeviceDirection::Input, false, true)
    }

    #[test]
    fn discovers_a_clear_known_pair() {
        let routes = discover_external_routes(
            &[capture("capture", "CABLE Output (VB-Audio Virtual Cable)")],
            &[playback("playback", "CABLE Input (VB-Audio Virtual Cable)")],
        );
        assert_eq!(routes[0].pairing_confidence, PairingConfidence::Exact);
        assert_eq!(routes[0].pairing_source, PairingSource::KnownPattern);
        assert_eq!(routes[0].capture_device.as_ref().unwrap().id, "capture");
    }

    #[test]
    fn discovers_a_high_confidence_normalized_pair() {
        let routes = discover_external_routes(
            &[capture("capture", "Studio Virtual Capture")],
            &[playback("playback", "Studio Virtual Playback")],
        );
        assert_eq!(routes[0].pairing_confidence, PairingConfidence::High);
        assert_eq!(routes[0].pairing_source, PairingSource::NormalizedName);
        assert!(routes[0].capture_device.is_some());
    }

    #[test]
    fn manual_pair_is_explicit_stable_and_never_claims_automatic_confidence() {
        let playback = DeviceInfo::test(
            "playback",
            "Advanced physical playback",
            DeviceDirection::Output,
            false,
            false,
        );
        let capture = DeviceInfo::test(
            "capture",
            "Advanced physical capture",
            DeviceDirection::Input,
            false,
            false,
        );
        let first = manual_route(&playback, &capture);
        let second = manual_route(&playback, &capture);

        assert_eq!(first.route_id, second.route_id);
        assert_eq!(first.pairing_confidence, PairingConfidence::Manual);
        assert_eq!(first.pairing_source, PairingSource::Manual);
        assert!(first.manual);
        assert_eq!(first.validation_status, RouteValidationStatus::Ready);
    }

    #[test]
    fn equal_capture_candidates_remain_ambiguous() {
        let routes = discover_external_routes(
            &[
                capture("capture-a", "CABLE Output (VB-Audio Virtual Cable)"),
                capture("capture-b", "CABLE Output (VB-Audio Virtual Cable)"),
            ],
            &[playback("playback", "CABLE Input (VB-Audio Virtual Cable)")],
        );
        assert_eq!(routes[0].pairing_confidence, PairingConfidence::Ambiguous);
        assert_eq!(routes[0].candidate_capture_devices.len(), 2);
    }

    #[test]
    fn equal_playback_candidates_remain_ambiguous() {
        let routes = discover_external_routes(
            &[capture("capture", "CABLE Output (VB-Audio Virtual Cable)")],
            &[
                playback("playback-a", "CABLE Input (VB-Audio Virtual Cable)"),
                playback("playback-b", "CABLE Input (VB-Audio Virtual Cable)"),
            ],
        );
        assert!(routes
            .iter()
            .all(|route| route.pairing_confidence == PairingConfidence::Ambiguous));
    }

    #[test]
    fn unrelated_families_and_physical_devices_are_never_automatically_paired() {
        let unrelated = discover_external_routes(
            &[capture("capture", "Vendor B Virtual Output")],
            &[playback("playback", "Vendor A Virtual Input")],
        );
        assert_eq!(unrelated[0].pairing_confidence, PairingConfidence::Unpaired);

        let physical = DeviceInfo::test(
            "physical",
            "Physical input microphone",
            DeviceDirection::Input,
            true,
            false,
        );
        let routes =
            discover_external_routes(&[physical], &[playback("playback", "Studio Virtual Input")]);
        assert!(routes[0].capture_device.is_none());
    }

    #[test]
    fn duplicate_ids_and_unpaired_capture_are_reported_conservatively() {
        let captures = [
            capture("same", "CABLE Output (VB-Audio Virtual Cable)"),
            capture("same", "CABLE Output (VB-Audio Virtual Cable)"),
            capture("orphan", "Other Virtual Output"),
        ];
        let routes = discover_external_routes(
            &captures,
            &[playback("playback", "CABLE Input (VB-Audio Virtual Cable)")],
        );
        assert_eq!(routes[0].pairing_confidence, PairingConfidence::Ambiguous);
        assert!(unpaired_capture_devices(&captures, &routes)
            .iter()
            .any(|device| device.id == "orphan"));
    }
}
