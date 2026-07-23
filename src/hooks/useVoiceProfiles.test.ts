import { describe, expect, it } from 'vitest';
import type { VoiceProfileSummary } from '../types/voiceProfile';
import { clearDeletedProfileSelection, validateSelectedProfileId } from './useVoiceProfiles';

function summary(id: string, health: VoiceProfileSummary['health'] = 'healthy') {
  return {
    profile: {
      id,
      displayName: id,
      description: null,
      primaryLanguage: 'English',
      localeTag: 'en-US',
      collectionGoalMinutes: 10,
      createdAt: '1',
      updatedAt: '1',
    },
    health,
    managedStorageBytes: 0,
  };
}

describe('shared voice-profile selection', () => {
  it('restores only an existing non-corrupt opaque profile ID', () => {
    const profiles = [
      summary('profile-healthy'),
      summary('profile-corrupt', 'corruptManifest'),
      summary('profile-future', 'unsupportedSchema'),
    ];
    expect(validateSelectedProfileId(profiles, 'profile-healthy')).toBe('profile-healthy');
    expect(validateSelectedProfileId(profiles, 'profile-missing')).toBeNull();
    expect(validateSelectedProfileId(profiles, 'profile-corrupt')).toBeNull();
    expect(validateSelectedProfileId(profiles, 'profile-future')).toBeNull();
  });

  it('clears the one shared selection when that profile is deleted', () => {
    expect(clearDeletedProfileSelection('profile-a', 'profile-a')).toBeNull();
    expect(clearDeletedProfileSelection('profile-b', 'profile-a')).toBe('profile-b');
  });
});
