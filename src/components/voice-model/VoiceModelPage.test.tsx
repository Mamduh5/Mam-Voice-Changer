import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it, vi } from 'vitest';
import type { useVoiceDataset } from '../../hooks/useVoiceDataset';
import type { useVoiceModels } from '../../hooks/useVoiceModels';
import { emptyVoiceDatasetStatus, type VoiceDatasetManifest } from '../../types/voiceDataset';
import { emptyVoiceModelStatus, type VoiceModelArtifact } from '../../types/voiceModel';
import { trainingPresets, type TrainingJob } from '../../types/trainingJob';
import { VoiceModelPage } from './VoiceModelPage';

const manifest: VoiceDatasetManifest = {
  schemaVersion: 1,
  profile: {
    id: 'profile-1',
    displayName: 'Consenting speaker',
    description: null,
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
    totalTakes: 12,
    acceptedTakes: 10,
    rejectedTakes: 1,
    pendingTakes: 1,
    warningTakes: 2,
    failedTakes: 1,
    acceptedDurationMs: 90_000,
    completedPrompts: 8,
    totalPrompts: 12,
    categoryCoverage: { neutralStatement: 3, question: 2, plosives: 2, sibilants: 1 },
    customTakes: 0,
    excludedTakes: 1,
  },
  createdAt: '1',
  updatedAt: '1',
};

const artifact: VoiceModelArtifact = {
  schemaVersion: 1,
  artifactId: 'artifact-1',
  profileId: 'profile-1',
  displayName: 'Synthetic model one',
  backendId: 'seed-vc-local',
  backendVersion: 'configured',
  workerProtocolVersion: 1,
  snapshotId: 'snapshot-1',
  snapshotHash: 'snapshot-hash',
  consentVersion: 'voice-dataset-consent-v1',
  consentConfirmedAt: '1',
  trainingConfiguration: trainingPresets.quickExperiment,
  trainingSummary: {
    completedSteps: 100,
    finalTrainingLoss: null,
    finalValidationLoss: null,
    checkpointCount: 1,
    durationMs: 1000,
    warnings: [],
  },
  modelFiles: [{ relativePath: 'model/file-000.pth', contentHash: 'hash', sizeBytes: 10 }],
  modelContentHash: 'hash',
  evaluation: null,
  approvalStatus: 'unevaluated',
  notes: null,
  createdAt: '1',
  updatedAt: '1',
};

const trainingJob: TrainingJob = {
  schemaVersion: 1,
  jobId: 'job-1',
  backendId: 'seed-vc-local',
  backendVersion: 'configured',
  workerProtocolVersion: 1,
  snapshotId: 'snapshot-1',
  snapshotHash: 'snapshot-hash',
  profileId: 'profile-1',
  consentVersion: 'voice-dataset-consent-v1',
  configuration: trainingPresets.quickExperiment,
  state: 'training',
  overallProgress: 0.5,
  currentStep: 50,
  maximumSteps: 100,
  latestMetrics: {
    trainingLoss: 0.2,
    validationLoss: 0.3,
    learningRate: 0.0001,
    backendReported: true,
    additional: {},
  },
  startedAt: '1',
  updatedAt: '2',
  completedAt: null,
  workerPid: 123,
  lastCheckpoint: 'runs/checkpoint.pth',
  logFile: 'worker.log',
  errorSummary: null,
  cancellationRequested: false,
  warnings: ['CPU-only training may be extremely slow.'],
};

const action = vi.fn(async () => null);
function dataset(selectedManifest: VoiceDatasetManifest | null = manifest) {
  return {
    profiles: [{ profile: manifest.profile, health: 'healthy' as const, managedStorageBytes: 100 }],
    prompts: null,
    status: {
      ...emptyVoiceDatasetStatus,
      currentProfileId: selectedManifest?.profile.id ?? null,
      manifest: selectedManifest,
    },
    busy: false,
    error: null,
    createProfile: action,
    selectProfile: vi.fn(async () => true),
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
  } as unknown as ReturnType<typeof useVoiceDataset>;
}

