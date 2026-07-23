import type { PromptCategory, QualityClassification, TakeQualityReport } from './voiceDataset';
import type {
  BackendValidationStatus,
  EnvironmentFingerprint,
  FileFingerprint,
  ModelDevice,
  ModelPrecision,
  QualificationLevel,
  QualificationRun,
} from './modelBackend';
import type { TrainingConfiguration, TrainingJob, TrainingPreflightReport } from './trainingJob';

export type SplitMembership = 'training' | 'validation';
export type SnapshotTake = {
  takeId: string;
  file: string;
  rawContentHash: string;
  selectedContentHash: string;
  selectedVersion: 'raw' | 'trimmed';
  promptId: string | null;
  promptText: string | null;
  promptCategory: PromptCategory | null;
  quality: TakeQualityReport;
  durationMs: number;
  manualOverride: boolean;
  split: SplitMembership;
};
export type TrainingSnapshot = {
  schemaVersion: number;
  snapshotId: string;
  contentHash: string;
  profileId: string;
  datasetSchemaVersion: number;
  consentVersion: string;
  consentConfirmedAt: string;
  promptPackId: string;
  promptPackVersion: number;
  canonicalSampleRate: number;
  canonicalChannels: number;
  totalDurationMs: number;
  takes: SnapshotTake[];
  split: {
    seed: number;
    trainingTakeCount: number;
    validationTakeCount: number;
    trainingDurationMs: number;
    validationDurationMs: number;
  };
  warnings: string[];
  createdAt: string;
};
export type CreateTrainingSnapshotRequest = {
  profileId: string;
  minimumAcceptedDurationMs: number;
  validationPercent: number;
  splitSeed: number;
};

export type ModelApprovalStatus =
  | 'unevaluated'
  | 'evaluationInProgress'
  | 'approvedForOfflineUse'
  | 'rejected'
  | 'disabledByConsent'
  | 'invalid'
  | 'missingFiles';
export type ModelArtifactFile = {
  relativePath: string;
  contentHash: string;
  sizeBytes: number;
  role: 'modelWeights' | 'modelConfiguration' | 'auxiliary' | 'unknown';
  licensingStatus: 'verifiedRedistributable' | 'restricted' | 'unknown';
};
export type ManualModelRatings = {
  intelligibility: number;
  targetSimilarity: number;
  naturalness: number;
  stability: number;
  noiseAndArtifacts: number;
  notes: string | null;
  listeningConfirmed: boolean;
};
export type ModelEvaluationSummary = {
  schemaVersion: number;
  clips: Array<{ phraseId: string; phraseLabel: string; resultId: string; successful: boolean }>;
  ratings: ManualModelRatings;
  completedAt: string;
};
export type EvaluationPhrase = {
  phraseId: string;
  category: string;
  text: string;
};
export type VoiceModelArtifact = {
  schemaVersion: number;
  artifactId: string;
  profileId: string;
  displayName: string;
  backendId: string;
  backendVersion: string;
  workerProtocolVersion: number;
  compatibilityProfileId: string;
  environmentFingerprint: EnvironmentFingerprint | null;
  checkpointIdentities: FileFingerprint[];
  backendRevision: string | null;
  adapterVersion: string;
  snapshotId: string;
  snapshotHash: string;
  consentVersion: string;
  consentConfirmedAt: string;
  trainingConfiguration: TrainingConfiguration;
  trainingSummary: {
    completedSteps: number;
    finalTrainingLoss: number | null;
    finalValidationLoss: number | null;
    checkpointCount: number;
    durationMs: number;
    warnings: string[];
  };
  modelFiles: ModelArtifactFile[];
  modelContentHash: string;
  expectedInferenceSampleRate: number;
  supportedInferenceControls: string[];
  portabilityStatus:
    'localOnly' | 'portableWithExternalDependencies' | 'portable' | 'incompatible' | 'unknown';
  qualificationLevel: QualificationLevel;
  licenseNotices: Array<{
    role: string;
    label: string;
    status: 'verifiedRedistributable' | 'restricted' | 'unknown';
    notice: string;
  }>;
  syntheticUseNoticeVersion: string;
  health:
    | 'healthy'
    | 'unqualified'
    | 'incompatibleEnvironment'
    | 'missingFiles'
    | 'unexpectedFiles'
    | 'hashMismatch'
    | 'disabledByConsent'
    | 'unsupportedBackend'
    | 'unsupportedSchema';
  importedPackageId: string | null;
  evaluation: ModelEvaluationSummary | null;
  approvalStatus: ModelApprovalStatus;
  notes: string | null;
  createdAt: string;
  updatedAt: string;
};

export type InferenceConfiguration = {
  diffusionSteps: number;
  f0Conditioning: boolean;
  pitchAdjustmentSemitones: number;
  lengthAdjustment: number;
  device: ModelDevice;
  precision: ModelPrecision;
  referenceTakeIds: string[];
};
export type OfflineConversionResult = {
  resultId: string;
  artifactId: string;
  artifactDisplayName: string;
  profileId: string;
  targetProfileDisplayName: string;
  sourceClipId: string;
  referenceTakeIds: string[];
  referenceHashes: string[];
  backendId: string;
  backendVersion: string;
  synthetic: true;
  outputFile: string;
  durationMs: number;
  peak: number;
  clipping: boolean;
  waveform: number[];
  createdAt: string;
};
export type VoiceModelError = {
  code: string;
  message: string;
  jobId?: string;
  artifactId?: string;
};
export type VoiceModelStatus = {
  backend: BackendValidationStatus;
  activeTrainingJob: TrainingJob | null;
  activeInference: boolean;
  latestConversion: OfflineConversionResult | null;
  selectedArtifactId: string | null;
  lastError: VoiceModelError | null;
  logs: string[];
  snapshots: TrainingSnapshot[];
  artifacts: VoiceModelArtifact[];
  qualification: QualificationRun | null;
  qualificationActive: boolean;
  trainingPreflight: TrainingPreflightReport | null;
};

export type SnapshotQualitySummary = {
  classification: QualityClassification;
};

export const emptyVoiceModelStatus: VoiceModelStatus = {
  backend: {
    readiness: 'notConfigured',
    message: 'Configure the optional local model backend.',
    capabilityReport: null,
  },
  activeTrainingJob: null,
  activeInference: false,
  latestConversion: null,
  selectedArtifactId: null,
  lastError: null,
  logs: [],
  snapshots: [],
  artifacts: [],
  qualification: null,
  qualificationActive: false,
  trainingPreflight: null,
};
