import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it, vi } from 'vitest';
import type { useVoiceDataset } from '../../hooks/useVoiceDataset';
import type { useVoiceModels } from '../../hooks/useVoiceModels';
import type { useVoiceProfiles } from '../../hooks/useVoiceProfiles';
import { emptyVoiceDatasetStatus, type VoiceDatasetManifest } from '../../types/voiceDataset';
import { emptyVoiceModelStatus, type VoiceModelArtifact } from '../../types/voiceModel';
import type { QualificationRun } from '../../types/modelBackend';
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
  compatibilityProfileId: 'seed-vc-experimental-v1',
  environmentFingerprint: null,
  checkpointIdentities: [],
  backendRevision: null,
  adapterVersion: 'mam-seed-vc-adapter-v2-experimental',
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
  modelFiles: [
    {
      relativePath: 'model/file-000.pth',
      contentHash: 'hash',
      sizeBytes: 10,
      role: 'modelWeights',
      licensingStatus: 'unknown',
    },
  ],
  modelContentHash: 'hash',
  expectedInferenceSampleRate: 48_000,
  supportedInferenceControls: ['diffusionSteps'],
  portabilityStatus: 'portableWithExternalDependencies',
  qualificationLevel: 'backendLoaded',
  licenseNotices: [
    {
      role: 'baseCheckpoint',
      label: 'Base checkpoint',
      status: 'unknown',
      notice: 'Redistribution permission has not been verified for this file.',
    },
  ],
  syntheticUseNoticeVersion: 'mam-synthetic-use-v1',
  health: 'unqualified',
  importedPackageId: null,
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
  compatibilityProfileId: 'seed-vc-experimental-v1',
  environmentFingerprint: null,
  checkpointIdentities: [],
  backendRevision: null,
  adapterVersion: 'mam-seed-vc-adapter-v2-experimental',
  qualificationLevel: 'backendLoaded',
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
  lastCheckpointHash: 'hash',
  logFile: 'worker.log',
  errorSummary: null,
  cancellationRequested: false,
  warnings: ['CPU-only training may be extremely slow.'],
};

const qualification: QualificationRun = {
  schemaVersion: 1,
  qualificationId: 'qualification-1',
  compatibilityProfileId: 'seed-vc-experimental-v1',
  compatibilityProfileStatus: 'experimental',
  startedAt: '1',
  endedAt: '2',
  state: 'qualifiedWithWarnings',
  completedChecks: [
    {
      code: 'package:torch',
      label: 'Python package torch',
      layer: 'worker',
      status: 'failed',
      message: 'torch version is missing.',
    },
    {
      code: 'audioPreprocess',
      label: 'Project fixture WAV preprocessing',
      layer: 'audio',
      status: 'passed',
      message: 'Project-generated WAV passed.',
    },
  ],
  warnings: ['The backend checkout is dirty.'],
  failures: ['A required Python package is missing.'],
  environmentFingerprint: {
    schemaVersion: 1,
    fingerprintId: 'fingerprint-1',
    generatedAt: '1',
    operatingSystem: 'windows',
    architecture: 'x86_64',
    python: { implementation: 'CPython', version: '3.10.1', executableLabel: 'python.exe' },
    worker: {
      workerVersion: '0.2.0',
      adapterVersion: 'mam-seed-vc-adapter-v2-experimental',
      protocolVersion: 1,
    },
    backend: {
      backendId: 'seed-vc-local',
      compatibilityProfileId: 'seed-vc-experimental-v1',
      repositoryRemote: 'https://example.test/repo',
      commitSha: 'a'.repeat(40),
      checkoutCleanliness: 'dirty',
    },
    packages: [{ package: 'torch', version: null, required: true, compatible: null }],
    accelerator: {
      cudaAvailable: false,
      cudaRuntimeVersion: null,
      gpuName: null,
      gpuCount: 0,
      totalVramBytes: null,
      availableVramBytes: null,
      selectedDevice: 'cpu',
      selectedPrecision: 'float32',
    },
    checkpoints: [
      {
        role: 'baseModel',
        displayPath: 'base.pth',
        sizeBytes: 10,
        contentHash: 'b'.repeat(64),
        hashAlgorithm: 'sha256',
        expectedHash: null,
        validationState: 'identityUnspecified',
        checkedAt: '1',
      },
    ],
    configurationFiles: [],
    aggregateHash: 'c'.repeat(64),
  },
  repository: {
    checkoutLabel: 'seed-vc',
    gitDirectoryPresent: true,
    gitAvailable: true,
    remoteIdentity: 'https://example.test/repo',
    commitSha: 'a'.repeat(40),
    detachedHead: true,
    cleanliness: 'dirty',
    trackedChanges: 1,
    untrackedAdapterFiles: 1,
    warnings: ['Dirty checkout'],
  },
  resources: {
    logicalCpuCount: 8,
    totalMemoryBytes: 16_000_000_000,
    availableMemoryBytes: 4_000_000_000,
    processMemoryBytes: 100_000_000,
    freeDiskBytes: 2_000_000_000,
    snapshotSizeBytes: 1000,
    checkpointSizeBytes: 10,
    estimatedTemporaryBytes: 20,
    totalVramBytes: null,
    availableVramBytes: null,
    riskLevel: 'high',
    reasons: ['cpuOnlyTraining', 'unavailableVramMeasurement'],
  },
  finalLevel: 'backendLoaded',
  manualListening: {
    syntheticOutputPlayed: false,
    speechIntelligible: false,
    noSevereClipping: false,
    noSevereTruncation: false,
    noSourceTargetMixUp: false,
    syntheticLabelReviewed: false,
    notes: null,
    confirmedAt: null,
  },
  inferenceSmokeResult: null,
  applicationVersion: '0.1.0',
  adapterVersion: 'mam-seed-vc-adapter-v2-experimental',
};

