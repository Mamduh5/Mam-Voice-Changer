import { describe, expect, it } from 'vitest';
import type { AudioDevice } from '../types/audio';
import { preferredDevice, reconcileSelection } from './deviceSelection';

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
});