function models(overrides: Partial<ReturnType<typeof useVoiceModels>['status']> = {}) {
  const status = {
    ...emptyVoiceModelStatus,
    backend: {
      readiness: 'ready' as const,
      message: 'The optional local model backend is ready.',
      capabilityReport: null,
    },
    snapshots: [
      {
        schemaVersion: 1,
        snapshotId: 'snapshot-1',
        contentHash: 'snapshot-hash',
        profileId: 'profile-1',
        datasetSchemaVersion: 1,
        consentVersion: 'voice-dataset-consent-v1',
        consentConfirmedAt: '1',
        promptPackId: 'mam-english-core',
        promptPackVersion: 1,
        canonicalSampleRate: 48_000,
        canonicalChannels: 1,
        totalDurationMs: 90_000,
        takes: [],
        split: {
          seed: 13,
          trainingTakeCount: 8,
          validationTakeCount: 2,
          trainingDurationMs: 72_000,
          validationDurationMs: 18_000,
        },
        warnings: ['This is a small Dataset; training quality is not guaranteed.'],
        createdAt: '1',
      },
    ],
    artifacts: [artifact],
    ...overrides,
  };
  return {
    status,
    settings: {
      schemaVersion: 1,
      seedVc: {
        pythonExecutable: 'selected',
        workerPackageDirectory: 'selected',
        seedVcDirectory: 'selected',
        modelConfigurationPath: 'selected',
        pretrainedCheckpointPaths: ['selected'],
        outputDirectory: 'selected',
        device: 'cpu' as const,
        precision: 'float32' as const,
      },
    },
    busy: false,
    error: null,
    evaluationPhrases: [
      { phraseId: 'neutral', category: 'Neutral', text: 'The quiet room is easy to hear.' },
    ],
    refresh: vi.fn(async () => status),
    saveSettings: vi.fn(async () => true),
    validateBackend: action,
    createSnapshot: action,
    deleteSnapshot: action,
    startTraining: action,
    cancelTraining: action,
    resumeTraining: action,
    deleteJob: action,
    renameArtifact: action,
    approveArtifact: action,
    rejectArtifact: action,
    deleteArtifact: action,
    startConversion: action,
    startEvaluationConversion: action,
    cancelConversion: action,
    loadConversion: action,
    clearConversion: action,
    saveEvaluation: action,
  } as unknown as ReturnType<typeof useVoiceModels>;
}

describe('Voice Models workspace', () => {
  it('renders the complete offline workflow without arbitrary commands or realtime routing', () => {
    const markup = renderToStaticMarkup(
      <VoiceModelPage
        dataset={dataset()}
        models={models({ activeTrainingJob: trainingJob, logs: ['backend log'] })}
        hasVoiceLabSource
        disabled={false}
      />,
    );
    for (const label of [
      'Voice Models',
      'Synthetic voice output',
      'Create training snapshot',
      'Configure local model backend',
      'Validate backend',
      'Quick experiment',
      'Start local fine-tuning',
      '50% overall',
      'Latest checkpoint',
      'Cancel training',
      'Versioned local model artifacts',
      'Model unevaluated',
      'Convert test phrase',
      'Manual model evaluation',
      'Approve for offline Voice Lab',
      'No realtime model conversion was added',
    ])
      expect(markup).toContain(label);
    expect(markup).not.toContain('Perfect clone');
    expect(markup).not.toContain('Clone instantly');
    expect(markup).not.toContain('arbitrary command');
    expect(markup).not.toContain('Route model to Discord');
    expect(markup).not.toContain('samples:[');
  });

  it('shows no-profile and consent-disabled states precisely', () => {
    const emptyMarkup = renderToStaticMarkup(
      <VoiceModelPage
        dataset={dataset(null)}
        models={models()}
        hasVoiceLabSource={false}
        disabled={false}
      />,
    );
    expect(emptyMarkup).toContain('Select a Dataset profile');
    expect(emptyMarkup).toContain('Record or import a Voice Lab source clip first');

    const disabledArtifact = { ...artifact, approvalStatus: 'disabledByConsent' as const };
    const disabledMarkup = renderToStaticMarkup(
      <VoiceModelPage
        dataset={dataset()}
        models={models({ artifacts: [disabledArtifact] })}
        hasVoiceLabSource
        disabled={false}
      />,
    );
    expect(disabledMarkup).toContain('Disabled by consent');
    expect(disabledMarkup).toMatch(
      /<button type="button" class="start" disabled="">Convert test phrase/,
    );
  });
});