const action = vi.fn(async () => null);
function dataset(selectedManifest: VoiceDatasetManifest | null = manifest) {
  return {
    prompts: null,
    status: {
      ...emptyVoiceDatasetStatus,
      currentProfileId: selectedManifest?.profile.id ?? null,
      manifest: selectedManifest,
    },
    busy: false,
    error: null,
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
  } as unknown as ReturnType<typeof useVoiceDataset>;
}

function profiles(selectedManifest: VoiceDatasetManifest | null = manifest) {
  const summary = selectedManifest
    ? { profile: selectedManifest.profile, health: 'healthy' as const, managedStorageBytes: 100 }
    : null;
  return {
    profiles: summary ? [summary] : [],
    selectedProfileId: selectedManifest?.profile.id ?? null,
    selectedSummary: summary,
    status: selectedManifest
      ? {
          ...emptyVoiceDatasetStatus,
          currentProfileId: selectedManifest.profile.id,
          manifest: selectedManifest,
        }
      : emptyVoiceDatasetStatus,
    manifest: selectedManifest,
    consentActive: Boolean(selectedManifest),
    datasetSummary: selectedManifest?.statistics ?? null,
    modelSummary: { snapshots: 1, artifacts: 1, activeTraining: false },
    busy: false,
    error: null,
    selectProfile: vi.fn(async () => true),
  } as unknown as ReturnType<typeof useVoiceProfiles>;
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
        compatibilityProfileId: 'seed-vc-experimental-v1',
        pythonExecutable: 'selected',
        workerPackageDirectory: 'selected',
        seedVcDirectory: 'selected',
        modelConfigurationPath: 'selected',
        modelConfigurationExpectedSha256: null,
        pretrainedCheckpointPaths: ['selected'],
        pretrainedCheckpointExpectedSha256: [],
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
    compatibilityProfiles: [
      {
        schemaVersion: 1,
        profileId: 'seed-vc-experimental-v1',
        backendId: 'seed-vc-local',
        displayName: 'Seed-VC local (experimental, revision unpinned)',
        supportStatus: 'experimental',
        repositoryIdentity: {
          provider: 'git',
          owner: 'Plachtaa',
          name: 'seed-vc',
          canonicalRemote: 'https://github.com/Plachtaa/seed-vc',
        },
        supportedCommitShas: [],
        workerAdapterVersion: 'mam-seed-vc-adapter-v2-experimental',
        protocolVersion: 1,
        pythonRequirement: { minimumInclusive: '3.10.0', maximumExclusive: '3.12.0' },
        packageRequirements: [],
        expectedFiles: [],
        configurationFiles: [],
        checkpointRoles: [],
        supportedDevices: ['cpu'],
        supportedPrecisions: ['float32'],
        capabilities: {
          training: true,
          resume: true,
          offlineInference: true,
          multipleReferences: false,
          checkpointInspection: true,
        },
        notices: [],
      },
    ],
    refresh: vi.fn(async () => status),
    saveSettings: vi.fn(async () => true),
    validateBackend: action,
    repairIndexes: action,
    runQualification: action,
    loadQualificationSmoke: action,
    cancelQualification: action,
    confirmManualListening: action,
    saveQualificationReport: action,
    createSnapshot: action,
    deleteSnapshot: action,
    startTraining: action,
    createTrainingPreflight: action,
    cancelTraining: action,
    resumeTraining: action,
    deleteJob: action,
    renameArtifact: action,
    approveArtifact: action,
    rejectArtifact: action,
    deleteArtifact: action,
    exportArtifact: action,
    importArtifact: action,
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
        profiles={profiles()}
        models={models({ activeTrainingJob: trainingJob, logs: ['backend log'] })}
        hasVoiceLabSource
        disabled={false}
        onOpenProfiles={vi.fn()}
      />,
    );
    for (const label of [
      'Voice Models',
      'Synthetic voice output',
      'Create training snapshot',
      'Configure local model backend',
      'Check worker handshake',
      'Backend Qualification',
      'Run layered qualification',
      'Optional inference smoke reference',
      'Quick experiment',
      'Start local fine-tuning',
      '50% overall',
      'Latest checkpoint',
      'Cancel training',
      'Versioned local model artifacts',
      'Model unevaluated',
      'Convert test phrase',
      'Manual model evaluation',
      'Model portability',
      'Export model package',
      'Approve for offline Voice Lab',
      'No realtime model conversion was added',
    ])
      expect(markup).toContain(label);
    expect(markup).not.toContain('Perfect clone');
    expect(markup).not.toContain('Clone instantly');
    expect(markup).not.toContain('arbitrary command');
    expect(markup).not.toContain('Route model to Discord');
    expect(markup).not.toContain('samples:[');
    expect(markup).toContain('Shared voice profile');
    expect(markup).toContain('Open Profiles');
    expect(markup).toContain('View compatibility matrix');
    expect(markup).toContain('View full worker logs');
    expect(markup).not.toContain('Create profile');
    expect(markup).not.toContain('Save profile');
    expect(markup).not.toContain('Delete profile');
  });

  it('shows no-profile and consent-disabled states precisely', () => {
    const emptyMarkup = renderToStaticMarkup(
      <VoiceModelPage
        dataset={dataset(null)}
        profiles={profiles(null)}
        models={models()}
        hasVoiceLabSource={false}
        disabled={false}
        onOpenProfiles={vi.fn()}
      />,
    );
    expect(emptyMarkup).toContain('Select a Dataset profile');
    expect(emptyMarkup).toContain('Record or import a Voice Lab source clip first');

    const disabledArtifact = { ...artifact, approvalStatus: 'disabledByConsent' as const };
    const disabledMarkup = renderToStaticMarkup(
      <VoiceModelPage
        dataset={dataset()}
        profiles={profiles()}
        models={models({ artifacts: [disabledArtifact] })}
        hasVoiceLabSource
        disabled={false}
        onOpenProfiles={vi.fn()}
      />,
    );
    expect(disabledMarkup).toContain('Disabled by consent');
    expect(disabledMarkup).toMatch(
      /<button type="button" class="start" disabled="">Convert test phrase/,
    );
  });

  it('renders qualification depth, dirty checkout, package mismatch, hashes, resources, and pending listening honestly', () => {
    const markup = renderToStaticMarkup(
      <VoiceModelPage
        dataset={dataset()}
        profiles={profiles()}
        models={models({ qualification })}
        hasVoiceLabSource
        disabled={false}
        onOpenProfiles={vi.fn()}
      />,
    );
    for (const label of [
      'qualifiedWithWarnings / backendLoaded',
      'The backend checkout is dirty.',
      'torch version is missing.',
      'identityUnspecified',
      'Resource risk: high',
      'Optional inference smoke test: pending',
      'Confirm manual listening gate',
      'No automatic downloads are permitted',
      'Import an untrusted package',
    ])
      expect(markup).toContain(label);
    expect(markup).not.toContain('Route model to Discord');
    expect(markup).not.toContain('Automatic download');
    expect(markup).not.toContain('Command arguments');
  });

  it('keeps imported artifacts unapproved and consent-dependent', () => {
    const imported = {
      ...artifact,
      importedPackageId: 'package-1',
      approvalStatus: 'unevaluated' as const,
      health: 'unqualified' as const,
    };
    const markup = renderToStaticMarkup(
      <VoiceModelPage
        dataset={dataset()}
        profiles={profiles()}
        models={models({ artifacts: [imported], qualification })}
        hasVoiceLabSource
        disabled={false}
        onOpenProfiles={vi.fn()}
      />,
    );
    expect(markup).toContain('Model unevaluated');
    expect(markup).toContain('associated with this consent-active profile by opaque ID');
    expect(markup).toContain('Imported models remain unevaluated and unapproved');
  });

  it('exposes generated qualification audio only through the isolated Voice Lab path', () => {
    const generated = {
      ...qualification,
      finalLevel: 'inferenceGenerated' as const,
      inferenceSmokeResult: {
        synthetic: true,
        outputFile: 'synthetic-smoke.wav',
        durationMs: 1000,
        peak: 0.25,
        clipping: false,
      },
    };
    const markup = renderToStaticMarkup(
      <VoiceModelPage
        dataset={dataset()}
        profiles={profiles()}
        models={models({ qualification: generated })}
        hasVoiceLabSource
        disabled={false}
        onOpenProfiles={vi.fn()}
      />,
    );
    expect(markup).toContain('Load synthetic smoke into Voice Lab');
    expect(markup).toContain('clipping not detected');
    expect(markup).not.toContain('Route model to Discord');
  });

  it('filters snapshots and artifacts when the one shared profile changes', () => {
    const otherManifest = {
      ...manifest,
      profile: { ...manifest.profile, id: 'profile-2', displayName: 'Second speaker' },
    };
    const markup = renderToStaticMarkup(
      <VoiceModelPage
        dataset={dataset(otherManifest)}
        profiles={profiles(otherManifest)}
        models={models()}
        hasVoiceLabSource
        disabled={false}
        onOpenProfiles={vi.fn()}
      />,
    );
    expect(markup).toContain('Second speaker');
    expect(markup).toContain('No immutable training snapshot yet.');
    expect(markup).toContain('No model artifact exists for this profile.');
    expect(markup).not.toContain('Synthetic model one');
  });
});
