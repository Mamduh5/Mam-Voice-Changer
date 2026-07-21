import { describe, expect, it } from 'vitest';
import type { AudioDevice } from '../types/audio';
import {
  preferredDevice,
  preferredProcessedDestination,
  reconcileSelection,
  resolveStoredSelection,
} from './deviceSelection';

const metadata = {
  direction: 'output' as const,
  virtualFamily: null,
  minimumSampleRate: 44_100,
  maximumSampleRate: 48_000,
  commonSampleRates: [44_100, 48_000],
  channelCounts: [2],
};

const devices: AudioDevice[] = [
  { ...metadata, id: 'speakers', name: 'Speakers', isDefault: true, isLikelyVirtual: false },
  {
    ...metadata,
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
      {
        ...metadata,
        id: 'first-usb',
        name: 'USB Microphone',
        isDefault: false,
        isLikelyVirtual: false,
      },
      {
        ...metadata,
        id: 'second-usb',
        name: 'USB Microphone',
        isDefault: false,
        isLikelyVirtual: false,
      },
      {
        ...metadata,
        id: 'default-mic',
        name: 'Built-in Microphone',
        isDefault: true,
        isLikelyVirtual: false,
      },
    ];

    expect(resolveStoredSelection('missing', 'USB Microphone', duplicateNames)).toEqual({
      id: 'default-mic',
      source: 'fallback',
    });
  });

  it('does not keep a duplicated derived identifier as a confirmed selection', () => {
    const duplicateIds: AudioDevice[] = [
      {
        ...metadata,
        id: 'same-id',
        name: 'USB Microphone',
        isDefault: false,
        isLikelyVirtual: false,
      },
      {
        ...metadata,
        id: 'same-id',
        name: 'USB Microphone',
        isDefault: false,
        isLikelyVirtual: false,
      },
      {
        ...metadata,
        id: 'default-mic',
        name: 'Built-in Microphone',
        isDefault: true,
        isLikelyVirtual: false,
      },
    ];

    expect(reconcileSelection('same-id', duplicateIds)).toBe('default-mic');
  });
});
