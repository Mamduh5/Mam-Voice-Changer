import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it, vi } from 'vitest';
import type { ComponentProps } from 'react';
import type { AudioDevice } from '../../types/audio';
import {
  emptyVoiceDatasetStatus,
  type DatasetTake,
  type VoiceDatasetManifest,
  type VoiceDatasetStatus,
} from '../../types/voiceDataset';
import { VoiceDatasetPage } from './VoiceDatasetPage';

const input: AudioDevice = {
  id: 'mic',
  name: 'Physical microphone',
  direction: 'input',
  isDefault: true,
  isLikelyVirtual: false,
  virtualFamily: null,
  minimumSampleRate: 44_100,
  maximumSampleRate: 48_000,
  commonSampleRates: [44_100, 48_000],
  channelCounts: [1, 2],
};
const output: AudioDevice = { ...input, id: 'headphones', name: 'Headphones', direction: 'output' };
const action = vi.fn(async () => true);

const take: DatasetTake = {
  id: 'take-1',
  promptId: 'en-neutral-01',
  promptText: 'The window is open to the morning air.',
  promptCategory: 'neutralStatement',
  source: 'recorded',
  rawFile: 'raw/take-1.wav',
  derivedFile: null,
  selectedVersion: 'raw',
  sampleRate: 48_000,
  channels: 1,
  frameCount: 48_000,
  durationMs: 1_000,
  waveformEnvelope: [{ minimum: -0.2, maximum: 0.3 }],
  quality: {
    classification: 'warning',
    reasons: [{ code: 'levelTooLow', guidance: 'Move closer to the microphone.' }],
    durationMs: 1_000,
    peakAmplitude: 0.3,
    rmsLevel: 0.08,
    clippedSampleCount: 0,
    clippedSampleRatio: 0,
    dcOffset: 0,
    leadingSilenceMs: 100,
    trailingSilenceMs: 100,
    totalLowEnergyRatio: 0.1,
    estimatedActiveSpeechRatio: 0.9,
    estimatedBackgroundNoiseFloor: 0.002,
    heuristicSignalToNoiseDb: 20,
    consecutiveZeroRegions: 0,
    recordingQueueOverflowCount: 0,
    droppedFrames: 0,
    callbackGaps: 0,
    nonFiniteSampleCountBeforeSanitization: 0,
    sampleRate: 48_000,
    channels: 1,
  },
  trim: null,
  reviewStatus: 'pending',
  excludeFromTraining: false,
  notes: null,
  manualOverride: false,
  warningAcknowledged: false,
  createdAt: '1000',
  contentHash: 'abc',
};
const manifest: VoiceDatasetManifest = {
  schemaVersion: 1,
  profile: {
    id: 'profile-1',
    displayName: 'Consenting speaker',
    description: null,
    primaryLanguage: 'English',
    localeTag: 'en-US',
    collectionGoalMinutes: 10,
    createdAt: '1000',
    updatedAt: '1000',
  },
  consent: {
    consentConfirmed: true,
    consentVersion: 'voice-dataset-consent-v1',
    confirmedAt: '1000',
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
  takes: [take],
  statistics: {
    totalTakes: 1,
    acceptedTakes: 0,
    rejectedTakes: 0,
    pendingTakes: 1,
    warningTakes: 1,
    failedTakes: 0,
    acceptedDurationMs: 0,
    completedPrompts: 0,
    totalPrompts: 12,
    categoryCoverage: {},
    customTakes: 0,
    excludedTakes: 0,
  },
  createdAt: '1000',
  updatedAt: '1000',
};

type Dataset = ComponentProps<typeof VoiceDatasetPage>['dataset'];
function dataset(
  status: VoiceDatasetStatus = {
    ...emptyVoiceDatasetStatus,
    currentProfileId: 'profile-1',
    currentPromptId: 'en-neutral-01',
    currentPromptText: take.promptText,
    currentPromptCategory: 'neutralStatement' as const,
    manifest,
  },
): Dataset {
  return {
    profiles: [{ profile: manifest.profile, health: 'healthy', managedStorageBytes: 1234 }],
    prompts: {
      id: 'mam-english-core',
      version: 1,
      displayName: 'Mam English Core',
      language: 'English',
      prompts: [
        {
          id: 'en-neutral-01',
          text: take.promptText!,
          category: 'neutralStatement',
          recommendedTakeDurationMs: 6000,
        },
      ],
    },
    status,
    busy: false,
    error: null,
    createProfile: action,
    selectProfile: action,
    updateProfile: action,
    deleteProfile: action,
    selectPrompt: action,
    record: action,
    stopRecording: action,
    discardRecording: action,
    importWavs: action,
    reviewTake: action,
    autoTrim: action,
    setTrim: action,
    applyTrim: action,
    resetTrim: action,
    preview: action,
    pausePreview: action,
    stopPreview: action,
    deleteTake: action,
    exportDataset: action,
    repairProfile: action,
  };
}

describe('Voice Dataset workspace', () => {
  it('renders consent, recording, review, quality, progress, import, export, and deletion controls without cloning claims', () => {
    const markup = renderToStaticMarkup(
      <VoiceDatasetPage
        dataset={dataset()}
        inputs={[input]}
        outputs={[output]}
        defaultInputId={input.id}
        defaultOutputId={output.id}
        disabled={false}
        liveActive={false}
      />,
    );
    for (const label of [
      'Voice Dataset Capture',
      'Consent confirmed',
      'Collection progress',
      'Recording microphone',
      'Record phrase',
      'Input level',
      'Review required',
      'Take has warnings',
      'Accept take',
      'Reject take',
      'Redo take',
      'Auto-detect trim',
      'Raw',
      'Trimmed',
      'Import recordings',
      'Export dataset',
      'Delete take',
      'Delete profile and all recordings',
      'managed',
    ])
      expect(markup).toContain(label);
    expect(markup).toContain('does not clone a voice');
    expect(markup).not.toContain('Train model');
    expect(markup).not.toContain('clone ready');
    expect(markup).not.toContain('samples:[');
  });

  it('renders the empty profile and consent-required state', () => {
    const empty = {
      ...dataset(emptyVoiceDatasetStatus),
      profiles: [],
      status: emptyVoiceDatasetStatus,
    };
    const markup = renderToStaticMarkup(
      <VoiceDatasetPage
        dataset={empty}
        inputs={[]}
        outputs={[]}
        defaultInputId=""
        defaultOutputId=""
        disabled={false}
        liveActive={false}
      />,
    );
    expect(markup).toContain('No local voice profiles');
    expect(markup).toContain('Consent required');
    expect(markup).toContain('Create voice profile');
  });

  it('shows backend ownership blocking without requiring audio hardware', () => {
    const markup = renderToStaticMarkup(
      <VoiceDatasetPage
        dataset={dataset()}
        inputs={[input]}
        outputs={[output]}
        defaultInputId={input.id}
        defaultOutputId={output.id}
        disabled={false}
        liveActive
      />,
    );
    expect(markup).toContain('Audio device busy');
    expect(markup).toMatch(
      /<button type="button" class="start" disabled="">Record phrase<\/button>/,
    );
  });
});
