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

pub fn find_device(direction: DeviceDirection, id: &str) -> Result<cpal::Device, AudioError> {
    let host = cpal::default_host();
    let devices = devices(&host, direction)?;

    for device in devices {
        let name = device
            .name()
            .map_err(|error| AudioError::DeviceName(error.to_string()))?;
        if stable_device_id(direction, &name) == id {
            return Ok(device);
        }
    }

    Err(AudioError::DeviceNotFound {
        direction: direction.label(),
        id: id.to_owned(),
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
                name,
            })
        })
        .collect()
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
    use super::{stable_device_id, DeviceDirection};

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
}
