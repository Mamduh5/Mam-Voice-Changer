export type ModelDevice = 'cpu' | 'cuda' | 'directMl';
export type ModelPrecision = 'float32' | 'float16' | 'bfloat16';

export type SeedVcBackendConfiguration = {
  compatibilityProfileId: string;
  pythonExecutable: string;
  workerPackageDirectory: string;
  seedVcDirectory: string;
  modelConfigurationPath: string;
  modelConfigurationExpectedSha256: string | null;
  pretrainedCheckpointPaths: string[];
  pretrainedCheckpointExpectedSha256: string[];
  outputDirectory: string;
  device: ModelDevice;
  precision: ModelPrecision;
};

export type QualificationSupportStatus =
  'unknown' | 'experimental' | 'candidate' | 'qualified' | 'deprecated' | 'blocked';
export type BackendCompatibilityProfile = {
  schemaVersion: number;
  profileId: string;
  backendId: string;
  displayName: string;
  supportStatus: QualificationSupportStatus;
  repositoryIdentity: { provider: string; owner: string; name: string; canonicalRemote: string };
  supportedCommitShas: string[];
  workerAdapterVersion: string;
  protocolVersion: number;
  pythonRequirement: { minimumInclusive: string; maximumExclusive: string };
  packageRequirements: Array<{ package: string; requirement: string; required: boolean }>;
  expectedFiles: Array<{
    role: string;
    relativePath: string;
    required: boolean;
    expectedSha256: string | null;
  }>;
  configurationFiles: Array<{
    role: string;
    relativePath: string;
    required: boolean;
    expectedSha256: string | null;
  }>;
  checkpointRoles: Array<{
    role: string;
    displayName: string;
    required: boolean;
    redistributable: boolean;
  }>;
  supportedDevices: ModelDevice[];
  supportedPrecisions: ModelPrecision[];
  capabilities: {
    training: boolean;
    resume: boolean;
    offlineInference: boolean;
    multipleReferences: boolean;
    checkpointInspection: boolean;
  };
  notices: string[];
};

export type QualificationState =
  | 'notStarted'
  | 'collectingIdentity'
  | 'validatingFiles'
  | 'hashingCheckpoints'
  | 'startingWorker'
  | 'checkingProtocol'
  | 'inspectingPackages'
  | 'inspectingAccelerator'
  | 'runningImportSmokeTest'
  | 'runningAudioSmokeTest'
  | 'runningInferenceSmokeTest'
  | 'evaluatingResults'
  | 'qualified'
  | 'qualifiedWithWarnings'
  | 'failed'
  | 'cancelled'
  | 'interrupted';
export type QualificationLevel =
  | 'none'
  | 'configurationValidated'
  | 'backendLoaded'
  | 'inferenceGenerated'
  | 'manuallyListened'
  | 'trainingCompleted';
export type FileValidationState =
  | 'missing'
  | 'unreadable'
  | 'hashing'
  | 'hashKnown'
  | 'hashMismatch'
  | 'identityUnspecified'
  | 'valid'
  | 'unsupported';
export type FileFingerprint = {
  role: string;
  displayPath: string;
  sizeBytes: number;
  contentHash: string | null;
  hashAlgorithm: string;
  expectedHash: string | null;
  validationState: FileValidationState;
  checkedAt: string;
};
export type EnvironmentFingerprint = {
  schemaVersion: number;
  fingerprintId: string;
  generatedAt: string;
  operatingSystem: string;
  architecture: string;
  python: { implementation: string; version: string; executableLabel: string };
  worker: { workerVersion: string; adapterVersion: string; protocolVersion: number };
  backend: {
    backendId: string;
    compatibilityProfileId: string;
    repositoryRemote: string | null;
    commitSha: string | null;
    checkoutCleanliness: 'clean' | 'dirty' | 'unknown' | null;
  };
  packages: Array<{
    package: string;
    version: string | null;
    required: boolean;
    compatible: boolean | null;
  }>;
  accelerator: {
    cudaAvailable: boolean;
    cudaRuntimeVersion: string | null;
    gpuName: string | null;
    gpuCount: number;
    totalVramBytes: number | null;
    availableVramBytes: number | null;
    selectedDevice: ModelDevice | null;
    selectedPrecision: ModelPrecision | null;
  };
  checkpoints: FileFingerprint[];
  configurationFiles: FileFingerprint[];
  aggregateHash: string;
};
export type QualificationCheck = {
  code: string;
  label: string;
  layer:
    'static' | 'worker' | 'framework' | 'backendImport' | 'audio' | 'inference' | 'manualListening';
  status: 'passed' | 'passedWithWarning' | 'failed' | 'pending' | 'skipped';
  message: string;
};
export type ResourceRiskReason =
  | 'cpuOnlyTraining'
  | 'insufficientDisk'
  | 'lowSystemMemory'
  | 'lowVram'
  | 'unavailableVramMeasurement'
  | 'unsupportedPrecision'
  | 'oversizedBatch'
  | 'excessiveWorkers'
  | 'largeTrainingStepCount'
  | 'tinyDataset';
