import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it, vi } from 'vitest';
import type { useVoiceProfiles } from '../../hooks/useVoiceProfiles';
import { emptyVoiceDatasetStatus, type VoiceDatasetManifest } from '../../types/voiceDataset';
import { VoiceProfilesPage } from './VoiceProfilesPage';

const manifest: VoiceDatasetManifest = {
  schemaVersion: 1,
  profile: {
    id: 'profile-opaque-1',
    displayName: 'Consenting speaker',
    description: 'Local profile',
    primaryLanguage: 'English',
    localeTag: 'en-US',
    collectionGoalMinutes: 10,
    createdAt: '1',
    updatedAt: '1',
  },
  consent: {
    consentConfirmed: true,
    consentVersion: 'voice-dataset-consent-v1',
    confirmedAt: '1',
    confirmedByUser: true,
    recordedConsentTakeId: null,
    revokedAt: null,
    notes: null,
  },
  recordingFormat: {
    container: 'wav',
    sampleFormat: 'pcm',
    sampleRate: 48_000,
    channels: 1,
    bitsPerSample: 24,
  },
  promptPack: { id: 'mam-english-core', version: 1 },
  takes: [],
  statistics: {
    totalTakes: 4,
    acceptedTakes: 3,
    rejectedTakes: 0,
    pendingTakes: 1,
    warningTakes: 0,
    failedTakes: 0,
    acceptedDurationMs: 60_000,
    completedPrompts: 3,
    totalPrompts: 12,
    categoryCoverage: {},
    customTakes: 0,
    excludedTakes: 0,
  },
  createdAt: '1',
  updatedAt: '1',
};

const action = vi.fn(async () => true);
function profiles(selected = true) {
  const summary = {
    profile: manifest.profile,
    health: 'healthy' as const,
    managedStorageBytes: 1024,
  };
  return {
    profiles: [summary],
    selectedProfileId: selected ? manifest.profile.id : null,
    selectedSummary: selected ? summary : null,
    status: selected
      ? { ...emptyVoiceDatasetStatus, currentProfileId: manifest.profile.id, manifest }
      : emptyVoiceDatasetStatus,
    manifest: selected ? manifest : null,
    consentActive: selected,
    datasetSummary: selected ? manifest.statistics : null,
    modelSummary: { snapshots: 2, artifacts: 1, activeTraining: false },
    busy: false,
    error: null,
    selectProfile: action,
    createProfile: action,
    updateProfile: action,
    repairProfile: action,
    deleteProfile: action,
    exportDataset: action,
  } as unknown as ReturnType<typeof useVoiceProfiles>;
}

describe('Voice Profiles workspace', () => {
  it('owns profile list, editing, health, summaries, export, repair, and deletion', () => {
    const markup = renderToStaticMarkup(<VoiceProfilesPage profiles={profiles()} />);
    for (const label of [
      'Voice Profiles',
      'Search profiles',
      'Consenting speaker',
      'Profile details',
      'Consent active',
      'Accepted Dataset',
      'Snapshots',
      'Models',
      'Save profile',
      'Repair profile',
      'Export Dataset',
      'Delete profile',
      'Storage and dependency details',
    ])
      expect(markup).toContain(label);
    expect(markup).not.toContain('Record phrase');
    expect(markup).not.toContain('Start local fine-tuning');
  });

  it('offers consent-gated profile creation only in Profiles', () => {
    const markup = renderToStaticMarkup(<VoiceProfilesPage profiles={profiles(false)} />);
    expect(markup).toContain('Create profile');
    expect(markup).toContain('Select or create a voice profile');
  });
});
