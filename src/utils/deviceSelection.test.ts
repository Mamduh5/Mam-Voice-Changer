import { describe, expect, it } from 'vitest';
import type { AudioDevice } from '../types/audio';
import { preferredDevice, reconcileSelection, resolveStoredSelection } from './deviceSelection';

const devices: AudioDevice[] = [
  { id: 'speakers', name: 'Speakers', isDefault: true },
  { id: 'cable', name: 'CABLE Input (VB-Audio Virtual Cable)', isDefault: false },
];

describe('device selection', () => {
  it('keeps a selected device that is still present', () => {
    expect(reconcileSelection('speakers', devices, true)).toBe('speakers');
  });

  it('prefers CABLE Input for a missing output selection', () => {
    expect(reconcileSelection('missing', devices, true)).toBe('cable');
  });

  it('falls back to the Windows default when cable preference is disabled', () => {
    expect(preferredDevice(devices)).toBe('speakers');
  });

  it('restores a uniquely matching friendly name when the saved identifier changed', () => {
    expect(resolveStoredSelection('old-id', '  speakers ', devices)).toEqual({
      id: 'speakers',
      source: 'friendlyName',
    });
  });

  it('does not claim an ambiguous friendly-name match', () => {
    const duplicateNames: AudioDevice[] = [
      { id: 'first-usb', name: 'USB Microphone', isDefault: false },
      { id: 'second-usb', name: 'USB Microphone', isDefault: false },
      { id: 'default-mic', name: 'Built-in Microphone', isDefault: true },
    ];

    expect(resolveStoredSelection('missing', 'USB Microphone', duplicateNames)).toEqual({
      id: 'default-mic',
      source: 'fallback',
    });
  });

  it('does not keep a duplicated derived identifier as a confirmed selection', () => {
    const duplicateIds: AudioDevice[] = [
      { id: 'same-id', name: 'USB Microphone', isDefault: false },
      { id: 'same-id', name: 'USB Microphone', isDefault: false },
      { id: 'default-mic', name: 'Built-in Microphone', isDefault: true },
    ];

    expect(reconcileSelection('same-id', duplicateIds)).toBe('default-mic');
  });
});
