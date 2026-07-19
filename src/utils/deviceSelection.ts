import type { AudioDevice } from '../types/audio';

export type DeviceSelectionResolution = {
  id: string;
  source: 'identifier' | 'friendlyName' | 'fallback' | 'unavailable';
};

function uniqueMatch(
  devices: AudioDevice[],
  predicate: (device: AudioDevice) => boolean,
): AudioDevice | null {
  const matches = devices.filter(predicate);
  return matches.length === 1 ? matches[0] : null;
}

export function preferredDevice(devices: AudioDevice[], preferCable = false): string {
  if (preferCable) {
    const cable = uniqueMatch(devices, (device) =>
      device.name.toLowerCase().includes('cable input'),
    );
    if (cable) return cable.id;
  }
  return uniqueMatch(devices, (device) => device.isDefault)?.id ?? devices[0]?.id ?? '';
}

export function reconcileSelection(
  selected: string,
  devices: AudioDevice[],
  preferCable = false,
): string {
  return (
    uniqueMatch(devices, (device) => device.id === selected)?.id ??
    preferredDevice(devices, preferCable)
  );
}

export function resolveStoredSelection(
  storedId: string | null,
  storedFriendlyName: string | null,
  devices: AudioDevice[],
  preferCable = false,
): DeviceSelectionResolution {
  if (storedId) {
    const identifierMatch = uniqueMatch(devices, (device) => device.id === storedId);
    if (identifierMatch) {
      return { id: identifierMatch.id, source: 'identifier' };
    }
  }

  const normalizedName = storedFriendlyName?.trim().toLowerCase();
  if (normalizedName) {
    const friendlyNameMatch = uniqueMatch(
      devices,
      (device) => device.name.trim().toLowerCase() === normalizedName,
    );
    if (friendlyNameMatch) {
      return { id: friendlyNameMatch.id, source: 'friendlyName' };
    }
  }

  const fallback = preferredDevice(devices, preferCable);
  return fallback ? { id: fallback, source: 'fallback' } : { id: '', source: 'unavailable' };
}
