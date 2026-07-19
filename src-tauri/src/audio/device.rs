use cpal::traits::{DeviceTrait, HostTrait};
use serde::Serialize;

use crate::error::AudioError;

#[derive(Clone, Copy)]
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

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceInfo {
    pub id: String,
    pub name: String,
    pub is_default: bool,
    pub is_likely_virtual: bool,
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
            Ok(DeviceInfo {
                id: stable_device_id(direction, &name),
                is_default: default_name.as_deref() == Some(name.as_str()),
                is_likely_virtual: matches!(direction, DeviceDirection::Output)
                    && is_likely_virtual_output_name(&name),
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

pub fn is_likely_virtual_output_name(name: &str) -> bool {
    let normalized = name.trim().to_lowercase();
    [
        "virtual",
        "cable input",
        "voicemeeter input",
        "loopback",
        "audio router",
        "vac input",
    ]
    .iter()
    .any(|hint| normalized.contains(hint))
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
        is_likely_virtual_output_name, restoration_index, stable_device_id, DeviceDirection,
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
    fn virtual_output_classification_is_advisory_and_not_vendor_specific() {
        assert!(is_likely_virtual_output_name("VB-Audio CABLE Input"));
        assert!(is_likely_virtual_output_name("My Virtual Audio Router"));
        assert!(!is_likely_virtual_output_name("Realtek Speakers"));
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