export type ResourceDiagnostics = {
  logicalCpuCount: number | null;
  totalMemoryBytes: number | null;
  availableMemoryBytes: number | null;
  processMemoryBytes: number | null;
  freeDiskBytes: number | null;
  snapshotSizeBytes: number | null;
  checkpointSizeBytes: number | null;
  estimatedTemporaryBytes: number | null;
  totalVramBytes: number | null;
  availableVramBytes: number | null;
  riskLevel: 'low' | 'moderate' | 'high' | 'unsupported' | 'unknown' | null;
  reasons: ResourceRiskReason[];
};
export type ManualListeningQualification = {
  syntheticOutputPlayed: boolean;
  speechIntelligible: boolean;
  noSevereClipping: boolean;
  noSevereTruncation: boolean;
  noSourceTargetMixUp: boolean;
  syntheticLabelReviewed: boolean;
  notes: string | null;
  confirmedAt: string | null;
};
export type QualificationRun = {
  schemaVersion: number;
  qualificationId: string;
  compatibilityProfileId: string;
  compatibilityProfileStatus: QualificationSupportStatus;
  startedAt: string;
  endedAt: string | null;
  state: QualificationState;
  completedChecks: QualificationCheck[];
  warnings: string[];
  failures: string[];
  environmentFingerprint: EnvironmentFingerprint | null;
  repository: {
    checkoutLabel: string;
    gitDirectoryPresent: boolean;
    gitAvailable: boolean;
    remoteIdentity: string | null;
    commitSha: string | null;
    detachedHead: boolean | null;
    cleanliness: 'clean' | 'dirty' | 'unknown';
    trackedChanges: number;
    untrackedAdapterFiles: number;
    warnings: string[];
  } | null;
  resources: ResourceDiagnostics | null;
  finalLevel: QualificationLevel;
  manualListening: ManualListeningQualification;
  inferenceSmokeResult: {
    synthetic: boolean;
    outputFile: string;
    durationMs: number;
    peak: number;
    clipping: boolean;
  } | null;
  applicationVersion: string;
  adapterVersion: string;
};

export type ModelBackendSettings = {
  schemaVersion: number;
  seedVc: SeedVcBackendConfiguration | null;
};

export type BackendReadiness =
  | 'notConfigured'
  | 'pythonMissing'
  | 'workerMissing'
  | 'backendMissing'
  | 'checkpointMissing'
  | 'configurationInvalid'
  | 'protocolMismatch'
  | 'unsupportedHardware'
  | 'ready';

export type BackendResourceReport = {
  systemMemoryBytes: number | null;
  gpuMemoryBytes: number | null;
  availableDiskBytes: number | null;
  snapshotSizeBytes: number | null;
  checkpointSizeBytes: number | null;
  riskLevel: string | null;
};

export type BackendCapabilityReport = {
  backendId: string;
  backendVersion: string;
  workerVersion: string;
  protocolVersion: number;
  devices: ModelDevice[];
  precisions: ModelPrecision[];
  supportsResume: boolean;
  supportsMultipleReferences: boolean;
  resources: BackendResourceReport;
  warnings: string[];
};

export type BackendValidationStatus = {
  readiness: BackendReadiness;
  message: string;
  capabilityReport: BackendCapabilityReport | null;
};

export const emptyBackendSettings: ModelBackendSettings = { schemaVersion: 1, seedVc: null };
