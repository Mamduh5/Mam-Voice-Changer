import type {
  EnvironmentFingerprint,
  FileFingerprint,
  ModelDevice,
  ModelPrecision,
  QualificationLevel,
  QualificationSupportStatus,
  ResourceRiskReason,
} from './modelBackend';

export type TrainingPreset = 'quickExperiment' | 'balancedFineTune' | 'extendedFineTune';
export type ResumeBehavior = 'never' | 'fromLatestCheckpoint';
export type TrainingJobState =
  | 'idle'
  | 'validating'
  | 'snapshotting'
  | 'preparing'
  | 'preprocessing'
  | 'training'
  | 'savingCheckpoint'
  | 'evaluatingCheckpoint'
  | 'cancelling'
  | 'cancelled'
  | 'completed'
  | 'failed'
  | 'interrupted'
  | 'needsRecovery';

export type TrainingConfiguration = {
  preset: TrainingPreset;
  maximumSteps: number;
  saveInterval: number;
  batchSize: number;
  workerCount: number;
  device: ModelDevice;
  precision: ModelPrecision;
  resumeBehavior: ResumeBehavior;
  randomSeed: number;
};

export type TrainingMetrics = {
  trainingLoss: number | null;
  validationLoss: number | null;
  learningRate: number | null;
  backendReported: boolean;
  additional: Record<string, number>;
};

export type TrainingJob = {
  schemaVersion: number;
  jobId: string;
  backendId: string;
  backendVersion: string;
  workerProtocolVersion: number;
  compatibilityProfileId: string;
  environmentFingerprint: EnvironmentFingerprint | null;
  checkpointIdentities: FileFingerprint[];
  backendRevision: string | null;
  adapterVersion: string;
  qualificationLevel: QualificationLevel;
  snapshotId: string;
  snapshotHash: string;
  profileId: string;
  consentVersion: string;
  configuration: TrainingConfiguration;
  state: TrainingJobState;
  overallProgress: number;
  currentStep: number;
  maximumSteps: number;
  latestMetrics: TrainingMetrics;
  startedAt: string;
  updatedAt: string;
  completedAt: string | null;
  workerPid: number | null;
  lastCheckpoint: string | null;
  lastCheckpointHash: string | null;
  logFile: string;
  errorSummary: string | null;
  cancellationRequested: boolean;
  warnings: string[];
};

export type TrainingPreflightReport = {
  schemaVersion: number;
  snapshotId: string;
  snapshotTakeCount: number;
  trainingDurationMs: number;
  validationDurationMs: number;
  snapshotBytes: number;
  compatibilityProfileStatus: QualificationSupportStatus;
  environmentFingerprintStatus:
    'identical' | 'compatible' | 'changedWithWarning' | 'incompatible' | 'unknown';
  device: ModelDevice;
  precision: ModelPrecision;
  batchSize: number;
  workerCount: number;
  maximumSteps: number;
  checkpointInterval: number;
  estimatedCheckpointCount: number;
  estimatedDiskMinimumBytes: number;
  estimatedDiskMaximumBytes: number;
  resourceWarnings: ResourceRiskReason[];
  consentActive: boolean;
  qualificationLevel: QualificationLevel;
  fatalFailures: string[];
  acknowledgementsRequired: string[];
  canStart: boolean;
};

export const trainingPresets: Record<TrainingPreset, TrainingConfiguration> = {
  quickExperiment: {
    preset: 'quickExperiment',
    maximumSteps: 100,
    saveInterval: 50,
    batchSize: 1,
    workerCount: 0,
    device: 'cpu',
    precision: 'float32',
    resumeBehavior: 'never',
    randomSeed: 13,
  },
  balancedFineTune: {
    preset: 'balancedFineTune',
    maximumSteps: 1000,
    saveInterval: 250,
    batchSize: 2,
    workerCount: 0,
    device: 'cuda',
    precision: 'float16',
    resumeBehavior: 'fromLatestCheckpoint',
    randomSeed: 13,
  },
  extendedFineTune: {
    preset: 'extendedFineTune',
    maximumSteps: 3000,
    saveInterval: 500,
    batchSize: 2,
    workerCount: 0,
    device: 'cuda',
    precision: 'float16',
    resumeBehavior: 'fromLatestCheckpoint',
    randomSeed: 13,
  },
};
