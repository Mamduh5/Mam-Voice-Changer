use crate::audio::device::{self, DeviceList};

#[tauri::command]
pub fn list_audio_devices() -> Result<DeviceList, String> {
    device::list_devices().map_err(|error| error.to_string())
}
