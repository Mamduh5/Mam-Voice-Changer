import type { VoiceProfileMetadata } from './voiceProfile';

export { VOICE_DATASET_CONSENT_VERSION } from './voiceProfile';
export type {
  CreateVoiceProfileRequest,
  ProfileHealth,
  UpdateVoiceProfileRequest,
  VoiceProfileMetadata,
  VoiceProfileSummary,
} from './voiceProfile';
export type PromptCategory =
  | 'neutralStatement'
  | 'question'
  | 'numbersAndDates'
  | 'namesAndProperNouns'
  | 'plosives'
  | 'sibilants'
  | 'sustainedVowels'
  | 'shortPhrase'
  | 'longPhrase'
  | 'expressiveVariation'
  | 'custom';
export type TakeSource = 'recorded' | 'imported' | 'recordedConsent';
export type TakeReviewStatus = 'pending' | 'accepted' | 'rejected' | 'needsRedo' | 'deleting';
export type SelectedTakeVersion = 'raw' | 'trimmed';
export type QualityClassification = 'pass' | 'warning' | 'fail';

export type VoicePrompt = {
  id: string;
  text: string;
  category: PromptCategory;
  recommendedTakeDurationMs: number | null;
};
export type PromptPack = {
  id: string;
  version: number;
  displayName: string;
  language: string;
  prompts: VoicePrompt[];
};
export type PromptSelection = { promptId: string | null; customPromptText: string | null };

export type QualityReason = { code: string; guidance: string };
export type TakeQualityReport = {
  classification: QualityClassification;
  reasons: QualityReason[];
  durationMs: number;
  peakAmplitude: number;
  rmsLevel: number;
  clippedSampleCount: number;
  clippedSampleRatio: number;
  dcOffset: number;
  leadingSilenceMs: number;
  trailingSilenceMs: number;
  totalLowEnergyRatio: number;
  estimatedActiveSpeechRatio: number;
  estimatedBackgroundNoiseFloor: number;
  heuristicSignalToNoiseDb: number;
  consecutiveZeroRegions: number;
  recordingQueueOverflowCount: number;
  droppedFrames: number;
  callbackGaps: number;
  nonFiniteSampleCountBeforeSanitization: number;
  sampleRate: number;
  channels: number;
};
export type WaveformPoint = { minimum: number; maximum: number };
export type DatasetTake = {
  id: string;
  promptId: string | null;
  promptText: string | null;
  promptCategory: PromptCategory | null;
  source: TakeSource;
  rawFile: string;
  derivedFile: string | null;
  selectedVersion: SelectedTakeVersion;
  sampleRate: number;
  channels: number;
  frameCount: number;
  durationMs: number;
  waveformEnvelope: WaveformPoint[];
  quality: TakeQualityReport;
  trim: {
    startFrame: number;
    endFrame: number;
    derivedQuality: TakeQualityReport;
    derivedWaveformEnvelope: WaveformPoint[];
  } | null;
  reviewStatus: TakeReviewStatus;
  excludeFromTraining: boolean;
  notes: string | null;
  manualOverride: boolean;
  warningAcknowledged: boolean;
  createdAt: string;
  contentHash: string;
};
export type DatasetStatistics = {
  totalTakes: number;
  acceptedTakes: number;
  rejectedTakes: number;
  pendingTakes: number;
  warningTakes: number;
  failedTakes: number;
  acceptedDurationMs: number;
  completedPrompts: number;
  totalPrompts: number;
  categoryCoverage: Partial<Record<PromptCategory, number>>;
  customTakes: number;
  excludedTakes: number;
};
export type VoiceDatasetManifest = {
  schemaVersion: number;
  profile: VoiceProfileMetadata;
  consent: {
    consentConfirmed: boolean;
    consentVersion: string;
    confirmedAt: string;
    confirmedByUser: boolean;
    recordedConsentTakeId: string | null;
    revokedAt: string | null;
    notes: string | null;
  };
  recordingFormat: {
    container: string;
    sampleFormat: string;
    sampleRate: number;
    channels: number;
    bitsPerSample: number;
  };
  promptPack: { id: string; version: number };
  takes: DatasetTake[];
  statistics: DatasetStatistics;
  createdAt: string;
  updatedAt: string;
};
export type DatasetError = { code: string; message: string; profileId?: string; takeId?: string };
export type VoiceDatasetStatus = {
  currentProfileId: string | null;
  currentPromptId: string | null;
  currentPromptText: string | null;
  currentPromptCategory: PromptCategory | null;
  manifest: VoiceDatasetManifest | null;
  recording: {
    active: boolean;
    finalizing: boolean;
    durationMs: number;
    maximumDurationMs: number;
    remainingMs: number;
    inputLevel: number;
    clipping: boolean;
    droppedFrames: number;
  };
  preview: {
    active: boolean;
    paused: boolean;
    takeId: string | null;
    version: SelectedTakeVersion | null;
    positionMs: number;
    durationMs: number;
  };
  lastError: DatasetError | null;
};
export type ReviewTakeRequest = {
  status: TakeReviewStatus;
  excludeFromTraining: boolean;
  notes: string | null;
  warningAcknowledged: boolean;
  selectedVersion: SelectedTakeVersion;
};
export type DatasetExportOptions = {
  includeRejected: boolean;
  includeRawMasters: boolean;
};

export const emptyVoiceDatasetStatus: VoiceDatasetStatus = {
  currentProfileId: null,
  currentPromptId: null,
  currentPromptText: null,
  currentPromptCategory: null,
  manifest: null,
  recording: {
    active: false,
    finalizing: false,
    durationMs: 0,
    maximumDurationMs: 20_000,
    remainingMs: 20_000,
    inputLevel: 0,
    clipping: false,
    droppedFrames: 0,
  },
  preview: {
    active: false,
    paused: false,
    takeId: null,
    version: null,
    positionMs: 0,
    durationMs: 0,
  },
  lastError: null,
};
