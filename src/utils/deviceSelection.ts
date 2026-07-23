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

export function preferredDevice(devices: AudioDevice[]): string {
  return uniqueMatch(devices, (device) => device.isDefault)?.id ?? devices[0]?.id ?? '';
}

export function preferredProcessedDestination(devices: AudioDevice[]): string {
  return uniqueMatch(devices, (device) => device.isLikelyVirtual)?.id ?? '';
}

export function reconcileSelection(selected: string, devices: AudioDevice[]): string {
  return uniqueMatch(devices, (device) => device.id === selected)?.id ?? preferredDevice(devices);
}

export function resolveStoredSelection(
  storedId: string | null,
  storedFriendlyName: string | null,
  devices: AudioDevice[],
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

  const fallback = preferredDevice(devices);
  return fallback ? { id: fallback, source: 'fallback' } : { id: '', source: 'unavailable' };
}
