import { describe, expect, it } from 'vitest';
import type { AudioDevice } from '../types/audio';
import {
  preferredDevice,
  preferredProcessedDestination,
  reconcileSelection,
  resolveStoredSelection,
} from './deviceSelection';

const devices: AudioDevice[] = [
  { id: 'speakers', name: 'Speakers', isDefault: true, isLikelyVirtual: false },
  {
    id: 'cable',
    name: 'CABLE Input (VB-Audio Virtual Cable)',
    isDefault: false,
    isLikelyVirtual: true,
  },
];

describe('device selection', () => {
  it('keeps a selected device that is still present', () => {
    expect(reconcileSelection('speakers', devices)).toBe('speakers');
  });

  it('does not treat physical fallback as a processed destination', () => {
    expect(preferredProcessedDestination(devices)).toBe('cable');
    expect(preferredProcessedDestination([devices[0]])).toBe('');
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
      { id: 'first-usb', name: 'USB Microphone', isDefault: false, isLikelyVirtual: false },
      { id: 'second-usb', name: 'USB Microphone', isDefault: false, isLikelyVirtual: false },
      { id: 'default-mic', name: 'Built-in Microphone', isDefault: true, isLikelyVirtual: false },
    ];

    expect(resolveStoredSelection('missing', 'USB Microphone', duplicateNames)).toEqual({
      id: 'default-mic',
      source: 'fallback',
    });
  });

  it('does not keep a duplicated derived identifier as a confirmed selection', () => {
    const duplicateIds: AudioDevice[] = [
      { id: 'same-id', name: 'USB Microphone', isDefault: false, isLikelyVirtual: false },
      { id: 'same-id', name: 'USB Microphone', isDefault: false, isLikelyVirtual: false },
      { id: 'default-mic', name: 'Built-in Microphone', isDefault: true, isLikelyVirtual: false },
    ];

    expect(reconcileSelection('same-id', duplicateIds)).toBe('default-mic');
  });
});
