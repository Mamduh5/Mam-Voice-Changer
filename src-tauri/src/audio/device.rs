use cpal::traits::{DeviceTrait, HostTrait};
use serde::Serialize;

use crate::error::AudioError;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DeviceDirection {
    Input,
    Output,
}

impl DeviceDirection {
    pub fn label(self) -> &'static str {
        match self {
            Self::Input => "input",
            Self::Output => "output",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceInfo {
    pub id: String,
    pub name: String,
    pub direction: DeviceDirection,
    pub is_default: bool,
    pub is_likely_virtual: bool,
    pub virtual_family: Option<String>,
    pub minimum_sample_rate: Option<u32>,
    pub maximum_sample_rate: Option<u32>,
    pub common_sample_rates: Vec<u32>,
    pub channel_counts: Vec<u16>,
}

impl DeviceInfo {
    #[cfg(test)]
    pub(crate) fn test(
        id: &str,
        name: &str,
        direction: DeviceDirection,
        is_default: bool,
        is_likely_virtual: bool,
    ) -> Self {
        Self {
            id: id.to_owned(),
            name: name.to_owned(),
            direction,
            is_default,
            is_likely_virtual,
            virtual_family: is_likely_virtual.then(|| normalized_virtual_family(name)),
            minimum_sample_rate: Some(44_100),
            maximum_sample_rate: Some(48_000),
            common_sample_rates: vec![44_100, 48_000],
            channel_counts: vec![2],
        }
    }
}

#[derive(Debug, Serialize)]
pub struct DeviceList {
    pub inputs: Vec<DeviceInfo>,
    pub outputs: Vec<DeviceInfo>,
}

pub fn list_devices() -> Result<DeviceList, AudioError> {
    let host = cpal::default_host();
    Ok(DeviceList {
        inputs: enumerate(&host, DeviceDirection::Input)?,
        outputs: enumerate(&host, DeviceDirection::Output)?,
    })
}

fn enumerate(host: &cpal::Host, direction: DeviceDirection) -> Result<Vec<DeviceInfo>, AudioError> {
    let default_name = match direction {
        DeviceDirection::Input => host.default_input_device(),
        DeviceDirection::Output => host.default_output_device(),
    }
    .and_then(|device| device.name().ok());

    devices(host, direction)?
        .into_iter()
        .map(|device| {
            let name = device
                .name()
                .map_err(|error| AudioError::DeviceName(error.to_string()))?;
            let (minimum_sample_rate, maximum_sample_rate, common_sample_rates, channel_counts) =
                capability_summary(&device, direction);
            let is_likely_virtual = is_likely_virtual_endpoint_name(direction, &name);
            Ok(DeviceInfo {
                id: stable_device_id(direction, &name),
                direction,
                is_default: default_name.as_deref() == Some(name.as_str()),
                is_likely_virtual,
                virtual_family: is_likely_virtual.then(|| normalized_virtual_family(&name)),
                minimum_sample_rate,
                maximum_sample_rate,
                common_sample_rates,
                channel_counts,
                name,
            })
        })
        .collect()
}

pub fn find_device_with_fallback(
    direction: DeviceDirection,
    id: &str,
    friendly_name: &str,
) -> Result<cpal::Device, AudioError> {
    let host = cpal::default_host();
    let available = devices(&host, direction)?;
    let names: Vec<String> = available
        .iter()
        .map(|device| {
            device
                .name()
                .map_err(|error| AudioError::DeviceName(error.to_string()))
        })
        .collect::<Result<_, _>>()?;
    if let Some(index) = restoration_index(direction, id, friendly_name, &names) {
        return available
            .into_iter()
            .nth(index)
            .ok_or_else(|| AudioError::DeviceNotFound {
                direction: direction.label(),
                id: id.to_owned(),
            });
    }
    Err(AudioError::DeviceNotFound {
        direction: direction.label(),
        id: id.to_owned(),
    })
}

fn restoration_index(
    direction: DeviceDirection,
    id: &str,
    friendly_name: &str,
    names: &[String],
) -> Option<usize> {
    let exact: Vec<_> = names
        .iter()
        .enumerate()
        .filter_map(|(index, name)| (stable_device_id(direction, name) == id).then_some(index))
        .collect();
    if exact.len() == 1 {
        return exact.first().copied();
    }
    let normalized = friendly_name.trim().to_lowercase();
    let friendly: Vec<_> = names
        .iter()
        .enumerate()
        .filter_map(|(index, name)| (name.trim().to_lowercase() == normalized).then_some(index))
        .collect();
    (friendly.len() == 1).then(|| friendly[0])
}

pub fn is_likely_virtual_endpoint_name(direction: DeviceDirection, name: &str) -> bool {
    let normalized = name.trim().to_lowercase();
    let strong_hint = [
        "virtual",
        "vb-audio",
        "voicemeeter",
        "loopback",
        "audio router",
        "virtual audio cable",
        "vac ",
    ]
    .iter()
    .any(|hint| normalized.contains(hint));
    strong_hint
        || match direction {
            DeviceDirection::Output => {
                normalized.contains("cable input") || normalized.contains("virtual in")
            }
            DeviceDirection::Input => {
                normalized.contains("cable output") || normalized.contains("virtual out")
            }
        }
}

pub fn normalized_virtual_family(name: &str) -> String {
    let normalized = name
        .trim()
        .to_lowercase()
        .replace("vb-audio", " vbaudio ")
        .replace("voicemeeter", " voicemeeter ")
        .replace("virtual audio cable", " vac ");
    let ignored = [
        "input",
        "output",
        "playback",
        "recording",
        "capture",
        "microphone",
        "speakers",
        "endpoint",
        "device",
        "virtual",
        "audio",
    ];
    let mut tokens = normalized
        .split(|character: char| !character.is_alphanumeric())
        .filter(|token| !token.is_empty() && !ignored.contains(token))
        .collect::<Vec<_>>();
    tokens.sort_unstable();
    tokens.dedup();
    if tokens.is_empty() {
        "generic-virtual-route".to_owned()
    } else {
        tokens.join("-")
    }
}

fn capability_summary(
    device: &cpal::Device,
    direction: DeviceDirection,
) -> (Option<u32>, Option<u32>, Vec<u32>, Vec<u16>) {
    let configs = match direction {
        DeviceDirection::Input => device
            .supported_input_configs()
            .map(|configs| configs.collect()),
        DeviceDirection::Output => device
            .supported_output_configs()
            .map(|configs| configs.collect()),
    };
    let Ok(configs): Result<Vec<_>, _> = configs else {
        return (None, None, Vec::new(), Vec::new());
    };
    let minimum = configs
        .iter()
        .map(|config| config.min_sample_rate().0)
        .min();
    let maximum = configs
        .iter()
        .map(|config| config.max_sample_rate().0)
        .max();
    let mut common_sample_rates = [44_100, 48_000]
        .into_iter()
        .filter(|rate| {
            configs.iter().any(|config| {
                (config.min_sample_rate().0..=config.max_sample_rate().0).contains(rate)
            })
        })
        .collect::<Vec<_>>();
    common_sample_rates.sort_unstable();
    let mut channel_counts = configs
        .iter()
        .map(cpal::SupportedStreamConfigRange::channels)
        .collect::<Vec<_>>();
    channel_counts.sort_unstable();
    channel_counts.dedup();
    (minimum, maximum, common_sample_rates, channel_counts)
}

fn devices(host: &cpal::Host, direction: DeviceDirection) -> Result<Vec<cpal::Device>, AudioError> {
    match direction {
        DeviceDirection::Input => host
            .input_devices()
            .map(|devices| devices.collect())
            .map_err(|error| AudioError::DeviceEnumeration {
                direction: direction.label(),
                details: error.to_string(),
            }),
        DeviceDirection::Output => host
            .output_devices()
            .map(|devices| devices.collect())
            .map_err(|error| AudioError::DeviceEnumeration {
                direction: direction.label(),
                details: error.to_string(),
            }),
    }
}

pub fn stable_device_id(direction: DeviceDirection, name: &str) -> String {
    // CPAL 0.15 does not expose the WASAPI endpoint GUID. A deterministic FNV-1a
    // fingerprint avoids the previous enumeration-index IDs and remains stable
    // across refreshes and restarts while the Windows friendly name is unchanged.
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in direction
        .label()
        .bytes()
        .chain(b":".iter().copied())
        .chain(name.trim().to_lowercase().bytes())
    {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("wasapi-{}-{hash:016x}", direction.label())
}

#[cfg(test)]
mod tests {
    use super::{
        is_likely_virtual_endpoint_name, normalized_virtual_family, restoration_index,
        stable_device_id, DeviceDirection,
    };

    #[test]
    fn device_ids_do_not_depend_on_enumeration_order() {
        let first = stable_device_id(DeviceDirection::Input, "USB Microphone");
        let second = stable_device_id(DeviceDirection::Input, "USB Microphone");
        assert_eq!(first, second);
        assert_ne!(
            first,
            stable_device_id(DeviceDirection::Output, "USB Microphone")
        );
    }

    #[test]
    fn virtual_endpoint_classification_is_directional_and_advisory() {
        assert!(is_likely_virtual_endpoint_name(
            DeviceDirection::Output,
            "VB-Audio CABLE Input"
        ));
        assert!(is_likely_virtual_endpoint_name(
            DeviceDirection::Input,
            "VB-Audio CABLE Output"
        ));
        assert!(is_likely_virtual_endpoint_name(
            DeviceDirection::Output,
            "My Virtual Audio Router"
        ));
        assert!(!is_likely_virtual_endpoint_name(
            DeviceDirection::Output,
            "Realtek HDMI Output"
        ));
        assert!(!is_likely_virtual_endpoint_name(
            DeviceDirection::Input,
            "Physical input microphone"
        ));
    }

    #[test]
    fn complementary_endpoint_names_share_a_normalized_family() {
        assert_eq!(
            normalized_virtual_family("CABLE Input (VB-Audio Virtual Cable)"),
            normalized_virtual_family("CABLE Output (VB-Audio Virtual Cable)")
        );
    }

    #[test]
    fn restoration_prefers_id_then_only_uses_a_unique_friendly_name() {
        let direction = DeviceDirection::Output;
        let names = vec!["Speakers".to_owned(), "Virtual Route".to_owned()];
        let virtual_id = stable_device_id(direction, "Virtual Route");
        assert_eq!(
            restoration_index(direction, &virtual_id, "wrong name", &names),
            Some(1)
        );
        assert_eq!(
            restoration_index(direction, "missing", "Speakers", &names),
            Some(0)
        );

        let duplicates = vec!["USB Audio".to_owned(), "USB Audio".to_owned()];
        assert_eq!(
            restoration_index(direction, "missing", "USB Audio", &duplicates),
            None
        );
    }
}
