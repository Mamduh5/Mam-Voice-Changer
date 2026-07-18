import type { AudioDevice } from '../types/audio';

export function preferredDevice(devices: AudioDevice[], preferCable = false): string {
  if (preferCable) {
    const cable = devices.find((device) => device.name.toLowerCase().includes('cable input'));
    if (cable) return cable.id;
  }
  return devices.find((device) => device.isDefault)?.id ?? devices[0]?.id ?? '';
}

export function reconcileSelection(
  selected: string,
  devices: AudioDevice[],
  preferCable = false,
): string {
  return devices.some((device) => device.id === selected)
    ? selected
    : preferredDevice(devices, preferCable);
}
